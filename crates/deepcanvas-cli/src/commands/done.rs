use super::tasks::resolve_project;
use chrono::Utc;
use colored::Colorize;
use deepcanvas_core::{
    active_task, token,
    transcript::{aggregate_transcripts, TaskState},
    types::{AgentSessionDto, CompleteRequest, TaskCompleteResponse},
    ApiClient, Config, DeepError,
};

const AGENT_CODE: &str = "claude-code";

pub async fn run(
    config: Config,
    task_code: Option<String>,
    headless: bool,
    verbose: bool,
) -> Result<(), DeepError> {
    let (project, root) = resolve_project(None)?;

    let code = match task_code {
        Some(c) => c.to_uppercase(),
        None => active_task::read(&root)?
            .ok_or(DeepError::NoActiveTask)?
            .to_uppercase(),
    };

    if !is_valid_task_code(&code) {
        return Err(DeepError::InvalidTaskCode(code));
    }

    let task_dir = root.join(".deep").join(&code);

    if verbose {
        eprintln!("[verbose] project root: {}", root.display());
        eprintln!("[verbose] code: {}", code);
        eprintln!("[verbose] task_dir: {}", task_dir.display());
        let state_path = task_dir.join(".state.json");
        eprintln!(
            "[verbose] state file: {} (exists: {})",
            state_path.display(),
            state_path.exists()
        );
        if state_path.exists() {
            if let Ok(s) = std::fs::read_to_string(&state_path) {
                eprintln!("[verbose] state content:\n{}", s);
            }
        }
    }

    let agent_session = build_agent_session(&task_dir, &root, verbose);

    if verbose {
        match &agent_session {
            Some(s) => eprintln!(
                "[verbose] agent_session built: tokens={}, duration={}s",
                s.tokens_used, s.duration_seconds
            ),
            None => eprintln!("[verbose] agent_session: NULL (state or transcripts missing)"),
        }
    }

    let token_val = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token_val);

    let path = format!(
        "/cli/tasks/{}/complete?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );

    let body = CompleteRequest { agent_session };

    if verbose {
        eprintln!("[verbose] POST {}", path);
        eprintln!(
            "[verbose] body:\n{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    }

    let response: TaskCompleteResponse = match client.post(&path, &body).await {
        Ok(r) => r,
        Err(DeepError::Api {
            status: 409,
            message,
        }) => {
            // Stale active marker pointed to an already-completed task.
            // Clean up locally so the next `deep done` doesn't loop on the same mistake.
            let _ = active_task::clear(&root);
            let _ = std::fs::remove_file(task_dir.join(".state.json"));
            if verbose {
                eprintln!(
                    "[verbose] 409 detected — cleared stale active marker for {}",
                    code
                );
            }
            return Err(DeepError::Api {
                status: 409,
                message: format!(
                    "{} ({} was already done — cleared local active marker; run `deep pull <code>` for your next task)",
                    message, code
                ),
            });
        }
        Err(e) => return Err(e),
    };

    if verbose {
        eprintln!(
            "[verbose] response: usage_recorded={}, status={}",
            response.usage_recorded, response.task.status
        );
    }

    // API succeeded — task is done. Cleanup local markers, then report.
    let mut active_cleared = false;
    if matches!(
        active_task::read(&root).ok().flatten(),
        Some(ref a) if a == &code
    ) && active_task::clear(&root).is_ok()
    {
        active_cleared = true;
    }
    let _ = std::fs::remove_file(task_dir.join(".state.json"));

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "task": {
                "code": response.task.code,
                "title": response.task.title,
                "status": response.task.status,
                "completed_at": response.task.completed_at,
            },
            "active_cleared": active_cleared,
            "usage_recorded": response.usage_recorded,
        });
        println!("{}", payload);
    } else {
        println!();
        println!(
            "{} Completed {}: {}",
            "✓".green().bold(),
            response.task.code.bold(),
            response.task.title
        );
        if response.usage_recorded {
            println!("  {}", "Agent session logged.".dimmed());
        }
    }

    Ok(())
}

fn build_agent_session(
    task_dir: &std::path::Path,
    root: &std::path::Path,
    verbose: bool,
) -> Option<AgentSessionDto> {
    let state = match TaskState::read(task_dir).ok().flatten() {
        Some(s) => s,
        None => {
            if verbose {
                eprintln!("[verbose] no .state.json or unreadable → agent_session NULL");
            }
            return None;
        }
    };

    if verbose {
        eprintln!(
            "[verbose] transcript_dir: {} (exists: {})",
            state.transcript_dir.display(),
            state.transcript_dir.exists()
        );
        eprintln!("[verbose] started_at_ms: {}", state.started_at_ms);
    }

    let ended_at_ms = Utc::now().timestamp_millis();
    let agg = match aggregate_transcripts(&state.transcript_dir, state.started_at_ms, ended_at_ms)
        .ok()
        .flatten()
    {
        Some(a) => a,
        None => {
            if verbose {
                eprintln!("[verbose] aggregate returned None → agent_session NULL");
            }
            return None;
        }
    };

    if verbose {
        eprintln!(
            "[verbose] aggregated: messages={}, input={}, output={}, cache_read={}, cache_write={}, models={:?}",
            agg.message_count,
            agg.input_tokens,
            agg.output_tokens,
            agg.cache_read_tokens,
            agg.cache_write_tokens,
            agg.model_ids
        );
    }

    let duration_ms = (ended_at_ms - state.started_at_ms).max(0) as u64;
    let duration_seconds = duration_ms / 1000;

    let local_repo = root.to_str().map(|s| s.to_string());

    let metadata = serde_json::json!({
        "input_tokens": agg.input_tokens,
        "output_tokens": agg.output_tokens,
        "cache_read_tokens": agg.cache_read_tokens,
        "cache_write_tokens": agg.cache_write_tokens,
        "message_count": agg.message_count,
        "model_ids": agg.model_ids,
        "started_at_ms": state.started_at_ms,
        "ended_at_ms": ended_at_ms,
    });

    Some(AgentSessionDto {
        agent_code: std::env::var("DEEPCANVAS_AGENT_CODE")
            .unwrap_or_else(|_| AGENT_CODE.to_string()),
        local_repo,
        duration_seconds,
        tokens_used: agg.total_tokens(),
        metadata,
    })
}

fn is_valid_task_code(code: &str) -> bool {
    let mut parts = code.splitn(2, '-');
    let prefix = parts.next().unwrap_or("");
    let number = parts.next().unwrap_or("");
    !prefix.is_empty()
        && !number.is_empty()
        && prefix.chars().all(|c| c.is_ascii_uppercase())
        && number.chars().all(|c| c.is_ascii_digit())
}
