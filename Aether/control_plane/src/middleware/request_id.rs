//! Request identity — generates and propagates `X-Request-ID`.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;
use uuid::Uuid;

pub static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let incoming = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let id = incoming.unwrap_or_else(|| Uuid::new_v4().to_string());
    req.extensions_mut().insert(RequestId(id.clone()));

    let span = tracing::info_span!("request", request_id = %id);
    let mut response = next.run(req).instrument(span).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER.clone(), value);
    }
    response
}

/// Extract request id from extensions, or generate a fallback.
pub fn request_id_from_extensions(ext: &axum::http::Extensions) -> String {
    ext.get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
