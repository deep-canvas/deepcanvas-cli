use crate::error::DeepError;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectBinding {
    pub organization_slug: String,
    pub project_slug: String,
}

#[derive(Deserialize)]
struct DeepConfigFile {
    project: ProjectBinding,
}

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
        if org.is_empty() || proj.is_empty() {
            return Err(DeepError::InvalidProjectFormat);
        }
        Ok(Self {
            organization_slug: org.to_string(),
            project_slug: proj.to_string(),
        })
    }

    pub fn to_toml(&self) -> String {
        format!(
            "[project]\norganization_slug = \"{}\"\nproject_slug = \"{}\"\n",
            self.organization_slug, self.project_slug
        )
    }
}
