pub mod audit;
pub mod operators;
pub mod policies;
pub mod refresh_tokens;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::config::Config;
use crate::error::Result;

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn connect(db_path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS operators (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS refresh_tokens (
                id TEXT PRIMARY KEY NOT NULL,
                operator_id TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                revoked INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(operator_id) REFERENCES operators(id)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY NOT NULL,
                operator_id TEXT,
                action TEXT NOT NULL,
                target TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mutation_audit (
                id TEXT PRIMARY KEY NOT NULL,
                request_id TEXT NOT NULL,
                operator_id TEXT NOT NULL,
                role TEXT NOT NULL,
                action TEXT NOT NULL,
                target TEXT,
                requested_at TEXT NOT NULL,
                approved_at TEXT,
                signer_identity TEXT,
                protocol_result TEXT NOT NULL,
                failure_reason TEXT,
                payload_hash TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS policy_templates (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                target_agent_id TEXT,
                policy_type TEXT NOT NULL,
                policy_data TEXT NOT NULL,
                status TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                submitted_by TEXT,
                submitted_at TEXT,
                approved_by TEXT,
                approved_at TEXT,
                rejected_by TEXT,
                rejected_at TEXT,
                rejection_reason TEXT,
                version INTEGER NOT NULL DEFAULT 1,
                hash TEXT NOT NULL,
                FOREIGN KEY(created_by) REFERENCES operators(id)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS policy_template_versions (
                id TEXT PRIMARY KEY NOT NULL,
                policy_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                target_agent_id TEXT,
                policy_type TEXT NOT NULL,
                policy_data TEXT NOT NULL,
                status TEXT NOT NULL,
                hash TEXT NOT NULL,
                changed_by TEXT NOT NULL,
                changed_at TEXT NOT NULL,
                change_action TEXT NOT NULL,
                FOREIGN KEY(policy_id) REFERENCES policy_templates(id)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signer_audit (
                id TEXT PRIMARY KEY NOT NULL,
                action TEXT NOT NULL,
                request_id TEXT NOT NULL,
                operation_id TEXT NOT NULL,
                signer_identity TEXT NOT NULL,
                operator_id TEXT NOT NULL,
                policy_id TEXT,
                outcome TEXT NOT NULL,
                failure_reason TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signed_operation_replay (
                operation_id TEXT PRIMARY KEY NOT NULL,
                execution_hash TEXT NOT NULL,
                dry_run_id TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN (
                    'reserved', 'executing', 'executed', 'rejected', 'stuck', 'aborted'
                )),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                reserved_at TEXT NOT NULL,
                terminal_reason TEXT,
                audit_reference TEXT,
                finalised_at TEXT
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_replay_status_reserved_at
            ON signed_operation_replay(status, reserved_at);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS dry_run_attestations (
                dry_run_id TEXT PRIMARY KEY NOT NULL,
                operation_intent TEXT NOT NULL,
                policy_id TEXT NOT NULL,
                policy_version INTEGER NOT NULL,
                execution_hash TEXT NOT NULL CHECK(length(execution_hash) = 64),
                simulation_result TEXT NOT NULL,
                protocol_operation_kind TEXT NOT NULL,
                predicted_changes TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                created_by TEXT NOT NULL,
                audit_reference TEXT,
                status TEXT NOT NULL CHECK(status IN (
                    'executable', 'expired', 'invalidated'
                ))
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dry_run_policy
            ON dry_run_attestations(policy_id);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_dry_run_expires_at
            ON dry_run_attestations(expires_at);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS apply_approvals (
                approval_id TEXT PRIMARY KEY NOT NULL,
                dry_run_id TEXT NOT NULL,
                execution_hash TEXT NOT NULL CHECK(length(execution_hash) = 64),
                policy_id TEXT NOT NULL,
                policy_version INTEGER NOT NULL,
                operation_intent TEXT NOT NULL,
                approved_by TEXT NOT NULL,
                approved_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN (
                    'active', 'expired', 'cancelled', 'consumed'
                )),
                consumed_at TEXT,
                cancelled_at TEXT,
                audit_reference TEXT,
                request_id TEXT NOT NULL,
                approver_role TEXT NOT NULL,
                consumed_by_operation_id TEXT,
                cancelled_by TEXT
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_apply_approval_dry_run
            ON apply_approvals(dry_run_id);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_apply_approval_policy_status
            ON apply_approvals(policy_id, status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_apply_approval_status
            ON apply_approvals(status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_apply_approval_expires_at
            ON apply_approvals(expires_at);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_apply_approval_active
            ON apply_approvals(policy_id, dry_run_id, execution_hash)
            WHERE status = 'active';
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signed_operations (
                operation_id TEXT PRIMARY KEY NOT NULL,
                approval_id TEXT NOT NULL,
                dry_run_id TEXT NOT NULL,
                execution_hash TEXT NOT NULL CHECK(length(execution_hash) = 64),
                payload_hash TEXT NOT NULL,
                signature TEXT NOT NULL,
                signer_id TEXT NOT NULL,
                signer_role TEXT NOT NULL,
                signer_identity TEXT NOT NULL,
                signed_at TEXT NOT NULL,
                purpose TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN (
                    'prepared', 'valid', 'expired'
                )),
                policy_id TEXT NOT NULL,
                policy_version INTEGER NOT NULL,
                operation_intent TEXT NOT NULL,
                signature_algorithm TEXT NOT NULL,
                request_id TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signed_ops_approval
            ON signed_operations(approval_id);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signed_ops_dry_run
            ON signed_operations(dry_run_id);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signed_ops_status
            ON signed_operations(status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signed_ops_expires_at
            ON signed_operations(expires_at);
            "#,
        )
        .execute(self.pool())
        .await?;

        // Phase 21 — treasury write governance (CP metadata; ledger stays in aether-treasury)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS treasury_mutation_requests (
                mutation_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                operation TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                target TEXT,
                requested_by TEXT NOT NULL,
                requester_role TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN (
                    'pending', 'approved', 'rejected', 'cancelled',
                    'executing', 'executed', 'failed'
                )),
                approval_id TEXT,
                approved_by TEXT,
                approved_at TEXT,
                executed_at TEXT,
                journal_batch_id TEXT,
                outcome TEXT,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                exec_idempotency_key TEXT
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_mutation_idempotency
            ON treasury_mutation_requests(organisation_id, idempotency_key);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_treasury_mutation_status
            ON treasury_mutation_requests(organisation_id, status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS treasury_mutation_approvals (
                approval_id TEXT PRIMARY KEY NOT NULL,
                mutation_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                approved_by TEXT NOT NULL,
                approver_role TEXT NOT NULL,
                decision TEXT NOT NULL,
                reason TEXT,
                payload_hash TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN (
                    'active', 'consumed', 'expired', 'cancelled'
                )),
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                consumed_at TEXT,
                request_id TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS treasury_mutation_audit (
                id TEXT PRIMARY KEY NOT NULL,
                mutation_id TEXT,
                organisation_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                actor TEXT NOT NULL,
                operation TEXT NOT NULL,
                target TEXT,
                approval_state TEXT,
                journal_reference TEXT,
                outcome TEXT NOT NULL,
                metadata_json TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_treasury_mutation_audit_org
            ON treasury_mutation_audit(organisation_id, created_at);
            "#,
        )
        .execute(self.pool())
        .await?;

        // Phase 28 — lab agent registry & runtime sessions (CP-local; not PROTO-0).
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_organisations (
                organisation_id TEXT PRIMARY KEY NOT NULL,
                status TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_agents (
                agent_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                provider_type TEXT NOT NULL,
                model_identifier TEXT,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT NOT NULL,
                UNIQUE(organisation_id, display_name)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_runtime_agents_org
            ON runtime_agents(organisation_id, status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_sessions (
                session_id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                issued_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                request_counter INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                credential_id TEXT NOT NULL,
                FOREIGN KEY(agent_id) REFERENCES runtime_agents(agent_id)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_runtime_sessions_agent
            ON runtime_sessions(agent_id, status);
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_credential_jti (
                jti TEXT PRIMARY KEY NOT NULL,
                credential_id TEXT NOT NULL,
                session_id TEXT,
                organisation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                consumed_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        // Phase 30 — lab tool registry & decision gateway (no execution).
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_tools (
                tool_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                risk_level TEXT NOT NULL,
                required_capability TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT NOT NULL,
                PRIMARY KEY (organisation_id, tool_id)
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tool_request_replay (
                request_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                parameters_hash TEXT NOT NULL,
                decision_code TEXT NOT NULL,
                decision_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        // Phase 31 — agent runtime sandbox (simulated lifecycle only).
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sandbox_agents (
                sandbox_key TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                runtime_type TEXT NOT NULL,
                status TEXT NOT NULL,
                scenario TEXT NOT NULL,
                simulated_budget_minor INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sandbox_outcomes (
                request_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                decision_code TEXT NOT NULL,
                reason TEXT NOT NULL,
                simulated_result TEXT NOT NULL,
                review_event_id TEXT,
                evidence_json TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sandbox_reviews (
                review_id TEXT PRIMARY KEY NOT NULL,
                request_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        // Phase 32 — lab provider session governors (no live providers).
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS provider_session_governors (
                session_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                action_count INTEGER NOT NULL DEFAULT 0,
                consecutive_denies INTEGER NOT NULL DEFAULT 0,
                cancelled INTEGER NOT NULL DEFAULT 0,
                circuit_breaker TEXT NOT NULL,
                max_actions INTEGER NOT NULL,
                deny_storm_threshold INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(self.pool())
        .await?;

        Ok(())
    }

    pub async fn seed_default_operators(&self, config: &Config) -> Result<()> {
        use operators::{create_operator, find_by_username, OperatorRole};

        if find_by_username(self.pool(), "admin").await?.is_none() {
            let password = std::env::var("CP_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into());
            create_operator(self.pool(), "admin", &password, OperatorRole::Admin).await?;
            tracing::warn!(
                "seeded default admin operator (change CP_ADMIN_PASSWORD in production)"
            );
        }

        if find_by_username(self.pool(), "viewer").await?.is_none() {
            let password = std::env::var("CP_VIEWER_PASSWORD").unwrap_or_else(|_| "viewer".into());
            create_operator(self.pool(), "viewer", &password, OperatorRole::Viewer).await?;
        }

        if find_by_username(self.pool(), "operator").await?.is_none() {
            let password =
                std::env::var("CP_OPERATOR_PASSWORD").unwrap_or_else(|_| "operator".into());
            create_operator(self.pool(), "operator", &password, OperatorRole::Operator).await?;
        }

        let _ = config;
        Ok(())
    }
}
