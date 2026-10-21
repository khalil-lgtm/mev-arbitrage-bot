//! Turso-backed PnL tracking and bundle execution log.
//!
//! Uses the remote libSQL client so data persists outside the Vercel runtime filesystem.
//! All wei values remain TEXT and are summed in Rust as i128 to avoid SQLite i64 overflow.

use eyre::{eyre, Result};
use libsql::{Builder, Connection};
use std::env;

/// Remote Turso database wrapper.
pub struct Database {
    db: libsql::Database,
}

impl Database {
    /// Connect to the configured Turso database and ensure the schema exists.
    pub async fn open() -> Result<Self> {
        let url = env::var("TURSO_DATABASE_URL")
            .map_err(|_| eyre!("TURSO_DATABASE_URL must be configured"))?;
        let token = env::var("TURSO_AUTH_TOKEN")
            .map_err(|_| eyre!("TURSO_AUTH_TOKEN must be configured"))?;

        if url.trim().is_empty() || token.trim().is_empty() {
            return Err(eyre!("TURSO_DATABASE_URL and TURSO_AUTH_TOKEN must not be empty"));
        }

        let db = Builder::new_remote(url, token).build().await?;
        let conn = db.connect()?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS bundle_logs (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp         DATETIME DEFAULT CURRENT_TIMESTAMP,
                target_tx_hash    TEXT NOT NULL,
                target_block      INTEGER NOT NULL,
                status            TEXT NOT NULL,
                miner_reward_wei  TEXT NOT NULL DEFAULT '0',
                net_profit_wei    TEXT NOT NULL DEFAULT '0',
                gas_used          INTEGER NOT NULL DEFAULT 0,
                route_hops        INTEGER NOT NULL DEFAULT 0,
                base_token        TEXT NOT NULL DEFAULT ''
            );

            CREATE INDEX IF NOT EXISTS idx_bundle_logs_timestamp
                ON bundle_logs(timestamp);

            CREATE INDEX IF NOT EXISTS idx_bundle_logs_status
                ON bundle_logs(status);",
        ).await?;

        tracing::info!("Turso database connected and schema initialized");
        Ok(Self { db })
    }

    fn connect(&self) -> Result<Connection> {
        Ok(self.db.connect()?)
    }

    /// Log a bundle submission result.
    #[allow(clippy::too_many_arguments)]
    pub async fn log_bundle(
        &self,
        tx_hash: &str,
        block: u64,
        status: &str,
        miner_reward_wei: u128,
        net_profit_wei: i128,
        gas_used: u64,
        route_hops: usize,
        base_token: &str,
    ) {
        let result = async {
            let conn = self.connect()?;
            conn.execute(
                "INSERT INTO bundle_logs
                    (target_tx_hash, target_block, status, miner_reward_wei, net_profit_wei, gas_used, route_hops, base_token)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                libsql::params![
                    tx_hash,
                    block as i64,
                    status,
                    miner_reward_wei.to_string(),
                    net_profit_wei.to_string(),
                    gas_used as i64,
                    route_hops as i64,
                    base_token,
                ],
            ).await?;
            Ok::<(), eyre::Report>(())
        }.await;

        if let Err(e) = result {
            tracing::error!(error = %e, "Failed to log bundle to Turso");
        }
    }

    /// Returns the net PnL (in wei) over a rolling time window.
    pub async fn rolling_pnl(&self, window_minutes: u64) -> i128 {
        let result = async {
            let conn = self.connect()?;
            let mut rows = conn.query(
                "SELECT net_profit_wei
                 FROM bundle_logs
                 WHERE timestamp >= datetime('now', ?1)",
                libsql::params![format!("-{window_minutes} minutes")],
            ).await?;

            let mut total = 0_i128;
            while let Some(row) = rows.next().await? {
                let value: String = row.get(0)?;
                total = total.saturating_add(value.parse::<i128>().unwrap_or(0));
            }
            Ok::<i128, eyre::Report>(total)
        }.await;

        match result {
            Ok(total) => total,
            Err(e) => {
                tracing::error!(error = %e, "Failed to read rolling PnL from Turso");
                0
            }
        }
    }

    /// Returns aggregate statistics for the Telegram health report.
    pub async fn stats_summary(&self) -> (u64, u64, i128) {
        let result = async {
            let conn = self.connect()?;

            let mut submitted_rows = conn.query(
                "SELECT COUNT(*) FROM bundle_logs", ()
            ).await?;
            let total_submitted = match submitted_rows.next().await? {
                Some(row) => row.get::<i64>(0)?.max(0) as u64,
                None => 0,
            };

            let mut included_rows = conn.query(
                "SELECT COUNT(*) FROM bundle_logs WHERE status = 'INCLUDED'", ()
            ).await?;
            let total_included = match included_rows.next().await? {
                Some(row) => row.get::<i64>(0)?.max(0) as u64,
                None => 0,
            };

            let mut pnl_rows = conn.query(
                "SELECT net_profit_wei FROM bundle_logs", ()
            ).await?;
            let mut total_pnl = 0_i128;
            while let Some(row) = pnl_rows.next().await? {
                let value: String = row.get(0)?;
                total_pnl = total_pnl.saturating_add(value.parse::<i128>().unwrap_or(0));
            }

            Ok::<(u64, u64, i128), eyre::Report>((total_submitted, total_included, total_pnl))
        }.await;

        match result {
            Ok(stats) => stats,
            Err(e) => {
                tracing::error!(error = %e, "Failed to read database statistics from Turso");
                (0, 0, 0)
            }
        }
    }
}
