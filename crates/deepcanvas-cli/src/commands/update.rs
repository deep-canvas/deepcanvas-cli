use colored::Colorize;
use deepcanvas_core::DeepError;

const REPO_OWNER: &str = "deep-canvas";
const REPO_NAME: &str = "deepcanvas-cli";

pub async fn run(headless: bool) -> Result<(), DeepError> {
    tokio::task::spawn_blocking(move || {
        check_homebrew(headless)?;
        do_update(headless)
    })
    .await
    .map_err(|e| DeepError::Update(format!("task join error: {}", e)))?
}

fn check_homebrew(headless: bool) -> Result<(), DeepError> {
    let exe = std::env::current_exe().map_err(|e| DeepError::Update(e.to_string()))?;
    let path = exe.to_string_lossy();
    if path.contains("/Cellar/") || path.contains("/opt/homebrew/") || path.contains("/linuxbrew/")
    {
        if !headless {
            eprintln!("{}", "Detected Homebrew installation.".yellow());
            eprintln!("Use `brew upgrade deep` instead.");
        }
        return Err(DeepError::Update("homebrew-managed binary".into()));
    }
    Ok(())
}

fn do_update(headless: bool) -> Result<(), DeepError> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("deep")
        .show_download_progress(!headless)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()
        .map_err(|e| DeepError::Update(e.to_string()))?
        .update()
        .map_err(|e| DeepError::Update(e.to_string()))?;

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "updated": status.updated(),
            "version": status.version(),
        });
        println!("{}", payload);
        return Ok(());
    }

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
