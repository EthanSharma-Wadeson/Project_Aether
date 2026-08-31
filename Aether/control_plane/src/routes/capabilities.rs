use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::error::Result;
use crate::protocol::proto0;
use crate::routes::AppState;

pub async fn list_capabilities(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>> {
    let capabilities = proto0::list_capabilities(&state.protocol);
    Ok(Json(json!({ "capabilities": capabilities })))
}
