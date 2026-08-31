use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::operators::OperatorRole;
use crate::error::{Error, Result};

pub const JWT_ISSUER: &str = "aether-control-plane";
pub const JWT_AUDIENCE_API: &str = "cp-api";
pub const TOKEN_TYPE_ACCESS: &str = "access";
pub const TOKEN_TYPE_REFRESH: &str = "refresh";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub iss: String,
    pub aud: String,
    /// Token type: "access" or "refresh".
    pub typ: String,
    /// Unique token id — required for refresh rotation uniqueness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_access_token(
    secret: &str,
    operator_id: &str,
    username: &str,
    role: OperatorRole,
    ttl_secs: i64,
) -> Result<String> {
    issue_token(
        secret,
        operator_id,
        username,
        role,
        ttl_secs,
        TOKEN_TYPE_ACCESS,
        JWT_AUDIENCE_API,
        None,
    )
}

/// Issue a JWT refresh token (typ=refresh). The token value is persisted hashed
/// in SQLite; the JWT wrapper enforces type separation. Each refresh token
/// carries a unique `jti` so rotations within the same second never collide.
pub fn issue_refresh_token(
    secret: &str,
    operator_id: &str,
    username: &str,
    role: OperatorRole,
    ttl_secs: i64,
) -> Result<String> {
    issue_token(
        secret,
        operator_id,
        username,
        role,
        ttl_secs,
        TOKEN_TYPE_REFRESH,
        JWT_AUDIENCE_API,
        Some(Uuid::new_v4().to_string()),
    )
}

#[allow(clippy::too_many_arguments)]
fn issue_token(
    secret: &str,
    operator_id: &str,
    username: &str,
    role: OperatorRole,
    ttl_secs: i64,
    typ: &str,
    aud: &str,
    jti: Option<String>,
) -> Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: operator_id.into(),
        username: username.into(),
        role: role.as_str().into(),
        iss: JWT_ISSUER.into(),
        aud: aud.into(),
        typ: typ.into(),
        jti,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Error::from)
}

fn validation_for() -> Validation {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE_API]);
    validation
}

pub fn verify_access_token(secret: &str, token: &str) -> Result<Claims> {
    verify_typed_token(secret, token, TOKEN_TYPE_ACCESS)
}

pub fn verify_refresh_token(secret: &str, token: &str) -> Result<Claims> {
    let claims = verify_typed_token(secret, token, TOKEN_TYPE_REFRESH)?;
    if claims.jti.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        return Err(Error::Auth("refresh token missing jti".into()));
    }
    Ok(claims)
}

fn verify_typed_token(secret: &str, token: &str, expected_typ: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation_for(),
    )?;
    if data.claims.typ != expected_typ {
        return Err(Error::Auth(format!(
            "invalid token type: expected {expected_typ}"
        )));
    }
    if data.claims.iss != JWT_ISSUER {
        return Err(Error::Auth("invalid token issuer".into()));
    }
    if data.claims.aud != JWT_AUDIENCE_API {
        return Err(Error::Auth("invalid token audience".into()));
    }
    Ok(data.claims)
}

pub fn role_from_claims(claims: &Claims) -> Result<OperatorRole> {
    OperatorRole::parse(&claims.role)
}
