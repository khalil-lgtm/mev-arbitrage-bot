//! Low-overhead exporter that appends each discovered profitable
//! opportunity as a single JSON line to a file so the static dashboard
//! (`dashboard/opportunities.json`, JSONL-friendly) can render it.
//!
//! The path is taken from `OPPS_EXPORT_FILE` (env / `.env`), defaulting to
//! `opportunities.jsonl` in the engine's working directory. Writes are
//! fire-and-forget: they never fail the search loop on disk errors.

use crate::types::ArbitrageRoute;
use alloy_primitives::U256;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// A single opportunity record, serialized to one JSONL line.
#[derive(Serialize)]
pub struct OppRecord {
    pub ts: String,
    pub tx_hash: String,
    pub protocol: String,
    pub hops: usize,
    pub base_token: String,
    pub gross_profit_eth: f64,
    pub optimal_loan_eth: f64,
    pub confidence: f64,
    pub pools: Vec<String>,
    pub net_eth: f64,
}

/// A single line writer. Serialized through a Mutex to avoid interleaved
/// lines / races across the per-opportunity spawned tasks.
static WRITER: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Configure (once) the export target. Called from startup.
pub fn init(path: impl Into<String>) {
    let mut w = WRITER.lock().unwrap();
    w.replace(PathBuf::from(path.into()));
}

/// Append one record. Silently no-ops if the writer was not initialized or
/// the disk write fails — telemetry must never break the hot path.
pub fn append(record: &OppRecord) {
    let w = WRITER.lock().unwrap();
    let Some(path) = w.as_ref() else {
        return;
    };
    let line = match serde_json::to_string(record) {
        Ok(l) => l,
        Err(_) => return,
    };
    let appended = format!("{line}\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(appended.as_bytes()));
}

/// Build a record from live engine state.
pub fn build_record(
    tx_hash: alloy_primitives::TxHash,
    protocol: &str,
    route: &ArbitrageRoute,
    gross_profit: U256,
    optimal_loan_size: U256,
    net: U256,
) -> OppRecord {
    use chrono::Utc;

    let to_eth = |v: U256| v.to::<u128>() as f64 / 1e18;

    OppRecord {
        ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        tx_hash: format!("{:?}", tx_hash),
        protocol: protocol.to_string(),
        hops: route.num_hops(),
        base_token: format!("{:?}", route.base_token),
        gross_profit_eth: to_eth(gross_profit),
        optimal_loan_eth: to_eth(optimal_loan_size),
        confidence: route.confidence,
        pools: route.legs.iter().map(|l| format!("{:?}", l.pool.address())).collect(),
        net_eth: to_eth(net),
    }
}
