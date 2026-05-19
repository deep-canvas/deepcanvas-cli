# Deep CLI — Interactive Project Picker Update

**Versiyon:** v1.1 ekleme
**Bağımlılık:** `cli-implementation-spec.md` v1.0 uygulanmış olmalı. Bu dosya o spec'in üzerine sadece patch'tir.
**Yeni API endpoint:** `GET /cli/projects` (zaten `cli-auth-api-spec.md` §4.5'te tanımlı).

---

## 1. Workspace Cargo.toml — Yeni Dep

`[workspace.dependencies]` altına ekle:

```toml
dialoguer = "0.11"
```

`crates/deepcanvas-cli/Cargo.toml`'a ekle:

```toml
dialoguer = { workspace = true }
```

---

## 2. Core: `types.rs` — Yeni Tipler

`DocumentFull` struct'ının altına ekle:

```rust
#[derive(Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectListEntry>,
    pub count: u32,
}

#[derive(Deserialize)]
pub struct ProjectListEntry {
    pub organization_slug: String,
    pub organization_name: String,
    pub project_slug: String,
    pub project_name: String,
    pub role: String,
}
```

---

## 3. CLI: `main.rs` — Init Slug Opsiyonel

`Commands` enum'unda `Init` satırını değiştir:

```rust
// ÖNCE:
Init { slug_pair: String },

// SONRA:
Init {
    /// Project as <org-slug>/<project-slug>. Omit for interactive picker.
    slug_pair: Option<String>,
},
```

---

## 4. CLI: `commands/init.rs` — Tam Yeniden Yaz

Mevcut dosyayı şununla değiştir:

```rust
use colored::Colorize;
use deepcanvas_core::{
    ApiClient, Config, DeepError, ProjectBinding, token,
    types::{ProjectListEntry, ProjectListResponse, TasksResponse},
};
use dialoguer::{Select, theme::ColorfulTheme};

pub async fn run(config: Config, slug_pair: Option<String>) -> Result<(), DeepError> {
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".deep").join("config.toml");
    if config_path.exists() {
        return Err(DeepError::AlreadyInitialized(config_path.display().to_string()));
    }

    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let project = match slug_pair {
        Some(s) => resolve_from_slug(&client, &s).await?,
        None => resolve_interactive(&client).await?,
    };

    std::fs::create_dir_all(cwd.join(".deep"))?;
    std::fs::write(&config_path, project.to_toml())?;
    update_gitignore(&cwd)?;

    println!();
    println!("{} Linked to: {} / {}",
        "✓".green().bold(),
        project.organization_slug,
        project.project_slug.bold());
    println!("  Wrote {}", config_path.display().to_string().cyan());
    println!("  Updated .gitignore");
    println!();
    println!("Try: deep tasks");
    Ok(())
}

async fn resolve_from_slug(client: &ApiClient, s: &str) -> Result<ProjectBinding, DeepError> {
    let project = ProjectBinding::from_flag(s)?;
    let path = format!(
        "/cli/tasks?org={}&project={}&limit=1",
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );
    let _: TasksResponse = client.get(&path).await?;
    Ok(project)
}

async fn resolve_interactive(client: &ApiClient) -> Result<ProjectBinding, DeepError> {
    let list: ProjectListResponse = client.get("/cli/projects").await?;

    if list.projects.is_empty() {
        println!("{}", "You don't have access to any projects yet.".yellow());
        println!("Ask a project manager to add you, then run `deep init` again.");
        std::process::exit(0);
    }

    let items: Vec<String> = list.projects.iter()
        .map(|p| format!("{} / {}  ({})",
            p.organization_name, p.project_name, p.role))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a project to link this directory to")
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|e| DeepError::Io(std::io::Error::new(
            std::io::ErrorKind::Interrupted, e.to_string(),
        )))?;

    let idx = selection.ok_or_else(|| DeepError::Io(std::io::Error::new(
        std::io::ErrorKind::Interrupted, "selection cancelled",
    )))?;

    let chosen: &ProjectListEntry = &list.projects[idx];
    Ok(ProjectBinding {
        organization_slug: chosen.organization_slug.clone(),
        project_slug: chosen.project_slug.clone(),
    })
}

fn update_gitignore(cwd: &std::path::Path) -> Result<(), DeepError> {
    // (Aynı, değişmedi — orijinal spec'teki fonksiyonu koru)
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

## 5. CLI: `commands/login.rs` — Sonuna Init Prompt Ekle

`run` fonksiyonunda `token::store` ve `match location { ... }` bloğunun sonundaki şu satırı:

```rust
println!();
println!("Try: deep init <org>/<project>");
Ok(())
```

Şununla değiştir:

```rust
println!();

// Suggest init if this directory is not already linked
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
    .map_err(|e| DeepError::Io(std::io::Error::new(
        std::io::ErrorKind::Interrupted, e.to_string(),
    )))?;

match do_init {
    Some(true) => super::init::run(config, None).await,
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
```

Ayrıca `run` fonksiyonunun başındaki şu satırı:

```rust
let client = ApiClient::new(config);
```

Şununla değiştir (config'i sonradan init'e geçirmek için):

```rust
let client = ApiClient::new(config.clone());
```

---

## 6. Test Script: `scripts/test-init.sh` — Slug Opsiyonel

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -ge 1 ]; then
    cargo run --quiet --bin deep -- init "$1"
else
    cargo run --quiet --bin deep -- init
fi
```

---

## 7. Yeni Acceptance Criteria

Mevcut listeye ekle:

- `deep init` (slug'suz) `/cli/projects`'i çağırır, dialoguer ile arrow key picker gösterir, seçilen projeyle binding yazılır.
- `deep init` (slug'suz) kullanıcının hiç projesi yoksa açıklayıcı mesajla exit 0.
- `deep login` başarılı olduğunda `.deep/config.toml` yoksa "Link this directory to a project now? [Y/n]" sorusu sorar; Y → interaktif init flow; N → çıkar; non-TTY → hint.
- `deep login` zaten link'li dizinde "already linked" mesajı, prompt sormaz.

---

## 8. UX Akışı (Yeni)

```
$ deep login
  → browser açılır, approve
  → ✓ Authorized
  → ? Link this directory to a project now? [Y/n]: y
  
  ? Select a project to link this directory to
    ▸ Datosfer / DeepCanvas Platform  (manager)
      Datosfer / ACO Platform         (contributor)
      Linkiste / Linkiste Core        (viewer)
  
  → ✓ Linked to: datosfer / deepcanvas-platform
  → Wrote .deep/config.toml
  → Updated .gitignore

$ deep tasks
  → çalışır
```