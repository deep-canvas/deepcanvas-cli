use colored::Colorize;
use deepcanvas_core::{token, DeepError};

pub fn run() -> Result<(), DeepError> {
    let removed = token::remove()?;
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
