//! Lab HTTP transport — injectable for tests; Reqwest for live lab calls.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::providers::errors::ProviderAdapterError;
use crate::providers::secrets::redact_for_audit;

#[async_trait]
pub trait LabHttpTransport: Send + Sync {
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: Value,
    ) -> Result<(u16, String), ProviderAdapterError>;
}

/// Live lab transport — never logs Authorization / x-api-key values.
pub struct ReqwestLabTransport {
    client: reqwest::Client,
}

impl ReqwestLabTransport {
    pub fn new() -> Result<Self, ProviderAdapterError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ProviderAdapterError::ProviderHttp(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl LabHttpTransport for ReqwestLabTransport {
    async fn post_json(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: Value,
    ) -> Result<(u16, String), ProviderAdapterError> {
        let mut req = self.client.post(url).json(&body);
        for (k, v) in headers {
            // Never put secrets into tracing — header values with api-key are opaque here.
            let _ = redact_for_audit(&format!("{k}=len:{}", v.len()));
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ProviderAdapterError::ProviderHttp(sanitize_http_err(&e)))?;
        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderAdapterError::ProviderHttp(sanitize_http_err(&e)))?;
        Ok((status, text))
    }
}

fn sanitize_http_err(err: &reqwest::Error) -> String {
    redact_for_audit(&err.to_string())
}

/// Test/lab scripted transport — returns queued Anthropic-shaped JSON bodies.
pub struct ScriptedLabTransport {
    responses: Mutex<VecDeque<(u16, String)>>,
}

impl ScriptedLabTransport {
    pub fn new(responses: Vec<(u16, String)>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }

    pub fn single(status: u16, body: impl Into<String>) -> Self {
        Self::new(vec![(status, body.into())])
    }
}

#[async_trait]
impl LabHttpTransport for ScriptedLabTransport {
    async fn post_json(
        &self,
        _url: &str,
        _headers: Vec<(String, String)>,
        _body: Value,
    ) -> Result<(u16, String), ProviderAdapterError> {
        let mut q = self
            .responses
            .lock()
            .map_err(|_| ProviderAdapterError::ProviderHttp("scripted lock".into()))?;
        q.pop_front()
            .ok_or_else(|| ProviderAdapterError::ProviderHttp("no scripted responses left".into()))
    }
}
