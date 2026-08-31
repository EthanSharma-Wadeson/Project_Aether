use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::error::Result;
use crate::protocol::proto2;
use crate::routes::AppState;

pub async fn list_escrows(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>> {
    let escrows = proto2::list_escrows(&state.protocol);
    Ok(Json(json!({ "escrows": escrows })))
}
