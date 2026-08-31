//! Read-only Treasury HTTP surface (Phase 19).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use serde_json::{json, Value};

use crate::auth::middleware::AuthContext;
use crate::db::audit;
use crate::error::Result;
use crate::middleware::request_id::RequestId;
use crate::routes::AppState;
use uuid::Uuid;

fn request_id(rid: Option<Extension<RequestId>>) -> String {
    rid.map(|Extension(r)| r.0.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

async fn audit_obs(
    state: &AppState,
    ctx: &AuthContext,
    action: &str,
    target: Option<&str>,
    request_id: &str,
) -> Result<()> {
    audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        action,
        target,
        Some(json!({ "request_id": request_id })),
    )
    .await?;
    Ok(())
}

pub async fn list_treasury(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(&state, &ctx, "observation.treasury.list", None, &rid).await?;
    let overview = state.treasury.overview(&ctx).await?;
    Ok(Json(serde_json::to_value(overview)?))
}

pub async fn get_treasury(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.get",
        Some(&id),
        &rid,
    )
    .await?;
    let detail = state.treasury.get_treasury(&ctx, &id).await?;
    Ok(Json(serde_json::to_value(detail)?))
}

pub async fn treasury_allocations(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.allocations.list",
        Some(&id),
        &rid,
    )
    .await?;
    let rows = state.treasury.allocations(&ctx, &id).await?;
    Ok(Json(json!({ "allocations": rows, "read_only": true })))
}

pub async fn treasury_reservations(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.reservations.list",
        Some(&id),
        &rid,
    )
    .await?;
    let rows = state.treasury.reservations(&ctx, &id).await?;
    Ok(Json(json!({
        "reservations": rows,
        "read_only": false,
        "note": "Create via POST /api/treasury/:id/reservations (ledger hold only)."
    })))
}

pub async fn treasury_journal(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.journal.list",
        Some(&id),
        &rid,
    )
    .await?;
    let rows = state.treasury.journal(&ctx, &id).await?;
    Ok(Json(json!({
        "entries": rows,
        "immutable": true,
        "read_only": true
    })))
}

pub async fn treasury_settlements(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.settlements.list",
        Some(&id),
        &rid,
    )
    .await?;
    let rows = state.treasury.settlements(&ctx, &id).await?;
    Ok(Json(json!({
        "posts": rows,
        "read_only": true,
        "label": "treasury_settlement_post",
        "note": "Not PROTO-4 settlement evidence. See GET /api/settlements for protocol observations."
    })))
}

pub async fn treasury_security(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let rid = request_id(rid);
    audit_obs(
        &state,
        &ctx,
        "observation.treasury.security",
        Some(&id),
        &rid,
    )
    .await?;
    let view = state.treasury.security(&ctx, &id).await?;
    Ok(Json(serde_json::to_value(view)?))
}

// ── Phase 21 write surface ───────────────────────────────────────────────

use axum::extract::Query;
use axum::http::HeaderMap;
use serde::Deserialize;

use crate::security::write_guard::guard_mutation;
use crate::treasury::write::models::{
    ApproveBody, CreateMutationBody, ExecuteBody, TreasuryOperation, LEDGER_NOTICE,
};
use crate::treasury::write::TreasuryWriteError;

#[derive(Deserialize)]
pub struct MutationListQuery {
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct ReserveBody {
    pub allocation_id: String,
    pub asset_id: String,
    pub amount_minor: i64,
    pub expires_at: Option<String>,
    pub confirm: bool,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct ConfirmIdemBody {
    pub confirm: bool,
    pub idempotency_key: String,
}

#[derive(Deserialize)]
pub struct FreezeBody {
    pub reason: Option<String>,
    pub confirm: bool,
    pub idempotency_key: String,
}

pub async fn list_mutations(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<MutationListQuery>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let rows = state
        .treasury_write
        .list_mutations(&state.treasury, &ctx, q.status.as_deref())
        .await?;
    Ok(Json(json!({
        "mutations": rows,
        "ledger_notice": LEDGER_NOTICE,
    })))
}

pub async fn mutation_timeline(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    Ok(Json(
        state
            .treasury_write
            .timeline(&state.treasury, &ctx)
            .await?,
    ))
}

pub async fn get_mutation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(mutation_id): Path<String>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let m = state
        .treasury_write
        .get_mutation(&state.treasury, &ctx, &mutation_id)
        .await?;
    Ok(Json(serde_json::to_value(m).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn create_mutation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(body): Json<CreateMutationBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    crate::treasury::ops::mfa::enforce_admin_mfa_if_required(
        state.config.admin_mfa_required,
        &ctx,
        &headers,
    )?;
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), false)?;
    let resp = state
        .treasury_write
        .create_request(&state.treasury, &ctx, &request_id, body)
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn approve_mutation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(mutation_id): Path<String>,
    Json(body): Json<ApproveBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    crate::treasury::ops::mfa::enforce_admin_mfa_if_required(
        state.config.admin_mfa_required,
        &ctx,
        &headers,
    )?;
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), false)?;
    let resp = state
        .treasury_write
        .approve(&state.treasury, &ctx, &request_id, &mutation_id, body)
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn execute_mutation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(mutation_id): Path<String>,
    Json(body): Json<ExecuteBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    crate::treasury::ops::mfa::enforce_admin_mfa_if_required(
        state.config.admin_mfa_required,
        &ctx,
        &headers,
    )?;
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let resp = state
        .treasury_write
        .execute(&state.treasury, &ctx, &request_id, &mutation_id, body)
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn cancel_mutation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(mutation_id): Path<String>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), false)?;
    let resp = state
        .treasury_write
        .cancel(&state.treasury, &ctx, &request_id, &mutation_id)
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn create_reservation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
    Json(body): Json<ReserveBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let payload = json!({
        "treasury_id": id,
        "allocation_id": body.allocation_id,
        "asset_id": body.asset_id,
        "amount_minor": body.amount_minor,
        "expires_at": body.expires_at,
    });
    let resp = state
        .treasury_write
        .single_shot(
            &state.treasury,
            &ctx,
            &request_id,
            TreasuryOperation::Reserve,
            payload,
            &body.idempotency_key,
            body.confirm,
        )
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn release_reservation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(reservation_id): Path<String>,
    Json(body): Json<ConfirmIdemBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let payload = json!({ "reservation_id": reservation_id });
    let resp = state
        .treasury_write
        .single_shot(
            &state.treasury,
            &ctx,
            &request_id,
            TreasuryOperation::Release,
            payload,
            &body.idempotency_key,
            body.confirm,
        )
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn settlement_post(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(reservation_id): Path<String>,
    Json(body): Json<ConfirmIdemBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let payload = json!({ "reservation_id": reservation_id });
    let resp = state
        .treasury_write
        .single_shot(
            &state.treasury,
            &ctx,
            &request_id,
            TreasuryOperation::SettlementPost,
            payload,
            &body.idempotency_key,
            body.confirm,
        )
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn freeze_treasury(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(id): Path<String>,
    Json(body): Json<FreezeBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let payload = json!({
        "treasury_id": id,
        "reason": body.reason,
    });
    let resp = state
        .treasury_write
        .single_shot(
            &state.treasury,
            &ctx,
            &request_id,
            TreasuryOperation::Freeze,
            payload,
            &body.idempotency_key,
            body.confirm,
        )
        .await?;
    Ok(Json(serde_json::to_value(resp).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

// ── Phase 22.5 ops hardening ─────────────────────────────────────────────

use crate::auth::roles::require_admin;
use crate::treasury::ops::mfa::enforce_admin_mfa_if_required;
use crate::treasury::ops::metrics::collect_metrics;
use crate::treasury::ops::reconcile::ReconcileService;
use crate::treasury::ops::sweeper::run_sweeper_once;

#[derive(Deserialize)]
pub struct ReconcileRunBody {
    pub confirm: bool,
    #[serde(default = "default_true")]
    pub run_sweep: bool,
    #[serde(default = "default_true")]
    pub fix_mutations: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct ForceReleaseBody {
    pub reason: String,
    pub confirm: bool,
}

pub async fn ops_metrics(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    require_admin(&ctx)?;
    let m = collect_metrics(&state.treasury, state.db.pool(), &state.config).await?;
    Ok(Json(serde_json::to_value(m).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn ops_reconcile_scan(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    require_admin(&ctx)?;
    let svc = ReconcileService::new(state.db.pool().clone());
    let report = svc.scan(&state.treasury, &state.config).await?;
    Ok(Json(serde_json::to_value(report).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn ops_reconcile_run(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Json(body): Json<ReconcileRunBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    require_admin(&ctx)?;
    enforce_admin_mfa_if_required(state.config.admin_mfa_required, &ctx, &headers)?;
    if !body.confirm {
        return Err(TreasuryWriteError::BadRequest(
            "confirm: true required".into(),
        ));
    }
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let svc = ReconcileService::new(state.db.pool().clone());
    let report = svc
        .run_full(
            &state.treasury,
            &ctx,
            &state.config,
            &request_id,
            body.run_sweep,
            body.fix_mutations,
        )
        .await?;
    Ok(Json(serde_json::to_value(report).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn ops_sweep(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    require_admin(&ctx)?;
    enforce_admin_mfa_if_required(state.config.admin_mfa_required, &ctx, &headers)?;
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let report = run_sweeper_once(&state.treasury, state.db.pool(), &state.config).await;
    let _ = crate::db::audit::append(
        state.db.pool(),
        Some(&ctx.operator_id),
        "treasury.mutation.reservation_reconciled",
        None,
        Some(json!({
            "request_id": request_id,
            "kind": "manual_sweep",
            "released": report.released
        })),
    )
    .await;
    Ok(Json(serde_json::to_value(report).map_err(|e| {
        TreasuryWriteError::BadRequest(e.to_string())
    })?))
}

pub async fn ops_force_release(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    headers: HeaderMap,
    rid: Option<Extension<RequestId>>,
    Path(reservation_id): Path<String>,
    Json(body): Json<ForceReleaseBody>,
) -> std::result::Result<Json<Value>, TreasuryWriteError> {
    require_admin(&ctx)?;
    enforce_admin_mfa_if_required(state.config.admin_mfa_required, &ctx, &headers)?;
    if !body.confirm {
        return Err(TreasuryWriteError::BadRequest(
            "confirm: true required".into(),
        ));
    }
    let request_id = guard_mutation(&state, &ctx, &headers, rid.as_ref().map(|e| &e.0), true)?;
    let svc = ReconcileService::new(state.db.pool().clone());
    let out = svc
        .force_release_reservation(
            &state.treasury,
            &ctx,
            &reservation_id,
            &body.reason,
            &request_id,
        )
        .await?;
    Ok(Json(out))
}
