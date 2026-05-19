use colored::Colorize;
use deepcanvas_core::{
    token,
    types::{AuthStartRequest, AuthStartResponse, ClientInfo, PollResponse},
    ApiClient, Config, DeepError, TokenLocation,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn run(config: Config) -> Result<(), DeepError> {
    let client = ApiClient::new(config.clone());
    let req = AuthStartRequest {
        client_info: collect_client_info(),
    };
    let session: AuthStartResponse = client.post("/cli/auth", &req).await?;

    println!();
    println!("Opening browser for authorization...");
    println!("  → {}", session.verify_url.cyan());
    println!();
    println!("If the browser doesn't open, visit the URL above.");
    println!(
        "Verify the code matches: {}",
        session.user_code_display.bold()
    );
    println!();

    let _ = webbrowser::open(&session.verify_url);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap());
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Waiting for approval...");

    let access_token = poll(&client, &session.device_token).await?;
    spinner.finish_and_clear();

    let location = token::store(&access_token)?;
    println!("{} Authorized", "✓".green().bold());
    match location {
        TokenLocation::Keyring => println!("  Token saved to system keychain."),
        TokenLocation::File => println!(
            "  {}",
            "Token saved to local file (keyring unavailable).".yellow()
        ),
    }
    println!();

    let cwd = std::env::current_dir()?;
    let already_init = cwd.join(".deep").join("config.toml").exists();
    if already_init {
        println!("This directory is already linked to a project.");
        return Ok(());
    }

    use dialoguer::Confirm;
    let do_init = Confirm::new()
        .with_prompt("Link this directory to a project now?")
        .default(true)
        .interact_opt()
        .map_err(|e| {
            DeepError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                e.to_string(),
            ))
        })?;

    match do_init {
        Some(true) => super::init::run_with_token(config, None, access_token).await,
        Some(false) => {
            println!();
            println!("Run `deep init` later when you're in a project directory.");
            Ok(())
        }
        None => {
            println!("Run `deep init <org>/<project>` to link a directory.");
            Ok(())
        }
    }
}

async fn poll(client: &ApiClient, device_token: &str) -> Result<String, DeepError> {
    let path = format!("/cli/auth/poll?device_token={}&wait=25", device_token);
    loop {
        match client.get_long_poll::<PollResponse>(&path, 30).await {
            Ok(PollResponse::Pending) => continue,
            Ok(PollResponse::Approved { access_token }) => return Ok(access_token),
            Ok(PollResponse::Denied) => return Err(DeepError::SessionDenied),
            Ok(PollResponse::Expired) => return Err(DeepError::SessionExpired),
            Err(DeepError::Network(e)) if e.is_timeout() => continue,
            Err(e) => return Err(e),
        }
    }
}

fn collect_client_info() -> ClientInfo {
    ClientInfo {
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into()),
        os: std::env::consts::OS.to_string(),
        os_version: os_info::get().version().to_string(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
