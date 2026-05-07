use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const MARKET_SIZE_TIER_LARGE: &str = "large";
pub const MARKET_SIZE_TIER_MID: &str = "mid";
pub const MARKET_SIZE_TIER_SMALL: &str = "small";

#[derive(Debug, Clone, PartialEq)]
pub struct AssetMetadataRecord {
    pub base_asset: String,
    pub provider: String,
    pub provider_asset_id: String,
    pub provider_symbol: String,
    pub provider_name: String,
    pub market_cap_usd: Option<f64>,
    pub market_cap_rank: Option<i64>,
    pub fetched_at: String,
    pub source_updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetMetadataFetchOutcome {
    pub rows: Vec<AssetMetadataRecord>,
    pub partial: bool,
}

pub fn base_asset_from_symbol(symbol: &str) -> String {
    symbol
        .split('/')
        .next()
        .unwrap_or(symbol)
        .trim()
        .to_ascii_uppercase()
}

pub fn classify_market_size_tier(rank: Option<i64>) -> &'static str {
    match rank {
        Some(rank) if (1..=20).contains(&rank) => MARKET_SIZE_TIER_LARGE,
        Some(rank) if (21..=100).contains(&rank) => MARKET_SIZE_TIER_MID,
        _ => MARKET_SIZE_TIER_SMALL,
    }
}

pub fn current_metadata_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

pub fn metadata_refresh_is_due(_last_fetched_at: Option<&str>, _now: &str) -> bool {
    false
}

pub async fn fetch_asset_metadata(
    _client: &reqwest::Client,
) -> anyhow::Result<AssetMetadataFetchOutcome> {
    Ok(AssetMetadataFetchOutcome {
        rows: Vec::new(),
        partial: false,
    })
}
