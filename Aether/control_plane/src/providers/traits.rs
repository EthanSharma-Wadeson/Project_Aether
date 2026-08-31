use async_trait::async_trait;

use super::errors::ProviderAdapterError;
use super::models::{ProviderReasonRequest, ProviderReasonResponse};

/// Untrusted inference backend. Implementations return proposals only (never authority).
#[async_trait]
pub trait Provider: Send + Sync {
    fn model_id(&self) -> &str;

    async fn reason(
        &self,
        request: &ProviderReasonRequest,
    ) -> Result<ProviderReasonResponse, ProviderAdapterError>;
}
