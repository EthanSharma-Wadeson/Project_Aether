use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::error::Result;
use crate::protocol::proto4;
use crate::routes::AppState;

pub async fn list_settlements(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>> {
    let settlements = proto4::list_settlements(&state.protocol);
    Ok(Json(json!({ "settlements": settlements })))
}
