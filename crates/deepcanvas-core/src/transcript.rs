use crate::error::DeepError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// "/Users/uygun/work/foo" → "-Users-uygun-work-foo"
pub fn encode_cwd(cwd: &Path) -> String {
    cwd.to_string_lossy().replace(['/', '\\'], "-")
}

/// ~/.claude/projects/<encoded>
pub fn default_transcript_dir(cwd: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let encoded = encode_cwd(cwd);
    Some(
        PathBuf::from(home)
            .join(".claude")
            .join("projects")
            .join(encoded),
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskState {
    pub started_at_ms: i64,
    pub transcript_dir: PathBuf,
}

impl TaskState {
    pub fn write(task_dir: &Path, state: &TaskState) -> Result<(), DeepError> {
        std::fs::create_dir_all(task_dir)?;
        let path = task_dir.join(".state.json");
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read(task_dir: &Path) -> Result<Option<TaskState>, DeepError> {
        let path = task_dir.join(".state.json");
        if !path.is_file() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&raw)?))
    }
}

#[derive(Debug, Default)]
pub struct TranscriptAggregate {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub message_count: u32,
    pub model_ids: Vec<String>,
}

impl TranscriptAggregate {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_write_tokens
    }
}

#[derive(Deserialize)]
struct TranscriptEntry {
    #[serde(default, rename = "type")]
    entry_type: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<InnerMessage>,
}

#[derive(Deserialize)]
struct InnerMessage {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

fn parse_iso_to_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Scan all JSONL files in transcript_dir, sum tokens for assistant messages
/// whose timestamp falls within [started_at_ms, ended_at_ms].
/// Returns None if dir doesn't exist.
pub fn aggregate_transcripts(
    transcript_dir: &Path,
    started_at_ms: i64,
    ended_at_ms: i64,
) -> Result<Option<TranscriptAggregate>, DeepError> {
    if !transcript_dir.is_dir() {
        return Ok(None);
    }

    let mut agg = TranscriptAggregate::default();
    let mut models: std::collections::BTreeSet<String> = Default::default();

    for entry in std::fs::read_dir(transcript_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let entry: TranscriptEntry = match serde_json::from_str(line) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if entry.entry_type.as_deref() != Some("assistant") {
                continue;
            }

            let ts_ms = match entry.timestamp.as_deref().and_then(parse_iso_to_ms) {
                Some(ms) => ms,
                None => continue,
            };
            if ts_ms < started_at_ms || ts_ms > ended_at_ms {
                continue;
            }

            let inner = match &entry.message {
                Some(m) => m,
                None => continue,
            };

            if let Some(u) = &inner.usage {
                agg.input_tokens += u.input_tokens;
                agg.output_tokens += u.output_tokens;
                agg.cache_write_tokens += u.cache_creation_input_tokens;
                agg.cache_read_tokens += u.cache_read_input_tokens;
            }
            if let Some(model) = &inner.model {
                if !model.is_empty() {
                    models.insert(model.clone());
                }
            }
            agg.message_count += 1;
        }
    }

    agg.model_ids = models.into_iter().collect();
    Ok(Some(agg))
}
