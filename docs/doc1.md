# Deep CLI — Implementation Spec

**Hedef proje:** `deepcanvas-cli` (Rust workspace)
**Workspace:** `crates/deepcanvas-cli` (binary `deep`) + `crates/deepcanvas-core` (library)
**Versiyon:** v1.0
**Komutlar:** `login`, `logout`, `init`, `tasks`, `pull`, `completion`, `update`
**Bağımlılık:** Backend `cli-auth-api-spec.md` endpoint'leri deploy edilmiş.

---

## 1. Sorumluluk Ayrımı

| Crate | İçerik |
|---|---|
| `deepcanvas-core` | API client, types, config, token storage (keyring + file fallback), project binding, error types. **TTY-agnostic** — `println!` yok, browser açmaz. |
| `deepcanvas-cli` | Clap CLI, command handler'ları, TTY (browser, spinner, table, colored output), file path kararları, exit code'ları. |

---

## 2. Workspace Bağımlılıkları

Workspace root `Cargo.toml` `[workspace.dependencies]`:

```toml
clap = { version = "4.5", features = ["derive"] }
clap_complete = "4.5"
tokio = { version = "1.41", features = ["macros", "rt-multi-thread"] }
reqwest = { version = "0.13", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
directories = "5.0"
keyring = "3"
webbrowser = "1"
indicatif = "0.17"
comfy-table = "7"
colored = "2"
toml = "0.8"
thiserror = "1"
hostname = "0.4"
os_info = "3"
urlencoding = "2"
self_update = { version = "0.42", default-features = false, features = ["rustls", "compression-flate2"] }
```

`crates/deepcanvas-core/Cargo.toml`:
```toml
[dependencies]
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
directories = { workspace = true }
keyring = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
```

`crates/deepcanvas-cli/Cargo.toml`:
```toml
[dependencies]
deepcanvas-core = { path = "../deepcanvas-core" }
clap = { workspace = true }
clap_complete = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
webbrowser = { workspace = true }
indicatif = { workspace = true }
comfy-table = { workspace = true }
colored = { workspace = true }
hostname = { workspace = true }
os_info = { workspace = true }
urlencoding = { workspace = true }
self_update = { workspace = true }
toml = { workspace = true }
```

---

## 3. Core: `config.rs`

```rust
use std::env;

pub const DEFAULT_API_BASE: &str = "https://api0910.deepcanvas.studio";
pub const DEFAULT_FRONTEND_BASE: &str = "https://app.deepcanvas.studio";
pub const API_PATH_PREFIX: &str = "/api/v1";

#[derive(Debug, Clone)]
pub struct Config {
    pub api_base: String,
    pub frontend_base: String,
}

impl Config {
    pub fn load() -> Self {
        Self {
            api_base: env::var("DEEPCANVAS_API_URL")
                .unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
            frontend_base: env::var("DEEPCANVAS_FRONTEND_URL")
                .unwrap_or_else(|_| DEFAULT_FRONTEND_BASE.to_string()),
        }
    }

    pub fn api_url(&self, path: &str) -> String {
        format!("{}{}{}", self.api_base, API_PATH_PREFIX, path)
    }
}
```

CLI crate'inden eski `src/config.rs` silinir.

---

## 4. Core: `error.rs`

```rust
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

    #[error("project binding not found. Run `deep init <org>/<project>` first or use --project flag.")]
    NoProjectBinding,

    #[error(".deep/config.toml already exists at {0}. Remove it first to re-init.")]
    AlreadyInitialized(String),

    #[error("update error: {0}")]
    Update(String),
}
```

---

## 5. Core: `token.rs` (Keyring + File Fallback)

```rust
use directories::ProjectDirs;
use keyring::Entry;
use std::path::PathBuf;
use crate::error::DeepError;

const SERVICE: &str = "deepcanvas-cli";
const ACCOUNT: &str = "default";

#[derive(Debug, Clone, Copy)]
pub enum TokenLocation { Keyring, File }

pub fn store(token: &str) -> Result<TokenLocation, DeepError> {
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        if entry.set_password(token).is_ok() {
            let _ = remove_file_token();
            return Ok(TokenLocation::Keyring);
        }
    }
    let path = token_file_path()?;
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&path, token)?;
    set_mode_0600(&path)?;
    Ok(TokenLocation::File)
}

pub fn load() -> Result<Option<String>, DeepError> {
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        match entry.get_password() {
            Ok(t) => return Ok(Some(t)),
            Err(keyring::Error::NoEntry) => {}
            Err(_) => {}
        }
    }
    let path = token_file_path()?;
    if path.is_file() {
        return Ok(Some(std::fs::read_to_string(&path)?.trim().to_string()));
    }
    Ok(None)
}

pub fn remove() -> Result<bool, DeepError> {
    let mut removed = false;
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        match entry.delete_credential() {
            Ok(()) => removed = true,
            Err(keyring::Error::NoEntry) => {}
            Err(_) => {}
        }
    }
    if remove_file_token()? { removed = true; }
    Ok(removed)
}

fn remove_file_token() -> Result<bool, DeepError> {
    let path = token_file_path()?;
    if path.is_file() { std::fs::remove_file(&path)?; Ok(true) } else { Ok(false) }
}

fn token_file_path() -> Result<PathBuf, DeepError> {
    ProjectDirs::from("studio", "deepcanvas", "deep")
        .map(|d| d.config_dir().join("credentials"))
        .ok_or_else(|| DeepError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound, "could not resolve config dir")))
}

#[cfg(unix)]
fn set_mode_0600(path: &std::path::Path) -> Result<(), DeepError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}
#[cfg(not(unix))]
fn set_mode_0600(_: &std::path::Path) -> Result<(), DeepError> { Ok(()) }
```

**Path örnekleri:** macOS `~/Library/Application Support/studio.deepcanvas.deep/credentials`, Linux `~/.config/deep/credentials`, Windows `%APPDATA%\deepcanvas\deep\config\credentials`.

---

## 6. Core: `api.rs`

```rust
use reqwest::{Client as HttpClient, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use crate::{config::Config, error::DeepError};

pub struct ApiClient {
    config: Config,
    http: HttpClient,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(config: Config) -> Self {
        let http = HttpClient::builder()
            .user_agent(concat!("deepcanvas-cli/", env!("CARGO_PKG_VERSION")))
            .build().expect("reqwest client build");
        Self { config, http, token: None }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token); self
    }

    pub async fn post<B, R>(&self, path: &str, body: &B) -> Result<R, DeepError>
    where B: Serialize + ?Sized, R: DeserializeOwned,
    {
        let mut req = self.http.post(self.config.api_url(path)).json(body);
        if let Some(t) = &self.token { req = req.bearer_auth(t); }
        handle(req.send().await?).await
    }

    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, DeepError> {
        let mut req = self.http.get(self.config.api_url(path));
        if let Some(t) = &self.token { req = req.bearer_auth(t); }
        handle(req.send().await?).await
    }

    pub async fn get_long_poll<R: DeserializeOwned>(&self, path: &str, timeout: u64) -> Result<R, DeepError> {
        let mut req = self.http.get(self.config.api_url(path))
            .timeout(std::time::Duration::from_secs(timeout + 5));
        if let Some(t) = &self.token { req = req.bearer_auth(t); }
        handle(req.send().await?).await
    }
}

async fn handle<R: DeserializeOwned>(res: reqwest::Response) -> Result<R, DeepError> {
    let status = res.status();
    if status.is_success() { return Ok(res.json().await?); }
    if status == StatusCode::UNAUTHORIZED { return Err(DeepError::Unauthorized); }
    let body: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({}));
    let message = body.get("detail").and_then(|v| v.as_str()).unwrap_or("unknown error").to_string();
    Err(DeepError::Api { status: status.as_u16(), message })
}
```

---

## 7. Core: `types.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct AuthStartRequest { pub client_info: ClientInfo }

#[derive(Serialize, Clone)]
pub struct ClientInfo {
    pub hostname: String,
    pub os: String,
    pub os_version: String,
    pub cli_version: String,
}

#[derive(Deserialize)]
pub struct AuthStartResponse {
    pub device_token: String,
    pub user_code: String,
    pub user_code_display: String,
    pub verify_url: String,
    pub expires_in: u64,
    pub poll_interval: u64,
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PollResponse {
    Pending,
    Denied,
    Expired,
    Approved { access_token: String },
}

#[derive(Deserialize)]
pub struct TasksResponse {
    pub project: ProjectRef,
    pub tasks: Vec<TaskSummary>,
    pub count: u32,
}

#[derive(Deserialize)]
pub struct ProjectRef {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub organization_slug: String,
}

#[derive(Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub energy: Option<String>,
    pub priority: Option<i32>,
    pub assignee: Option<UserRef>,
    pub primary_document: Option<DocumentRef>,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct UserRef { pub id: String, pub email: String, pub name: Option<String> }

#[derive(Deserialize)]
pub struct DocumentRef { pub id: String, pub code: String, pub title: String }

#[derive(Deserialize)]
pub struct TaskContextResponse {
    pub task: TaskDetail,
    pub documents: TaskDocuments,
}

#[derive(Deserialize)]
pub struct TaskDetail {
    pub id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub energy: Option<String>,
    pub priority: Option<i32>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub assignee: Option<UserRef>,
    pub reporter: Option<UserRef>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct AcceptanceCriterion { pub id: String, pub text: String, pub is_done: bool }

#[derive(Deserialize)]
pub struct TaskDocuments {
    pub primary: Option<DocumentFull>,
    pub related: Vec<DocumentFull>,
}

#[derive(Deserialize)]
pub struct DocumentFull {
    pub id: String,
    pub code: String,
    #[serde(rename = "type")] pub doc_type: String,
    pub title: String,
    pub content_markdown: String,
    pub version: i32,
    pub updated_at: String,
}
```

---

## 8. Core: `project.rs`

```rust
use serde::Deserialize;
use std::path::{Path, PathBuf};
use crate::error::DeepError;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectBinding {
    pub organization_slug: String,
    pub project_slug: String,
}

#[derive(Deserialize)]
struct DeepConfigFile { project: ProjectBinding }

impl ProjectBinding {
    pub fn find_from(start: &Path) -> Result<Option<(Self, PathBuf)>, DeepError> {
        let mut cur: Option<&Path> = Some(start);
        while let Some(dir) = cur {
            let candidate = dir.join(".deep").join("config.toml");
            if candidate.is_file() {
                let raw = std::fs::read_to_string(&candidate)?;
                let cfg: DeepConfigFile = toml::from_str(&raw)?;
                return Ok(Some((cfg.project, dir.to_path_buf())));
            }
            cur = dir.parent();
        }
        Ok(None)
    }

    pub fn from_flag(s: &str) -> Result<Self, DeepError> {
        let (org, proj) = s.split_once('/').ok_or(DeepError::InvalidProjectFormat)?;
        if org.is_empty() || proj.is_empty() { return Err(DeepError::InvalidProjectFormat); }
        Ok(Self {
            organization_slug: org.to_string(),
            project_slug: proj.to_string(),
        })
    }

    pub fn to_toml(&self) -> String {
        format!("[project]\norganization_slug = \"{}\"\nproject_slug = \"{}\"\n",
            self.organization_slug, self.project_slug)
    }
}
```

---

## 9. Core: `lib.rs`

```rust
pub mod api;
pub mod config;
pub mod error;
pub mod project;
pub mod token;
pub mod types;

pub use api::ApiClient;
pub use config::Config;
pub use error::DeepError;
pub use project::ProjectBinding;
pub use token::TokenLocation;
pub use types::*;
```

---

## 10. CLI: `main.rs`

```rust
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use deepcanvas_core::Config;

mod commands;
mod ui;

#[derive(Parser)]
#[command(name = "deep", version, about = "DeepCanvas CLI")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate this device with DeepCanvas
    Login,
    /// Remove stored credentials from this device
    Logout,
    /// Link the current directory to a DeepCanvas project
    Init { slug_pair: String },
    /// List assigned tasks
    Tasks {
        #[arg(long, short)] project: Option<String>,
    },
    /// Pull task context into .deep/<task-code>/
    Pull {
        #[arg(required = true)] task_codes: Vec<String>,
        #[arg(long, short)] project: Option<String>,
    },
    /// Generate shell completion script
    Completion { shell: Shell },
    /// Self-update the deep binary from GitHub releases
    Update,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();

    let result = match cli.command {
        Commands::Login => commands::login::run(config).await,
        Commands::Logout => commands::logout::run(),
        Commands::Init { slug_pair } => commands::init::run(config, slug_pair).await,
        Commands::Tasks { project } => commands::tasks::run(config, project).await,
        Commands::Pull { task_codes, project } =>
            commands::pull::run(config, task_codes, project).await,
        Commands::Completion { shell } => { commands::completion::run(shell); Ok(()) }
        Commands::Update => commands::update::run().await,
    };

    if let Err(e) = result {
        ui::print_error(&e);
        std::process::exit(1);
    }
}
```

`crates/deepcanvas-cli/src/commands/mod.rs`:
```rust
pub mod completion;
pub mod init;
pub mod login;
pub mod logout;
pub mod pull;
pub mod tasks;
pub mod update;
```

---

## 11. CLI: `commands/login.rs`

```rust
use colored::Colorize;
use deepcanvas_core::{
    ApiClient, Config, DeepError, TokenLocation, token,
    types::{AuthStartRequest, AuthStartResponse, ClientInfo, PollResponse},
};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn run(config: Config) -> Result<(), DeepError> {
    let client = ApiClient::new(config);
    let req = AuthStartRequest { client_info: collect_client_info() };
    let session: AuthStartResponse = client.post("/cli/auth", &req).await?;

    println!();
    println!("Opening browser for authorization...");
    println!("  → {}", session.verify_url.cyan());
    println!();
    println!("If the browser doesn't open, visit the URL above.");
    println!("Verify the code matches: {}", session.user_code_display.bold());
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
        TokenLocation::File => println!("  {}",
            "Token saved to local file (keyring unavailable).".yellow()),
    }
    println!();
    println!("Try: deep init <org>/<project>");
    Ok(())
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
        hostname: hostname::get().ok().and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into()),
        os: std::env::consts::OS.to_string(),
        os_version: os_info::get().version().to_string(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
```

---

## 12. CLI: `commands/logout.rs`

```rust
use colored::Colorize;
use deepcanvas_core::{DeepError, token};

pub fn run() -> Result<(), DeepError> {
    let removed = token::remove()?;
    if removed {
        println!("{} Credentials removed from this device.", "✓".green().bold());
    } else {
        println!("No credentials stored on this device.");
    }
    Ok(())
}
```

Backend revoke isteği yapmaz — yalnız lokal keyring + file cleanup.

---

## 13. CLI: `commands/init.rs`

```rust
use colored::Colorize;
use deepcanvas_core::{
    ApiClient, Config, DeepError, ProjectBinding, token, types::TasksResponse,
};

pub async fn run(config: Config, slug_pair: String) -> Result<(), DeepError> {
    let project = ProjectBinding::from_flag(&slug_pair)?;

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".deep").join("config.toml");
    if config_path.exists() {
        return Err(DeepError::AlreadyInitialized(config_path.display().to_string()));
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
    println!("{} Linked to: {} / {}",
        "✓".green().bold(),
        project.organization_slug,
        response.project.name.bold());
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
        if content.lines().any(|l| l.trim() == marker) { return Ok(()); }
        let mut new_content = content;
        if !new_content.ends_with('\n') { new_content.push('\n'); }
        new_content.push_str("\n# Deep CLI task contexts\n");
        new_content.push_str(marker);
        new_content.push('\n');
        std::fs::write(&gitignore, new_content)?;
    } else {
        std::fs::write(&gitignore, format!("# Deep CLI task contexts\n{}\n", marker))?;
    }
    Ok(())
}
```

---

## 14. CLI: `commands/tasks.rs`

```rust
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL, ContentArrangement};
use deepcanvas_core::{
    ApiClient, Config, DeepError, ProjectBinding, token, types::TasksResponse,
};

pub async fn run(config: Config, project_flag: Option<String>) -> Result<(), DeepError> {
    let project = resolve_project(project_flag)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let path = format!(
        "/cli/tasks?org={}&project={}",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TasksResponse = client.get(&path).await?;

    if response.tasks.is_empty() {
        println!("No assigned tasks in {} / {}.",
            project.organization_slug, project.project_slug);
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["CODE", "TITLE", "ENERGY", "STATUS", "PRD"]);

    for t in &response.tasks {
        table.add_row(vec![
            t.code.clone(),
            truncate(&t.title, 60),
            t.energy.clone().unwrap_or_else(|| "—".into()),
            t.status.clone(),
            t.primary_document.as_ref().map(|d| d.code.clone()).unwrap_or_else(|| "—".into()),
        ]);
    }

    println!();
    println!("Project: {}", response.project.name.bold());
    println!();
    println!("{table}");
    println!();
    println!("{} task(s). Run `deep pull <code>` to fetch context.", response.count);
    Ok(())
}

pub fn resolve_project(flag: Option<String>) -> Result<ProjectBinding, DeepError> {
    if let Some(s) = flag { return ProjectBinding::from_flag(&s); }
    let cwd = std::env::current_dir()?;
    ProjectBinding::find_from(&cwd)?
        .map(|(b, _)| b)
        .ok_or(DeepError::NoProjectBinding)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n { return s.to_string(); }
    let mut out: String = s.chars().take(n - 1).collect();
    out.push('…');
    out
}
```

---

## 15. CLI: `commands/pull.rs` (Multi-Task)

```rust
use colored::Colorize;
use deepcanvas_core::{
    ApiClient, Config, DeepError, ProjectBinding, token,
    types::{TaskContextResponse, TaskDetail, TaskDocuments},
};
use std::path::PathBuf;
use super::tasks::resolve_project;

pub async fn run(
    config: Config,
    task_codes: Vec<String>,
    project_flag: Option<String>,
) -> Result<(), DeepError> {
    let normalized: Vec<String> = task_codes.into_iter().map(|c| c.to_uppercase()).collect();
    for code in &normalized {
        if !is_valid_task_code(code) {
            return Err(DeepError::InvalidTaskCode(code.clone()));
        }
    }

    let project = resolve_project(project_flag)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    for code in &normalized {
        pull_one(&client, &project, code).await?;
    }

    if normalized.len() > 1 {
        println!();
        println!("{} Pulled {} tasks.", "✓".green().bold(), normalized.len());
    }
    Ok(())
}

async fn pull_one(client: &ApiClient, project: &ProjectBinding, code: &str)
    -> Result<(), DeepError>
{
    let path = format!(
        "/cli/tasks/{}/context?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let response: TaskContextResponse = client.get(&path).await?;

    let task_dir = PathBuf::from(".deep").join(code);
    std::fs::create_dir_all(&task_dir)?;

    let task_md_path = task_dir.join("task.md");
    std::fs::write(&task_md_path, format_task_md(&response.task, &response.documents))?;

    let mut written: Vec<String> = vec![];
    if let Some(doc) = &response.documents.primary {
        std::fs::write(task_dir.join(format!("{}.md", doc.code)), &doc.content_markdown)?;
        written.push(doc.code.clone());
    }
    for doc in &response.documents.related {
        std::fs::write(task_dir.join(format!("{}.md", doc.code)), &doc.content_markdown)?;
        written.push(doc.code.clone());
    }

    println!();
    println!("{} Fetched {}: {}",
        "✓".green().bold(), response.task.code.bold(), response.task.title);
    println!("  {} {}", "→".dimmed(), task_md_path.display());
    for c in &written {
        println!("  {} {}/{}.md", "→".dimmed(), task_dir.display(), c);
    }
    Ok(())
}

fn is_valid_task_code(code: &str) -> bool {
    let mut parts = code.splitn(2, '-');
    let prefix = parts.next().unwrap_or("");
    let number = parts.next().unwrap_or("");
    !prefix.is_empty() && !number.is_empty()
        && prefix.chars().all(|c| c.is_ascii_uppercase())
        && number.chars().all(|c| c.is_ascii_digit())
}

fn format_task_md(task: &TaskDetail, docs: &TaskDocuments) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}: {}\n\n", task.code, task.title));
    out.push_str(&format!("**Status:** {}  \n", task.status));
    if let Some(e) = &task.energy { out.push_str(&format!("**Energy:** {}  \n", e)); }
    if let Some(p) = task.priority { out.push_str(&format!("**Priority:** {}  \n", p)); }
    if let Some(a) = &task.assignee {
        let name = a.name.clone().unwrap_or_else(|| a.email.clone());
        out.push_str(&format!("**Assignee:** {} <{}>  \n", name, a.email));
    }
    out.push('\n');

    if let Some(d) = &task.description {
        out.push_str("## Description\n\n");
        out.push_str(d.trim());
        out.push_str("\n\n");
    }
    if !task.acceptance_criteria.is_empty() {
        out.push_str("## Acceptance Criteria\n\n");
        for ac in &task.acceptance_criteria {
            out.push_str(&format!("- [{}] {}\n", if ac.is_done { "x" } else { " " }, ac.text));
        }
        out.push('\n');
    }

    let mut linked = Vec::new();
    if let Some(p) = &docs.primary { linked.push((p.code.clone(), p.title.clone(), true)); }
    for r in &docs.related { linked.push((r.code.clone(), r.title.clone(), false)); }
    if !linked.is_empty() {
        out.push_str("## Linked Documents\n\n");
        for (code, title, primary) in linked {
            let suffix = if primary { " (primary)" } else { "" };
            out.push_str(&format!("- [{}](./{}.md) — {}{}\n", code, code, title, suffix));
        }
    }
    out
}
```

---

## 16. CLI: `commands/completion.rs`

```rust
use crate::Cli;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn run(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "deep", &mut io::stdout());
}
```

**Kullanım:**
```bash
deep completion bash > ~/.local/share/bash-completion/completions/deep
deep completion zsh  > "${fpath[1]}/_deep"
deep completion fish > ~/.config/fish/completions/deep.fish
```

---

## 17. CLI: `commands/update.rs`

```rust
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
    if path.contains("/Cellar/") || path.contains("/opt/homebrew/") || path.contains("/linuxbrew/") {
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
        .build().map_err(|e| DeepError::Update(e.to_string()))?
        .update().map_err(|e| DeepError::Update(e.to_string()))?;

    if status.updated() {
        println!("{} Updated to {}", "✓".green().bold(), status.version().bold());
    } else {
        println!("Already on latest version ({}).", status.version());
    }
    Ok(())
}
```

GitHub release asset adlandırma `cli-release-spec.md` ile tutarlı.

---

## 18. CLI: `ui.rs`

```rust
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
```

`colored` `NO_COLOR` env varını otomatik respect eder.

---

## 19. `.deep/config.toml` Formatı

`deep init` yazar:

```toml
[project]
organization_slug = "datosfer"
project_slug = "deepcanvas-platform"
```

`.gitignore` otomatik update:
```
# Deep CLI task contexts
.deep/*/
```

`.deep/config.toml` commit edilir. `.deep/<task-code>/` ignore — lokal cache.

---

## 20. Test Script'leri

**`scripts/dev-env.sh`:**
```bash
#!/usr/bin/env bash
export DEEPCANVAS_API_URL="http://localhost:8000"
export DEEPCANVAS_FRONTEND_URL="http://localhost:3000"
```

**`scripts/test-login.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
echo "→ API: $DEEPCANVAS_API_URL"
echo "→ FE:  $DEEPCANVAS_FRONTEND_URL"
echo
cargo run --quiet --bin deep -- login
```

**`scripts/test-logout.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
cargo run --quiet --bin deep -- logout
```

**`scripts/test-init.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
if [ "$#" -lt 1 ]; then
    echo "usage: $0 <org-slug>/<project-slug>"
    exit 1
fi
cargo run --quiet --bin deep -- init "$1"
```

**`scripts/test-tasks.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
if [ "$#" -ge 1 ]; then
    cargo run --quiet --bin deep -- tasks --project "$1"
else
    cargo run --quiet --bin deep -- tasks
fi
```

**`scripts/test-pull.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
if [ "$#" -lt 1 ]; then
    echo "usage: $0 <task-code> [<task-code>...]"
    exit 1
fi
cargo run --quiet --bin deep -- pull "$@"
```

**`scripts/test-completion.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
SHELL_TYPE="${1:-bash}"
cargo run --quiet --bin deep -- completion "$SHELL_TYPE"
```

**`scripts/test-all.sh`:**
```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -lt 2 ]; then
    echo "usage: $0 <org-slug>/<project-slug> <task-code>"
    exit 1
fi
PROJECT="$1"
TASK="$2"

echo "=== build ==="
cargo build --quiet

echo
echo "=== deep --help ==="
cargo run --quiet --bin deep -- --help

echo
echo "Run: ./scripts/test-login.sh (separately if not already logged in)"

echo
echo "=== deep init $PROJECT ==="
cargo run --quiet --bin deep -- init "$PROJECT" || \
    echo "(already initialized — skipping)"

echo
echo "=== deep tasks ==="
cargo run --quiet --bin deep -- tasks

echo
echo "=== deep pull $TASK ==="
cargo run --quiet --bin deep -- pull "$TASK"

echo
echo "=== .deep/$TASK/ output ==="
ls -la ".deep/$TASK/"
echo
cat ".deep/$TASK/task.md"
```

`chmod +x scripts/*.sh` zorunlu.

---

## 21. Çalışma Sırası

```bash
# 1. Backend (8000) + Frontend (3000) lokalde ayağa kaldır.

# 2. Build
cargo build

# 3. Login (cihaz başına 1 kez)
./scripts/test-login.sh

# 4. Init (repo başına 1 kez)
./scripts/test-init.sh datosfer/deepcanvas-platform

# 5. Liste
./scripts/test-tasks.sh

# 6. Pull
./scripts/test-pull.sh DC-142

# 7. Multi-task pull
./scripts/test-pull.sh DC-142 DC-156

# 8. Shell completion
./scripts/test-completion.sh bash > /tmp/deep-bash.sh

# 9. Logout
./scripts/test-logout.sh
```

---

## 22. Acceptance Criteria

1. `cargo build` workspace hatasız compile eder.
2. `deep --help` 7 komutu listeler.
3. `deep login` browser açar, user_code'u gösterir, long-poll'da bekler, başarılı sonuçta token store edilir (keyring veya file fallback).
4. `deep logout` keyring + file token kaynaklarını temizler, idempotent.
5. `deep init <org>/<project>` `.deep/config.toml` ve `.gitignore` yazar; ikinci kez çağrılırsa `AlreadyInitialized` hatası.
6. `deep init` invalid slug formatında `InvalidProjectFormat`.
7. `deep tasks` `.deep/config.toml` mevcutsa flag'siz çalışır, yoksa `NoProjectBinding`.
8. `deep tasks --project X/Y` flag config'i override eder.
9. `deep pull DC-142` `.deep/DC-142/` klasörü oluşturur, `task.md` + doküman markdown'larını yazar.
10. `deep pull DC-1 DC-2 DC-3` sıralı işler; biri fail olursa kalanlar çalışmaz ama önceki yazılanlar korunur.
11. `deep pull` invalid task code formatında `InvalidTaskCode` (validation önce, fetch sonra).
12. `deep completion bash` valid bash completion script'i stdout'a basar.
13. `deep update` Homebrew yolundaysa exit + `brew upgrade` önerisi; değilse GitHub Releases'ten kontrol eder.
14. Backend `{detail: "..."}` hata response'u parse edilip kullanıcıya gösterilir.
15. 401 → `Unauthorized` → "run deep login" hint.
16. Tüm hata mesajları `colored` ile renkli (`NO_COLOR` env'de düz).
17. `DEEPCANVAS_API_URL=http://localhost:8000` env varı ile lokal backend'e bağlanır.
18. Test script'leri `set -euo pipefail` ile strict mode, başarılı çıkışta exit 0.
19. CLI hiçbir komutta panic etmez — tüm hatalar `DeepError` üzerinden geçer.

---

## 23. Bu Fazda Yapılmayacaklar

- MCP server (`deep mcp-serve`) — bu fazda yok.

(Diğer tüm planlı özellikler bu spec'te.)

---

**Çalışma bittiğinde:** Release packaging için `cli-release-spec.md`.