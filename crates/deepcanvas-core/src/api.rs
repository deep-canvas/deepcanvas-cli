use crate::{config::Config, error::DeepError};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

pub struct ApiClient {
    config: Config,
    http: HttpClient,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(config: Config) -> Self {
        let http = HttpClient::builder()
            .user_agent(concat!("deepcanvas-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client build");
        Self {
            config,
            http,
            token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub async fn post<B, R>(&self, path: &str, body: &B) -> Result<R, DeepError>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let mut req = self.http.post(self.config.api_url(path)).json(body);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        handle(req.send().await?).await
    }

    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, DeepError> {
        let mut req = self.http.get(self.config.api_url(path));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        handle(req.send().await?).await
    }

    pub async fn get_long_poll<R: DeserializeOwned>(
        &self,
        path: &str,
        timeout: u64,
    ) -> Result<R, DeepError> {
        let mut req = self
            .http
            .get(self.config.api_url(path))
            .timeout(std::time::Duration::from_secs(timeout + 5));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        handle(req.send().await?).await
    }
}

async fn handle<R: DeserializeOwned>(res: reqwest::Response) -> Result<R, DeepError> {
    let status = res.status();
    if status.is_success() {
        return Ok(res.json().await?);
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(DeepError::Unauthorized);
    }
    let body: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({}));
    let message = body
        .get("detail")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error")
        .to_string();
    Err(DeepError::Api {
        status: status.as_u16(),
        message,
    })
}
