import { invoke } from "@tauri-apps/api/core";
import type {
  AnalyzeJob,
  BacktestDataset,
  BacktestFetchFailure,
  BacktestFetchProgress,
  BacktestRun,
  BacktestSignal,
  BacktestSummary,
  BacktestTrade,
  FinancialReportAnalysis,
  FinancialReportAnalysisProgress,
  FinancialReportFetchProgress,
  FinancialReportOverview,
  FinancialReportSnapshot,
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
  RecommendationGenerationProgress,
  RecommendationGenerationProgressItem,
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
  amount_cny: number | null;
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

type RecommendationGenerationProgressItemDto = {
  stock_code?: string;
  stockCode?: string;
  short_name?: string;
  shortName?: string;
  status: string;
  attempt: number;
  error_message?: string | null;
  errorMessage?: string | null;
};

type RecommendationGenerationProgressDto = {
  status: string;
  completed_count?: number;
  completedCount?: number;
  total_count?: number;
  totalCount?: number;
  message: string;
  items: RecommendationGenerationProgressItemDto[];
};

type FinancialReportMetricPointDto = {
  report_date: string;
  value: number;
  yoy?: number | null;
  qoq?: number | null;
};

type FinancialReportMetricSeriesDto = {
  metric_key: string;
  metric_label: string;
  unit: string;
  points: FinancialReportMetricPointDto[];
};

type FinancialReportCategoryScoresDto = {
  revenueQuality: number;
  grossMargin: number;
  netProfitReturn: number;
  earningsManipulation: number;
  solvency: number;
  cashFlow: number;
  growth: number;
  researchCapital: number;
  operatingEfficiency: number;
  assetQuality: number;
};

type FinancialReportRadarScoresDto = {
  profitability: number;
  authenticity: number;
  cashGeneration: number;
  safety: number;
  growthPotential: number;
  operatingEfficiency: number;
};

type FinancialReportAnalysisProgressItemDto = {
  stockCode: string;
  shortName: string;
  status: "pending" | "running" | "retrying" | "succeeded" | "failed";
  attempt: number;
  errorMessage?: string | null;
};

type FinancialReportAnalysisProgressDto = {
  status: string;
  completedCount: number;
  totalCount: number;
  message: string;
  items: FinancialReportAnalysisProgressItemDto[];
};

type BacktestDatasetDto = {
  dataset_id?: string;
  datasetId?: string;
  name: string;
  status: string;
  symbols: string[];
  start_date?: string;
  startDate?: string;
  end_date?: string;
  endDate?: string;
  interval_minutes?: number;
  intervalMinutes?: number;
  total_snapshots?: number;
  totalSnapshots?: number;
  fetched_count?: number;
  fetchedCount?: number;
  estimated_llm_calls?: number;
  estimatedLlmCalls?: number;
  error_message?: string | null;
  errorMessage?: string | null;
  created_at?: string;
  createdAt?: string;
  completed_at?: string | null;
  completedAt?: string | null;
};

type BacktestFetchFailureDto = {
  failure_id?: string;
  failureId?: string;
  dataset_id?: string;
  datasetId?: string;
  symbol: string;
  captured_at?: string | null;
  capturedAt?: string | null;
  timeframe: string;
  stage: string;
  reason: string;
  error_detail?: string | null;
  errorDetail?: string | null;
  created_at?: string;
  createdAt?: string;
};

type BacktestFetchProgressDto = {
  dataset_id?: string;
  datasetId?: string;
  status: string;
  total_snapshots?: number;
  totalSnapshots?: number;
  fetched_count?: number;
  fetchedCount?: number;
  failure_count?: number;
  failureCount?: number;
  error_message?: string | null;
  errorMessage?: string | null;
  recent_failures?: BacktestFetchFailureDto[];
  recentFailures?: BacktestFetchFailureDto[];
};

type BacktestRunDto = {
  backtest_id?: string;
  backtestId?: string;
  dataset_id?: string;
  datasetId?: string;
  name: string;
  status: string;
  model_provider?: string;
  modelProvider?: string;
  model_name?: string;
  modelName?: string;
  prompt_version?: string;
  promptVersion?: string;
  max_holding_days?: number;
  maxHoldingDays?: number;
  total_ai_calls?: number;
  totalAiCalls?: number;
  processed_ai_calls?: number;
  processedAiCalls?: number;
  total_timepoints?: number;
  totalTimepoints?: number;
  processed_timepoints?: number;
  processedTimepoints?: number;
  total_signals?: number;
  totalSignals?: number;
  trade_signals?: number;
  tradeSignals?: number;
  open_trades?: number;
  openTrades?: number;
  win_count?: number;
  winCount?: number;
  loss_count?: number;
  lossCount?: number;
  flat_count?: number;
  flatCount?: number;
  total_pnl_cny?: number;
  totalPnlCny?: number;
  total_pnl_percent?: number;
  totalPnlPercent?: number;
  max_drawdown_percent?: number;
  maxDrawdownPercent?: number;
  profit_factor?: number | null;
  profitFactor?: number | null;
  error_message?: string | null;
  errorMessage?: string | null;
  created_at?: string;
  createdAt?: string;
  completed_at?: string | null;
  completedAt?: string | null;
};

type BacktestSignalDto = {
  signal_id?: string;
  signalId?: string;
  backtest_id?: string;
  backtestId?: string;
  symbol: string;
  stock_name?: string | null;
  stockName?: string | null;
  captured_at?: string;
  capturedAt?: string;
  has_trade?: boolean;
  hasTrade?: boolean;
  direction?: string | null;
  confidence_score?: number | null;
  confidenceScore?: number | null;
  risk_status?: string | null;
  riskStatus?: string | null;
  entry_low?: number | null;
  entryLow?: number | null;
  entry_high?: number | null;
  entryHigh?: number | null;
  stop_loss?: number | null;
  stopLoss?: number | null;
  take_profit?: string | null;
  takeProfit?: string | null;
  amount_cny?: number | null;
  amountCny?: number | null;
  max_loss_cny?: number | null;
  maxLossCny?: number | null;
  rationale?: string | null;
  result: string;
};

type BacktestTradeDto = {
  trade_id?: string;
  tradeId?: string;
  backtest_id?: string;
  backtestId?: string;
  signal_id?: string | null;
  signalId?: string | null;
  symbol: string;
  stock_name?: string | null;
  stockName?: string | null;
  direction: string;
  entry_price?: number;
  entryPrice?: number;
  entry_at?: string;
  entryAt?: string;
  exit_price?: number;
  exitPrice?: number;
  exit_at?: string;
  exitAt?: string;
  exit_reason?: string;
  exitReason?: string;
  stop_loss?: number | null;
  stopLoss?: number | null;
  take_profit?: number | null;
  takeProfit?: number | null;
  amount_cny?: number | null;
  amountCny?: number | null;
  holding_periods?: number;
  holdingPeriods?: number;
  pnl_cny?: number;
  pnlCny?: number;
  pnl_percent?: number;
  pnlPercent?: number;
};

type BacktestOpenPositionDto = {
  signal_id?: string;
  signalId?: string;
  symbol: string;
  stock_name?: string | null;
  stockName?: string | null;
  entry_price?: number;
  entryPrice?: number;
  entry_at?: string;
  entryAt?: string;
  mark_price?: number;
  markPrice?: number;
  amount_cny?: number;
  amountCny?: number;
  holding_periods?: number;
  holdingPeriods?: number;
  unrealized_pnl_cny?: number;
  unrealizedPnlCny?: number;
  unrealized_pnl_percent?: number;
  unrealizedPnlPercent?: number;
};

type BacktestSummaryDto = {
  backtest_id?: string;
  backtestId?: string;
  total_signals?: number;
  totalSignals?: number;
  trade_count?: number;
  tradeCount?: number;
  win_rate?: number;
  winRate?: number;
  total_pnl_cny?: number;
  totalPnlCny?: number;
  total_pnl_percent?: number;
  totalPnlPercent?: number;
  max_drawdown_percent?: number;
  maxDrawdownPercent?: number;
  profit_factor?: number | null;
  profitFactor?: number | null;
  equity_curve?: Array<{ captured_at?: string; capturedAt?: string; cumulative_pnl_percent?: number; cumulativePnlPercent?: number }>;
  equityCurve?: Array<{ captured_at?: string; capturedAt?: string; cumulative_pnl_percent?: number; cumulativePnlPercent?: number }>;
  open_positions?: BacktestOpenPositionDto[];
  openPositions?: BacktestOpenPositionDto[];
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

type CandleCacheUpdatedEvent = {
  symbol: string;
  interval: string;
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
  position_id: string;
  account_id: string;
  exchange: string;
  symbol: string;
  side: string;
  quantity: number;
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
    amountCny: dto.amount_cny ?? undefined,
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

function mapRecommendationGenerationProgressItem(
  dto: RecommendationGenerationProgressItemDto,
): RecommendationGenerationProgressItem {
  return {
    stockCode: dto.stock_code ?? dto.stockCode ?? "",
    shortName: dto.short_name ?? dto.shortName ?? "",
    status: dto.status,
    attempt: dto.attempt,
    errorMessage: dto.error_message ?? dto.errorMessage ?? undefined,
  };
}

function mapRecommendationGenerationProgress(
  dto: RecommendationGenerationProgressDto,
): RecommendationGenerationProgress {
  return {
    status: dto.status,
    completedCount: dto.completed_count ?? dto.completedCount ?? 0,
    totalCount: dto.total_count ?? dto.totalCount ?? 0,
    message: dto.message,
    items: (dto.items ?? []).map(mapRecommendationGenerationProgressItem),
  };
}

function mapBacktestDataset(dto: BacktestDatasetDto): BacktestDataset {
  return {
    datasetId: dto.dataset_id ?? dto.datasetId ?? "",
    name: dto.name,
    status: dto.status,
    symbols: dto.symbols,
    startDate: dto.start_date ?? dto.startDate ?? "",
    endDate: dto.end_date ?? dto.endDate ?? "",
    intervalMinutes: dto.interval_minutes ?? dto.intervalMinutes ?? 30,
    totalSnapshots: dto.total_snapshots ?? dto.totalSnapshots ?? 0,
    fetchedCount: dto.fetched_count ?? dto.fetchedCount ?? 0,
    estimatedLlmCalls: dto.estimated_llm_calls ?? dto.estimatedLlmCalls ?? 0,
    errorMessage: dto.error_message ?? dto.errorMessage ?? undefined,
    createdAt: dto.created_at ?? dto.createdAt ?? "",
    completedAt: dto.completed_at ?? dto.completedAt ?? undefined,
  };
}

function mapBacktestFetchFailure(dto: BacktestFetchFailureDto): BacktestFetchFailure {
  return {
    failureId: dto.failure_id ?? dto.failureId ?? "",
    datasetId: dto.dataset_id ?? dto.datasetId ?? "",
    symbol: dto.symbol,
    capturedAt: dto.captured_at ?? dto.capturedAt ?? undefined,
    timeframe: dto.timeframe,
    stage: dto.stage,
    reason: dto.reason,
    errorDetail: dto.error_detail ?? dto.errorDetail ?? undefined,
    createdAt: dto.created_at ?? dto.createdAt ?? "",
  };
}

function mapBacktestFetchProgress(dto: BacktestFetchProgressDto): BacktestFetchProgress {
  const failures = dto.recent_failures ?? dto.recentFailures ?? [];
  return {
    datasetId: dto.dataset_id ?? dto.datasetId ?? "",
    status: dto.status,
    totalSnapshots: dto.total_snapshots ?? dto.totalSnapshots ?? 0,
    fetchedCount: dto.fetched_count ?? dto.fetchedCount ?? 0,
    failureCount: dto.failure_count ?? dto.failureCount ?? 0,
    errorMessage: dto.error_message ?? dto.errorMessage ?? undefined,
    recentFailures: failures.map(mapBacktestFetchFailure),
  };
}

function mapBacktestRun(dto: BacktestRunDto): BacktestRun {
  return {
    backtestId: dto.backtest_id ?? dto.backtestId ?? "",
    datasetId: dto.dataset_id ?? dto.datasetId ?? "",
    name: dto.name,
    status: dto.status,
    modelProvider: dto.model_provider ?? dto.modelProvider ?? "",
    modelName: dto.model_name ?? dto.modelName ?? "",
    promptVersion: dto.prompt_version ?? dto.promptVersion ?? "",
    maxHoldingDays: dto.max_holding_days ?? dto.maxHoldingDays ?? 7,
    totalAiCalls: dto.total_ai_calls ?? dto.totalAiCalls ?? 0,
    processedAiCalls: dto.processed_ai_calls ?? dto.processedAiCalls ?? 0,
    totalTimepoints: dto.total_timepoints ?? dto.totalTimepoints ?? 0,
    processedTimepoints: dto.processed_timepoints ?? dto.processedTimepoints ?? 0,
    totalSignals: dto.total_signals ?? dto.totalSignals ?? 0,
    tradeSignals: dto.trade_signals ?? dto.tradeSignals ?? 0,
    openTrades: dto.open_trades ?? dto.openTrades ?? 0,
    winCount: dto.win_count ?? dto.winCount ?? 0,
    lossCount: dto.loss_count ?? dto.lossCount ?? 0,
    flatCount: dto.flat_count ?? dto.flatCount ?? 0,
    totalPnlCny: dto.total_pnl_cny ?? dto.totalPnlCny ?? 0,
    totalPnlPercent: dto.total_pnl_percent ?? dto.totalPnlPercent ?? 0,
    maxDrawdownPercent: dto.max_drawdown_percent ?? dto.maxDrawdownPercent ?? 0,
    profitFactor: dto.profit_factor ?? dto.profitFactor ?? undefined,
    errorMessage: dto.error_message ?? dto.errorMessage ?? undefined,
    createdAt: dto.created_at ?? dto.createdAt ?? "",
    completedAt: dto.completed_at ?? dto.completedAt ?? undefined,
  };
}

function mapBacktestSignal(dto: BacktestSignalDto): BacktestSignal {
  return {
    signalId: dto.signal_id ?? dto.signalId ?? "",
    backtestId: dto.backtest_id ?? dto.backtestId ?? "",
    symbol: dto.symbol,
    stockName: dto.stock_name ?? dto.stockName ?? undefined,
    capturedAt: dto.captured_at ?? dto.capturedAt ?? "",
    hasTrade: dto.has_trade ?? dto.hasTrade ?? false,
    direction: dto.direction ?? undefined,
    confidenceScore: dto.confidence_score ?? dto.confidenceScore ?? undefined,
    riskStatus: dto.risk_status ?? dto.riskStatus ?? undefined,
    entryLow: dto.entry_low ?? dto.entryLow ?? undefined,
    entryHigh: dto.entry_high ?? dto.entryHigh ?? undefined,
    stopLoss: dto.stop_loss ?? dto.stopLoss ?? undefined,
    takeProfit: dto.take_profit ?? dto.takeProfit ?? undefined,
    amountCny: dto.amount_cny ?? dto.amountCny ?? undefined,
    maxLossCny: dto.max_loss_cny ?? dto.maxLossCny ?? undefined,
    rationale: dto.rationale ?? undefined,
    result: dto.result,
  };
}

function mapBacktestTrade(dto: BacktestTradeDto): BacktestTrade {
  return {
    tradeId: dto.trade_id ?? dto.tradeId ?? "",
    backtestId: dto.backtest_id ?? dto.backtestId ?? "",
    signalId: dto.signal_id ?? dto.signalId ?? undefined,
    symbol: dto.symbol,
    stockName: dto.stock_name ?? dto.stockName ?? undefined,
    direction: dto.direction,
    entryPrice: dto.entry_price ?? dto.entryPrice ?? 0,
    entryAt: dto.entry_at ?? dto.entryAt ?? "",
    exitPrice: dto.exit_price ?? dto.exitPrice ?? 0,
    exitAt: dto.exit_at ?? dto.exitAt ?? "",
    exitReason: dto.exit_reason ?? dto.exitReason ?? "",
    stopLoss: dto.stop_loss ?? dto.stopLoss ?? undefined,
    takeProfit: dto.take_profit ?? dto.takeProfit ?? undefined,
    amountCny: dto.amount_cny ?? dto.amountCny ?? undefined,
    holdingPeriods: dto.holding_periods ?? dto.holdingPeriods ?? 0,
    pnlCny: dto.pnl_cny ?? dto.pnlCny ?? 0,
    pnlPercent: dto.pnl_percent ?? dto.pnlPercent ?? 0,
  };
}

function mapBacktestOpenPosition(dto: BacktestOpenPositionDto) {
  return {
    signalId: dto.signal_id ?? dto.signalId ?? "",
    symbol: dto.symbol,
    stockName: dto.stock_name ?? dto.stockName ?? undefined,
    entryPrice: dto.entry_price ?? dto.entryPrice ?? 0,
    entryAt: dto.entry_at ?? dto.entryAt ?? "",
    markPrice: dto.mark_price ?? dto.markPrice ?? 0,
    amountCny: dto.amount_cny ?? dto.amountCny ?? 0,
    holdingPeriods: dto.holding_periods ?? dto.holdingPeriods ?? 0,
    unrealizedPnlCny: dto.unrealized_pnl_cny ?? dto.unrealizedPnlCny ?? 0,
    unrealizedPnlPercent:
      dto.unrealized_pnl_percent ?? dto.unrealizedPnlPercent ?? 0,
  };
}

function mapBacktestSummary(dto: BacktestSummaryDto): BacktestSummary {
  const curve = dto.equity_curve ?? dto.equityCurve ?? [];
  const openPositions = dto.open_positions ?? dto.openPositions ?? [];
  return {
    backtestId: dto.backtest_id ?? dto.backtestId ?? "",
    totalSignals: dto.total_signals ?? dto.totalSignals ?? 0,
    tradeCount: dto.trade_count ?? dto.tradeCount ?? 0,
    winRate: dto.win_rate ?? dto.winRate ?? 0,
    totalPnlCny: dto.total_pnl_cny ?? dto.totalPnlCny ?? 0,
    totalPnlPercent: dto.total_pnl_percent ?? dto.totalPnlPercent ?? 0,
    maxDrawdownPercent: dto.max_drawdown_percent ?? dto.maxDrawdownPercent ?? 0,
    profitFactor: dto.profit_factor ?? dto.profitFactor ?? undefined,
    equityCurve: curve.map((point) => ({
      capturedAt: point.captured_at ?? point.capturedAt ?? "",
      cumulativePnlPercent: point.cumulative_pnl_percent ?? point.cumulativePnlPercent ?? 0,
    })),
    openPositions: openPositions.map(mapBacktestOpenPosition),
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
    positionId: dto.position_id,
    accountId: dto.account_id,
    exchange: dto.exchange,
    symbol: dto.symbol,
    side:
      dto.side.toLowerCase() === "short"
        ? "Short"
        : dto.side.toLowerCase() === "spot"
          ? "Holding"
          : "Long",
    quantity: dto.quantity,
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

export async function startRecommendationGeneration(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("start_recommendation_generation");
}

export async function getRecommendationGenerationProgress(): Promise<RecommendationGenerationProgress> {
  if (!isTauriRuntime()) {
    return {
      status: "idle",
      completedCount: 0,
      totalCount: 0,
      message: "尚未开始 AI 建议生成",
      items: [],
    };
  }

  const dto = await invoke<RecommendationGenerationProgressDto>("get_recommendation_generation_progress");
  return mapRecommendationGenerationProgress(dto);
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

export async function createBacktestDataset(request: {
  name: string;
  symbols: string[];
  startDate: string;
  endDate: string;
  intervalMinutes: number;
}): Promise<BacktestDataset> {
  if (!isTauriRuntime()) {
    return mapBacktestDataset({
      datasetId: `dataset-${Date.now()}`,
      name: request.name,
      status: "pending",
      symbols: request.symbols.length > 0 ? request.symbols : ["SHSE.600000"],
      startDate: request.startDate,
      endDate: request.endDate,
      intervalMinutes: request.intervalMinutes,
      totalSnapshots: 0,
      fetchedCount: 0,
      estimatedLlmCalls: 48,
      createdAt: new Date().toISOString(),
    });
  }

  const dto = await invoke<BacktestDatasetDto>("create_backtest_dataset", { params: request });
  return mapBacktestDataset(dto);
}

export async function startFetchSnapshots(datasetId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("start_fetch_snapshots", { datasetId });
}

export async function cancelFetchSnapshots(datasetId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("cancel_fetch_snapshots", { datasetId });
}

export async function listBacktestDatasets(): Promise<BacktestDataset[]> {
  if (!isTauriRuntime()) {
    return [
      mapBacktestDataset({
        datasetId: "dataset-preview",
        name: "本地预览数据",
        status: "ready",
        symbols: ["SHSE.600000"],
        startDate: "2026-04-01",
        endDate: "2026-04-03",
        intervalMinutes: 30,
        totalSnapshots: 48,
        fetchedCount: 48,
        estimatedLlmCalls: 48,
        createdAt: "2026-05-07T09:00:00+08:00",
        completedAt: "2026-05-07T09:05:00+08:00",
      }),
    ];
  }

  const rows = await invoke<BacktestDatasetDto[]>("list_backtest_datasets");
  return rows.map(mapBacktestDataset);
}

export async function getBacktestFetchProgress(datasetId: string): Promise<BacktestFetchProgress> {
  if (!isTauriRuntime()) {
    return {
      datasetId,
      status: "ready",
      totalSnapshots: 48,
      fetchedCount: 46,
      failureCount: 2,
      errorMessage: "拉取完成，46 个快照可用，2 个股票-时间点失败，可在失败记录中查看。",
      recentFailures: [
        {
          failureId: "failure-preview-1",
          datasetId,
          symbol: "SHSE.600000",
          capturedAt: "2026-04-01T10:00:00+08:00",
          timeframe: "1h",
          stage: "history_bars",
          reason: "SHSE.600000 1h K 线拉取失败",
          errorDetail: "浏览器预览失败样例",
          createdAt: "2026-05-07T09:04:00+08:00",
        },
      ],
    };
  }

  const dto = await invoke<BacktestFetchProgressDto>("get_backtest_fetch_progress", { datasetId });
  return mapBacktestFetchProgress(dto);
}

export async function listBacktestFetchFailures(datasetId: string): Promise<BacktestFetchFailure[]> {
  if (!isTauriRuntime()) {
    return (await getBacktestFetchProgress(datasetId)).recentFailures;
  }

  const rows = await invoke<BacktestFetchFailureDto[]>("list_backtest_fetch_failures", { datasetId });
  return rows.map(mapBacktestFetchFailure);
}

export async function deleteBacktestDataset(datasetId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("delete_backtest_dataset", { datasetId });
}

export async function createBacktest(request: {
  datasetId: string;
  name: string;
  maxHoldingDays?: number;
}): Promise<BacktestRun> {
  if (!isTauriRuntime()) {
    return mapBacktestRun({
      backtestId: `bt-${Date.now()}`,
      datasetId: request.datasetId,
      name: request.name,
      status: "pending",
      modelProvider: "OpenAI-compatible",
      modelName: "gpt-5.5",
      promptVersion: "recommendation-system-v2",
      maxHoldingDays: request.maxHoldingDays ?? 7,
      totalAiCalls: 0,
      processedAiCalls: 0,
      totalTimepoints: 0,
      processedTimepoints: 0,
      totalSignals: 0,
      tradeSignals: 0,
      openTrades: 0,
      winCount: 0,
      lossCount: 0,
      flatCount: 0,
      totalPnlCny: 0,
      totalPnlPercent: 0,
      maxDrawdownPercent: 0,
      createdAt: new Date().toISOString(),
    });
  }

  const dto = await invoke<BacktestRunDto>("create_backtest", { params: request });
  return mapBacktestRun(dto);
}

export async function startBacktest(backtestId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("start_backtest", { backtestId });
}

export async function startGenerateBacktestSignals(backtestId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("start_generate_backtest_signals", { backtestId });
}

export async function startReplayBacktest(backtestId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("start_replay_backtest", { backtestId });
}

export async function cancelBacktest(backtestId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("cancel_backtest", { backtestId });
}

export async function listBacktestRuns(): Promise<BacktestRun[]> {
  if (!isTauriRuntime()) {
    return [
      mapBacktestRun({
        backtestId: "bt-preview",
        datasetId: "dataset-preview",
        name: "本地预览回测",
        status: "completed",
        modelProvider: "OpenAI-compatible",
        modelName: "gpt-5.5",
        promptVersion: "recommendation-system-v2",
        maxHoldingDays: 7,
        totalAiCalls: 48,
        processedAiCalls: 48,
        totalTimepoints: 24,
        processedTimepoints: 24,
        totalSignals: 48,
        tradeSignals: 8,
        openTrades: 0,
        winCount: 5,
        lossCount: 3,
        flatCount: 0,
        totalPnlCny: 1260,
        totalPnlPercent: 6.3,
        maxDrawdownPercent: 1.8,
        profitFactor: 1.7,
        createdAt: "2026-05-07T09:10:00+08:00",
        completedAt: "2026-05-07T09:20:00+08:00",
      }),
    ];
  }

  const rows = await invoke<BacktestRunDto[]>("list_backtest_runs");
  return rows.map(mapBacktestRun);
}

export async function listBacktestSignals(backtestId: string): Promise<BacktestSignal[]> {
  if (!isTauriRuntime()) {
    return [
      mapBacktestSignal({
        signalId: "sig-preview",
        backtestId,
        symbol: "SHSE.600000",
        stockName: "浦发银行",
        capturedAt: "2026-04-01T10:00:00+08:00",
        hasTrade: true,
        direction: "买入",
        confidenceScore: 72,
        riskStatus: "approved",
        rationale: "历史 K 线回踩后放量。",
        result: "opened",
      }),
    ];
  }

  const rows = await invoke<BacktestSignalDto[]>("list_backtest_signals", { backtestId });
  return rows.map(mapBacktestSignal);
}

export async function listBacktestTrades(backtestId: string): Promise<BacktestTrade[]> {
  if (!isTauriRuntime()) {
    return [
      mapBacktestTrade({
        tradeId: "trade-preview",
        backtestId,
        signalId: "sig-preview",
        symbol: "SHSE.600000",
        stockName: "浦发银行",
        direction: "long",
        entryPrice: 8.7,
        entryAt: "2026-04-01T10:00:00+08:00",
        exitPrice: 8.95,
        exitAt: "2026-04-02T10:00:00+08:00",
        exitReason: "take_profit",
        amountCny: 20000,
        holdingPeriods: 8,
        pnlCny: 534.7,
        pnlPercent: 2.67,
      }),
    ];
  }

  const rows = await invoke<BacktestTradeDto[]>("list_backtest_trades", { backtestId });
  return rows.map(mapBacktestTrade);
}

export async function getBacktestSummary(backtestId: string): Promise<BacktestSummary> {
  if (!isTauriRuntime()) {
    return mapBacktestSummary({
      backtestId,
      totalSignals: 48,
      tradeCount: 8,
      winRate: 62.5,
      totalPnlCny: 1260,
      totalPnlPercent: 6.3,
      maxDrawdownPercent: 1.8,
      profitFactor: 1.7,
      equityCurve: [
        { capturedAt: "2026-04-01T10:00:00+08:00", cumulativePnlPercent: 0 },
        { capturedAt: "2026-04-02T10:00:00+08:00", cumulativePnlPercent: 2.67 },
      ],
      openPositions: [],
    });
  }

  const dto = await invoke<BacktestSummaryDto>("get_backtest_summary", { backtestId });
  return mapBacktestSummary(dto);
}

export async function deleteBacktest(backtestId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("delete_backtest", { backtestId });
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

export async function closePaperPosition(positionId: string): Promise<PaperOrderDraft> {
  if (!isTauriRuntime()) {
    return {
      orderId: `close-${positionId}`,
      accountId: "paper-cny",
      exchange: "模拟账户",
      symbol: "SHSE.600000",
      side: "sell",
      quantity: 100,
      estimatedFillPrice: 8.72,
    };
  }

  const dto = await invoke<PaperOrderDraftDto>("close_paper_position", {
    positionId,
  });
  return mapPaperOrderDraft(dto);
}

export async function resetPaperAccount(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("reset_paper_account");
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

export async function listenToMarketEvents(
  listener: (event: CandleCacheUpdatedEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    const browserHandler = (event: Event) => {
      listener((event as CustomEvent<CandleCacheUpdatedEvent>).detail);
    };
    window.addEventListener("kittyred-market-event", browserHandler);
    return () => window.removeEventListener("kittyred-market-event", browserHandler);
  }

  const { listen } = await import("@tauri-apps/api/event");
  return listen<CandleCacheUpdatedEvent>("market://candle-cache-updated", (event) => listener(event.payload));
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

export async function startFinancialReportFetch(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>("start_financial_report_fetch");
}

export async function cancelFinancialReportFetch(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>("cancel_financial_report_fetch");
}

export async function getFinancialReportFetchProgress(): Promise<FinancialReportFetchProgress> {
  if (!isTauriRuntime()) {
    return {
      stockCode: "ALL",
      status: "idle",
      completedSections: 0,
      totalSections: 6,
      message: "本地预览未启动财报拉取",
      errorMessage: null,
    };
  }
  return invoke<FinancialReportFetchProgress>("get_financial_report_fetch_progress");
}

export async function getFinancialReportOverview(): Promise<FinancialReportOverview> {
  if (!isTauriRuntime()) {
    return {
      stockCount: 0,
      rowCount: 0,
      refreshedAt: null,
      sections: [],
      analyses: [],
    };
  }
  const dto = await invoke<{
    stockCount: number;
    rowCount: number;
    refreshedAt?: string | null;
    sections: Array<{ section: string; label: string; source: string; rowCount: number }>;
    analyses: Array<{
      stockCode: string;
      stockName?: string | null;
      financialScore: number;
      categoryScores: FinancialReportCategoryScoresDto;
      radarScores: FinancialReportRadarScoresDto;
      sourceRevision: string;
      keySummary: string;
      positiveFactors: string;
      negativeFactors: string;
      fraudRiskPoints: string;
      modelProvider?: string | null;
      modelName?: string | null;
      generatedAt: string;
      stale: boolean;
    }>;
  }>("get_financial_report_overview");
  return {
    stockCount: dto.stockCount,
    rowCount: dto.rowCount,
    refreshedAt: dto.refreshedAt ?? null,
    sections: dto.sections.map((item) => ({
      section: item.section,
      label: item.label,
      source: item.source,
      rowCount: item.rowCount,
    })),
    analyses: dto.analyses.map((item) => ({
      stockCode: item.stockCode,
      stockName: item.stockName ?? null,
      financialScore: item.financialScore,
      categoryScores: {
        revenueQuality: item.categoryScores.revenueQuality,
        grossMargin: item.categoryScores.grossMargin,
        netProfitReturn: item.categoryScores.netProfitReturn,
        earningsManipulation: item.categoryScores.earningsManipulation,
        solvency: item.categoryScores.solvency,
        cashFlow: item.categoryScores.cashFlow,
        growth: item.categoryScores.growth,
        researchCapital: item.categoryScores.researchCapital,
        operatingEfficiency: item.categoryScores.operatingEfficiency,
        assetQuality: item.categoryScores.assetQuality,
      },
      radarScores: {
        profitability: item.radarScores.profitability,
        authenticity: item.radarScores.authenticity,
        cashGeneration: item.radarScores.cashGeneration,
        safety: item.radarScores.safety,
        growthPotential: item.radarScores.growthPotential,
        operatingEfficiency: item.radarScores.operatingEfficiency,
      },
      sourceRevision: item.sourceRevision,
      keySummary: item.keySummary,
      positiveFactors: item.positiveFactors,
      negativeFactors: item.negativeFactors,
      fraudRiskPoints: item.fraudRiskPoints,
      modelProvider: item.modelProvider ?? null,
      modelName: item.modelName ?? null,
      generatedAt: item.generatedAt,
      stale: item.stale,
    })),
  };
}

export async function getFinancialReportSnapshot(
  stockCode: string,
): Promise<FinancialReportSnapshot> {
  if (!isTauriRuntime()) {
    return {
      stockCode,
      stockName: null,
      sections: [],
      sourceRevision: "",
      refreshedAt: null,
      metricSeries: [],
      analysis: null,
    };
  }
  const dto = await invoke<{
    stockCode: string;
    stockName?: string | null;
    sections: Array<{
      section: string;
      label: string;
      source: string;
      rows: Array<{
        stockCode: string;
        reportDate?: string | null;
        stockName?: string | null;
        raw: Record<string, unknown>;
      }>;
      error?: string | null;
    }>;
    sourceRevision: string;
    refreshedAt?: string | null;
    metricSeries: FinancialReportMetricSeriesDto[];
    analysis?: {
      stockCode: string;
      stockName?: string | null;
      financialScore: number;
      categoryScores: FinancialReportCategoryScoresDto;
      radarScores: FinancialReportRadarScoresDto;
      sourceRevision: string;
      keySummary: string;
      positiveFactors: string;
      negativeFactors: string;
      fraudRiskPoints: string;
      modelProvider?: string | null;
      modelName?: string | null;
      generatedAt: string;
      stale: boolean;
    } | null;
  }>("get_financial_report_snapshot", { stockCode });
  return {
    stockCode: dto.stockCode,
    stockName: dto.stockName ?? null,
    sections: dto.sections.map((section) => ({
      section: section.section,
      label: section.label,
      source: section.source,
      rows: section.rows.map((row) => ({
        stockCode: row.stockCode,
        reportDate: row.reportDate ?? null,
        stockName: row.stockName ?? null,
        raw: row.raw,
      })),
      error: section.error ?? null,
    })),
    sourceRevision: dto.sourceRevision,
    refreshedAt: dto.refreshedAt ?? null,
    metricSeries: dto.metricSeries.map((series) => ({
      metricKey: series.metric_key,
      metricLabel: series.metric_label,
      unit: series.unit,
      points: series.points.map((point) => ({
        reportDate: point.report_date,
        value: point.value,
        yoy: point.yoy ?? null,
        qoq: point.qoq ?? null,
      })),
    })),
    analysis: dto.analysis
      ? {
          stockCode: dto.analysis.stockCode,
          stockName: dto.analysis.stockName ?? null,
          financialScore: dto.analysis.financialScore,
          categoryScores: {
            revenueQuality: dto.analysis.categoryScores.revenueQuality,
            grossMargin: dto.analysis.categoryScores.grossMargin,
            netProfitReturn: dto.analysis.categoryScores.netProfitReturn,
            earningsManipulation: dto.analysis.categoryScores.earningsManipulation,
            solvency: dto.analysis.categoryScores.solvency,
            cashFlow: dto.analysis.categoryScores.cashFlow,
            growth: dto.analysis.categoryScores.growth,
            researchCapital: dto.analysis.categoryScores.researchCapital,
            operatingEfficiency: dto.analysis.categoryScores.operatingEfficiency,
            assetQuality: dto.analysis.categoryScores.assetQuality,
          },
          radarScores: {
            profitability: dto.analysis.radarScores.profitability,
            authenticity: dto.analysis.radarScores.authenticity,
            cashGeneration: dto.analysis.radarScores.cashGeneration,
            safety: dto.analysis.radarScores.safety,
            growthPotential: dto.analysis.radarScores.growthPotential,
            operatingEfficiency: dto.analysis.radarScores.operatingEfficiency,
          },
          sourceRevision: dto.analysis.sourceRevision,
          keySummary: dto.analysis.keySummary,
          positiveFactors: dto.analysis.positiveFactors,
          negativeFactors: dto.analysis.negativeFactors,
          fraudRiskPoints: dto.analysis.fraudRiskPoints,
          modelProvider: dto.analysis.modelProvider ?? null,
          modelName: dto.analysis.modelName ?? null,
          generatedAt: dto.analysis.generatedAt,
          stale: dto.analysis.stale,
        }
      : null,
  };
}

export async function getFinancialReportAnalysis(
  stockCode: string,
): Promise<FinancialReportAnalysis | null> {
  if (!isTauriRuntime()) return null;
  return invoke<FinancialReportAnalysis | null>("get_financial_report_analysis", { stockCode });
}

export async function getFinancialReportAnalysisProgress(): Promise<FinancialReportAnalysisProgress> {
  if (!isTauriRuntime()) {
    return {
      status: "idle",
      completedCount: 0,
      totalCount: 0,
      message: "尚未开始财报 AI 分析",
      items: [],
    };
  }
  const dto = await invoke<FinancialReportAnalysisProgressDto>("get_financial_report_analysis_progress");
  return {
    status: dto.status,
    completedCount: dto.completedCount,
    totalCount: dto.totalCount,
    message: dto.message,
    items: dto.items.map((item) => ({
      stockCode: item.stockCode,
      shortName: item.shortName,
      status: item.status,
      attempt: item.attempt,
      errorMessage: item.errorMessage ?? null,
    })),
  };
}

export async function startFinancialReportAnalysis(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke<void>("start_financial_report_analysis");
}
