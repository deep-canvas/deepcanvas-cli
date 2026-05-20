use crate::error::DeepError;
use directories::ProjectDirs;
use keyring::Entry;
use std::path::{Path, PathBuf};

const SERVICE: &str = "deepcanvas-cli";
const ACCOUNT: &str = "default";
const LOCAL_FILE: &str = "credentials";

#[derive(Debug, Clone, Copy)]
pub enum TokenLocation {
    Keyring,
    File,
    Local,
}

pub fn store(token: &str) -> Result<TokenLocation, DeepError> {
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        if entry.set_password(token).is_ok() {
            // Verify read-back: unsigned dev binaries on macOS can store but
            // fail to read back across processes due to keychain ACL.
            if matches!(entry.get_password(), Ok(t) if t == token) {
                let _ = remove_file_token();
                return Ok(TokenLocation::Keyring);
            }
            let _ = entry.delete_credential();
        }
    }
    let path = token_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, token)?;
    set_mode_0600(&path)?;
    Ok(TokenLocation::File)
}

const LOCAL_KEYRING_PREFIX: &str = "keyring:";

/// Project-scoped token: store in keychain under random account, write pointer
/// to .deep/credentials. Falls back to plaintext file if keychain is unusable.
pub fn store_local(cwd: &Path, token: &str) -> Result<(), DeepError> {
    let deep_dir = cwd.join(".deep");
    std::fs::create_dir_all(&deep_dir)?;
    let path = deep_dir.join(LOCAL_FILE);

    // Try keychain-backed storage first.
    let uuid = uuid::Uuid::new_v4().to_string();
    let account = format!("local:{}", uuid);
    if let Ok(entry) = Entry::new(SERVICE, &account) {
        if entry.set_password(token).is_ok()
            && matches!(entry.get_password(), Ok(ref t) if t == token)
        {
            let pointer = format!("{}{}", LOCAL_KEYRING_PREFIX, uuid);
            std::fs::write(&path, pointer)?;
            set_mode_0600(&path)?;
            return Ok(());
        }
        // verify failed — discard partial keychain entry
        let _ = entry.delete_credential();
    }

    // Fallback: plain file (legacy / unsigned-dev path).
    std::fs::write(&path, token)?;
    set_mode_0600(&path)?;
    Ok(())
}

/// Walks parent directories from cwd looking for .deep/credentials. Resolves
/// keyring pointers transparently; falls back to legacy raw-token files.
pub fn load_local(cwd: &Path) -> Result<Option<String>, DeepError> {
    let mut cur: Option<&Path> = Some(cwd);
    while let Some(dir) = cur {
        let path = dir.join(".deep").join(LOCAL_FILE);
        if path.is_file() {
            let raw = std::fs::read_to_string(&path)?;
            let trimmed = raw.trim();
            if let Some(uuid) = trimmed.strip_prefix(LOCAL_KEYRING_PREFIX) {
                let account = format!("local:{}", uuid);
                if let Ok(entry) = Entry::new(SERVICE, &account) {
                    match entry.get_password() {
                        Ok(t) => return Ok(Some(t)),
                        Err(keyring::Error::NoEntry) => return Ok(None),
                        Err(e) => {
                            eprintln!("warning: project keychain read failed ({e})");
                            return Ok(None);
                        }
                    }
                }
                return Ok(None);
            }
            // Legacy: raw token in file.
            return Ok(Some(trimmed.to_string()));
        }
        cur = dir.parent();
    }
    Ok(None)
}

pub fn remove_local(cwd: &Path) -> Result<bool, DeepError> {
    let path = cwd.join(".deep").join(LOCAL_FILE);
    if !path.is_file() {
        return Ok(false);
    }
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Some(uuid) = raw.trim().strip_prefix(LOCAL_KEYRING_PREFIX) {
            let account = format!("local:{}", uuid);
            if let Ok(entry) = Entry::new(SERVICE, &account) {
                let _ = entry.delete_credential();
            }
        }
    }
    std::fs::remove_file(&path)?;
    Ok(true)
}

/// Load priority: project-local (.deep/credentials, parent walk) → keyring → global file.
pub fn load() -> Result<Option<String>, DeepError> {
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(t) = load_local(&cwd)? {
            return Ok(Some(t));
        }
    }
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        match entry.get_password() {
            Ok(t) => return Ok(Some(t)),
            Err(keyring::Error::NoEntry) => {}
            Err(e) => {
                eprintln!("warning: keyring read failed ({e}); trying file fallback");
            }
        }
    }
    let path = token_file_path()?;
    if path.is_file() {
        return Ok(Some(std::fs::read_to_string(&path)?.trim().to_string()));
    }
    Ok(None)
}

/// Removes global tokens (keyring + global file). Does NOT touch project-local.
pub fn remove() -> Result<bool, DeepError> {
    let mut removed = false;
    if let Ok(entry) = Entry::new(SERVICE, ACCOUNT) {
        match entry.delete_credential() {
            Ok(()) => removed = true,
            Err(keyring::Error::NoEntry) => {}
            Err(_) => {}
        }
    }
    if remove_file_token()? {
        removed = true;
    }
    Ok(removed)
}

fn remove_file_token() -> Result<bool, DeepError> {
    let path = token_file_path()?;
    if path.is_file() {
        std::fs::remove_file(&path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn token_file_path() -> Result<PathBuf, DeepError> {
    ProjectDirs::from("studio", "deepcanvas", "deep")
        .map(|d| d.config_dir().join("credentials"))
        .ok_or_else(|| {
            DeepError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not resolve config dir",
            ))
        })
}

#[cfg(unix)]
fn set_mode_0600(path: &std::path::Path) -> Result<(), DeepError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode_0600(_: &std::path::Path) -> Result<(), DeepError> {
    Ok(())
}
