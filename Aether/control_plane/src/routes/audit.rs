use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::auth::middleware::AuthContext;
use crate::db::audit;
use crate::error::Result;
use crate::routes::AppState;

pub async fn list_audit(
    State(state): State<Arc<AppState>>,
    axum::Extension(ctx): axum::Extension<AuthContext>,
) -> Result<Json<serde_json::Value>> {
    audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "observation.audit.list",
        None,
        None,
    )
    .await?;
    let events = audit::list(state.db.pool(), 200).await?;
    Ok(Json(json!({ "events": events })))
}
