use std::path::Path;

#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn creates_core_tables() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"jobs".to_string()));
        assert!(tables.contains(&"settings_app".to_string()));
        assert!(tables.contains(&"exchange_credentials".to_string()));
        assert!(tables.contains(&"recommendation_runs".to_string()));
        assert!(tables.contains(&"recommendation_evaluations".to_string()));
        assert!(tables.contains(&"recommendation_user_actions".to_string()));
    }

    #[test]
    fn creates_asset_metadata_table() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"market_asset_metadata".to_string()));
    }

    #[test]
    fn creates_a_share_symbol_cache_table() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"a_share_symbol_cache".to_string()));
    }

    #[test]
    fn creates_market_candle_cache_table() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"market_candle_cache".to_string()));
    }

    #[test]
    fn creates_backtest_tables() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"backtest_datasets".to_string()));
        assert!(tables.contains(&"backtest_fetch_failures".to_string()));
        assert!(tables.contains(&"backtest_snapshots".to_string()));
        assert!(tables.contains(&"backtest_runs".to_string()));
        assert!(tables.contains(&"backtest_signals".to_string()));
        assert!(tables.contains(&"backtest_trades".to_string()));
        assert!(tables.contains(&"backtest_equity_curve".to_string()));
    }

    #[test]
    fn creates_financial_report_tables() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"financial_report_cache".to_string()));
        assert!(tables.contains(&"financial_report_analysis_cache".to_string()));
    }

    #[test]
    fn creates_sentiment_analysis_tables() {
        let db = Database::in_memory().unwrap();
        db.run_migrations().unwrap();
        let tables = db.list_tables().unwrap();

        assert!(tables.contains(&"sentiment_platform_auth_state".to_string()));
        assert!(tables.contains(&"sentiment_discussion_cache".to_string()));
        assert!(tables.contains(&"sentiment_analysis_cache".to_string()));
    }
}

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Self {
            connection: rusqlite::Connection::open(path)?,
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn in_memory() -> anyhow::Result<Self> {
        Ok(Self {
            connection: rusqlite::Connection::open_in_memory()?,
        })
    }

    pub fn run_migrations(&self) -> anyhow::Result<()> {
        self.connection
            .execute_batch(include_str!("migrations/0001_core.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0002_market.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0003_accounts.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0004_recommendations.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0005_assistant.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0006_notifications.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0007_recommendation_events.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0008_asset_metadata.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0009_a_share_symbols.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0010_candle_cache.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0011_backtest.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0012_financial_reports.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0013_backtest_equity_curve.sql"))?;
        self.connection
            .execute_batch(include_str!("migrations/0014_sentiment_analysis.sql"))?;
        self.run_market_cache_arbitrage_migration()?;
        self.run_market_cache_base_asset_migration()?;
        self.run_backtest_progress_migration()?;
        self.run_backtest_ai_data_migration()?;
        self.run_financial_report_score_migration()?;
        self.run_financial_report_subscore_migrations()?;
        Ok(())
    }

    fn has_column(&self, table: &str, column: &str) -> anyhow::Result<bool> {
        let mut stmt = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

        for item in rows {
            if item? == column {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn run_market_cache_arbitrage_migration(&self) -> anyhow::Result<()> {
        let statements = include_str!("migrations/0008_arbitrage_cache.sql")
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
            .map(|statement| {
                let column = statement
                    .strip_prefix("ALTER TABLE market_ticker_cache ADD COLUMN ")
                    .and_then(|suffix| suffix.split_whitespace().next())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "unexpected arbitrage cache migration statement: {statement}"
                        )
                    })?;
                Ok((column, statement))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        for (column, statement) in statements {
            if !self.has_column("market_ticker_cache", column)? {
                self.connection.execute_batch(statement)?;
            }
        }

        Ok(())
    }

    fn run_market_cache_base_asset_migration(&self) -> anyhow::Result<()> {
        if !self.has_column("market_ticker_cache", "base_asset")? {
            if let Err(error) = self.connection.execute_batch(
                "ALTER TABLE market_ticker_cache ADD COLUMN base_asset TEXT NOT NULL DEFAULT ''",
            ) {
                if !error.to_string().contains("duplicate column name") {
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    fn run_backtest_progress_migration(&self) -> anyhow::Result<()> {
        let columns = [
            (
                "total_ai_calls",
                "ALTER TABLE backtest_runs ADD COLUMN total_ai_calls INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "processed_ai_calls",
                "ALTER TABLE backtest_runs ADD COLUMN processed_ai_calls INTEGER NOT NULL DEFAULT 0",
            ),
        ];
        for (column, statement) in columns {
            if !self.has_column("backtest_runs", column)? {
                if let Err(error) = self.connection.execute_batch(statement) {
                    if !error.to_string().contains("duplicate column name") {
                        return Err(error.into());
                    }
                }
            }
        }
        Ok(())
    }

    fn run_backtest_ai_data_migration(&self) -> anyhow::Result<()> {
        let columns = [(
            "kline_data_json",
            "ALTER TABLE backtest_snapshots ADD COLUMN kline_data_json TEXT NOT NULL DEFAULT '{}'",
        )];
        for (column, statement) in columns {
            if !self.has_column("backtest_snapshots", column)? {
                if let Err(error) = self.connection.execute_batch(statement) {
                    if !error.to_string().contains("duplicate column name") {
                        return Err(error.into());
                    }
                }
            }
        }
        Ok(())
    }

    fn run_financial_report_score_migration(&self) -> anyhow::Result<()> {
        if !self.has_column("financial_report_analysis_cache", "financial_score")? {
            if let Err(error) = self.connection.execute_batch(
                "ALTER TABLE financial_report_analysis_cache ADD COLUMN financial_score INTEGER NOT NULL DEFAULT 0",
            ) {
                if !error.to_string().contains("duplicate column name") {
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    fn run_financial_report_subscore_migrations(&self) -> anyhow::Result<()> {
        let columns = [
            ("revenue_quality_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN revenue_quality_score INTEGER NOT NULL DEFAULT 0"),
            ("gross_margin_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN gross_margin_score INTEGER NOT NULL DEFAULT 0"),
            ("net_profit_return_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN net_profit_return_score INTEGER NOT NULL DEFAULT 0"),
            ("earnings_manipulation_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN earnings_manipulation_score INTEGER NOT NULL DEFAULT 0"),
            ("solvency_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN solvency_score INTEGER NOT NULL DEFAULT 0"),
            ("cash_flow_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN cash_flow_score INTEGER NOT NULL DEFAULT 0"),
            ("growth_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN growth_score INTEGER NOT NULL DEFAULT 0"),
            ("research_capital_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN research_capital_score INTEGER NOT NULL DEFAULT 0"),
            ("operating_efficiency_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN operating_efficiency_score INTEGER NOT NULL DEFAULT 0"),
            ("asset_quality_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN asset_quality_score INTEGER NOT NULL DEFAULT 0"),
            ("profitability_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN profitability_score REAL NOT NULL DEFAULT 0"),
            ("authenticity_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN authenticity_score REAL NOT NULL DEFAULT 0"),
            ("cash_generation_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN cash_generation_score REAL NOT NULL DEFAULT 0"),
            ("safety_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN safety_score REAL NOT NULL DEFAULT 0"),
            ("growth_potential_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN growth_potential_score REAL NOT NULL DEFAULT 0"),
            ("operating_radar_score", "ALTER TABLE financial_report_analysis_cache ADD COLUMN operating_radar_score REAL NOT NULL DEFAULT 0"),
        ];
        for (column, statement) in columns {
            if !self.has_column("financial_report_analysis_cache", column)? {
                if let Err(error) = self.connection.execute_batch(statement) {
                    if !error.to_string().contains("duplicate column name") {
                        return Err(error.into());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn list_tables(&self) -> anyhow::Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn connection(&self) -> &rusqlite::Connection {
        &self.connection
    }
}
