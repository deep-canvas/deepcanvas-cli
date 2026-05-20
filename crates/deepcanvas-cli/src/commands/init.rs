use colored::Colorize;
use deepcanvas_core::{
    token,
    types::{ProjectListEntry, ProjectListResponse, TasksResponse},
    ApiClient, Config, DeepError, ProjectBinding,
};
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn run(
    config: Config,
    slug_pair: Option<String>,
    headless: bool,
) -> Result<(), DeepError> {
    if headless && slug_pair.is_none() {
        return Err(DeepError::HeadlessUnavailable);
    }
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    run_with_token(config, slug_pair, token, headless).await
}

pub async fn run_with_token(
    config: Config,
    slug_pair: Option<String>,
    token: String,
    headless: bool,
) -> Result<(), DeepError> {
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".deep").join("config.toml");
    if config_path.exists() {
        return Err(DeepError::AlreadyInitialized(
            config_path.display().to_string(),
        ));
    }

    let client = ApiClient::new(config).with_token(token);

    let project = match slug_pair {
        Some(s) => resolve_from_slug(&client, &s).await?,
        None => resolve_interactive(&client).await?,
    };

    std::fs::create_dir_all(cwd.join(".deep"))?;
    std::fs::write(&config_path, project.to_toml())?;
    update_gitignore(&cwd)?;

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "linked": {
                "organization_slug": project.organization_slug,
                "project_slug": project.project_slug,
            },
            "config_path": config_path.display().to_string(),
        });
        println!("{}", payload);
        return Ok(());
    }

    println!();
    println!(
        "{} Linked to: {} / {}",
        "✓".green().bold(),
        project.organization_slug,
        project.project_slug.bold()
    );
    println!("  Wrote {}", config_path.display().to_string().cyan());
    println!("  Updated .gitignore");
    println!();
    println!("Try: deep tasks");
    Ok(())
}

async fn resolve_from_slug(client: &ApiClient, s: &str) -> Result<ProjectBinding, DeepError> {
    let project = ProjectBinding::from_flag(s)?;
    let path = format!(
        "/cli/tasks?org={}&project={}&limit=1",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let _: TasksResponse = client.get(&path).await?;
    Ok(project)
}

async fn resolve_interactive(client: &ApiClient) -> Result<ProjectBinding, DeepError> {
    let list: ProjectListResponse = client.get("/cli/projects").await?;

    if list.projects.is_empty() {
        println!("{}", "You don't have access to any projects yet.".yellow());
        println!("Ask a project manager to add you, then run `deep init` again.");
        std::process::exit(0);
    }

    let items: Vec<String> = list
        .projects
        .iter()
        .map(|p| format!("{} / {}  ({})", p.organization_name, p.project_name, p.role))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a project to link this directory to")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|e| {
            DeepError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                e.to_string(),
            ))
        })?;

    let idx = selection.ok_or_else(|| {
        DeepError::Io(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "selection cancelled",
        ))
    })?;

    let chosen: &ProjectListEntry = &list.projects[idx];
    Ok(ProjectBinding {
        organization_slug: chosen.organization_slug.clone(),
        project_slug: chosen.project_slug.clone(),
    })
}

fn update_gitignore(cwd: &std::path::Path) -> Result<(), DeepError> {
    let gitignore = cwd.join(".gitignore");
    let marker = ".deep/*/";
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore)?;
        if content.lines().any(|l| l.trim() == marker) {
            return Ok(());
        }
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n# Deep CLI task contexts\n");
        new_content.push_str(".deep/*/\n");
        new_content.push_str(".deep/active\n");
        std::fs::write(&gitignore, new_content)?;
    } else {
        std::fs::write(
            &gitignore,
            "# Deep CLI task contexts\n.deep/*/\n.deep/active\n",
        )?;
    }
    Ok(())
}
