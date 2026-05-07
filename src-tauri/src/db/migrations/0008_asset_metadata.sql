CREATE TABLE IF NOT EXISTS market_asset_metadata (
  base_asset TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  provider_asset_id TEXT NOT NULL,
  provider_symbol TEXT NOT NULL,
  provider_name TEXT NOT NULL,
  market_cap_usd REAL,
  market_cap_rank INTEGER,
  fetched_at TEXT NOT NULL,
  source_updated_at TEXT
);
