use crate::error::DeepError;
use std::path::Path;

const FILE_NAME: &str = "active";

pub fn read(cwd: &Path) -> Result<Option<String>, DeepError> {
    let path = cwd.join(".deep").join(FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&path)?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
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
