use colored::Colorize;
use deepcanvas_core::DeepError;

const REPO_OWNER: &str = "deepcanvas-studio";
const REPO_NAME: &str = "deepcanvas-cli";

pub async fn run() -> Result<(), DeepError> {
    tokio::task::spawn_blocking(|| {
        check_homebrew()?;
        do_update()
    })
    .await
    .map_err(|e| DeepError::Update(format!("task join error: {}", e)))?
}

fn check_homebrew() -> Result<(), DeepError> {
    let exe = std::env::current_exe().map_err(|e| DeepError::Update(e.to_string()))?;
    let path = exe.to_string_lossy();
    if path.contains("/Cellar/") || path.contains("/opt/homebrew/") || path.contains("/linuxbrew/")
    {
        eprintln!("{}", "Detected Homebrew installation.".yellow());
        eprintln!("Use `brew upgrade deep` instead.");
        return Err(DeepError::Update("homebrew-managed binary".into()));
    }
    Ok(())
}

fn do_update() -> Result<(), DeepError> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("deep")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()
        .map_err(|e| DeepError::Update(e.to_string()))?
        .update()
        .map_err(|e| DeepError::Update(e.to_string()))?;

    if status.updated() {
        println!(
            "{} Updated to {}",
            "✓".green().bold(),
            status.version().bold()
        );
    } else {
        println!("Already on latest version ({}).", status.version());
    }
    Ok(())
}
