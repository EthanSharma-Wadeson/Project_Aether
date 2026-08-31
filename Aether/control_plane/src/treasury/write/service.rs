use aether_treasury::{Amount, AssetId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::auth::roles::{require_admin, require_operator, reject_viewer_writes};
use crate::db::operators::OperatorRole;
use crate::treasury::TreasuryAdapter;

use super::approval::MutationStore;
use super::audit::{append_cp_audit, append_mutation_audit, list_mutation_audit};
use super::errors::TreasuryWriteError;
use super::models::*;
use super::validation::{validate_dual_control_payload, validate_single_shot};

#[derive(Clone)]
pub struct TreasuryWriteService {
    store: MutationStore,
}

impl TreasuryWriteService {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            store: MutationStore::new(pool),
        }
    }

    pub fn store(&self) -> &MutationStore {
        &self.store
    }

    pub async fn create_request(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        request_id: &str,
        body: CreateMutationBody,
    ) -> Result<MutationResponse, TreasuryWriteError> {
        reject_viewer_writes(ctx)?;
        require_operator(ctx)?;
        if !body.operation.requires_dual_control() {
            return Err(TreasuryWriteError::BadRequest(
                "use single-shot endpoint for this operation".into(),
            ));
        }
        validate_dual_control_payload(body.operation, &body.payload)?;
        let org = adapter.caller_organisation(ctx);
        self.ensure_org_targets(adapter, org, body.operation, &body.payload)
            .await?;

        let (m, dup) = self
            .store
            .create_or_get_idempotent(org, ctx, request_id, &body)
            .await?;
        append_mutation_audit(
            self.store.pool(),
            org,
            Some(&m.mutation_id),
            request_id,
            &ctx.operator_id,
            m.operation,
            m.target.as_deref(),
            Some("pending"),
            None,
            if dup { "duplicate" } else { "requested" },
            json!({ "payload_hash": m.payload_hash }),
        )
        .await?;
        let _ = append_cp_audit(
            self.store.pool(),
            &ctx.operator_id,
            "treasury.mutation.request",
            m.target.as_deref(),
            request_id,
            json!({ "mutation_id": m.mutation_id, "operation": m.operation.as_str() }),
        )
        .await;
        let mut resp = MutationResponse::from(&m);
        resp.duplicate = dup;
        Ok(resp)
    }

    pub async fn approve(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        request_id: &str,
        mutation_id: &str,
        body: ApproveBody,
    ) -> Result<MutationResponse, TreasuryWriteError> {
        require_admin(ctx)?;
        let org = adapter.caller_organisation(ctx);
        let m = if body.decision == "reject" {
            self.store
                .reject(org, mutation_id, ctx, body.reason.as_deref(), request_id)
                .await?
        } else if body.decision == "approve" {
            self.store
                .approve(org, mutation_id, ctx, body.reason.as_deref(), request_id)
                .await?
        } else {
            return Err(TreasuryWriteError::BadRequest(
                "decision must be approve or reject".into(),
            ));
        };
        append_mutation_audit(
            self.store.pool(),
            org,
            Some(&m.mutation_id),
            request_id,
            &ctx.operator_id,
            m.operation,
            m.target.as_deref(),
            Some(m.status.as_str()),
            None,
            m.status.as_str(),
            json!({ "approval_id": m.approval_id }),
        )
        .await?;
        let action = if m.status == MutationStatus::Rejected {
            "treasury.mutation.reject"
        } else {
            "treasury.mutation.approve"
        };
        let _ = append_cp_audit(
            self.store.pool(),
            &ctx.operator_id,
            action,
            m.target.as_deref(),
            request_id,
            json!({ "mutation_id": m.mutation_id }),
        )
        .await;
        Ok(MutationResponse::from(&m))
    }

    pub async fn cancel(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        request_id: &str,
        mutation_id: &str,
    ) -> Result<MutationResponse, TreasuryWriteError> {
        reject_viewer_writes(ctx)?;
        require_operator(ctx)?;
        let org = adapter.caller_organisation(ctx);
        let m = self.store.cancel(org, mutation_id, ctx).await?;
        append_mutation_audit(
            self.store.pool(),
            org,
            Some(&m.mutation_id),
            request_id,
            &ctx.operator_id,
            m.operation,
            m.target.as_deref(),
            Some("cancelled"),
            None,
            "cancelled",
            json!({}),
        )
        .await?;
        Ok(MutationResponse::from(&m))
    }

    pub async fn execute(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        request_id: &str,
        mutation_id: &str,
        body: ExecuteBody,
    ) -> Result<ExecuteResult, TreasuryWriteError> {
        require_admin(ctx)?;
        if !body.confirm {
            return Err(TreasuryWriteError::BadRequest(
                "confirm: true required".into(),
            ));
        }
        if body.idempotency_key.trim().is_empty() {
            return Err(TreasuryWriteError::BadRequest(
                "idempotency_key required".into(),
            ));
        }
        let org = adapter.caller_organisation(ctx);
        let m = self
            .store
            .begin_execute(org, mutation_id, &body.idempotency_key)
            .await?;

        if m.status == MutationStatus::Executed {
            return Ok(ExecuteResult {
                request_id: request_id.into(),
                outcome: "success".into(),
                operation: m.operation,
                journal_batch_id: m.journal_batch_id.clone(),
                mutation_id: Some(m.mutation_id),
                resource: json!({}),
                approval_id: m.approval_id.clone(),
                duplicate: true,
                ledger_notice: LEDGER_NOTICE,
            });
        }

        match self.run_engine(adapter, org, &m, request_id).await {
            Ok((batch_id, resource)) => {
                let finished = self
                    .store
                    .finish_execute(org, mutation_id, batch_id.as_deref(), "success", true)
                    .await?;
                append_mutation_audit(
                    self.store.pool(),
                    org,
                    Some(&finished.mutation_id),
                    request_id,
                    &ctx.operator_id,
                    finished.operation,
                    finished.target.as_deref(),
                    Some("executed"),
                    batch_id.as_deref(),
                    "success",
                    json!({ "approval_id": finished.approval_id }),
                )
                .await?;
                let _ = append_cp_audit(
                    self.store.pool(),
                    &ctx.operator_id,
                    "treasury.mutation.execute",
                    finished.target.as_deref(),
                    request_id,
                    json!({
                        "mutation_id": finished.mutation_id,
                        "journal_batch_id": batch_id,
                        "outcome": "success"
                    }),
                )
                .await;
                Ok(ExecuteResult {
                    request_id: request_id.into(),
                    outcome: "success".into(),
                    operation: finished.operation,
                    journal_batch_id: batch_id,
                    mutation_id: Some(finished.mutation_id),
                    resource,
                    approval_id: finished.approval_id,
                    duplicate: false,
                    ledger_notice: LEDGER_NOTICE,
                })
            }
            Err(e) => {
                let _ = self
                    .store
                    .finish_execute(org, mutation_id, None, &e.to_string(), false)
                    .await;
                append_mutation_audit(
                    self.store.pool(),
                    org,
                    Some(mutation_id),
                    request_id,
                    &ctx.operator_id,
                    m.operation,
                    m.target.as_deref(),
                    Some("failed"),
                    None,
                    "failed",
                    json!({ "error": e.to_string() }),
                )
                .await?;
                Err(e)
            }
        }
    }

    pub async fn single_shot(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        request_id: &str,
        op: TreasuryOperation,
        payload: Value,
        idempotency_key: &str,
        confirm: bool,
    ) -> Result<ExecuteResult, TreasuryWriteError> {
        reject_viewer_writes(ctx)?;
        if op == TreasuryOperation::Freeze {
            require_admin(ctx)?;
        } else {
            require_operator(ctx)?;
        }
        if !confirm {
            return Err(TreasuryWriteError::BadRequest(
                "confirm: true required".into(),
            ));
        }
        if !op.is_single_shot() {
            return Err(TreasuryWriteError::BadRequest(
                "operation requires dual-control flow".into(),
            ));
        }
        validate_single_shot(op, &payload)?;
        let org = adapter.caller_organisation(ctx);
        self.ensure_org_targets(adapter, org, op, &payload).await?;

        // Idempotency via synthetic mutation row for single-shot.
        let body = CreateMutationBody {
            operation: op,
            payload: payload.clone(),
            idempotency_key: idempotency_key.into(),
            target: None,
        };

        // Reuse mutation table: create pending then immediately execute path for L ops
        // is awkward because create_request rejects non-dual. Use direct engine + audit
        // with idempotency table lookup on synthetic keys.
        if let Some(existing) = self
            .store
            .get_by_idempotency(org, idempotency_key)
            .await?
        {
            if existing.status == MutationStatus::Executed
                && existing.operation == op
            {
                return Ok(ExecuteResult {
                    request_id: request_id.into(),
                    outcome: "success".into(),
                    operation: op,
                    journal_batch_id: existing.journal_batch_id,
                    mutation_id: Some(existing.mutation_id),
                    resource: json!({}),
                    approval_id: None,
                    duplicate: true,
                    ledger_notice: LEDGER_NOTICE,
                });
            }
        }

        // Insert as approved+execute in one go for single-shot tracking
        let (mut m, dup) = self
            .insert_single_shot(org, ctx, request_id, &body)
            .await?;
        if dup && m.status == MutationStatus::Executed {
            return Ok(ExecuteResult {
                request_id: request_id.into(),
                outcome: "success".into(),
                operation: op,
                journal_batch_id: m.journal_batch_id,
                mutation_id: Some(m.mutation_id),
                resource: json!({}),
                approval_id: None,
                duplicate: true,
                ledger_notice: LEDGER_NOTICE,
            });
        }

        let fake = MutationRequest {
            operation: op,
            payload: payload.clone(),
            ..m.clone()
        };
        match self.run_engine(adapter, org, &fake, request_id).await {
            Ok((batch_id, resource)) => {
                m = self
                    .store
                    .finish_execute(org, &m.mutation_id, batch_id.as_deref(), "success", true)
                    .await?;
                append_mutation_audit(
                    self.store.pool(),
                    org,
                    Some(&m.mutation_id),
                    request_id,
                    &ctx.operator_id,
                    op,
                    m.target.as_deref(),
                    Some("executed"),
                    batch_id.as_deref(),
                    "success",
                    json!({ "single_shot": true }),
                )
                .await?;
                let action = format!("treasury.mutation.{}", op.as_str());
                let _ = append_cp_audit(
                    self.store.pool(),
                    &ctx.operator_id,
                    &action,
                    m.target.as_deref(),
                    request_id,
                    json!({ "journal_batch_id": batch_id, "mutation_id": m.mutation_id }),
                )
                .await;
                Ok(ExecuteResult {
                    request_id: request_id.into(),
                    outcome: "success".into(),
                    operation: op,
                    journal_batch_id: batch_id,
                    mutation_id: Some(m.mutation_id),
                    resource,
                    approval_id: None,
                    duplicate: false,
                    ledger_notice: LEDGER_NOTICE,
                })
            }
            Err(e) => {
                let _ = self
                    .store
                    .finish_execute(org, &m.mutation_id, None, &e.to_string(), false)
                    .await;
                Err(e)
            }
        }
    }

    pub async fn get_mutation(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        mutation_id: &str,
    ) -> Result<MutationResponse, TreasuryWriteError> {
        let org = adapter.caller_organisation(ctx);
        let m = self.store.get(org, mutation_id).await?;
        Ok(MutationResponse::from(&m))
    }

    pub async fn list_mutations(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
        status: Option<&str>,
    ) -> Result<Vec<MutationResponse>, TreasuryWriteError> {
        let org = adapter.caller_organisation(ctx);
        let rows = self.store.list(org, status, 100).await?;
        Ok(rows.iter().map(MutationResponse::from).collect())
    }

    pub async fn timeline(
        &self,
        adapter: &TreasuryAdapter,
        ctx: &AuthContext,
    ) -> Result<Value, TreasuryWriteError> {
        let org = adapter.caller_organisation(ctx);
        let events = list_mutation_audit(self.store.pool(), org, 200).await?;
        Ok(json!({
            "organisation_id": org,
            "events": events,
            "ledger_notice": LEDGER_NOTICE,
        }))
    }

    async fn insert_single_shot(
        &self,
        org: &str,
        ctx: &AuthContext,
        request_id: &str,
        body: &CreateMutationBody,
    ) -> Result<(MutationRequest, bool), TreasuryWriteError> {
        // Temporarily allow single-shot ops into the table as pending then flip to approved.
        if let Some(existing) = self
            .store
            .get_by_idempotency(org, &body.idempotency_key)
            .await?
        {
            return Ok((existing, true));
        }
        // Bypass dual-control check by writing directly.
        let hash = super::approval::payload_hash(&body.payload);
        let mutation_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::minutes(super::approval::DEFAULT_MUTATION_TTL_MINS);
        let target = body
            .payload
            .get("treasury_id")
            .and_then(|v| v.as_str())
            .or_else(|| body.payload.get("reservation_id").and_then(|v| v.as_str()))
            .map(str::to_string);

        sqlx::query(
            r#"
            INSERT INTO treasury_mutation_requests (
              mutation_id, organisation_id, request_id, idempotency_key, operation,
              payload_json, payload_hash, target, requested_by, requester_role, status,
              approval_id, approved_by, approved_at, executed_at, journal_batch_id,
              outcome, expires_at, created_at, exec_idempotency_key
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'executing', NULL, NULL, NULL, NULL, NULL, NULL, ?, ?, ?)
            "#,
        )
        .bind(&mutation_id)
        .bind(org)
        .bind(request_id)
        .bind(&body.idempotency_key)
        .bind(body.operation.as_str())
        .bind(body.payload.to_string())
        .bind(&hash)
        .bind(&target)
        .bind(&ctx.operator_id)
        .bind(ctx.role.as_str())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(&body.idempotency_key)
        .execute(self.store.pool())
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db) = e {
                if db.is_unique_violation() {
                    return TreasuryWriteError::Conflict("idempotency race".into());
                }
            }
            TreasuryWriteError::Unavailable(e.to_string())
        })?;

        Ok((
            MutationRequest {
                mutation_id,
                organisation_id: org.into(),
                request_id: request_id.into(),
                idempotency_key: body.idempotency_key.clone(),
                operation: body.operation,
                payload: body.payload.clone(),
                payload_hash: hash,
                target,
                requested_by: ctx.operator_id.clone(),
                requester_role: ctx.role.as_str().to_string(),
                status: MutationStatus::Executing,
                approval_id: None,
                approved_by: None,
                approved_at: None,
                executed_at: None,
                journal_batch_id: None,
                outcome: None,
                expires_at,
                created_at: now,
                exec_idempotency_key: Some(body.idempotency_key.clone()),
            },
            false,
        ))
    }

    async fn ensure_org_targets(
        &self,
        adapter: &TreasuryAdapter,
        org: &str,
        op: TreasuryOperation,
        payload: &Value,
    ) -> Result<(), TreasuryWriteError> {
        if let Some(tid) = payload.get("treasury_id").and_then(|v| v.as_str()) {
            let t = adapter
                .engine()
                .get_treasury(tid)
                .await
                .map_err(TreasuryWriteError::from)?;
            if t.organisation_id != org {
                return Err(TreasuryWriteError::NotFound("treasury not found".into()));
            }
        }
        if let Some(aid) = payload.get("allocation_id").and_then(|v| v.as_str()) {
            let a = adapter
                .engine()
                .get_allocation(aid)
                .await
                .map_err(TreasuryWriteError::from)?;
            if a.organisation_id != org {
                return Err(TreasuryWriteError::NotFound("allocation not found".into()));
            }
            let _ = op;
        }
        if let Some(rid) = payload.get("reservation_id").and_then(|v| v.as_str()) {
            let r = adapter
                .engine()
                .get_reservation(rid)
                .await
                .map_err(TreasuryWriteError::from)?;
            if r.organisation_id != org {
                return Err(TreasuryWriteError::NotFound("reservation not found".into()));
            }
        }
        Ok(())
    }

    async fn run_engine(
        &self,
        adapter: &TreasuryAdapter,
        org: &str,
        m: &MutationRequest,
        request_id: &str,
    ) -> Result<(Option<String>, Value), TreasuryWriteError> {
        let engine = adapter.engine();
        let p = &m.payload;
        let idem = m
            .exec_idempotency_key
            .as_deref()
            .unwrap_or(m.idempotency_key.as_str());

        match m.operation {
            TreasuryOperation::AllocationCreate => {
                let treasury_id = str_field(p, "treasury_id")?;
                let agent_id = str_field(p, "agent_id")?;
                let asset_id = AssetId::new(str_field(p, "asset_id")?);
                let ceiling = Amount(i64_field(p, "ceiling_minor")?);
                let initial = Amount(p.get("initial_minor").and_then(|v| v.as_i64()).unwrap_or(0));
                let expires_at = opt_rfc3339(p, "expires_at")?;
                let (alloc, batch) = engine
                    .create_allocation(
                        &treasury_id,
                        agent_id,
                        asset_id,
                        ceiling,
                        initial,
                        expires_at,
                        request_id,
                        idem,
                    )
                    .await?;
                if alloc.organisation_id != org {
                    return Err(TreasuryWriteError::NotFound("allocation not found".into()));
                }
                Ok((
                    batch.map(|b| b.batch_id),
                    json!({ "allocation_id": alloc.allocation_id, "ceiling_minor": alloc.ceiling_minor }),
                ))
            }
            TreasuryOperation::AllocationIncrease => {
                let allocation_id = str_field(p, "allocation_id")?;
                let new_ceiling = Amount(i64_field(p, "new_ceiling_minor")?);
                let fund = Amount(
                    p.get("fund_reserved_minor")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                );
                let (alloc, batch) = engine
                    .increase_allocation(&allocation_id, new_ceiling, fund, request_id, idem)
                    .await?;
                if alloc.organisation_id != org {
                    return Err(TreasuryWriteError::NotFound("allocation not found".into()));
                }
                Ok((
                    batch.map(|b| b.batch_id),
                    json!({ "allocation_id": alloc.allocation_id, "ceiling_minor": alloc.ceiling_minor }),
                ))
            }
            TreasuryOperation::AllocationDecrease => {
                let allocation_id = str_field(p, "allocation_id")?;
                let new_ceiling = Amount(i64_field(p, "new_ceiling_minor")?);
                let alloc = engine
                    .decrease_allocation(&allocation_id, new_ceiling)
                    .await?;
                if alloc.organisation_id != org {
                    return Err(TreasuryWriteError::NotFound("allocation not found".into()));
                }
                Ok((
                    None,
                    json!({ "allocation_id": alloc.allocation_id, "ceiling_minor": alloc.ceiling_minor }),
                ))
            }
            TreasuryOperation::Reserve => {
                let treasury_id = str_field(p, "treasury_id")?;
                let allocation_id = str_field(p, "allocation_id")?;
                let asset_id = AssetId::new(str_field(p, "asset_id")?);
                let amount = Amount(i64_field(p, "amount_minor")?);
                let expires_at = opt_rfc3339(p, "expires_at")?;
                let (res, batch) = engine
                    .reserve(
                        &treasury_id,
                        &allocation_id,
                        asset_id,
                        amount,
                        request_id,
                        idem,
                        expires_at,
                    )
                    .await?;
                Ok((
                    Some(batch.batch_id),
                    json!({ "reservation_id": res.reservation_id }),
                ))
            }
            TreasuryOperation::Release => {
                let reservation_id = str_field(p, "reservation_id")?;
                let (res, batch) = engine
                    .release_reservation(&reservation_id, request_id, idem)
                    .await?;
                Ok((
                    Some(batch.batch_id),
                    json!({ "reservation_id": res.reservation_id, "status": "released" }),
                ))
            }
            TreasuryOperation::SettlementPost => {
                let reservation_id = str_field(p, "reservation_id")?;
                let (res, batch) = engine
                    .settlement_post(&reservation_id, request_id, idem)
                    .await?;
                Ok((
                    Some(batch.batch_id),
                    json!({ "reservation_id": res.reservation_id, "status": "consumed" }),
                ))
            }
            TreasuryOperation::Refund => {
                let treasury_id = str_field(p, "treasury_id")?;
                let asset_id = AssetId::new(str_field(p, "asset_id")?);
                let amount = Amount(i64_field(p, "amount_minor")?);
                let batch = engine
                    .refund(&treasury_id, asset_id, amount, request_id, idem)
                    .await?;
                Ok((Some(batch.batch_id), json!({ "event": "refund" })))
            }
            TreasuryOperation::Chargeback => {
                let treasury_id = str_field(p, "treasury_id")?;
                let asset_id = AssetId::new(str_field(p, "asset_id")?);
                let amount = Amount(i64_field(p, "amount_minor")?);
                let batch = engine
                    .chargeback(&treasury_id, asset_id, amount, request_id, idem)
                    .await?;
                Ok((Some(batch.batch_id), json!({ "event": "chargeback" })))
            }
            TreasuryOperation::Adjustment => {
                let treasury_id = str_field(p, "treasury_id")?;
                let asset_id = AssetId::new(str_field(p, "asset_id")?);
                let amount = Amount(i64_field(p, "amount_minor")?);
                let batch = engine
                    .adjustment_to_suspense(&treasury_id, asset_id, amount, request_id, idem)
                    .await?;
                Ok((Some(batch.batch_id), json!({ "event": "adjustment" })))
            }
            TreasuryOperation::Freeze => {
                let treasury_id = str_field(p, "treasury_id")?;
                let t = engine.freeze_treasury(&treasury_id).await?;
                if t.organisation_id != org {
                    return Err(TreasuryWriteError::NotFound("treasury not found".into()));
                }
                Ok((
                    None,
                    json!({ "treasury_id": t.treasury_id, "status": "frozen" }),
                ))
            }
            TreasuryOperation::Unfreeze => {
                let treasury_id = str_field(p, "treasury_id")?;
                let t = engine.unfreeze_treasury(&treasury_id).await?;
                if t.organisation_id != org {
                    return Err(TreasuryWriteError::NotFound("treasury not found".into()));
                }
                Ok((
                    None,
                    json!({ "treasury_id": t.treasury_id, "status": "active" }),
                ))
            }
        }
    }
}

fn str_field(p: &Value, key: &str) -> Result<String, TreasuryWriteError> {
    p.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| TreasuryWriteError::BadRequest(format!("missing {key}")))
}

fn i64_field(p: &Value, key: &str) -> Result<i64, TreasuryWriteError> {
    p.get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| TreasuryWriteError::BadRequest(format!("missing {key}")))
}

fn opt_rfc3339(
    p: &Value,
    key: &str,
) -> Result<Option<DateTime<Utc>>, TreasuryWriteError> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&Utc)))
            .map_err(|e| TreasuryWriteError::BadRequest(e.to_string())),
        _ => Err(TreasuryWriteError::BadRequest(format!("invalid {key}"))),
    }
}

// silence unused import warning if OperatorRole not used
#[allow(dead_code)]
fn _role_rank(r: OperatorRole) -> u8 {
    match r {
        OperatorRole::Viewer => 0,
        OperatorRole::Operator => 1,
        OperatorRole::Admin => 2,
    }
}
