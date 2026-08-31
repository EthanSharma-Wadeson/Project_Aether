use serde_json::json;

use crate::db::Db;
use crate::error::Result;
use crate::db::audit;

pub async fn record(
    db: &Db,
    actor: &str,
    action: &str,
    agent_id: &str,
    organisation_id: &str,
    request_id: &str,
    extra: Option<serde_json::Value>,
) -> Result<()> {
    let mut meta = json!({
        "agent_id": agent_id,
        "organisation_id": organisation_id,
        "actor": actor,
        "request_id": request_id,
    });
    if let Some(serde_json::Value::Object(map)) = extra {
        if let Some(obj) = meta.as_object_mut() {
            for (k, v) in map {
                obj.insert(k, v);
            }
        }
    }
    audit::append(db.pool(), Some(actor), action, Some(agent_id), Some(meta)).await?;
    Ok(())
}
