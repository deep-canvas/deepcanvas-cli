use super::tasks::resolve_project;
use chrono::Utc;
use colored::Colorize;
use deepcanvas_core::{
    active_task, token,
    transcript::{aggregate_transcripts, TaskState},
    types::{AgentSessionDto, CompleteRequest, TaskCompleteResponse},
    ApiClient, Config, DeepError,
};
use std::path::PathBuf;

const AGENT_CODE: &str = "claude-code";

pub async fn run(
    config: Config,
    task_code: Option<String>,
    headless: bool,
) -> Result<(), DeepError> {
    let cwd = std::env::current_dir()?;

    let code = match task_code {
        Some(c) => c.to_uppercase(),
        None => active_task::read(&cwd)?
            .ok_or(DeepError::NoActiveTask)?
            .to_uppercase(),
    };

    if !is_valid_task_code(&code) {
        return Err(DeepError::InvalidTaskCode(code));
    }

    let task_dir = PathBuf::from(".deep").join(&code);

    let agent_session = build_agent_session(&task_dir, &cwd);

    let project = resolve_project(None)?;
    let token_val = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token_val);

    let path = format!(
        "/cli/tasks/{}/complete?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );

    let body = CompleteRequest { agent_session };
    let response: TaskCompleteResponse = client.post(&path, &body).await?;

    let will_clear_active = matches!(
        active_task::read(&cwd).ok().flatten(),
        Some(ref a) if a == &code
    );

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "task": {
                "code": response.task.code,
                "title": response.task.title,
                "status": response.task.status,
                "completed_at": response.task.completed_at,
            },
            "active_cleared": will_clear_active,
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

    if will_clear_active {
        let _ = active_task::clear(&cwd);
    }
    let _ = std::fs::remove_file(task_dir.join(".state.json"));

    Ok(())
}

fn build_agent_session(
    task_dir: &std::path::Path,
    cwd: &std::path::Path,
) -> Option<AgentSessionDto> {
    let state = TaskState::read(task_dir).ok().flatten()?;

    let ended_at_ms = Utc::now().timestamp_millis();
    let agg = aggregate_transcripts(&state.transcript_dir, state.started_at_ms, ended_at_ms)
        .ok()
        .flatten()?;

    let duration_ms = (ended_at_ms - state.started_at_ms).max(0) as u64;
    let duration_seconds = duration_ms / 1000;

    let local_repo = cwd.to_str().map(|s| s.to_string());

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
