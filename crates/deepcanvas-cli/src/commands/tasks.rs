use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use deepcanvas_core::{token, types::TasksResponse, ApiClient, Config, DeepError, ProjectBinding};
use dialoguer::Input;
use std::io::IsTerminal;

pub async fn run(
    config: Config,
    project_flag: Option<String>,
    headless: bool,
) -> Result<(), DeepError> {
    let project = resolve_project(project_flag.clone())?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config.clone()).with_token(token);

    let path = format!(
        "/cli/tasks?org={}&project={}",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TasksResponse = client.get(&path).await?;

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "project": {
                "slug": response.project.slug,
                "name": response.project.name,
                "organization_slug": response.project.organization_slug,
            },
            "tasks": response.tasks.iter().map(|t| serde_json::json!({
                "code": t.code,
                "title": t.title,
                "status": t.status,
                "energy": t.energy,
                "priority": t.priority,
                "primary_document_code": t.primary_document.as_ref().map(|d| &d.code),
                "updated_at": t.updated_at,
            })).collect::<Vec<_>>(),
            "count": response.count,
        });
        println!("{}", payload);
        return Ok(());
    }

    if response.tasks.is_empty() {
        println!(
            "No assigned tasks in {} / {}.",
            project.organization_slug, project.project_slug
        );
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["#", "CODE", "TITLE", "ENERGY", "STATUS", "PRD"]);

    for (i, t) in response.tasks.iter().enumerate() {
        table.add_row(vec![
            (i + 1).to_string(),
            t.code.clone(),
            truncate(&t.title, 60),
            t.energy.clone().unwrap_or_else(|| "—".into()),
            humanize_status(&t.status),
            t.primary_document
                .as_ref()
                .map(|d| d.code.clone())
                .unwrap_or_else(|| "—".into()),
        ]);
    }

    println!();
    println!("Project: {}", response.project.name.bold());
    println!();
    println!("{table}");
    println!();
    println!(
        "{} task(s). Run `deep pull <code>` to fetch context.",
        response.count
    );

    if headless || !std::io::stdin().is_terminal() {
        return Ok(());
    }

    println!();
    let answer: String = Input::new()
        .with_prompt(format!(
            "Which task do you want to start? (1-{}, Enter to skip)",
            response.tasks.len()
        ))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| {
            DeepError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                e.to_string(),
            ))
        })?;

    let trimmed = answer.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let n: usize = trimmed.parse().map_err(|_| {
        DeepError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("not a number: {}", trimmed),
        ))
    })?;
    if n == 0 || n > response.tasks.len() {
        return Err(DeepError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "number out of range: {} (expected 1-{})",
                n,
                response.tasks.len()
            ),
        )));
    }

    let code = response.tasks[n - 1].code.clone();
    super::pull::run(config, vec![code], project_flag, false).await
}

pub fn resolve_project(flag: Option<String>) -> Result<ProjectBinding, DeepError> {
    if let Some(s) = flag {
        return ProjectBinding::from_flag(&s);
    }
    let cwd = std::env::current_dir()?;
    ProjectBinding::find_from(&cwd)?
        .map(|(b, _)| b)
        .ok_or(DeepError::NoProjectBinding)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n - 1).collect();
    out.push('…');
    out
}

fn humanize_status(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = true;
    for c in s.chars() {
        if c == '_' || c == '-' {
            out.push(' ');
            next_upper = true;
        } else if next_upper {
            out.extend(c.to_uppercase());
            next_upper = false;
        } else {
            out.push(c);
        }
    }
    out
}
