use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use deepcanvas_core::{token, types::TasksResponse, ApiClient, Config, DeepError, ProjectBinding};

pub async fn run(config: Config, project_flag: Option<String>) -> Result<(), DeepError> {
    let project = resolve_project(project_flag)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let path = format!(
        "/cli/tasks?org={}&project={}",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TasksResponse = client.get(&path).await?;

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
        .set_header(vec!["CODE", "TITLE", "ENERGY", "STATUS", "PRD"]);

    for t in &response.tasks {
        table.add_row(vec![
            t.code.clone(),
            truncate(&t.title, 60),
            t.energy.clone().unwrap_or_else(|| "—".into()),
            t.status.clone(),
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
    Ok(())
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
