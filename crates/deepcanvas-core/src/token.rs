use crate::error::DeepError;
use directories::ProjectDirs;
use keyring::Entry;
use std::path::PathBuf;

const SERVICE: &str = "deepcanvas-cli";
const ACCOUNT: &str = "default";

#[derive(Debug, Clone, Copy)]
pub enum TokenLocation {
    Keyring,
    File,
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

pub fn load() -> Result<Option<String>, DeepError> {
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
