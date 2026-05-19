# Deep CLI — `deep done` Komutu Ekleme

**Versiyon:** v1.2 ekleme
**Hedef:** Rust CLI (`deepcanvas-cli`)
**Bağımlılık:** `cli-implementation-spec.md` v1.0 + `cli-implementation-update-v1.1.md` uygulanmış.
**Yeni API endpoint:** `POST /cli/tasks/{code}/complete` (bkz. `cli-api-update-v1.2.md`).

---

## 1. Kavram: Active Task

`.deep/active` text dosyası — sadece task code'unu içerir (örn. `DC-142`).

- `deep pull <code>` başarılı olunca son işlenen task code'unu `.deep/active`'e yazar.
- `deep done` (argümansız) `.deep/active`'i okur, o task'ı complete eder.
- `deep done <code>` explicit kod, `.deep/active`'i ignore eder.
- Complete başarılı olunca `.deep/active` silinir (no longer active).
- `.deep/active` `.gitignore` kapsamında (mevcut `.deep/*/` pattern'i bunu ignore etmez — açıkça eklenir).

---

## 2. `.gitignore` — Yeni Satır

`deep init` `update_gitignore` fonksiyonundaki marker bloğunu şu hale getir:

```rust
new_content.push_str("\n# Deep CLI task contexts\n");
new_content.push_str(".deep/*/\n");
new_content.push_str(".deep/active\n");
```

Mevcut repolarda kullanıcı manuel ekler ya da bir sonraki `deep init` ile eklenir (idempotent).

**Marker check'i güncelle:** Mevcut `.deep/*/` kontrolü yeterli — `.deep/active`'i ayrıca check etme, beraber yazılıyor.

---

## 3. Core: `active_task.rs` (Yeni Modül)

`crates/deepcanvas-core/src/active_task.rs`:

```rust
use std::path::Path;
use crate::error::DeepError;

const FILE_NAME: &str = "active";

pub fn read(cwd: &Path) -> Result<Option<String>, DeepError> {
    let path = cwd.join(".deep").join(FILE_NAME);
    if !path.is_file() { return Ok(None); }
    let s = std::fs::read_to_string(&path)?;
    let trimmed = s.trim();
    if trimmed.is_empty() { return Ok(None); }
    Ok(Some(trimmed.to_string()))
}

pub fn write(cwd: &Path, code: &str) -> Result<(), DeepError> {
    let deep_dir = cwd.join(".deep");
    std::fs::create_dir_all(&deep_dir)?;
    std::fs::write(deep_dir.join(FILE_NAME), code)?;
    Ok(())
}

pub fn clear(cwd: &Path) -> Result<(), DeepError> {
    let path = cwd.join(".deep").join(FILE_NAME);
    if path.is_file() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
```

`crates/deepcanvas-core/src/lib.rs`'e ekle:

```rust
pub mod active_task;
```

---

## 4. Core: `types.rs` — Yeni Tip

```rust
#[derive(Deserialize)]
pub struct TaskCompleteResponse {
    pub task: TaskCompleted,
}

#[derive(Deserialize)]
pub struct TaskCompleted {
    pub id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub completed_at: Option<String>,
    pub updated_at: String,
}
```

---

## 5. Core: `error.rs` — Yeni Varyant

```rust
#[error("no active task. Pull a task first or pass a code: deep done <code>")]
NoActiveTask,
```

---

## 6. CLI: `main.rs` — Yeni Komut

`Commands` enum'una ekle:

```rust
/// Mark a task as done
Done {
    /// Task code, e.g. DC-142. Omit to complete the active task.
    task_code: Option<String>,
},
```

`match` bloğuna:

```rust
Commands::Done { task_code } => commands::done::run(config, task_code).await,
```

`commands/mod.rs`'e:

```rust
pub mod done;
```

---

## 7. CLI: `commands/done.rs` (Yeni Dosya)

```rust
use colored::Colorize;
use deepcanvas_core::{
    ApiClient, Config, DeepError, active_task, token,
    types::TaskCompleteResponse,
};
use super::tasks::resolve_project;

pub async fn run(config: Config, task_code: Option<String>) -> Result<(), DeepError> {
    let cwd = std::env::current_dir()?;

    let code = match task_code {
        Some(c) => c.to_uppercase(),
        None => active_task::read(&cwd)?
            .ok_or(DeepError::NoActiveTask)?
            .to_uppercase(),
    };

    if !is_valid_task_code(&code) {
        return Err(DeepError::InvalidTaskCode(code));
    }

    let project = resolve_project(None)?;
    let token = token::load()?.ok_or(DeepError::NotAuthenticated)?;
    let client = ApiClient::new(config).with_token(token);

    let path = format!(
        "/cli/tasks/{}/complete?org={}&project={}",
        code,
        urlencoding::encode(&project.organization_slug),
        urlencoding::encode(&project.project_slug),
    );

    let response: TaskCompleteResponse = client.post(&path, &serde_json::json!({})).await?;

    // Clear active task marker if this was the active one
    if let Ok(Some(active)) = active_task::read(&cwd) {
        if active == code {
            active_task::clear(&cwd)?;
        }
    }

    println!();
    println!("{} Completed {}: {}",
        "✓".green().bold(),
        response.task.code.bold(),
        response.task.title);
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
```

---

## 8. CLI: `commands/pull.rs` — Active Task Yazma

`pull_one` fonksiyonunun **sonuna** (return Ok(()) öncesi) ekle:

```rust
// Mark this as the active task
let cwd = std::env::current_dir()?;
deepcanvas_core::active_task::write(&cwd, code)?;
```

Multi-task pull'da her başarılı pull active'i overwrite eder — son işlenen task active kalır. Bu doğru davranış.

Import üstte:
```rust
use deepcanvas_core::active_task;
```

(use ifadesini gruplamak istersen mevcut `deepcanvas_core::{...}` listesine `active_task` ekle.)

---

## 9. CLI: `ui.rs` — Yeni Hata Hint'i

`print_error` match'ine ekle:

```rust
DeepError::NoActiveTask => {
    eprintln!("  Pull a task first: {}", "deep pull <code>".cyan());
    eprintln!("  Or pass a code:    {}", "deep done <code>".cyan());
}
```

---

## 10. Test Script: `scripts/test-done.sh` (Yeni)

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -ge 1 ]; then
    cargo run --quiet --bin deep -- done "$1"
else
    cargo run --quiet --bin deep -- done
fi
```

---

## 11. Yeni Acceptance Criteria

- `deep pull DC-142` `.deep/active` dosyasına `DC-142` yazar.
- `deep pull DC-1 DC-2 DC-3` sonrası `.deep/active` = `DC-3` (son işlenen).
- `deep done` (argümansız) `.deep/active`'i okur ve o task'ı complete eder.
- `deep done` `.deep/active` yoksa `NoActiveTask` hatası.
- `deep done DC-156` explicit kod, `.deep/active`'i kullanmaz.
- Başarılı complete sonrası `.deep/active` siliniyorsa yalnız tamamlanan task active'di.
- Backend 409 (already done) hatası kullanıcıya gösterilir.
- `.deep/active` `.gitignore` kapsamında — yeni `deep init` veya manuel ekleme.

---

## 12. UX Akışı

```
$ deep pull DC-142
  ✓ Fetched DC-142: Google OAuth callback handler
  → .deep/DC-142/task.md
  → .deep/DC-142/AUTH-PRD.md

# ... çalış ...

$ deep done
  ✓ Completed DC-142: Google OAuth callback handler

$ deep done
  error: no active task. Pull a task first or pass a code: deep done <code>
    Pull a task first: deep pull <code>
    Or pass a code:    deep done <code>

$ deep done DC-156
  ✓ Completed DC-156: Brief auto-save debounce
```