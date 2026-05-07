CREATE TABLE IF NOT EXISTS settings_app (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS exchange_credentials (
  credential_id TEXT PRIMARY KEY,
  exchange TEXT NOT NULL,
  api_key_masked TEXT NOT NULL,
  permission_read INTEGER NOT NULL DEFAULT 0,
  permission_trade INTEGER NOT NULL DEFAULT 0,
  permission_withdraw INTEGER NOT NULL DEFAULT 0,
  ip_whitelist_detected INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  last_checked_at TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  message TEXT NOT NULL,
  started_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  ended_at TEXT,
  duration_ms INTEGER,
  input_params_json TEXT,
  result_summary TEXT,
  error_details TEXT
);
