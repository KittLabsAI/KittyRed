mod real_accounts;

#[cfg(test)]
mod tests {
    use super::{combine_orders, combine_overviews, combine_positions, compute_total_equity};
    use crate::models::{ExchangeEquity, PaperOrderRowDto, PortfolioOverviewDto, PositionDto};

    #[test]
    fn sums_balances_across_exchanges() {
        let total = compute_total_equity(&[1200.0, 800.0, 500.0]);
        assert_eq!(total, 2500.0);
    }

    #[test]
    fn keeps_paper_overview_when_mode_is_paper() {
        let combined = combine_overviews("paper", sample_paper_overview(), sample_real_overview());
        assert_eq!(combined.account_mode, "paper");
        assert_eq!(combined.total_equity_usdt, 70_000.0);
        assert_eq!(combined.exchanges.len(), 2);
        assert_eq!(combined.exchanges[0].exchange, "沪市A股");
    }

    #[test]
    fn keeps_real_overview_when_mode_is_real_read_only() {
        let combined = combine_overviews(
            "real_read_only",
            sample_paper_overview(),
            sample_real_overview(),
        );
        assert_eq!(combined.account_mode, "real_read_only");
        assert_eq!(combined.total_equity_usdt, 41_500.0);
        assert_eq!(combined.exchanges.len(), 2);
        assert_eq!(combined.exchanges[0].exchange, "沪市A股");
    }

    #[test]
    fn merges_paper_and_real_overviews_in_dual_mode() {
        let combined = combine_overviews("dual", sample_paper_overview(), sample_real_overview());
        assert_eq!(combined.account_mode, "dual");
        assert_eq!(combined.total_equity_usdt, 111_500.0);
        assert_eq!(combined.exchanges.len(), 4);
        assert!(combined.risk_summary.contains("paper"));
        assert!(combined.risk_summary.contains("real"));
        assert!(combined
            .exchanges
            .iter()
            .any(|exchange| exchange.exchange == "Paper: 沪市A股"));
    }

    #[test]
    fn merges_positions_in_dual_mode_and_sorts_consistently() {
        let positions = combine_positions(
            "dual",
            vec![PositionDto {
                exchange: "akshare".into(),
                symbol: "BTC/USDT".into(),
                side: "Long".into(),
                size: "0.100 BTC".into(),
                entry_price: 68_000.0,
                mark_price: 68_400.0,
                pnl_percent: 0.58,
                leverage: "3x".into(),
            }],
            vec![PositionDto {
                exchange: "沪市A股".into(),
                symbol: "ETH/USDT".into(),
                side: "Spot".into(),
                size: "1.500 ETH".into(),
                entry_price: 3_200.0,
                mark_price: 3_260.0,
                pnl_percent: 0.0,
                leverage: "1x".into(),
            }],
        );

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].exchange, "Paper: akshare");
        assert_eq!(positions[1].exchange, "沪市A股");
    }

    #[test]
    fn merges_orders_in_dual_mode_and_sorts_by_update_time() {
        let orders = combine_orders(
            "dual",
            vec![PaperOrderRowDto {
                order_id: "paper-1".into(),
                exchange: "akshare".into(),
                symbol: "BTC/USDT".into(),
                order_type: "Perpetual Long".into(),
                status: "Monitoring".into(),
                quantity: "0.100 BTC".into(),
                estimated_fill_price: 68_200.0,
                realized_pnl_usdt: None,
                updated_at: "2026-05-03T20:00:00+08:00".into(),
            }],
            vec![PaperOrderRowDto {
                order_id: "real-1".into(),
                exchange: "沪市A股".into(),
                symbol: "ETH/USDT".into(),
                order_type: "Spot Limit Buy".into(),
                status: "Filled".into(),
                quantity: "1.500 ETH".into(),
                estimated_fill_price: 3_200.0,
                realized_pnl_usdt: Some(24.0),
                updated_at: "2026-05-03T21:00:00+08:00".into(),
            }],
        );

        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order_id, "real-1");
        assert_eq!(orders[1].exchange, "Paper: akshare");
    }

    fn sample_paper_overview() -> PortfolioOverviewDto {
        PortfolioOverviewDto {
            total_equity_usdt: 70_000.0,
            total_market_value_usdt: 20_000.0,
            total_pnl_usdt: 1_200.0,
            daily_pnl_usdt: 800.0,
            daily_pnl_percent: 1.2,
            account_mode: "paper".into(),
            risk_summary: "Seven paper exchanges are active.".into(),
            exchanges: vec![
                ExchangeEquity {
                    exchange: "沪市A股".into(),
                    equity_usdt: 10_000.0,
                    change_percent: 0.0,
                },
                ExchangeEquity {
                    exchange: "akshare".into(),
                    equity_usdt: 60_000.0,
                    change_percent: 1.4,
                },
            ],
        }
    }

    fn sample_real_overview() -> PortfolioOverviewDto {
        PortfolioOverviewDto {
            total_equity_usdt: 41_500.0,
            total_market_value_usdt: 12_000.0,
            total_pnl_usdt: 600.0,
            daily_pnl_usdt: 300.0,
            daily_pnl_percent: 0.6,
            account_mode: "real_read_only".into(),
            risk_summary: "Two real exchanges are synced.".into(),
            exchanges: vec![
                ExchangeEquity {
                    exchange: "沪市A股".into(),
                    equity_usdt: 25_000.0,
                    change_percent: 0.5,
                },
                ExchangeEquity {
                    exchange: "akshare".into(),
                    equity_usdt: 16_500.0,
                    change_percent: 0.8,
                },
            ],
        }
    }
}

use crate::market::MarketDataService;
use crate::models::{
    ExchangeConnectionTestResultDto, ExchangeCredentialSummary, ExchangeEquity,
    PortfolioOverviewDto, PositionDto,
};
use crate::paper::PaperService;
use crate::settings::{ExchangeSecretMaterial, SettingsService};
use real_accounts::RealAccountService;

pub fn compute_total_equity(values: &[f64]) -> f64 {
    values.iter().sum()
}

#[derive(Clone)]
pub struct PortfolioService {
    paper_service: PaperService,
    real_account_service: RealAccountService,
}

impl PortfolioService {
    pub fn new(paper_service: PaperService) -> Self {
        Self {
            paper_service,
            real_account_service: RealAccountService::default(),
        }
    }

    pub async fn get_overview(
        &self,
        market_data_service: &MarketDataService,
        settings_service: &SettingsService,
    ) -> anyhow::Result<PortfolioOverviewDto> {
        let account_mode = settings_service.get_runtime_settings().account_mode;
        match account_mode.as_str() {
            "real_read_only" => {
                self.real_account_service
                    .build_overview(settings_service)
                    .await
            }
            "dual" => {
                let paper = self
                    .paper_service
                    .build_overview(market_data_service)
                    .await?;
                let real = self
                    .real_account_service
                    .build_overview(settings_service)
                    .await?;
                Ok(combine_overviews("dual", paper, real))
            }
            _ => self.paper_service.build_overview(market_data_service).await,
        }
    }

    pub async fn list_positions(
        &self,
        market_data_service: &MarketDataService,
        settings_service: &SettingsService,
    ) -> anyhow::Result<Vec<PositionDto>> {
        let account_mode = settings_service.get_runtime_settings().account_mode;
        match account_mode.as_str() {
            "real_read_only" => {
                self.real_account_service
                    .build_positions(settings_service)
                    .await
            }
            "dual" => {
                let paper = self
                    .paper_service
                    .build_positions(market_data_service)
                    .await?;
                let real = self
                    .real_account_service
                    .build_positions(settings_service)
                    .await?;
                Ok(combine_positions("dual", paper, real))
            }
            _ => {
                self.paper_service
                    .build_positions(market_data_service)
                    .await
            }
        }
    }

    pub async fn list_orders(
        &self,
        settings_service: &SettingsService,
    ) -> anyhow::Result<Vec<crate::models::PaperOrderRowDto>> {
        let account_mode = settings_service.get_runtime_settings().account_mode;
        match account_mode.as_str() {
            "real_read_only" => {
                self.real_account_service
                    .build_orders(settings_service)
                    .await
            }
            "dual" => {
                let paper = self.paper_service.list_orders_snapshot();
                let real = self
                    .real_account_service
                    .build_orders(settings_service)
                    .await?;
                Ok(combine_orders("dual", paper, real))
            }
            _ => Ok(self.paper_service.list_orders_snapshot()),
        }
    }

    pub async fn inspect_exchange_credentials(
        &self,
        settings_service: &SettingsService,
    ) -> Vec<ExchangeCredentialSummary> {
        self.real_account_service
            .inspect_exchange_credentials(settings_service)
            .await
    }

    pub async fn test_exchange_connection(
        &self,
        credentials: ExchangeSecretMaterial,
    ) -> ExchangeConnectionTestResultDto {
        self.real_account_service
            .test_exchange_connection(credentials)
            .await
    }
}

fn combine_overviews(
    mode: &str,
    paper: PortfolioOverviewDto,
    real: PortfolioOverviewDto,
) -> PortfolioOverviewDto {
    match mode {
        "real_read_only" => real,
        "dual" => {
            let total_equity = paper.total_equity_usdt + real.total_equity_usdt;
            let weighted_daily = if total_equity <= f64::EPSILON {
                0.0
            } else {
                ((paper.daily_pnl_percent * paper.total_equity_usdt)
                    + (real.daily_pnl_percent * real.total_equity_usdt))
                    / total_equity
            };
            let mut exchanges = tag_paper_exchanges(paper.exchanges);
            exchanges.extend(real.exchanges);
            exchanges.sort_by(|left, right| left.exchange.cmp(&right.exchange));

            PortfolioOverviewDto {
                total_equity_usdt: total_equity,
                total_market_value_usdt: paper.total_market_value_usdt
                    + real.total_market_value_usdt,
                total_pnl_usdt: paper.total_pnl_usdt + real.total_pnl_usdt,
                daily_pnl_usdt: paper.daily_pnl_usdt + real.daily_pnl_usdt,
                daily_pnl_percent: weighted_daily,
                account_mode: "dual".into(),
                risk_summary: format!("paper: {} real: {}", paper.risk_summary, real.risk_summary),
                exchanges,
            }
        }
        _ => paper,
    }
}

fn combine_positions(
    mode: &str,
    paper: Vec<PositionDto>,
    real: Vec<PositionDto>,
) -> Vec<PositionDto> {
    match mode {
        "real_read_only" => real,
        "dual" => {
            let mut positions = tag_paper_positions(paper);
            positions.extend(real);
            positions.sort_by(|left, right| {
                left.exchange
                    .cmp(&right.exchange)
                    .then(left.symbol.cmp(&right.symbol))
                    .then(left.side.cmp(&right.side))
            });
            positions
        }
        _ => paper,
    }
}

fn combine_orders(
    mode: &str,
    paper: Vec<crate::models::PaperOrderRowDto>,
    real: Vec<crate::models::PaperOrderRowDto>,
) -> Vec<crate::models::PaperOrderRowDto> {
    match mode {
        "real_read_only" => real,
        "dual" => {
            let mut orders = tag_paper_orders(paper);
            orders.extend(real);
            orders.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
            orders
        }
        _ => paper,
    }
}

fn tag_paper_exchanges(exchanges: Vec<ExchangeEquity>) -> Vec<ExchangeEquity> {
    exchanges
        .into_iter()
        .map(|mut exchange| {
            exchange.exchange = format!("Paper: {}", exchange.exchange);
            exchange
        })
        .collect()
}

fn tag_paper_positions(positions: Vec<PositionDto>) -> Vec<PositionDto> {
    positions
        .into_iter()
        .map(|mut position| {
            position.exchange = format!("Paper: {}", position.exchange);
            position
        })
        .collect()
}

fn tag_paper_orders(
    orders: Vec<crate::models::PaperOrderRowDto>,
) -> Vec<crate::models::PaperOrderRowDto> {
    orders
        .into_iter()
        .map(|mut order| {
            order.exchange = format!("Paper: {}", order.exchange);
            order
        })
        .collect()
}
