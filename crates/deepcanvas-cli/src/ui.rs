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
        DeepError::NoActiveTask => {
            eprintln!("  Pull a task first: {}", "deep pull <code>".cyan());
            eprintln!("  Or pass a code:    {}", "deep done <code>".cyan());
        }
        _ => {}
    }
    eprintln!();
}

pub fn print_error_json(e: &DeepError) {
    let kind = match e {
        DeepError::Network(_) => "Network",
        DeepError::Keyring(_) => "Keyring",
        DeepError::Io(_) => "Io",
        DeepError::Serde(_) => "Serde",
        DeepError::TomlParse(_) => "TomlParse",
        DeepError::NotAuthenticated => "NotAuthenticated",
        DeepError::Unauthorized => "Unauthorized",
        DeepError::SessionExpired => "SessionExpired",
        DeepError::SessionDenied => "SessionDenied",
        DeepError::Api { .. } => "Api",
        DeepError::InvalidTaskCode(_) => "InvalidTaskCode",
        DeepError::InvalidProjectFormat => "InvalidProjectFormat",
        DeepError::NoProjectBinding => "NoProjectBinding",
        DeepError::AlreadyInitialized(_) => "AlreadyInitialized",
        DeepError::Update(_) => "Update",
        DeepError::NoActiveTask => "NoActiveTask",
        DeepError::HeadlessUnavailable => "HeadlessUnavailable",
    };
    let payload = serde_json::json!({
        "ok": false,
        "error": {
            "kind": kind,
            "message": e.to_string(),
        }
    });
    eprintln!("{}", payload);
}
