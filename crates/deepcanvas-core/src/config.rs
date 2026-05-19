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
