use colored::Colorize;
use deepcanvas_core::{token, DeepError};

pub fn run(scope: Option<String>, headless: bool) -> Result<(), DeepError> {
    let local_only = scope.as_deref() == Some(".");
    let cwd = std::env::current_dir()?;

    let local_removed = token::remove_local(&cwd)?;
    let global_removed = if local_only { false } else { token::remove()? };
    let removed = local_removed || global_removed;

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "removed": removed,
            "local_removed": local_removed,
            "global_removed": global_removed,
        });
        println!("{}", payload);
        return Ok(());
    }

    if !removed {
        println!("No credentials stored on this device.");
        return Ok(());
    }

    println!("{} Credentials removed.", "✓".green().bold());
    if local_removed {
        println!("  Project-local token removed.");
    }
    if global_removed {
        println!("  Global token removed.");
    }
    Ok(())
}
