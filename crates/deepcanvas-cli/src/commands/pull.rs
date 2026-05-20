use super::tasks::resolve_project;
use chrono::Utc;
use colored::Colorize;
use deepcanvas_core::{
    active_task, token,
    transcript::{default_transcript_dir, TaskState},
    types::{TaskContextResponse, TaskDetail, TaskDocuments},
    ApiClient, Config, DeepError, ProjectBinding,
};

pub async fn run(
    config: Config,
    task_codes: Vec<String>,
    project_flag: Option<String>,
    headless: bool,
) -> Result<(), DeepError> {
    let normalized: Vec<String> = task_codes.into_iter().map(|c| c.to_uppercase()).collect();
    for code in &normalized {
        if !is_valid_task_code(code) {
            return Err(DeepError::InvalidTaskCode(code.clone()));
        }
    }

    let (project, root) = resolve_project(project_flag)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let mut results: Vec<serde_json::Value> = Vec::new();
    for code in &normalized {
        let summary = pull_one(&client, &project, &root, code, headless).await?;
        results.push(summary);
    }

    if headless {
        let active = normalized.last().cloned();
        let payload = serde_json::json!({
            "ok": true,
            "results": results,
            "active_task": active,
        });
        println!("{}", payload);
        return Ok(());
    }

    if normalized.len() > 1 {
        println!();
        println!("{} Pulled {} tasks.", "✓".green().bold(), normalized.len());
    }
    Ok(())
}

async fn pull_one(
    client: &ApiClient,
    project: &ProjectBinding,
    root: &std::path::Path,
    code: &str,
    headless: bool,
) -> Result<serde_json::Value, DeepError> {
    let path = format!(
        "/cli/tasks/{}/context?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TaskContextResponse = client.get(&path).await?;

    let task_dir = root.join(".deep").join(code);
    std::fs::create_dir_all(&task_dir)?;

    let task_md_path = task_dir.join("task.md");
    std::fs::write(
        &task_md_path,
        format_task_md(&response.task, &response.documents),
    )?;

    let mut written: Vec<(String, bool)> = vec![];
    if let Some(doc) = &response.documents.primary {
        std::fs::write(
            task_dir.join(format!("{}.md", doc.code)),
            &doc.content_markdown,
        )?;
        written.push((doc.code.clone(), true));
    }
    for doc in &response.documents.related {
        std::fs::write(
            task_dir.join(format!("{}.md", doc.code)),
            &doc.content_markdown,
        )?;
        written.push((doc.code.clone(), false));
    }

    active_task::write(root, code)?;

    if let Some(transcript_dir) = default_transcript_dir(root) {
        let state = TaskState {
            started_at_ms: Utc::now().timestamp_millis(),
            transcript_dir,
        };
        let _ = TaskState::write(&task_dir, &state);
    }

    if headless {
        let documents: Vec<serde_json::Value> = written
            .iter()
            .map(|(c, primary)| {
                serde_json::json!({
                    "code": c,
                    "path": task_dir.join(format!("{}.md", c)).display().to_string(),
                    "primary": primary,
                })
            })
            .collect();
        let acceptance: Vec<serde_json::Value> = response
            .task
            .acceptance_criteria
            .iter()
            .map(|ac| serde_json::json!({ "text": ac.text, "is_done": ac.is_done }))
            .collect();
        return Ok(serde_json::json!({
            "task": {
                "code": response.task.code,
                "title": response.task.title,
                "description": response.task.description,
                "status": response.task.status,
                "energy": response.task.energy,
                "priority": response.task.priority,
                "acceptance_criteria": acceptance,
            },
            "files": {
                "task_md": task_md_path.display().to_string(),
                "documents": documents,
            },
        }));
    }

    println!();
    println!(
        "{} Fetched {}: {}",
        "✓".green().bold(),
        response.task.code.bold(),
        response.task.title
    );
    println!("  {} {}", "→".dimmed(), task_md_path.display());
    for (c, _) in &written {
        println!("  {} {}/{}.md", "→".dimmed(), task_dir.display(), c);
    }

    Ok(serde_json::Value::Null)
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

fn format_task_md(task: &TaskDetail, docs: &TaskDocuments) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}: {}\n\n", task.code, task.title));
    out.push_str(&format!("**Status:** {}  \n", task.status));
    if let Some(p) = &task.parent_code {
        out.push_str(&format!("**Parent:** {}  \n", p));
    }
    if let Some(e) = &task.energy {
        out.push_str(&format!("**Energy:** {}  \n", e));
    }
    if let Some(p) = &task.priority {
        out.push_str(&format!("**Priority:** {}  \n", p));
    }
    if let Some(a) = &task.assignee {
        let name = a.name.clone().unwrap_or_else(|| a.email.clone());
        out.push_str(&format!("**Assignee:** {} <{}>  \n", name, a.email));
    }
    out.push('\n');

    if let Some(d) = &task.description {
        out.push_str("## Description\n\n");
        out.push_str(d.trim());
        out.push_str("\n\n");
    }
    if !task.acceptance_criteria.is_empty() {
        out.push_str("## Acceptance Criteria\n\n");
        for ac in &task.acceptance_criteria {
            out.push_str(&format!(
                "- [{}] {}\n",
                if ac.is_done { "x" } else { " " },
                ac.text
            ));
        }
        out.push('\n');
    }

    let mut linked = Vec::new();
    if let Some(p) = &docs.primary {
        linked.push((p.code.clone(), p.title.clone(), true));
    }
    for r in &docs.related {
        linked.push((r.code.clone(), r.title.clone(), false));
    }
    if !linked.is_empty() {
        out.push_str("## Linked Documents\n\n");
        for (code, title, primary) in linked {
            let suffix = if primary { " (primary)" } else { "" };
            out.push_str(&format!(
                "- [{}](./{}.md) — {}{}\n",
                code, code, title, suffix
            ));
        }
    }
    out
}
