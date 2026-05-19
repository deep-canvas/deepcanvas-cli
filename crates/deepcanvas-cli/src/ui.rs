use colored::Colorize;
use deepcanvas_core::DeepError;

pub fn print_error(e: &DeepError) {
    eprintln!();
    eprintln!("{} {}", "error:".red().bold(), e);
    match e {
        DeepError::NotAuthenticated => {
            eprintln!("  {}: deep login", "Run".dimmed());
        }
        DeepError::Unauthorized => {
            eprintln!("  Your token may be revoked.");
            eprintln!("  {}: deep login", "Run".dimmed());
        }
        DeepError::NoProjectBinding => {
            eprintln!("  Use: {}", "deep init <org>/<project>".cyan());
            eprintln!("  Or:  {}", "deep tasks --project <org>/<project>".cyan());
        }
        DeepError::AlreadyInitialized(path) => {
            eprintln!("  Remove the file first: {}", path.cyan());
        }
        DeepError::SessionExpired => {
            eprintln!("  Authorization code expired.");
            eprintln!("  {}: deep login", "Run".dimmed());
        }
        DeepError::SessionDenied => {
            eprintln!("  Authorization was denied in the browser.");
        }
        DeepError::InvalidTaskCode(c) => {
            eprintln!("  Got: {}. Expected format: DC-142, ENG-7", c.cyan());
        }
        _ => {}
    }
    eprintln!();
}
