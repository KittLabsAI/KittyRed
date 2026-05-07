CREATE TABLE IF NOT EXISTS recommendation_evaluations (
  evaluation_id TEXT PRIMARY KEY,
  recommendation_id TEXT NOT NULL,
  horizon TEXT NOT NULL,
  price_at_horizon REAL NOT NULL,
  max_favorable_price REAL NOT NULL,
  max_adverse_price REAL NOT NULL,
  take_profit_hit INTEGER NOT NULL,
  stop_loss_hit INTEGER NOT NULL,
  estimated_fee REAL NOT NULL,
  estimated_slippage REAL NOT NULL,
  funding_fee REAL NOT NULL,
  estimated_pnl REAL NOT NULL,
  estimated_pnl_percent REAL NOT NULL,
  result TEXT NOT NULL,
  evaluated_at TEXT NOT NULL,
  UNIQUE(recommendation_id, horizon)
);

CREATE TABLE IF NOT EXISTS recommendation_user_actions (
  action_id TEXT PRIMARY KEY,
  recommendation_id TEXT NOT NULL,
  action_type TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL
);
