#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyStats {
    pub strategy_id: String,
    pub total_signals: u32,
    pub buy_count: u32,
    pub sell_count: u32,
    pub neutral_count: u32,
    pub avg_score: f64,
    pub last_generated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunRecord {
    pub id: u32,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub symbols_scanned: u32,
    pub signals_found: u32,
    pub duration_ms: Option<u32>,
    pub status: String,
}
