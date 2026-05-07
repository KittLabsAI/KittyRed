pub mod fills;
pub mod risk;

#[cfg(test)]
mod tests {
    use super::{PaperOrderInput, PaperService};
    use crate::models::{MarketListRow, RecommendationRunDto};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn seeds_all_supported_paper_accounts_and_books_a_position_after_execution() {
        let service = PaperService::default();
        let recommendation = RecommendationRunDto {
            recommendation_id: "rec-paper-1".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("Long".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare".into()],
            confidence_score: 78.0,
            rationale: "Paper execution regression".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(68_280.0),
            entry_high: Some(68_420.0),
            stop_loss: Some(67_680.0),
            take_profit: Some("69,550 / 70,120".into()),
            leverage: Some(3.0),
            amount_cny: Some(1_800.0),
            invalidation: Some("Lose 67,680".into()),
            max_loss_cny: Some(47.4),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-03T20:00:00+08:00".into(),
            model_provider: "System".into(),
            model_name: "heuristic-fallback".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-paper-1".into(),
            generated_at: "2026-05-03T20:00:00+08:00".into(),
        };

        assert_eq!(service.account_count(), 1);

        let draft = futures::executor::block_on(
            service.create_draft_from_recommendation(&recommendation, "paper-cash"),
        )
        .expect("paper order draft should succeed");

        assert_eq!(draft.exchange, "人民币现金");
        let positions = service.list_positions_snapshot();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].exchange, "人民币现金");
        assert_eq!(positions[0].symbol, "SHSE.600000");
    }

    #[test]
    fn closes_paper_positions_when_take_profit_is_hit_and_updates_order_history() {
        let service = PaperService::default();
        let recommendation = RecommendationRunDto {
            recommendation_id: "rec-paper-2".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("Long".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare".into()],
            confidence_score: 82.0,
            rationale: "Paper TP monitor regression".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(68_000.0),
            entry_high: Some(68_100.0),
            stop_loss: Some(67_500.0),
            take_profit: Some("68,700 / 69,200".into()),
            leverage: Some(3.0),
            amount_cny: Some(1_000.0),
            invalidation: Some("Lose 67,500".into()),
            max_loss_cny: Some(21.5),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-03T20:10:00+08:00".into(),
            model_provider: "System".into(),
            model_name: "heuristic-fallback".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-paper-2".into(),
            generated_at: "2026-05-03T20:10:00+08:00".into(),
        };

        let draft = futures::executor::block_on(
            service.create_draft_from_recommendation(&recommendation, "paper-cash"),
        )
        .expect("paper order draft should succeed");

        let exits = futures::executor::block_on(service.sync_with_rows(&[MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "BTC".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 68_900.0,
            change_24h: 4.4,
            volume_24h: 210_000_000.0,
            funding_rate: Some(0.012),
            spread_bps: 2.8,
            exchanges: vec!["akshare".into()],
            updated_at: "2026-05-03T20:15:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]))
        .expect("paper sync should succeed");

        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].order_id, draft.order_id);
        assert_eq!(exits[0].status, "Closed Take Profit");
        assert!(service.list_positions_snapshot().is_empty());

        let orders = service.list_orders_snapshot();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].status, "Closed Take Profit");
        assert!(orders[0].realized_pnl_usdt.unwrap_or_default() > 0.0);
    }

    #[test]
    fn creates_recommendation_and_manual_drafts_through_shared_order_input() {
        let service = PaperService::default();
        let recommendation = RecommendationRunDto {
            recommendation_id: "rec-paper-shared".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("SZSE.000001".into()),
            stock_name: Some("平安银行".into()),
            direction: Some("Short".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare".into()],
            confidence_score: 82.0,
            rationale: "Shared paper input regression".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(3400.0),
            entry_high: Some(3420.0),
            stop_loss: Some(3460.0),
            take_profit: Some("3340 / 3300".into()),
            leverage: Some(2.0),
            amount_cny: Some(500.0),
            invalidation: None,
            max_loss_cny: Some(30.0),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-03T20:10:00+08:00".into(),
            model_provider: "System".into(),
            model_name: "heuristic-fallback".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-paper-shared".into(),
            generated_at: "2026-05-03T20:10:00+08:00".into(),
        };

        let recommendation_draft = futures::executor::block_on(
            service.create_draft_from_recommendation(&recommendation, "paper-cash"),
        )
        .expect("recommendation draft should succeed");
        let manual_draft =
            futures::executor::block_on(service.create_paper_order(PaperOrderInput {
                account_id: "paper-cash".into(),
                symbol: "SZSE.000001".into(),
                market_type: "ashare".into(),
                side: "sell".into(),
                quantity: 500.0 / 3410.0,
                entry_price: 3410.0,
                leverage: 2.0,
                stop_loss: Some(3460.0),
                take_profit: Some(3340.0),
                updated_at: "2026-05-03T20:11:00+08:00".into(),
            }))
            .expect("manual draft should succeed");

        assert_eq!(recommendation_draft.side, manual_draft.side);
        assert_eq!(recommendation_draft.quantity, manual_draft.quantity);
        assert_eq!(service.list_positions_snapshot().len(), 2);
    }

    #[test]
    fn manual_paper_orders_use_requested_share_quantity() {
        let service = PaperService::default();

        let draft = futures::executor::block_on(service.create_paper_order(PaperOrderInput {
            account_id: "paper-cash".into(),
            symbol: "SHSE.600000".into(),
            market_type: "ashare".into(),
            side: "buy".into(),
            quantity: 200.0,
            entry_price: 8.72,
            leverage: 1.0,
            stop_loss: None,
            take_profit: None,
            updated_at: "2026-05-07T10:00:00+08:00".into(),
        }))
        .expect("manual quantity order should succeed");

        assert_eq!(draft.quantity, 200.0);
        let accounts = service.list_accounts_snapshot();
        assert_eq!(accounts[0].available_usdt, 998_256.0);
    }

    #[test]
    fn portfolio_overview_daily_pnl_uses_previous_close_from_market_change() {
        let service = PaperService::default();
        futures::executor::block_on(service.create_paper_order(PaperOrderInput {
            account_id: "paper-cash".into(),
            symbol: "SHSE.600000".into(),
            market_type: "ashare".into(),
            side: "buy".into(),
            quantity: 200.0,
            entry_price: 7.0,
            leverage: 1.0,
            stop_loss: None,
            take_profit: None,
            updated_at: "2026-05-07T10:00:00+08:00".into(),
        }))
        .expect("manual order should succeed");

        let market_data = crate::market::MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "浦发银行".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 10.0,
            change_24h: 25.0,
            volume_24h: 210_000_000.0,
            funding_rate: None,
            spread_bps: 0.0,
            exchanges: vec!["akshare".into()],
            updated_at: "2026-05-07T10:00:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 1,
            fdv_usd: None,
        }]);

        let overview = futures::executor::block_on(service.build_overview(&market_data))
            .expect("overview should build");

        assert_eq!(overview.total_market_value_usdt, 2_000.0);
        assert_eq!(overview.total_pnl_usdt, 600.0);
        assert_eq!(overview.daily_pnl_usdt, 400.0);
    }

    #[test]
    fn restores_paper_state_from_sqlite_after_restart() {
        let path = unique_temp_paper_db_path("paper-state-restore");
        let service = PaperService::new(path.clone()).expect("paper service should initialize");

        let first = futures::executor::block_on(service.create_paper_order(PaperOrderInput {
            account_id: "paper-cash".into(),
            symbol: "SHSE.600000".into(),
            market_type: "ashare".into(),
            side: "buy".into(),
            quantity: 1000.0 / 50_000.0,
            entry_price: 50_000.0,
            leverage: 2.0,
            stop_loss: Some(49_000.0),
            take_profit: Some(52_000.0),
            updated_at: "2026-05-03T20:11:00+08:00".into(),
        }))
        .expect("paper order should persist");
        drop(service);

        let restored = PaperService::new(path.clone()).expect("paper service should reload");
        let positions = restored.list_positions_snapshot();
        let accounts = restored.list_accounts_snapshot();
        let orders = restored.list_orders_snapshot();
        let second = futures::executor::block_on(restored.create_paper_order(PaperOrderInput {
            account_id: "paper-cash".into(),
            symbol: "SZSE.000001".into(),
            market_type: "ashare".into(),
            side: "buy".into(),
            quantity: 500.0 / 2500.0,
            entry_price: 2500.0,
            leverage: 1.0,
            stop_loss: None,
            take_profit: None,
            updated_at: "2026-05-03T20:12:00+08:00".into(),
        }))
        .expect("next paper order should continue id sequence");

        assert_eq!(first.order_id, "PO-0001");
        assert_eq!(second.order_id, "PO-0002");
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "SHSE.600000");
        assert_eq!(orders.len(), 1);
        assert_eq!(
            accounts
                .iter()
                .find(|account| account.account_id == "paper-cash")
                .map(|account| account.available_usdt),
            Some(999000.0)
        );
    }

    fn unique_temp_paper_db_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.sqlite3"))
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail};
use rusqlite::{params, OptionalExtension};

use crate::db::Database;
use crate::market::MarketDataService;
use crate::models::{
    ExchangeEquity, MarketListRow, PaperAccountDto, PaperOrderDraftDto, PaperOrderRowDto,
    PortfolioOverviewDto, PositionDto, RecommendationRunDto,
};

#[derive(Clone)]
pub struct PaperService {
    state: Arc<RwLock<PaperState>>,
    path: Option<PathBuf>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PaperState {
    accounts: HashMap<String, PaperAccount>,
    positions: Vec<PaperPosition>,
    orders: Vec<PaperOrderRecord>,
    next_order_id: usize,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PaperAccount {
    account_id: String,
    exchange: String,
    available_usdt: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PaperPosition {
    entry_order_id: String,
    account_id: String,
    exchange: String,
    symbol: String,
    market_type: String,
    side: String,
    quantity: f64,
    entry_price: f64,
    mark_price: f64,
    leverage: f64,
    margin_usdt: f64,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
    updated_at: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PaperOrderRecord {
    order_id: String,
    exchange: String,
    symbol: String,
    order_type: String,
    status: String,
    quantity: f64,
    estimated_fill_price: f64,
    realized_pnl_usdt: Option<f64>,
    updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PaperExitEvent {
    pub order_id: String,
    pub exchange: String,
    pub symbol: String,
    pub status: String,
    pub realized_pnl_usdt: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PaperOrderInput {
    pub account_id: String,
    pub symbol: String,
    pub market_type: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub leverage: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub updated_at: String,
}

impl Default for PaperService {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(default_paper_state())),
            path: None,
        }
    }
}

impl PaperService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        ensure_paper_state_table(&path)?;
        let mut state = load_paper_state(&path)?.unwrap_or_else(default_paper_state);
        normalize_loaded_state(&mut state);
        persist_paper_state(&path, &state)?;
        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            path: Some(path),
        })
    }

    pub fn account_count(&self) -> usize {
        self.state
            .read()
            .expect("paper state lock poisoned")
            .accounts
            .len()
    }

    pub fn list_positions_snapshot(&self) -> Vec<PositionDto> {
        self.state
            .read()
            .expect("paper state lock poisoned")
            .positions
            .iter()
            .map(position_to_dto)
            .collect()
    }

    pub fn list_accounts_snapshot(&self) -> Vec<PaperAccountDto> {
        let mut accounts = self
            .state
            .read()
            .expect("paper state lock poisoned")
            .accounts
            .values()
            .map(|account| PaperAccountDto {
                account_id: account.account_id.clone(),
                exchange: account.exchange.clone(),
                available_usdt: round_money(account.available_usdt),
            })
            .collect::<Vec<_>>();
        accounts.sort_by(|left, right| left.exchange.cmp(&right.exchange));
        accounts
    }

    pub fn list_orders_snapshot(&self) -> Vec<PaperOrderRowDto> {
        self.state
            .read()
            .expect("paper state lock poisoned")
            .orders
            .iter()
            .map(order_to_dto)
            .collect()
    }

    pub async fn create_draft_from_recommendation(
        &self,
        recommendation: &RecommendationRunDto,
        account_id: &str,
    ) -> anyhow::Result<PaperOrderDraftDto> {
        if !recommendation.has_trade {
            bail!("cannot create paper order from a no-trade recommendation");
        }

        let symbol = recommendation
            .symbol
            .clone()
            .ok_or_else(|| anyhow!("recommendation symbol is missing"))?;
        let direction = recommendation
            .direction
            .clone()
            .ok_or_else(|| anyhow!("recommendation direction is missing"))?;
        let entry_price = average_price(recommendation.entry_low, recommendation.entry_high)
            .ok_or_else(|| anyhow!("recommendation entry range is missing"))?;
        let amount_cny = recommendation
            .amount_cny
            .ok_or_else(|| anyhow!("recommendation amount is missing"))?;
        let leverage = recommendation.leverage.unwrap_or(1.0).max(1.0);
        let quantity = amount_cny / entry_price.max(0.000_000_1);

        self.create_paper_order(PaperOrderInput {
            account_id: account_id.into(),
            symbol,
            market_type: recommendation.market_type.clone(),
            side: if direction.eq_ignore_ascii_case("short") {
                "sell".into()
            } else {
                "buy".into()
            },
            quantity,
            entry_price,
            leverage,
            stop_loss: recommendation.stop_loss,
            take_profit: parse_take_profit(recommendation.take_profit.as_deref()),
            updated_at: recommendation.generated_at.clone(),
        })
        .await
    }

    pub async fn create_paper_order(
        &self,
        input: PaperOrderInput,
    ) -> anyhow::Result<PaperOrderDraftDto> {
        if input.quantity <= 0.0 {
            bail!("paper order quantity must be positive");
        }
        if input.entry_price <= 0.0 {
            bail!("paper order entry price must be positive");
        }

        let leverage = input.leverage.max(1.0);
        let direction = if input.side.eq_ignore_ascii_case("sell")
            || input.side.eq_ignore_ascii_case("short")
        {
            "Short"
        } else {
            "Long"
        };

        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow!("failed to lock paper state"))?;
        let (account_id, exchange) = {
            let account = state
                .accounts
                .get_mut(&input.account_id)
                .ok_or_else(|| anyhow!("unknown paper account: {}", input.account_id))?;

            let required_cash = input.quantity * input.entry_price;
            if account.available_usdt + f64::EPSILON < required_cash {
                bail!(
                    "模拟账户 {} 可用人民币资金不足",
                    account.exchange
                );
            }

            account.available_usdt -= required_cash;
            (account.account_id.clone(), account.exchange.clone())
        };

        let quantity = input.quantity;
        let margin_usdt = quantity * input.entry_price;
        let order_id = format!("PO-{:04}", state.next_order_id);
        state.next_order_id += 1;

        state.orders.insert(
            0,
            PaperOrderRecord {
                order_id: order_id.clone(),
                exchange: exchange.clone(),
                symbol: input.symbol.clone(),
                order_type: order_type(&input.market_type, direction),
                status: "Filled".into(),
                quantity,
                estimated_fill_price: input.entry_price,
                realized_pnl_usdt: None,
                updated_at: input.updated_at.clone(),
            },
        );

        state.positions.push(PaperPosition {
            entry_order_id: order_id.clone(),
            account_id: account_id.clone(),
            exchange: exchange.clone(),
            symbol: input.symbol.clone(),
            market_type: input.market_type.clone(),
            side: direction.into(),
            quantity,
            entry_price: input.entry_price,
            mark_price: input.entry_price,
            leverage,
            margin_usdt,
            stop_loss: input.stop_loss,
            take_profit: input.take_profit,
            updated_at: input.updated_at,
        });

        self.persist_state(&state)?;

        Ok(PaperOrderDraftDto {
            order_id,
            account_id,
            exchange,
            symbol: input.symbol,
            side: if direction.eq_ignore_ascii_case("Short") {
                "sell".into()
            } else {
                "buy".into()
            },
            quantity,
            estimated_fill_price: input.entry_price,
            stop_loss: input.stop_loss,
            take_profit: input.take_profit,
        })
    }

    pub async fn sync_with_market_data(
        &self,
        market_data_service: &MarketDataService,
    ) -> anyhow::Result<Vec<PaperExitEvent>> {
        let snapshots = market_data_service.list_markets().await.unwrap_or_default();
        self.sync_with_rows(&snapshots).await
    }

    pub async fn sync_with_rows(
        &self,
        snapshots: &[MarketListRow],
    ) -> anyhow::Result<Vec<PaperExitEvent>> {
        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow!("failed to lock paper state"))?;
        let exits = refresh_positions(&mut state, snapshots);
        if !exits.is_empty() {
            self.persist_state(&state)?;
        }
        Ok(exits)
    }

    pub async fn build_overview(
        &self,
        market_data_service: &MarketDataService,
    ) -> anyhow::Result<PortfolioOverviewDto> {
        let snapshots = market_data_service.list_markets().await.unwrap_or_default();
        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow!("failed to lock paper state"))?;
        let exits = refresh_positions(&mut state, &snapshots);
        if !exits.is_empty() {
            self.persist_state(&state)?;
        }

        let mut exchanges = state
            .accounts
            .values()
            .map(|account| {
                let open_positions = state
                    .positions
                    .iter()
                    .filter(|position| position.account_id == account.account_id);
                let equity = account.available_usdt
                    + open_positions
                        .clone()
                        .map(|position| position.margin_usdt + unrealized_pnl_usdt(position))
                        .sum::<f64>();
                ExchangeEquity {
                    exchange: account.exchange.clone(),
                    equity_usdt: round_money(equity),
                    change_percent: open_positions.map(unrealized_pnl_pct).next().unwrap_or(0.0),
                }
            })
            .collect::<Vec<_>>();
        exchanges.sort_by(|left, right| left.exchange.cmp(&right.exchange));

        let total_equity = exchanges.iter().map(|item| item.equity_usdt).sum::<f64>();
        let total_market_value = state
            .positions
            .iter()
            .map(|position| position.quantity * position.mark_price)
            .sum::<f64>();
        let baseline = default_paper_accounts()
            .iter()
            .map(|(_, value)| value)
            .sum::<f64>();
        let total_pnl = total_equity - baseline;
        let daily_pnl = state
            .positions
            .iter()
            .map(|position| daily_pnl_usdt(position, &snapshots))
            .sum::<f64>();
        let open_positions = state.positions.len();
        let active_exchanges = state
            .positions
            .iter()
            .map(|position| position.exchange.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let risk_summary = if open_positions == 0 {
            "模拟账户持有 100 万人民币现金，当前没有持仓。".into()
        } else {
            format!(
                "模拟账户当前有 {open_positions} 个持仓，分布在 {active_exchanges} 个资金账户。"
            )
        };
        let daily_pnl_percent = if baseline <= 0.0 {
            0.0
        } else {
            (daily_pnl / baseline) * 100.0
        };

        Ok(PortfolioOverviewDto {
            total_equity_usdt: round_money(total_equity),
            total_market_value_usdt: round_money(total_market_value),
            total_pnl_usdt: round_money(total_pnl),
            daily_pnl_usdt: round_money(daily_pnl),
            daily_pnl_percent,
            account_mode: "paper".into(),
            risk_summary,
            exchanges,
        })
    }

    pub async fn build_positions(
        &self,
        market_data_service: &MarketDataService,
    ) -> anyhow::Result<Vec<PositionDto>> {
        let snapshots = market_data_service.list_markets().await.unwrap_or_default();
        let mut state = self
            .state
            .write()
            .map_err(|_| anyhow!("failed to lock paper state"))?;
        let exits = refresh_positions(&mut state, &snapshots);
        if !exits.is_empty() {
            self.persist_state(&state)?;
        }

        let mut positions = state
            .positions
            .iter()
            .map(position_to_dto)
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| {
            left.exchange
                .cmp(&right.exchange)
                .then(left.symbol.cmp(&right.symbol))
        });
        Ok(positions)
    }

    fn persist_state(&self, state: &PaperState) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            persist_paper_state(path, state)?;
        }
        Ok(())
    }
}

fn default_paper_state() -> PaperState {
    let mut accounts = HashMap::new();
    for (exchange, balance) in default_paper_accounts() {
        let account_id = paper_account_id(exchange);
        accounts.insert(
            account_id.clone(),
            PaperAccount {
                account_id,
                exchange: exchange.to_string(),
                available_usdt: balance,
            },
        );
    }

    PaperState {
        accounts,
        positions: Vec::new(),
        orders: Vec::new(),
        next_order_id: 1,
    }
}

fn normalize_loaded_state(state: &mut PaperState) {
    for (exchange, balance) in default_paper_accounts() {
        let account_id = paper_account_id(exchange);
        state
            .accounts
            .entry(account_id.clone())
            .or_insert(PaperAccount {
                account_id,
                exchange: exchange.to_string(),
                available_usdt: balance,
            });
    }

    let next_from_orders = state
        .orders
        .iter()
        .filter_map(|order| order.order_id.strip_prefix("PO-"))
        .filter_map(|value| value.parse::<usize>().ok())
        .max()
        .map(|value| value + 1)
        .unwrap_or(1);
    state.next_order_id = state.next_order_id.max(next_from_orders).max(1);
}

fn ensure_paper_state_table(path: &PathBuf) -> anyhow::Result<()> {
    let db = Database::open(path)?;
    db.connection().execute(
        "CREATE TABLE IF NOT EXISTS paper_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn load_paper_state(path: &PathBuf) -> anyhow::Result<Option<PaperState>> {
    let db = Database::open(path)?;
    let payload = db
        .connection()
        .query_row(
            "SELECT value FROM paper_state WHERE key = 'state'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    payload
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

fn persist_paper_state(path: &PathBuf, state: &PaperState) -> anyhow::Result<()> {
    let db = Database::open(path)?;
    db.connection().execute(
        "CREATE TABLE IF NOT EXISTS paper_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    db.connection().execute(
        "INSERT INTO paper_state (key, value, updated_at)
         VALUES ('state', ?1, ?2)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at = excluded.updated_at",
        params![serde_json::to_string(state)?, current_timestamp_marker()],
    )?;
    Ok(())
}

pub fn default_paper_accounts() -> Vec<(&'static str, f64)> {
    vec![("人民币现金", 1_000_000.0)]
}

pub fn paper_account_id(exchange: &str) -> String {
    if exchange == "人民币现金" {
        return "paper-cash".into();
    }
    format!("paper-{}", exchange.to_lowercase())
}

fn average_price(low: Option<f64>, high: Option<f64>) -> Option<f64> {
    match (low, high) {
        (Some(low), Some(high)) => Some((low + high) / 2.0),
        (Some(low), None) => Some(low),
        (None, Some(high)) => Some(high),
        (None, None) => None,
    }
}

fn parse_take_profit(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|value| {
        value
            .split('/')
            .next()
            .map(str::trim)
            .map(|item| item.replace(',', ""))
            .and_then(|item| item.parse::<f64>().ok())
    })
}

fn position_to_dto(position: &PaperPosition) -> PositionDto {
    PositionDto {
        exchange: position.exchange.clone(),
        symbol: position.symbol.clone(),
        side: position.side.clone(),
        size: quantity_label(&position.symbol, position.quantity),
        entry_price: position.entry_price,
        mark_price: position.mark_price,
        pnl_percent: unrealized_pnl_pct(position),
        leverage: format!("{:.0}x", position.leverage),
    }
}

fn order_to_dto(order: &PaperOrderRecord) -> PaperOrderRowDto {
    PaperOrderRowDto {
        order_id: order.order_id.clone(),
        exchange: order.exchange.clone(),
        symbol: order.symbol.clone(),
        order_type: order.order_type.clone(),
        status: order.status.clone(),
        quantity: quantity_label(&order.symbol, order.quantity),
        estimated_fill_price: order.estimated_fill_price,
        realized_pnl_usdt: order.realized_pnl_usdt,
        updated_at: order.updated_at.clone(),
    }
}

fn quantity_label(symbol: &str, quantity: f64) -> String {
    if symbol.starts_with("SHSE.") || symbol.starts_with("SZSE.") {
        if (quantity.fract()).abs() < f64::EPSILON {
            format!("{quantity:.0} 股")
        } else {
            format!("{quantity:.3} 股")
        }
    } else {
        let asset = symbol.split('/').next().unwrap_or("asset");
        format!("{quantity:.3} {asset}")
    }
}

fn unrealized_pnl_pct(position: &PaperPosition) -> f64 {
    let raw_return = if position.entry_price <= 0.0 {
        0.0
    } else if position.side.eq_ignore_ascii_case("short") {
        ((position.entry_price - position.mark_price) / position.entry_price) * 100.0
    } else {
        ((position.mark_price - position.entry_price) / position.entry_price) * 100.0
    };

    if position.market_type == "spot" {
        raw_return
    } else {
        raw_return * position.leverage
    }
}

fn unrealized_pnl_usdt(position: &PaperPosition) -> f64 {
    position.margin_usdt * (unrealized_pnl_pct(position) / 100.0)
}

fn daily_pnl_usdt(position: &PaperPosition, snapshots: &[MarketListRow]) -> f64 {
    let Some(snapshot) = snapshots
        .iter()
        .find(|row| row.symbol == position.symbol && row.market_type == position.market_type)
    else {
        return 0.0;
    };
    let change_ratio = snapshot.change_24h / 100.0;
    if snapshot.last_price <= 0.0 || change_ratio <= -1.0 {
        return 0.0;
    }
    let previous_close = snapshot.last_price / (1.0 + change_ratio);
    let direction = if position.side.eq_ignore_ascii_case("short") {
        -1.0
    } else {
        1.0
    };
    (position.mark_price - previous_close) * position.quantity * position.leverage * direction
}

fn refresh_positions(
    state: &mut PaperState,
    snapshots: &[crate::models::MarketListRow],
) -> Vec<PaperExitEvent> {
    let mut closed = Vec::new();

    for (index, position) in state.positions.iter_mut().enumerate() {
        let mut updated_at = if position.updated_at.is_empty() {
            current_timestamp_marker()
        } else {
            position.updated_at.clone()
        };
        if let Some(snapshot) = snapshots
            .iter()
            .find(|row| row.symbol == position.symbol && row.market_type == position.market_type)
        {
            position.mark_price = snapshot.last_price;
            position.updated_at = snapshot.updated_at.clone();
            updated_at = snapshot.updated_at.clone();
        }

        if let Some(status) = exit_status(position) {
            closed.push((index, status, updated_at));
        }
    }

    let mut exits = Vec::new();
    for (index, status, updated_at) in closed.into_iter().rev() {
        let position = state.positions.remove(index);
        let realized_pnl_usdt = round_money(unrealized_pnl_usdt(&position));
        if let Some(account) = state.accounts.get_mut(&position.account_id) {
            account.available_usdt += position.margin_usdt + realized_pnl_usdt;
        }
        if let Some(order) = state
            .orders
            .iter_mut()
            .find(|order| order.order_id == position.entry_order_id)
        {
            order.status = status.to_string();
            order.realized_pnl_usdt = Some(realized_pnl_usdt);
            order.updated_at = updated_at.clone();
        }

        exits.push(PaperExitEvent {
            order_id: position.entry_order_id,
            exchange: position.exchange,
            symbol: position.symbol,
            status: status.to_string(),
            realized_pnl_usdt,
            updated_at,
        });
    }

    exits.reverse();
    exits
}

fn exit_status(position: &PaperPosition) -> Option<&'static str> {
    let stop_hit = match position.stop_loss {
        Some(stop) if position.side.eq_ignore_ascii_case("short") => position.mark_price >= stop,
        Some(stop) => position.mark_price <= stop,
        None => false,
    };
    let take_profit_hit = match position.take_profit {
        Some(target) if position.side.eq_ignore_ascii_case("short") => {
            position.mark_price <= target
        }
        Some(target) => position.mark_price >= target,
        None => false,
    };

    if stop_hit {
        Some("Closed Stop Loss")
    } else if take_profit_hit {
        Some("Closed Take Profit")
    } else {
        None
    }
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn order_type(market_type: &str, direction: &str) -> String {
    match (market_type, direction.to_lowercase().as_str()) {
        ("spot", _) => "Spot Buy".into(),
        ("perpetual", "short") => "Perpetual Short".into(),
        ("perpetual", _) => "Perpetual Long".into(),
        _ => "Paper Entry".into(),
    }
}

fn current_timestamp_marker() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("epoch:{seconds}")
}
