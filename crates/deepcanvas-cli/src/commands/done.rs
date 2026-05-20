use super::tasks::resolve_project;
use colored::Colorize;
use deepcanvas_core::{
    active_task, token, types::TaskCompleteResponse, ApiClient, Config, DeepError,
};

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

    let project = resolve_project(None)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let path = format!(
        "/cli/tasks/{}/complete?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );

    let response: TaskCompleteResponse = client.post(&path, &serde_json::json!({})).await?;

    let mut active_cleared = false;
    if let Ok(Some(active)) = active_task::read(&cwd) {
        if active == code {
            active_task::clear(&cwd)?;
            active_cleared = true;
        }
    }

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
        });
        println!("{}", payload);
        return Ok(());
    }

    println!();
    println!(
        "{} Completed {}: {}",
        "✓".green().bold(),
        response.task.code.bold(),
        response.task.title
    );
    Ok(())
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
