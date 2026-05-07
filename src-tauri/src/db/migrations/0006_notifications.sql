CREATE TABLE IF NOT EXISTS notification_events (
  event_id TEXT PRIMARY KEY,
  channel TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL
);
