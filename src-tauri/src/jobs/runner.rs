pub fn route_job_kind(kind: &str) -> &'static str {
    match kind {
        crate::jobs::kinds::RECOMMENDATION_GENERATE => "recommendation.generate",
        crate::jobs::kinds::RECOMMENDATION_EVALUATE => "recommendation.evaluate",
        crate::jobs::kinds::PAPER_ORDER_MONITOR => "paper.monitor",
        crate::jobs::kinds::NOTIFICATION_DISPATCH => "notifications.dispatch",
        crate::jobs::kinds::SIGNAL_SCAN => "signal.scan",
        crate::jobs::kinds::MARKET_REFRESH_SYMBOLS => "market.refresh_symbols",
        crate::jobs::kinds::MARKET_REFRESH_SNAPSHOTS => "market.refresh_snapshots",
        crate::jobs::kinds::MARKET_REBUILD_ORDERBOOK => "market.rebuild_orderbook",
        crate::jobs::kinds::SPREAD_EVALUATE => "spread.evaluate",
        crate::jobs::kinds::CREDENTIAL_VALIDATE => "credential.validate",
        crate::jobs::kinds::PORTFOLIO_REFRESH => "portfolio.refresh",
        crate::jobs::kinds::ASSISTANT_RUN => "assistant.run",
        crate::jobs::kinds::FINANCIAL_REPORT_FETCH => "financial_report.fetch",
        crate::jobs::kinds::FINANCIAL_REPORT_ANALYZE => "financial_report.analyze",
        _ => "noop",
    }
}
