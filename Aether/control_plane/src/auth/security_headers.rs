//! Browser security headers for Control Plane responses.

use axum::extract::Request;
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

/// CSP compatible with the Vite-built SPA (hashed assets under `/assets`).
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'";

pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    insert(headers, header::CONTENT_SECURITY_POLICY, CSP);
    insert(headers, header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    insert(headers, header::X_FRAME_OPTIONS, "DENY");
    insert(
        headers,
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin",
    );

    response
}

fn insert(headers: &mut axum::http::HeaderMap, name: header::HeaderName, value: &'static str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(name, v);
    }
}
