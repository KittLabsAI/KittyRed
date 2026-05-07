CREATE TABLE IF NOT EXISTS account_exchange_accounts (
  account_id TEXT PRIMARY KEY,
  exchange TEXT NOT NULL,
  account_mode TEXT NOT NULL,
  status TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS account_balances (
  balance_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  asset TEXT NOT NULL,
  free_amount TEXT NOT NULL,
  locked_amount TEXT NOT NULL,
  usdt_value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
