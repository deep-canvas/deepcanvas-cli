use colored::Colorize;
use deepcanvas_core::{token, DeepError};

pub fn run(headless: bool) -> Result<(), DeepError> {
    let removed = token::remove()?;

    if headless {
        let payload = serde_json::json!({
            "ok": true,
            "removed": removed,
        });
        println!("{}", payload);
        return Ok(());
    }

    if removed {
        println!(
            "{} Credentials removed from this device.",
            "✓".green().bold()
        );
    } else {
        println!("No credentials stored on this device.");
    }
    Ok(())
}
