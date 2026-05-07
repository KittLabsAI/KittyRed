CREATE TABLE IF NOT EXISTS assistant_runs (
  assistant_session_id TEXT PRIMARY KEY,
  prompt_text TEXT NOT NULL,
  answer_text TEXT NOT NULL,
  tools_used_json TEXT NOT NULL,
  cited_at TEXT NOT NULL,
  created_at TEXT NOT NULL
);
