use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub role: String,
    /// Synchronizer CSRF token for mutating API calls.
    pub csrf_token: String,
}

#[derive(Debug, Serialize)]
pub struct OperatorProfile {
    pub id: String,
    pub username: String,
    pub role: String,
}
