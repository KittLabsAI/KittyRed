CREATE TABLE IF NOT EXISTS market_instruments (
  instrument_id TEXT PRIMARY KEY,
  exchange TEXT NOT NULL,
  exchange_symbol TEXT NOT NULL,
  symbol_normalized TEXT NOT NULL,
  market_type TEXT NOT NULL,
  base_asset TEXT NOT NULL,
  quote_asset TEXT NOT NULL,
  settle_asset TEXT,
  contract_size TEXT,
  tick_size TEXT NOT NULL,
  lot_size TEXT NOT NULL,
  min_notional TEXT,
  max_leverage INTEGER,
  status TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS market_ticker_cache (
  symbol TEXT NOT NULL,
  market_type TEXT NOT NULL,
  last_price REAL NOT NULL,
  bid_price REAL NOT NULL DEFAULT 0,
  ask_price REAL NOT NULL DEFAULT 0,
  change_24h REAL NOT NULL,
  volume_24h REAL NOT NULL,
  funding_rate REAL,
  spread_bps REAL NOT NULL,
  exchanges_json TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  stale INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (symbol, market_type)
);
