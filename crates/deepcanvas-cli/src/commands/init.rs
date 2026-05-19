use colored::Colorize;
use deepcanvas_core::{token, types::TasksResponse, ApiClient, Config, DeepError, ProjectBinding};

pub async fn run(config: Config, slug_pair: String) -> Result<(), DeepError> {
    let project = ProjectBinding::from_flag(&slug_pair)?;

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".deep").join("config.toml");
    if config_path.exists() {
        return Err(DeepError::AlreadyInitialized(
            config_path.display().to_string(),
        ));
    }

    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let path = format!(
        "/cli/tasks?org={}&project={}&limit=1",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TasksResponse = client.get(&path).await?;

    std::fs::create_dir_all(cwd.join(".deep"))?;
    std::fs::write(&config_path, project.to_toml())?;
    update_gitignore(&cwd)?;

    println!();
    println!(
        "{} Linked to: {} / {}",
        "✓".green().bold(),
        project.organization_slug,
        response.project.name.bold()
    );
    println!("  Wrote {}", config_path.display().to_string().cyan());
    println!("  Updated .gitignore");
    println!();
    println!("Try: deep tasks");
    Ok(())
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
        new_content.push_str(marker);
        new_content.push('\n');
        std::fs::write(&gitignore, new_content)?;
    } else {
        std::fs::write(
            &gitignore,
            format!("# Deep CLI task contexts\n{}\n", marker),
        )?;
    }
    Ok(())
}
