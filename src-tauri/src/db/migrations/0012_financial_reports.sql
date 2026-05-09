CREATE TABLE IF NOT EXISTS financial_report_cache (
  cache_id TEXT PRIMARY KEY,
  stock_code TEXT NOT NULL,
  section TEXT NOT NULL,
  section_label TEXT NOT NULL,
  source TEXT NOT NULL,
  report_date TEXT,
  stock_name TEXT,
  raw_row_json TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  refreshed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_financial_report_cache_stock_section
  ON financial_report_cache(stock_code, section, report_date DESC);

CREATE TABLE IF NOT EXISTS financial_report_analysis_cache (
  stock_code TEXT PRIMARY KEY,
  source_revision TEXT NOT NULL,
  key_summary TEXT NOT NULL,
  positive_factors TEXT NOT NULL,
  negative_factors TEXT NOT NULL,
  fraud_risk_points TEXT NOT NULL,
  model_provider TEXT,
  model_name TEXT,
  generated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_financial_report_analysis_revision
  ON financial_report_analysis_cache(stock_code, source_revision);
