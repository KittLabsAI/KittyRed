import { invoke } from "@tauri-apps/api/core";
import type {
  AnalyzeJob,
  AShareSymbolSearchResult,
  ArbitrageOpportunity,
  ArbitrageOpportunityPage,
  ArbitrageTypeFilter,
  CandleSeries,
  CoinInfo,
  MarketRow,
  NotificationEvent,
  OrderBookSourceSnapshot,
  PortfolioOverview,
  PairDetailSnapshot,
  PairVenueSnapshot,
  ManualPaperOrderRequest,
  PaperAccountSummary,
  PaperOrderDraft,
  OrderRow,
  PriceLevel,
  RecentTradeRow,
  RecommendationAudit,
  RecommendationHistoryRow,
  RiskDecision,
  RecommendationRun,
  ScanRunHistoryPage,
  SignalHistoryPage,
  SpreadOpportunity,
  StrategyConfig,
  StrategyMeta,
  StrategyStats,
  UnifiedSignal,
  PositionRow,
} from "./types";
import type { AssistantEvent } from "./assistantTypes";
import { useMarketStore } from "../store/marketStore";
import { usePortfolioStore } from "../store/portfolioStore";
import { useRecommendationStore } from "../store/recommendationStore";

type MarketListRowDto = {
  symbol: string;
  base_asset: string;
  market_type: string;
  market_cap_usd: number | null;
  market_cap_rank: number | null;
  market_size_tier: "large" | "mid" | "small";
  last_price: number;
  change_24h: number;
  volume_24h: number;
  funding_rate: number | null;
  spread_bps: number;
  exchanges: string[];
  updated_at: string;
  stale: boolean;
  venue_snapshots: VenueTickerSnapshotDto[];
  best_bid_exchange: string | null;
  best_ask_exchange: string | null;
  best_bid_price: number | null;
  best_ask_price: number | null;
  responded_exchange_count: number;
  fdv_usd: number | null;
};

type AShareSymbolSearchResultDto = {
  symbol: string;
  name: string;
  market: string;
};

type VenueTickerSnapshotDto = {
  exchange: string;
  last_price: number;
  bid_price: number;
  ask_price: number;
  volume_24h: number;
  funding_rate: number | null;
  mark_price: number | null;
  index_price: number | null;
  updated_at: string;
  stale: boolean;
};

type PairVenueSnapshotDto = {
  exchange: string;
  last_price: number;
  bid_price: number;
  ask_price: number;
  change_pct: number;
  volume_24h: number;
  funding_rate: number | null;
  mark_price: number | null;
  index_price: number | null;
  open_interest: string | null;
  next_funding_at: string | null;
  updated_at: string;
};

type PairDetailDto = {
  symbol: string;
  market_type: string;
  thesis: string;
  source_note: string;
  coin_info: {
    name: string;
    symbol: string;
    summary: string;
    website: string | null;
    whitepaper: string | null;
    explorer: string | null;
    ecosystem: string;
    market_cap: string | null;
    fdv: string | null;
    circulating_supply: string | null;
    total_supply: string | null;
    max_supply: string | null;
    volume_24h: string | null;
    listed_exchanges: string[];
    risk_tags: string[];
    github: string | null;
  };
  venues: PairVenueSnapshotDto[];
  orderbooks: Array<{
    exchange: string;
    bids: Array<{ price: number; size: number }>;
    asks: Array<{ price: number; size: number }>;
    updated_at: string;
  }>;
  recent_trades: Array<{
    exchange: string;
    side: string;
    price: number;
    size: number;
    timestamp: string;
  }>;
  spreads: SpreadOpportunityDto[];
};

type CandleSeriesDto = {
  exchange: string;
  symbol: string;
  market_type: string;
  interval: string;
  updated_at: string;
  bars: Array<{
    open_time: string;
    open: number;
    high: number;
    low: number;
    close: number;
    volume: number;
    turnover?: number | null;
  }>;
};

type SpreadOpportunityDto = {
  symbol: string;
  buy_exchange: string;
  sell_exchange: string;
  net_spread_pct: number;
  funding_context: string;
};

type ArbitrageOpportunityDto = {
  symbol: string;
  opportunity_type: string;
  primary_market_type: "spot" | "perpetual";
  secondary_market_type: "spot" | "perpetual" | null;
  buy_exchange: string;
  buy_market_type: "spot" | "perpetual";
  buy_price: number;
  sell_exchange: string;
  sell_market_type: "spot" | "perpetual";
  sell_price: number;
  fee_adjusted_net_spread_pct: number;
  simulated_carry_pct: number;
  simulated_total_yield_pct: number;
  liquidity_usdt_24h: number;
  market_cap_usd: number | null;
  funding_rate: number | null;
  borrow_rate_daily: number | null;
  recommendation_score: number;
  updated_at: string;
  stale: boolean;
};

type ArbitrageOpportunityPageDto = {
  items: ArbitrageOpportunityDto[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
};

type RecommendationRunDto = {
  recommendation_id: string;
  status: string;
  trigger_type: string;
  has_trade: boolean;
  symbol: string | null;
  stock_name: string | null;
  direction: string | null;
  market_type: string;
  exchanges: string[];
  confidence_score: number;
  rationale: string;
  symbol_recommendations?: Array<{
    symbol: string;
    stock_name?: string | null;
    direction: string | null;
    rationale: string;
    risk_status: string;
    has_trade: boolean;
  }>;
  risk_status: string;
  entry_low: number | null;
  entry_high: number | null;
  stop_loss: number | null;
  take_profit: string | null;
  leverage: number | null;
  amount_cny: number | null;
  invalidation: string | null;
  max_loss_cny: number | null;
  no_trade_reason: string | null;
  risk_details: RiskDecisionDto;
  data_snapshot_at: string;
  model_provider: string;
  model_name: string;
  prompt_version: string;
  user_preference_version: string;
  generated_at: string;
};

type RiskCheckDto = {
  name: string;
  status: string;
  detail: string | null;
};

type RiskDecisionDto = {
  status: string;
  risk_score: number;
  max_loss_estimate: string | null;
  checks: RiskCheckDto[];
  modifications: string[];
  block_reasons: string[];
};

type RecommendationHistoryRowDto = {
  recommendation_id: string;
  created_at: string;
  trigger_type: string;
  symbol: string;
  stock_name: string;
  shortlist: string[];
  exchange: string;
  market_type: string;
  direction: string;
  rationale: string;
  risk_status: string;
  result: string;
  entry_low: number | null;
  entry_high: number | null;
  stop_loss: number | null;
  take_profit: string | null;
  leverage: number | null;
  confidence_score: number;
  model_name: string;
  prompt_version: string;
  executed: boolean;
  modified: boolean;
  pnl_5m: number;
  pnl_10m: number;
  pnl_30m: number;
  pnl_60m: number;
  pnl_24h: number;
  pnl_7d: number;
  outcome: string;
};

type RecommendationAuditDto = {
  recommendation_id: string;
  trigger_type: string;
  symbol: string;
  exchange: string;
  market_type: string;
  created_at: string;
  model_provider: string;
  model_name: string;
  prompt_version: string;
  user_preference_version: string;
  ai_raw_output: string;
  ai_structured_output: string;
  risk_result: string;
  market_snapshot: string;
  account_snapshot: string;
};

type PaperOrderDraftDto = {
  order_id: string;
  account_id: string;
  exchange: string;
  symbol: string;
  side: string;
  quantity: number;
  estimated_fill_price: number;
  stop_loss: number | null;
  take_profit: number | null;
};

type NotificationEventDto = {
  event_id: string;
  channel: string;
  title: string;
  body: string;
  status: string;
  created_at: string;
};

type PaperAccountDto = {
  account_id: string;
  exchange: string;
  available_usdt: number;
};

type PaperOrderRowDto = {
  order_id: string;
  exchange: string;
  symbol: string;
  order_type: string;
  status: string;
  quantity: string;
  estimated_fill_price: number;
  realized_pnl_usdt: number | null;
  updated_at: string;
};

type PortfolioOverviewDto = {
  total_equity_usdt: number;
  total_market_value_usdt: number;
  total_pnl_usdt: number;
  daily_pnl_usdt: number;
  daily_pnl_percent: number;
  account_mode: string;
  risk_summary: string;
  exchanges: Array<{
    exchange: string;
    equity_usdt: number;
    change_percent: number;
  }>;
};

type PositionDto = {
  exchange: string;
  symbol: string;
  side: string;
  size: string;
  entry_price: number;
  mark_price: number;
  pnl_percent: number;
  leverage: string;
};

type JobRecordDto = {
  id: number;
  kind: string;
  status: string;
  message: string;
  started_at: string;
  updated_at: string;
  ended_at: string | null;
  duration_ms: number | null;
  input_params_json: string | null;
  result_summary: string | null;
  error_details: string | null;
};

type AssistantCommandAckDto = {
  started: boolean;
};

type AssistantStopAckDto = {
  stopped: boolean;
};

type AssistantClearAckDto = {
  cleared: boolean;
};

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const localAssistantTimers = new Map<string, number[]>();

function emitBrowserAssistantEvent(event: AssistantEvent) {
  if (typeof window === "undefined") {
    return;
  }
  window.dispatchEvent(
    new CustomEvent<AssistantEvent>("kittyred-assistant-event", {
      detail: event,
    }),
  );
}

function clearLocalAssistantTimers(sessionId: string) {
  for (const timeoutId of localAssistantTimers.get(sessionId) ?? []) {
    window.clearTimeout(timeoutId);
  }
  localAssistantTimers.delete(sessionId);
}

function scheduleLocalAssistantEvent(
  sessionId: string,
  delayMs: number,
  buildEvent: () => AssistantEvent,
) {
  if (typeof window === "undefined") {
    return;
  }
  const timeoutId = window.setTimeout(() => {
    const timers = localAssistantTimers.get(sessionId);
    if (!timers?.includes(timeoutId)) {
      return;
    }
    emitBrowserAssistantEvent(buildEvent());
  }, delayMs);
  localAssistantTimers.set(sessionId, [
    ...(localAssistantTimers.get(sessionId) ?? []),
    timeoutId,
  ]);
}

function mapVenue(dto: PairVenueSnapshotDto): PairVenueSnapshot {
  return {
    exchange: dto.exchange,
    last: dto.last_price,
    bid: dto.bid_price,
    ask: dto.ask_price,
    changePct: dto.change_pct,
    volume24h: dto.volume_24h,
    funding: dto.funding_rate ?? undefined,
    mark: dto.mark_price ?? undefined,
    index: dto.index_price ?? undefined,
    openInterest: dto.open_interest ?? undefined,
    nextFundingAt: dto.next_funding_at ?? undefined,
    updatedAt: dto.updated_at,
  };
}

function mapVenueTickerSnapshot(dto: VenueTickerSnapshotDto): PairVenueSnapshot {
  return {
    exchange: dto.exchange,
    last: dto.last_price,
    bid: dto.bid_price,
    ask: dto.ask_price,
    volume24h: dto.volume_24h,
    funding: dto.funding_rate ?? undefined,
    mark: dto.mark_price ?? undefined,
    index: dto.index_price ?? undefined,
    updatedAt: dto.updated_at,
  };
}

function mapLevel(level: { price: number; size: number }): PriceLevel {
  return {
    price: level.price,
    size: level.size,
  };
}

function mapTrade(trade: {
  exchange: string;
  side: string;
  price: number;
  size: number;
  timestamp: string;
}): RecentTradeRow {
  return {
    exchange: trade.exchange,
    side: trade.side,
    price: trade.price,
    size: trade.size,
    timestamp: trade.timestamp,
  };
}

function mapCoinInfo(dto: PairDetailDto["coin_info"]): CoinInfo {
  return {
    name: dto.name,
    symbol: dto.symbol,
    summary: dto.summary,
    website: dto.website ?? undefined,
    whitepaper: dto.whitepaper ?? undefined,
    explorer: dto.explorer ?? undefined,
    ecosystem: dto.ecosystem,
    marketCap: dto.market_cap ?? undefined,
    fdv: dto.fdv ?? undefined,
    circulatingSupply: dto.circulating_supply ?? undefined,
    totalSupply: dto.total_supply ?? undefined,
    maxSupply: dto.max_supply ?? undefined,
    volume24h: dto.volume_24h ?? undefined,
    listedExchanges: dto.listed_exchanges,
    riskTags: dto.risk_tags,
    github: dto.github ?? undefined,
  };
}

function mapCandleSeries(dto: CandleSeriesDto): CandleSeries {
  return {
    exchange: dto.exchange,
    symbol: dto.symbol,
    marketType: "A 股",
    interval: dto.interval,
    updatedAt: dto.updated_at,
    bars: dto.bars.map((bar) => ({
      openTime: bar.open_time,
      open: bar.open,
      high: bar.high,
      low: bar.low,
      close: bar.close,
      volume: bar.volume,
      turnover: bar.turnover ?? undefined,
    })),
  };
}

function mapOrderBookSource(dto: PairDetailDto["orderbooks"][number]): OrderBookSourceSnapshot {
  return {
    exchange: dto.exchange,
    bids: dto.bids.map(mapLevel),
    asks: dto.asks.map(mapLevel),
    updatedAt: dto.updated_at,
  };
}

function mapMarketRow(dto: MarketListRowDto): MarketRow {
  return {
    symbol: dto.symbol,
    baseAsset: dto.base_asset,
    marketType: "A 股",
    marketCapUsd: dto.market_cap_usd ?? undefined,
    marketCapRank: dto.market_cap_rank ?? undefined,
    marketSizeTier: dto.market_size_tier,
    last: dto.last_price,
    change24h: dto.change_24h,
    volume24h: dto.volume_24h,
    funding: dto.funding_rate ?? undefined,
    spreadBps: dto.spread_bps,
    venues: dto.exchanges.length > 0 ? dto.exchanges : ["akshare"],
    updatedAt: dto.updated_at,
    stale: dto.stale,
    venueSnapshots: dto.venue_snapshots.map(mapVenueTickerSnapshot),
    bestBidExchange: dto.best_bid_exchange ?? undefined,
    bestAskExchange: dto.best_ask_exchange ?? undefined,
    bestBidPrice: dto.best_bid_price ?? undefined,
    bestAskPrice: dto.best_ask_price ?? undefined,
    respondedExchangeCount: dto.responded_exchange_count,
    fdvUsd: dto.fdv_usd ?? undefined,
  };
}

function mapArbitrageOpportunity(dto: ArbitrageOpportunityDto): ArbitrageOpportunity {
  return {
    symbol: dto.symbol,
    opportunityType: dto.opportunity_type,
    primaryMarketType: dto.primary_market_type,
    secondaryMarketType: dto.secondary_market_type ?? undefined,
    buyExchange: dto.buy_exchange,
    buyMarketType: dto.buy_market_type,
    buyPrice: dto.buy_price,
    sellExchange: dto.sell_exchange,
    sellMarketType: dto.sell_market_type,
    sellPrice: dto.sell_price,
    feeAdjustedNetSpreadPct: dto.fee_adjusted_net_spread_pct,
    simulatedCarryPct: dto.simulated_carry_pct,
    simulatedTotalYieldPct: dto.simulated_total_yield_pct,
    liquidity24h: dto.liquidity_usdt_24h,
    marketCapUsd: dto.market_cap_usd ?? undefined,
    fundingRate: dto.funding_rate ?? undefined,
    borrowRateDaily: dto.borrow_rate_daily ?? undefined,
    recommendationScore: dto.recommendation_score,
    updatedAt: dto.updated_at,
    stale: dto.stale,
  };
}

function filterArbitrageItems(
  items: ArbitrageOpportunity[],
  typeFilter: ArbitrageTypeFilter,
): ArbitrageOpportunity[] {
  switch (typeFilter) {
    case "spot":
      return items.filter((item) => item.opportunityType === "spot_cross_exchange");
    case "perpetual":
      return items.filter((item) => item.opportunityType === "perpetual_cross_exchange");
    case "cross_market":
      return items.filter((item) => item.secondaryMarketType !== undefined);
    default:
      return items;
  }
}

function mapRecommendation(dto: RecommendationRunDto): RecommendationRun {
  return {
    id: dto.recommendation_id,
    status: dto.status,
    triggerType: dto.trigger_type,
    hasTrade: dto.has_trade,
    symbol: dto.symbol ?? "市场扫描",
    stockName: dto.stock_name ?? undefined,
    marketType: "A 股",
    direction: dto.direction ?? "不交易",
    venues: dto.exchanges.length > 0 ? dto.exchanges : ["模拟账户"],
    confidence: dto.confidence_score,
    riskStatus: dto.risk_status,
    thesis: dto.rationale,
    symbolRecommendations: dto.symbol_recommendations?.map((item) => ({
      symbol: item.symbol,
      stockName: item.stock_name ?? undefined,
      direction: item.direction ?? "观望",
      thesis: item.rationale,
      riskStatus: item.risk_status,
      hasTrade: item.has_trade,
    })),
    entryLow: dto.entry_low ?? undefined,
    entryHigh: dto.entry_high ?? undefined,
    stopLoss: dto.stop_loss ?? undefined,
    takeProfit: dto.take_profit ?? undefined,
    leverage: dto.leverage ?? undefined,
    amountCny: dto.amount_cny ?? undefined,
    invalidation: dto.invalidation ?? undefined,
    maxLossCny: dto.max_loss_cny ?? undefined,
    riskDetails: mapRiskDecision(dto.risk_details),
    dataSnapshotAt: dto.data_snapshot_at,
    modelProvider: dto.model_provider,
    modelName: dto.model_name,
    promptVersion: dto.prompt_version,
    userPreferenceVersion: dto.user_preference_version,
    generatedAt: dto.generated_at,
  };
}

function mapRiskDecision(dto: RiskDecisionDto): RiskDecision {
  return {
    status: dto.status,
    riskScore: dto.risk_score,
    maxLossEstimate: dto.max_loss_estimate ?? undefined,
    checks: dto.checks.map((check) => ({
      name: check.name,
      status: check.status,
      detail: check.detail ?? undefined,
    })),
    modifications: dto.modifications,
    blockReasons: dto.block_reasons,
  };
}

function mapRecommendationHistoryRow(dto: RecommendationHistoryRowDto): RecommendationHistoryRow {
  return {
    id: dto.recommendation_id,
    createdAt: dto.created_at,
    triggerType: dto.trigger_type,
    symbol: dto.symbol,
    stockName: dto.stock_name,
    shortlist: dto.shortlist,
    exchange: "模拟账户",
    marketType: "A 股",
    direction: dto.direction,
    rationale: dto.rationale,
    risk: dto.risk_status,
    result: dto.result,
    entryLow: dto.entry_low ?? undefined,
    entryHigh: dto.entry_high ?? undefined,
    stopLoss: dto.stop_loss ?? undefined,
    takeProfit: dto.take_profit ?? undefined,
    leverage: dto.leverage ?? undefined,
    confidence: dto.confidence_score,
    modelName: dto.model_name,
    promptVersion: dto.prompt_version,
    executed: dto.executed,
    modified: dto.modified,
    pnl5m: dto.pnl_5m,
    pnl10m: dto.pnl_10m,
    pnl30m: dto.pnl_30m,
    pnl60m: dto.pnl_60m,
    pnl24h: dto.pnl_24h,
    pnl7d: dto.pnl_7d,
    outcome: dto.outcome,
  };
}

function mapRecommendationAudit(dto: RecommendationAuditDto): RecommendationAudit {
  return {
    recommendationId: dto.recommendation_id,
    triggerType: dto.trigger_type,
    symbol: dto.symbol,
    exchange: "模拟账户",
    marketType: "A 股",
    createdAt: dto.created_at,
    modelProvider: dto.model_provider,
    modelName: dto.model_name,
    promptVersion: dto.prompt_version,
    userPreferenceVersion: dto.user_preference_version,
    aiRawOutput: dto.ai_raw_output,
    aiStructuredOutput: dto.ai_structured_output,
    riskResult: dto.risk_result,
    marketSnapshot: dto.market_snapshot,
    accountSnapshot: dto.account_snapshot,
  };
}

function mapPaperOrderDraft(dto: PaperOrderDraftDto): PaperOrderDraft {
  return {
    orderId: dto.order_id,
    accountId: dto.account_id,
    exchange: dto.exchange,
    symbol: dto.symbol,
    side: dto.side,
    quantity: dto.quantity,
    estimatedFillPrice: dto.estimated_fill_price,
    stopLoss: dto.stop_loss ?? undefined,
    takeProfit: dto.take_profit ?? undefined,
  };
}

function mapPaperAccount(dto: PaperAccountDto): PaperAccountSummary {
  return {
    accountId: dto.account_id,
    exchange: dto.exchange,
    availableUsdt: dto.available_usdt,
  };
}

function mapPaperOrder(dto: PaperOrderRowDto): OrderRow {
  return {
    id: dto.order_id,
    exchange: dto.exchange,
    symbol: dto.symbol,
    type: dto.order_type,
    status: dto.status,
    quantity: dto.quantity,
    fillPrice: dto.estimated_fill_price,
    realizedPnl: dto.realized_pnl_usdt ?? undefined,
    updatedAt: dto.updated_at,
  };
}

function mapPortfolioOverview(dto: PortfolioOverviewDto): PortfolioOverview {
  const total = dto.exchanges.reduce((sum, exchange) => sum + exchange.equity_usdt, 0);
  return {
    totalEquity: dto.total_equity_usdt,
    totalMarketValue: dto.total_market_value_usdt,
    totalPnl: dto.total_pnl_usdt,
    todayPnl: dto.daily_pnl_usdt,
    todayPnlPct: dto.daily_pnl_percent,
    riskSummary: dto.risk_summary,
    exchanges: dto.exchanges.map((exchange) => ({
      name: exchange.exchange,
      equity: exchange.equity_usdt,
      weight: total > 0 ? Math.round((exchange.equity_usdt / total) * 1000) / 10 : 0,
    })),
  };
}

function mapPosition(dto: PositionDto): PositionRow {
  return {
    exchange: dto.exchange,
    symbol: dto.symbol,
    side:
      dto.side.toLowerCase() === "short"
        ? "Short"
        : dto.side.toLowerCase() === "spot"
          ? "Holding"
          : "Long",
    size: dto.size,
    entry: dto.entry_price,
    mark: dto.mark_price,
    pnlPct: dto.pnl_percent,
    leverage: dto.leverage,
  };
}

function mapAnalyzeJob(dto: JobRecordDto): AnalyzeJob {
  return {
    id: dto.id,
    kind: dto.kind,
    status: dto.status as AnalyzeJob["status"],
    message: dto.message,
    startedAt: dto.started_at,
    updatedAt: dto.updated_at,
    endedAt: dto.ended_at,
    durationMs: dto.duration_ms,
    inputParamsJson: dto.input_params_json,
    resultSummary: dto.result_summary,
    errorDetails: dto.error_details,
  };
}

function mapNotificationEvent(dto: NotificationEventDto): NotificationEvent {
  return {
    id: dto.event_id,
    channel: dto.channel,
    title: dto.title,
    body: dto.body,
    status: dto.status,
    createdAt: dto.created_at,
  };
}

export async function listMarkets(): Promise<MarketRow[]> {
  if (!isTauriRuntime()) {
    return useMarketStore.getState().markets;
  }

  const rows = await invoke<MarketListRowDto[]>("list_markets");
  return rows.map(mapMarketRow);
}

const fallbackAShareSymbols: AShareSymbolSearchResult[] = [
  { symbol: "SHSE.600000", name: "浦发银行", market: "沪市A股" },
  { symbol: "SZSE.000001", name: "平安银行", market: "深市A股" },
  { symbol: "SHSE.600519", name: "贵州茅台", market: "沪市A股" },
  { symbol: "SHSE.601318", name: "中国平安", market: "沪市A股" },
  { symbol: "SZSE.300750", name: "宁德时代", market: "深市A股" },
];

export async function searchAShareSymbols(
  query: string,
): Promise<AShareSymbolSearchResult[]> {
  const normalized = query.trim().toLowerCase();
  if (normalized.length === 0) {
    return [];
  }

  if (!isTauriRuntime()) {
    return fallbackAShareSymbols
      .filter(
        (item) =>
          item.symbol.toLowerCase().includes(normalized) ||
          item.symbol.split(".").pop()?.includes(normalized) ||
          item.name.toLowerCase().includes(normalized),
      )
      .slice(0, 8);
  }

  return invoke<AShareSymbolSearchResultDto[]>("search_a_share_symbols", {
    query,
  });
}

export async function refreshWatchlistTickers(): Promise<MarketRow[]> {
  if (!isTauriRuntime()) {
    return useMarketStore.getState().markets;
  }

  const rows = await invoke<MarketListRowDto[]>("refresh_watchlist_tickers");
  return rows.map(mapMarketRow);
}

export async function listMarketSymbols(): Promise<string[]> {
  if (!isTauriRuntime()) {
    return Array.from(
      new Set([
        ...useMarketStore.getState().markets.map((row) => row.symbol),
        useMarketStore.getState().pairDetail.symbol,
      ]),
    ).sort((left, right) => left.localeCompare(right));
  }

  return invoke<string[]>("list_market_symbols");
}

export async function getPortfolioOverview(): Promise<PortfolioOverview> {
  if (!isTauriRuntime()) {
    return usePortfolioStore.getState().overview;
  }

  const dto = await invoke<PortfolioOverviewDto>("get_portfolio_overview");
  return mapPortfolioOverview(dto);
}

export async function listPositions(): Promise<PositionRow[]> {
  if (!isTauriRuntime()) {
    return usePortfolioStore.getState().positions;
  }

  const rows = await invoke<PositionDto[]>("list_positions");
  return rows.map(mapPosition);
}

export async function listPaperOrders(): Promise<OrderRow[]> {
  if (!isTauriRuntime()) {
    return usePortfolioStore.getState().orders;
  }

  const rows = await invoke<PaperOrderRowDto[]>("list_paper_orders");
  return rows.map(mapPaperOrder);
}

export async function listOrders(): Promise<OrderRow[]> {
  if (!isTauriRuntime()) {
    return usePortfolioStore.getState().orders;
  }

  const rows = await invoke<PaperOrderRowDto[]>("list_orders");
  return rows.map(mapPaperOrder);
}

export async function getPairDetail(
  symbol: string,
  marketType: string,
  exchange = "auto",
): Promise<PairDetailSnapshot> {
  if (!isTauriRuntime()) {
    return useMarketStore.getState().pairDetail;
  }

  const dto = await invoke<PairDetailDto>("get_pair_detail", {
    symbol,
    marketType,
    exchange,
  });
  return {
    symbol: dto.symbol,
    marketType: "A 股",
    thesis: dto.thesis,
    sourceNote: dto.source_note,
    coinInfo: mapCoinInfo(dto.coin_info),
    venues: dto.venues.map(mapVenue),
    orderbooks: dto.orderbooks.map(mapOrderBookSource),
    recentTrades: dto.recent_trades.map(mapTrade),
    spreads: dto.spreads.map((row) => ({
      symbol: row.symbol,
      buyExchange: row.buy_exchange,
      sellExchange: row.sell_exchange,
      netSpreadPct: row.net_spread_pct,
      funding: row.funding_context,
      liquidity: "实时",
    })),
  };
}

export async function getPairCandles(
  symbol: string,
  marketType: string,
  interval: string,
  exchange = "auto",
): Promise<CandleSeries> {
  if (!isTauriRuntime()) {
    return {
      exchange: "akshare",
      symbol,
      marketType: "A 股",
      interval,
      updatedAt: "2026-05-03T19:00:00+08:00",
      bars: [
        { openTime: "1714734000000", open: 8.68, high: 8.75, low: 8.64, close: 8.72, volume: 820000 },
        { openTime: "1714820400000", open: 8.72, high: 8.81, low: 8.7, close: 8.78, volume: 910000 },
        { openTime: "1714906800000", open: 8.78, high: 8.84, low: 8.73, close: 8.79, volume: 760000 },
        { openTime: "1714993200000", open: 8.79, high: 8.86, low: 8.76, close: 8.83, volume: 880000 },
      ],
    };
  }

  const dto = await invoke<CandleSeriesDto>("get_pair_candles", {
    symbol,
    marketType,
    interval,
    exchange,
  });
  return mapCandleSeries(dto);
}

export async function listSpreadOpportunities(): Promise<SpreadOpportunity[]> {
  if (!isTauriRuntime()) {
    return useMarketStore.getState().spreads;
  }

  const rows = await invoke<SpreadOpportunityDto[]>("list_spread_opportunities");
  return rows.map((row) => ({
    symbol: row.symbol,
    buyExchange: row.buy_exchange,
    sellExchange: row.sell_exchange,
    netSpreadPct: row.net_spread_pct,
    funding: row.funding_context,
    liquidity: "Live",
  }));
}

export async function listArbitrageOpportunities(
  page = 1,
  pageSize = 25,
  typeFilter: ArbitrageTypeFilter = "all",
): Promise<ArbitrageOpportunityPage> {
  if (!isTauriRuntime()) {
    const safePage = Math.max(1, page);
    const safePageSize = Math.max(1, pageSize);
    const items = filterArbitrageItems(useMarketStore.getState().arbitrage, typeFilter);
    const total = items.length;
    const totalPages = Math.max(1, Math.ceil(total / safePageSize));
    const start = (safePage - 1) * safePageSize;
    return {
      items: items.slice(start, start + safePageSize),
      total,
      page: safePage,
      pageSize: safePageSize,
      totalPages,
    };
  }

  const dto = await invoke<ArbitrageOpportunityPageDto>("list_arbitrage_opportunities", {
    page,
    pageSize,
    typeFilter,
  });
  return {
    items: dto.items.map(mapArbitrageOpportunity),
    total: dto.total,
    page: dto.page,
    pageSize: dto.page_size,
    totalPages: dto.total_pages,
  };
}

export async function listAnalyzeJobs(): Promise<AnalyzeJob[]> {
  if (!isTauriRuntime()) {
    return useMarketStore.getState().jobs;
  }

  const rows = await invoke<JobRecordDto[]>("list_jobs");
  return rows.map(mapAnalyzeJob);
}

export async function cancelAnalyzeJob(id: number): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("cancel_job", { id });
}

export async function listNotificationEvents(limit = 20): Promise<NotificationEvent[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  const rows = await invoke<NotificationEventDto[]>("list_notification_events", { limit });
  return rows.map(mapNotificationEvent);
}

export async function getLatestRecommendation(): Promise<RecommendationRun[]> {
  if (!isTauriRuntime()) {
    return useRecommendationStore.getState().latest;
  }

  const dto = await invoke<RecommendationRunDto[]>("get_latest_recommendation");
  return dto.map(mapRecommendation);
}

export async function triggerRecommendation(symbol?: string): Promise<RecommendationRun[]> {
  if (!isTauriRuntime()) {
    const latest = useRecommendationStore.getState().latest;
    if (latest.length === 0) {
      throw new Error("No local recommendation available");
    }
    return latest;
  }

  const dto = await invoke<RecommendationRunDto[]>("trigger_recommendation", { symbol });
  return dto.map(mapRecommendation);
}

export async function listRecommendationHistory(): Promise<RecommendationHistoryRow[]> {
  if (!isTauriRuntime()) {
    return useRecommendationStore.getState().history;
  }

  const rows = await invoke<RecommendationHistoryRowDto[]>("list_recommendation_history");
  return rows.map(mapRecommendationHistoryRow);
}

export async function getRecommendationAudit(
  recommendationId: string,
): Promise<RecommendationAudit | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const dto = await invoke<RecommendationAuditDto | null>("get_recommendation_audit", {
    recommendationId,
  });
  return dto ? mapRecommendationAudit(dto) : null;
}

export async function deleteRecommendation(recommendationId: string): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("delete_recommendation", {
    recommendationId,
  });
}

export async function listPaperAccounts(): Promise<PaperAccountSummary[]> {
  if (!isTauriRuntime()) {
    return [{ accountId: "paper-cny", exchange: "模拟账户", availableUsdt: 1_000_000 }];
  }

  const rows = await invoke<PaperAccountDto[]>("list_paper_accounts");
  return rows.map(mapPaperAccount);
}

export async function createPaperOrderFromRecommendation(
  recommendationId: string,
  accountId: string,
): Promise<PaperOrderDraft> {
  if (!isTauriRuntime()) {
    const latest = useRecommendationStore.getState().latest[0];
    return {
      orderId: `draft-${recommendationId}`,
      accountId,
      exchange: "模拟账户",
      symbol: latest?.symbol ?? "SHSE.600000",
      side: "buy",
      quantity: 100,
      estimatedFillPrice: latest?.entryHigh ?? latest?.entryLow ?? 8.72,
      stopLoss: latest?.stopLoss,
      takeProfit: latest?.entryHigh,
    };
  }

  const dto = await invoke<PaperOrderDraftDto>("create_paper_order_from_recommendation", {
    recommendationId,
    accountId,
  });
  return mapPaperOrderDraft(dto);
}

export async function createManualPaperOrder(
  request: ManualPaperOrderRequest,
): Promise<AnalyzeJob> {
  if (!isTauriRuntime()) {
    return {
      id: Date.now(),
      kind: "paper.order",
      status: "running",
      message: "模拟委托已加入后台任务",
      startedAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      endedAt: null,
      durationMs: null,
      inputParamsJson: JSON.stringify({ symbol: request.symbol, quantity: request.quantity }),
      resultSummary: null,
      errorDetails: null,
    };
  }

  const dto = await invoke<JobRecordDto>("create_manual_paper_order", {
    request: {
      account_id: request.accountId,
      symbol: request.symbol,
      market_type: request.marketType,
      side: request.side,
      quantity: request.quantity,
      entry_price: request.entryPrice ?? null,
      leverage: request.leverage,
      stop_loss: request.stopLoss ?? null,
      take_profit: request.takeProfit ?? null,
    },
  });
  return mapAnalyzeJob(dto);
}

export async function listenToAssistantEvents(
  listener: (event: AssistantEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    const browserHandler = (event: Event) => {
      listener((event as CustomEvent<AssistantEvent>).detail);
    };
    window.addEventListener("kittyred-assistant-event", browserHandler);
    return () => window.removeEventListener("kittyred-assistant-event", browserHandler);
  }

  const { listen } = await import("@tauri-apps/api/event");
  return listen<AssistantEvent>("assistant://event", (event) => listener(event.payload));
}

export async function startAssistantRun(sessionId: string, message: string): Promise<void> {
  if (!isTauriRuntime()) {
    clearLocalAssistantTimers(sessionId);
    const context = {
      usedTokens: 1240,
      maxTokens: 16000,
      remainingTokens: 14760,
      thinkingTokens: 160,
      breakdown: {
        system: 420,
        user: 180,
        assistant: 420,
        tool: 220,
      },
    };
    emitBrowserAssistantEvent({
      sessionId,
      type: "status",
      status: "running",
      context,
    });
    scheduleLocalAssistantEvent(sessionId, 40, () => ({
      sessionId,
      type: "thinking_status",
      status: "running",
    }));
    scheduleLocalAssistantEvent(sessionId, 100, () => ({
      sessionId,
      type: "thinking_delta",
      delta: `正在读取本地 A 股模拟上下文：${message}`,
    }));
    scheduleLocalAssistantEvent(sessionId, 180, () => ({
      sessionId,
      type: "thinking_status",
      status: "finished",
    }));
    scheduleLocalAssistantEvent(sessionId, 220, () => ({
      sessionId,
      type: "tool_start",
      toolCallId: "local-market-data",
      name: "market_data",
      summary: "读取缓存行情",
      arguments: { stockCode: "SHSE.600000", limit: 1 },
    }));
    scheduleLocalAssistantEvent(sessionId, 300, () => ({
      sessionId,
      type: "tool_output",
      toolCallId: "local-market-data",
      delta: JSON.stringify({
        ok: true,
        rows: [
          {
            stockCode: "SHSE.600000",
            last: 8.72,
            change24h: 0.81,
          },
        ],
      }),
    }));
    scheduleLocalAssistantEvent(sessionId, 360, () => ({
      sessionId,
      type: "tool_end",
      toolCallId: "local-market-data",
      name: "market_data",
      status: "done",
      resultPreview: "1 条缓存行情",
      context,
    }));
    scheduleLocalAssistantEvent(sessionId, 420, () => ({
      sessionId,
      type: "token",
      delta: "本地预览：**SHSE.600000** 处于温和反弹观察区间。\n\n",
    }));
    scheduleLocalAssistantEvent(sessionId, 500, () => ({
      sessionId,
      type: "token",
      delta: "当前为浏览器预览数据，桌面运行时会读取 AKShare 和本地模拟账本。",
    }));
    scheduleLocalAssistantEvent(sessionId, 580, () => ({
      sessionId,
      type: "done",
      reply:
        "本地预览：**SHSE.600000** 处于温和反弹观察区间。\n\n当前为浏览器预览数据，桌面运行时会读取 AKShare 和本地模拟账本。",
      context,
    }));
    return;
  }

  const response = await invoke<AssistantCommandAckDto>("start_assistant_run", {
    sessionId,
    message,
  });
  if (!response.started) {
    throw new Error("Assistant run did not start.");
  }
}

export async function stopAssistantRun(sessionId: string): Promise<void> {
  if (!isTauriRuntime()) {
    clearLocalAssistantTimers(sessionId);
    emitBrowserAssistantEvent({
      sessionId,
      type: "cancelled",
      context: {
        usedTokens: 0,
        maxTokens: 16000,
        remainingTokens: 16000,
        thinkingTokens: 0,
        breakdown: {
          system: 0,
          user: 0,
          assistant: 0,
          tool: 0,
        },
      },
    });
    return;
  }

  await invoke<AssistantStopAckDto>("stop_assistant_run", { sessionId });
}

export async function clearAssistantSession(sessionId: string): Promise<void> {
  if (!isTauriRuntime()) {
    clearLocalAssistantTimers(sessionId);
    return;
  }

  await invoke<AssistantClearAckDto>("clear_assistant_session", { sessionId });
}

export async function scanSignals(): Promise<UnifiedSignal[]> {
  return invoke<UnifiedSignal[]>("scan_signals");
}

export async function listSignalHistory(
  page: number,
  pageSize: number,
): Promise<SignalHistoryPage> {
  return invoke<SignalHistoryPage>("list_signal_history", { page, pageSize });
}

export async function executeSignal(
  signalId: string,
  accountId: string,
): Promise<PaperOrderDraft> {
  if (!isTauriRuntime()) {
    return {
      orderId: `signal-${signalId}`,
      accountId,
      exchange: "模拟账户",
      symbol: "SHSE.600000",
      side: "buy",
      quantity: 0,
      estimatedFillPrice: 0,
    };
  }

  const dto = await invoke<PaperOrderDraftDto>("execute_signal", { signalId, accountId });
  return mapPaperOrderDraft(dto);
}

export async function dismissSignal(signalId: string): Promise<void> {
  return invoke<void>("dismiss_signal", { signalId });
}

export async function getStrategyMeta(): Promise<StrategyMeta[]> {
  return invoke("get_strategy_meta");
}

export async function getStrategyConfigs(): Promise<StrategyConfig[]> {
  return invoke("get_strategy_configs");
}

export async function updateStrategyConfig(
  strategyId: string,
  enabled?: boolean,
  params?: Record<string, number>,
): Promise<StrategyConfig> {
  return invoke("update_strategy_config", {
    strategyId,
    payload: { enabled, params },
  });
}

export async function getStrategyStats(): Promise<StrategyStats[]> {
  return invoke("get_strategy_stats");
}

export async function listScanRuns(
  page: number,
  pageSize: number,
): Promise<ScanRunHistoryPage> {
  return invoke("list_scan_runs", { page, pageSize });
}
