CREATE TABLE IF NOT EXISTS recommendation_runs (
  recommendation_id TEXT PRIMARY KEY,
  trigger_type TEXT NOT NULL,
  status TEXT NOT NULL,
  ai_raw_output TEXT NOT NULL,
  ai_structured_output TEXT NOT NULL,
  risk_result TEXT NOT NULL,
  final_output TEXT NOT NULL,
  model_provider TEXT NOT NULL,
  model_name TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  user_preference_version TEXT NOT NULL,
  market_snapshot TEXT NOT NULL DEFAULT '{}',
  account_snapshot TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);
