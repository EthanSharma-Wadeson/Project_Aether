//! Durable dry-run attestation store.

use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use super::errors::AttestationError;
use super::model::{
    AttestationBinding, AttestationStatus, CreateAttestationRequest, DryRunAttestation,
    DEFAULT_ATTESTATION_TTL,
};
use crate::execution::types::{ProtocolOperationKind, SimulationOutcome};

/// SQLite-backed immutable dry-run attestation store.
#[derive(Clone)]
pub struct AttestationStore {
    pool: SqlitePool,
}

impl AttestationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Persist a new executable attestation.
    ///
    /// Rejects missing hash, failed simulation, unknown operation, or duplicate `dry_run_id`.
    /// No update path for immutable fields after insert.
    pub async fn create_attestation(
        &self,
        request: CreateAttestationRequest,
    ) -> Result<DryRunAttestation, AttestationError> {
        validate_create(&request)?;

        let now = Utc::now();
        let ttl = request.ttl.unwrap_or(DEFAULT_ATTESTATION_TTL);
        let expires_at = now + ttl;
        let simulation_result = simulation_to_json(&request.simulation);
        let status = AttestationStatus::Executable;

        let result = sqlx::query(
            r#"
            INSERT INTO dry_run_attestations (
                dry_run_id, operation_intent, policy_id, policy_version,
                execution_hash, simulation_result, protocol_operation_kind,
                predicted_changes, created_at, expires_at, created_by,
                audit_reference, status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.dry_run_id)
        .bind(&request.operation_intent)
        .bind(&request.policy_id)
        .bind(request.policy_version)
        .bind(&request.execution_hash)
        .bind(simulation_result.to_string())
        .bind(request.protocol_operation_kind.as_str())
        .bind(request.predicted_changes.to_string())
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(&request.created_by)
        .bind(&request.audit_reference)
        .bind(status.as_str())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(DryRunAttestation {
                dry_run_id: request.dry_run_id,
                operation_intent: request.operation_intent,
                policy_id: request.policy_id,
                policy_version: request.policy_version,
                execution_hash: request.execution_hash,
                simulation_result,
                protocol_operation_kind: request.protocol_operation_kind,
                predicted_changes: request.predicted_changes,
                created_at: now,
                expires_at,
                created_by: request.created_by,
                audit_reference: request.audit_reference,
                status,
            }),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                Err(AttestationError::DuplicateDryRunId {
                    dry_run_id: request.dry_run_id,
                })
            }
            Err(e) => Err(AttestationError::Database(e.to_string())),
        }
    }

    /// Load attestation by id (any status). Does not mutate.
    pub async fn get(
        &self,
        dry_run_id: &str,
    ) -> Result<Option<DryRunAttestation>, AttestationError> {
        fetch(self.pool(), dry_run_id).await
    }

    /// Return a still-valid executable attestation, enforcing binding fields.
    ///
    /// Automatically transitions `executable` → `expired` when past `expires_at`.
    pub async fn get_valid_attestation(
        &self,
        binding: &AttestationBinding,
    ) -> Result<DryRunAttestation, AttestationError> {
        let mut record =
            self.get(&binding.dry_run_id)
                .await?
                .ok_or_else(|| AttestationError::NotFound {
                    dry_run_id: binding.dry_run_id.clone(),
                })?;

        if record.policy_version != binding.policy_version {
            return Err(AttestationError::PolicyVersionMismatch {
                expected: binding.policy_version,
                actual: record.policy_version,
            });
        }
        if record.execution_hash != binding.execution_hash {
            return Err(AttestationError::ExecutionHashMismatch);
        }
        if record.operation_intent != binding.operation_intent {
            return Err(AttestationError::IntentMismatch);
        }

        match record.status {
            AttestationStatus::Invalidated => {
                return Err(AttestationError::Invalidated {
                    dry_run_id: binding.dry_run_id.clone(),
                });
            }
            AttestationStatus::Expired => {
                return Err(AttestationError::Expired {
                    dry_run_id: binding.dry_run_id.clone(),
                });
            }
            AttestationStatus::Executable => {}
        }

        if Utc::now() >= record.expires_at {
            self.mark_expired(&binding.dry_run_id).await?;
            return Err(AttestationError::Expired {
                dry_run_id: binding.dry_run_id.clone(),
            });
        }

        // Re-fetch after potential status change path above (none when still valid).
        if let Some(fresh) = self.get(&binding.dry_run_id).await? {
            record = fresh;
        }
        Ok(record)
    }

    /// Controlled invalidation — only status may change; hash/policy fields stay immutable.
    pub async fn invalidate(
        &self,
        dry_run_id: &str,
        reason: Option<&str>,
    ) -> Result<DryRunAttestation, AttestationError> {
        let existing = self
            .get(dry_run_id)
            .await?
            .ok_or_else(|| AttestationError::NotFound {
                dry_run_id: dry_run_id.to_string(),
            })?;

        if existing.status == AttestationStatus::Invalidated {
            return Ok(existing);
        }

        let _ = reason;
        let rows = sqlx::query(
            r#"
            UPDATE dry_run_attestations
            SET status = 'invalidated'
            WHERE dry_run_id = ?
              AND status IN ('executable', 'expired')
            "#,
        )
        .bind(dry_run_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AttestationError::Database(e.to_string()))?
        .rows_affected();

        if rows == 0 {
            return self
                .get(dry_run_id)
                .await?
                .ok_or_else(|| AttestationError::NotFound {
                    dry_run_id: dry_run_id.to_string(),
                });
        }

        self.get(dry_run_id)
            .await?
            .ok_or_else(|| AttestationError::NotFound {
                dry_run_id: dry_run_id.to_string(),
            })
    }

    async fn mark_expired(&self, dry_run_id: &str) -> Result<(), AttestationError> {
        sqlx::query(
            r#"
            UPDATE dry_run_attestations
            SET status = 'expired'
            WHERE dry_run_id = ? AND status = 'executable'
            "#,
        )
        .bind(dry_run_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AttestationError::Database(e.to_string()))?;
        Ok(())
    }
}

fn validate_create(request: &CreateAttestationRequest) -> Result<(), AttestationError> {
    if request.execution_hash.trim().is_empty() || request.execution_hash.len() != 64 {
        return Err(AttestationError::MissingExecutionHash);
    }
    if !request.simulation.executable {
        return Err(AttestationError::SimulationFailed {
            reason: request
                .simulation
                .reason
                .clone()
                .unwrap_or_else(|| "simulation not executable".into()),
        });
    }
    if matches!(
        request.protocol_operation_kind,
        ProtocolOperationKind::Unknown
    ) {
        return Err(AttestationError::UnknownOperation);
    }
    if request.dry_run_id.trim().is_empty() {
        return Err(AttestationError::Database("dry_run_id required".into()));
    }
    if request.policy_id.trim().is_empty() {
        return Err(AttestationError::Database("policy_id required".into()));
    }
    if request.policy_version < 1 {
        return Err(AttestationError::Database(
            "policy_version must be >= 1".into(),
        ));
    }
    if request.operation_intent.trim().is_empty() {
        return Err(AttestationError::Database(
            "operation_intent required".into(),
        ));
    }
    Ok(())
}

fn simulation_to_json(sim: &SimulationOutcome) -> Value {
    json!({
        "executable": sim.executable,
        "reason": sim.reason,
    })
}

async fn fetch(
    pool: &SqlitePool,
    dry_run_id: &str,
) -> Result<Option<DryRunAttestation>, AttestationError> {
    let row = sqlx::query(
        r#"
        SELECT dry_run_id, operation_intent, policy_id, policy_version,
               execution_hash, simulation_result, protocol_operation_kind,
               predicted_changes, created_at, expires_at, created_by,
               audit_reference, status
        FROM dry_run_attestations
        WHERE dry_run_id = ?
        "#,
    )
    .bind(dry_run_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AttestationError::Database(e.to_string()))?;

    row.map(map_row).transpose()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<DryRunAttestation, AttestationError> {
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let status = AttestationStatus::parse(&status_str)
        .ok_or_else(|| AttestationError::Database(format!("unknown status: {status_str}")))?;

    let kind_str: String = row.try_get("protocol_operation_kind").map_err(db_err)?;
    let protocol_operation_kind = parse_kind(&kind_str)?;

    let sim_raw: String = row.try_get("simulation_result").map_err(db_err)?;
    let predicted_raw: String = row.try_get("predicted_changes").map_err(db_err)?;
    let created_at: String = row.try_get("created_at").map_err(db_err)?;
    let expires_at: String = row.try_get("expires_at").map_err(db_err)?;

    Ok(DryRunAttestation {
        dry_run_id: row.try_get("dry_run_id").map_err(db_err)?,
        operation_intent: row.try_get("operation_intent").map_err(db_err)?,
        policy_id: row.try_get("policy_id").map_err(db_err)?,
        policy_version: row.try_get("policy_version").map_err(db_err)?,
        execution_hash: row.try_get("execution_hash").map_err(db_err)?,
        simulation_result: serde_json::from_str(&sim_raw).unwrap_or(Value::Null),
        protocol_operation_kind,
        predicted_changes: serde_json::from_str(&predicted_raw).unwrap_or(Value::Null),
        created_at: parse_ts(&created_at),
        expires_at: parse_ts(&expires_at),
        created_by: row.try_get("created_by").map_err(db_err)?,
        audit_reference: row.try_get("audit_reference").map_err(db_err)?,
        status,
    })
}

fn parse_kind(s: &str) -> Result<ProtocolOperationKind, AttestationError> {
    match s {
        "CapabilityGrant" => Ok(ProtocolOperationKind::CapabilityGrant),
        "CapabilityRevoke" => Ok(ProtocolOperationKind::CapabilityRevoke),
        "FreezeIdentity" => Ok(ProtocolOperationKind::FreezeIdentity),
        "PolicyApply" => Ok(ProtocolOperationKind::PolicyApply),
        "Unknown" => Ok(ProtocolOperationKind::Unknown),
        other => Err(AttestationError::Database(format!(
            "unknown protocol operation kind: {other}"
        ))),
    }
}

fn parse_ts(s: &str) -> chrono::DateTime<Utc> {
    s.parse().unwrap_or_else(|_| Utc::now())
}

fn db_err(e: impl std::fmt::Display) -> AttestationError {
    AttestationError::Database(e.to_string())
}
