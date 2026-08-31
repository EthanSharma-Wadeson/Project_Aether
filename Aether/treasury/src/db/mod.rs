use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::error::Result;

#[derive(Clone)]
pub struct TreasuryDb {
    pool: SqlitePool,
}

impl TreasuryDb {
    pub async fn connect(db_path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn connect_in_memory() -> Result<Self> {
        // Shared cache so the pool's connections see the same in-memory database.
        Self::connect("sqlite::memory:?cache=shared").await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        // Separate Treasury database schema — no payment / custody tables.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS asset_types (
                asset_id TEXT PRIMARY KEY NOT NULL,
                scale INTEGER NOT NULL,
                class TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS treasuries (
                treasury_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                parent_treasury_id TEXT,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                closed_at TEXT,
                FOREIGN KEY(parent_treasury_id) REFERENCES treasuries(treasury_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS allocations (
                allocation_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                treasury_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                ceiling_minor INTEGER NOT NULL,
                remaining_minor INTEGER NOT NULL,
                status TEXT NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(treasury_id) REFERENCES treasuries(treasury_id),
                FOREIGN KEY(asset_id) REFERENCES asset_types(asset_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS reservations (
                reservation_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                treasury_id TEXT NOT NULL,
                allocation_id TEXT,
                asset_id TEXT NOT NULL,
                amount_minor INTEGER NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                escrow_id TEXT,
                idempotency_key TEXT NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(treasury_id) REFERENCES treasuries(treasury_id),
                FOREIGN KEY(allocation_id) REFERENCES allocations(allocation_id),
                FOREIGN KEY(asset_id) REFERENCES asset_types(asset_id),
                UNIQUE(organisation_id, idempotency_key)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS journal_batches (
                batch_id TEXT PRIMARY KEY NOT NULL,
                organisation_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                request_id TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(organisation_id, idempotency_key)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS journal_entries (
                entry_id TEXT PRIMARY KEY NOT NULL,
                batch_id TEXT NOT NULL,
                organisation_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                treasury_id TEXT,
                allocation_id TEXT,
                reservation_id TEXT,
                asset_id TEXT NOT NULL,
                account_code TEXT NOT NULL,
                debit_minor INTEGER NOT NULL DEFAULT 0,
                credit_minor INTEGER NOT NULL DEFAULT 0,
                request_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(batch_id) REFERENCES journal_batches(batch_id),
                FOREIGN KEY(asset_id) REFERENCES asset_types(asset_id),
                CHECK(debit_minor >= 0),
                CHECK(credit_minor >= 0),
                CHECK(NOT (debit_minor > 0 AND credit_minor > 0))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS account_balances (
                organisation_id TEXT NOT NULL,
                account_code TEXT NOT NULL,
                asset_id TEXT NOT NULL,
                balance_minor INTEGER NOT NULL,
                PRIMARY KEY (organisation_id, account_code, asset_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_journal_org_created
            ON journal_entries(organisation_id, created_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_reservations_status
            ON reservations(organisation_id, status);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
