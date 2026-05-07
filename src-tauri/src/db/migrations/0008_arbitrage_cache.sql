ALTER TABLE market_ticker_cache ADD COLUMN venue_snapshots_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE market_ticker_cache ADD COLUMN best_bid_exchange TEXT;
ALTER TABLE market_ticker_cache ADD COLUMN best_ask_exchange TEXT;
ALTER TABLE market_ticker_cache ADD COLUMN best_bid_price REAL;
ALTER TABLE market_ticker_cache ADD COLUMN best_ask_price REAL;
ALTER TABLE market_ticker_cache ADD COLUMN responded_exchange_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE market_ticker_cache ADD COLUMN market_cap_usd REAL;
ALTER TABLE market_ticker_cache ADD COLUMN fdv_usd REAL;
