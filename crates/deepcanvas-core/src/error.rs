use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeepError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("toml parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("not authenticated. Run `deep login` first.")]
    NotAuthenticated,

    #[error("authorization expired or revoked. Run `deep login` again.")]
    Unauthorized,

    #[error("session expired before approval")]
    SessionExpired,

    #[error("session denied by user")]
    SessionDenied,

    #[error("api error {status}: {message}")]
    Api { status: u16, message: String },

    #[error("invalid task code: {0}")]
    InvalidTaskCode(String),

    #[error("invalid project format: expected <org-slug>/<project-slug>")]
    InvalidProjectFormat,

    #[error(
        "project binding not found. Run `deep init <org>/<project>` first or use --project flag."
    )]
    NoProjectBinding,

    #[error(".deep/config.toml already exists at {0}. Remove it first to re-init.")]
    AlreadyInitialized(String),

    #[error("update error: {0}")]
    Update(String),

    #[error("no active task. Pull a task first or pass a code: deep done <code>")]
    NoActiveTask,

    #[error("this command requires an interactive terminal; --headless not supported")]
    HeadlessUnavailable,
}
