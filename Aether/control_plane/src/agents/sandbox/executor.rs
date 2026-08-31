//! Simulated execution — NEVER calls external systems.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::tools::runtime::models::{GatewayDecision, GatewayOutcome};

use super::models::SimulatedResult;

pub struct ExecutionOutcome {
    pub simulated_result: SimulatedResult,
    pub review_event_id: Option<String>,
}

/// Phase 31 behaviour: ALLOW → simulate success; DENY → stop; REVIEW → review event.
pub async fn apply_decision(
    pool: &SqlitePool,
    org: &str,
    decision: &GatewayDecision,
) -> Result<ExecutionOutcome, sqlx::Error> {
    match decision.decision {
        GatewayOutcome::Allow => Ok(ExecutionOutcome {
            simulated_result: SimulatedResult::Success,
            review_event_id: None,
        }),
        GatewayOutcome::Deny => Ok(ExecutionOutcome {
            simulated_result: SimulatedResult::Stopped,
            review_event_id: None,
        }),
        GatewayOutcome::RequiresReview => {
            let review_id = format!("sbx-rev-{}", Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO sandbox_reviews
                  (review_id, request_id, organisation_id, agent_id, tool_id, status, created_at)
                VALUES (?, ?, ?, ?, ?, 'PENDING', ?)
                "#,
            )
            .bind(&review_id)
            .bind(&decision.request_id)
            .bind(org)
            .bind(&decision.agent_id)
            .bind(&decision.tool_id)
            .bind(Utc::now().to_rfc3339())
            .execute(pool)
            .await?;
            Ok(ExecutionOutcome {
                simulated_result: SimulatedResult::ReviewPending,
                review_event_id: Some(review_id),
            })
        }
    }
}
