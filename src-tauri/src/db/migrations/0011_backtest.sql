CREATE TABLE IF NOT EXISTS backtest_datasets (
  dataset_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  symbols_json TEXT NOT NULL,
  start_date TEXT NOT NULL,
  end_date TEXT NOT NULL,
  interval_minutes INTEGER NOT NULL DEFAULT 30,
  total_snapshots INTEGER NOT NULL DEFAULT 0,
  fetched_count INTEGER NOT NULL DEFAULT 0,
  estimated_llm_calls INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS backtest_snapshots (
  snapshot_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES backtest_datasets(dataset_id) ON DELETE CASCADE,
  symbol TEXT NOT NULL,
  stock_name TEXT,
  captured_at TEXT NOT NULL,
  last_price REAL NOT NULL,
  high_price REAL NOT NULL,
  low_price REAL NOT NULL,
  change_24h REAL NOT NULL,
  volume_24h REAL NOT NULL,
  spread_bps REAL NOT NULL DEFAULT 3.0,
  kline_5m TEXT NOT NULL DEFAULT '[]',
  kline_1h TEXT NOT NULL DEFAULT '[]',
  kline_1d TEXT NOT NULL DEFAULT '[]',
  kline_1w TEXT NOT NULL DEFAULT '[]',
  kline_data_json TEXT NOT NULL DEFAULT '{}',
  bid_ask_json TEXT,
  stock_info TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_backtest_snapshots_dataset ON backtest_snapshots(dataset_id, symbol, captured_at);

CREATE TABLE IF NOT EXISTS backtest_fetch_failures (
  failure_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES backtest_datasets(dataset_id) ON DELETE CASCADE,
  symbol TEXT NOT NULL,
  captured_at TEXT,
  timeframe TEXT NOT NULL,
  stage TEXT NOT NULL,
  reason TEXT NOT NULL,
  error_detail TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_backtest_fetch_failures_dataset ON backtest_fetch_failures(dataset_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_backtest_fetch_failures_symbol_time ON backtest_fetch_failures(dataset_id, symbol, captured_at);

CREATE TABLE IF NOT EXISTS backtest_runs (
  backtest_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES backtest_datasets(dataset_id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  model_provider TEXT NOT NULL,
  model_name TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  risk_settings_json TEXT NOT NULL,
  max_holding_days INTEGER NOT NULL DEFAULT 7,
  total_ai_calls INTEGER NOT NULL DEFAULT 0,
  processed_ai_calls INTEGER NOT NULL DEFAULT 0,
  total_timepoints INTEGER NOT NULL DEFAULT 0,
  processed_timepoints INTEGER NOT NULL DEFAULT 0,
  total_signals INTEGER NOT NULL DEFAULT 0,
  trade_signals INTEGER NOT NULL DEFAULT 0,
  open_trades INTEGER NOT NULL DEFAULT 0,
  win_count INTEGER NOT NULL DEFAULT 0,
  loss_count INTEGER NOT NULL DEFAULT 0,
  flat_count INTEGER NOT NULL DEFAULT 0,
  total_pnl_cny REAL NOT NULL DEFAULT 0.0,
  total_pnl_percent REAL NOT NULL DEFAULT 0.0,
  max_drawdown_percent REAL NOT NULL DEFAULT 0.0,
  profit_factor REAL,
  error_message TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS backtest_signals (
  signal_id TEXT PRIMARY KEY,
  backtest_id TEXT NOT NULL REFERENCES backtest_runs(backtest_id) ON DELETE CASCADE,
  symbol TEXT NOT NULL,
  stock_name TEXT,
  captured_at TEXT NOT NULL,
  has_trade INTEGER NOT NULL DEFAULT 0,
  direction TEXT,
  confidence_score REAL,
  risk_status TEXT,
  entry_low REAL,
  entry_high REAL,
  stop_loss REAL,
  take_profit TEXT,
  amount_cny REAL,
  max_loss_cny REAL,
  rationale TEXT,
  ai_raw_output TEXT NOT NULL DEFAULT '{}',
  ai_structured_output TEXT NOT NULL DEFAULT '{}',
  result TEXT NOT NULL DEFAULT 'pending'
);

CREATE INDEX IF NOT EXISTS idx_backtest_signals_backtest ON backtest_signals(backtest_id, captured_at);
CREATE INDEX IF NOT EXISTS idx_backtest_signals_symbol ON backtest_signals(backtest_id, symbol);

CREATE TABLE IF NOT EXISTS backtest_trades (
  trade_id TEXT PRIMARY KEY,
  backtest_id TEXT NOT NULL REFERENCES backtest_runs(backtest_id) ON DELETE CASCADE,
  signal_id TEXT REFERENCES backtest_signals(signal_id) ON DELETE SET NULL,
  symbol TEXT NOT NULL,
  stock_name TEXT,
  direction TEXT NOT NULL DEFAULT 'long',
  entry_price REAL NOT NULL,
  entry_at TEXT NOT NULL,
  exit_price REAL NOT NULL,
  exit_at TEXT NOT NULL,
  exit_reason TEXT NOT NULL,
  stop_loss REAL,
  take_profit REAL,
  amount_cny REAL,
  holding_periods INTEGER NOT NULL,
  pnl_cny REAL NOT NULL,
  pnl_percent REAL NOT NULL,
  max_favorable_price REAL,
  max_adverse_price REAL
);

CREATE INDEX IF NOT EXISTS idx_backtest_trades_backtest ON backtest_trades(backtest_id, exit_at);
CREATE INDEX IF NOT EXISTS idx_backtest_trades_symbol ON backtest_trades(backtest_id, symbol);
