CREATE TABLE IF NOT EXISTS market_candle_cache (
  symbol TEXT NOT NULL,
  interval TEXT NOT NULL,
  open_time TEXT NOT NULL,
  open REAL NOT NULL,
  high REAL NOT NULL,
  low REAL NOT NULL,
  close REAL NOT NULL,
  volume REAL NOT NULL,
  turnover REAL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (symbol, interval, open_time)
);

CREATE INDEX IF NOT EXISTS idx_market_candle_cache_lookup
  ON market_candle_cache(symbol, interval, open_time);
