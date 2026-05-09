CREATE TABLE IF NOT EXISTS backtest_equity_curve (
    backtest_id TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    cumulative_pnl_percent REAL NOT NULL,
    PRIMARY KEY (backtest_id, captured_at)
);
