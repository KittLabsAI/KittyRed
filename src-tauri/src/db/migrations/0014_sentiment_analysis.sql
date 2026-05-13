CREATE TABLE IF NOT EXISTS sentiment_platform_auth_state (
  platform TEXT PRIMARY KEY,
  secret_json TEXT NOT NULL,
  captured_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sentiment_discussion_cache (
  stock_code TEXT PRIMARY KEY,
  stock_name TEXT,
  source_revision TEXT NOT NULL,
  items_json TEXT NOT NULL,
  platform_statuses_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sentiment_analysis_cache (
  stock_code TEXT PRIMARY KEY,
  stock_name TEXT,
  source_revision TEXT NOT NULL,
  total_score INTEGER NOT NULL,
  sentiment_score INTEGER NOT NULL,
  sentiment_reason TEXT NOT NULL,
  attention_score INTEGER NOT NULL,
  attention_reason TEXT NOT NULL,
  momentum_score INTEGER NOT NULL,
  momentum_reason TEXT NOT NULL,
  impact_score INTEGER NOT NULL,
  impact_reason TEXT NOT NULL,
  reliability_score INTEGER NOT NULL,
  reliability_reason TEXT NOT NULL,
  consensus_score INTEGER NOT NULL,
  consensus_reason TEXT NOT NULL,
  model_provider TEXT,
  model_name TEXT,
  generated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sentiment_discussion_revision
  ON sentiment_discussion_cache(stock_code, source_revision);

CREATE INDEX IF NOT EXISTS idx_sentiment_analysis_revision
  ON sentiment_analysis_cache(stock_code, source_revision);
