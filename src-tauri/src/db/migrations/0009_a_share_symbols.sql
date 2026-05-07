CREATE TABLE IF NOT EXISTS a_share_symbol_cache (
  symbol TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  market TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_a_share_symbol_cache_name ON a_share_symbol_cache(name);
