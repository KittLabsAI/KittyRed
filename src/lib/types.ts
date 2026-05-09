export type AccountMode = "paper" | "real_read_only" | "dual";

export type JobStatus = "queued" | "running" | "done" | "blocked" | "failed" | "cancelling" | "cancelled";

export type SettingsTabId =
  | "exchanges"
  | "models"
  | "ai_recommendation"
  | "risk"
  | "trading_preferences"
  | "prompt"
  | "account_mode"
  | "notifications"
  | "signals"
  | "security";

export interface AnalyzeJob {
  id: number;
  kind: string;
  status: JobStatus;
  message: string;
  startedAt?: string | null;
  updatedAt: string;
  endedAt?: string | null;
  durationMs?: number | null;
  inputParamsJson?: string | null;
  resultSummary?: string | null;
  errorDetails?: string | null;
}

export interface NotificationEvent {
  id: string;
  channel: string;
  title: string;
  body: string;
  status: string;
  createdAt: string;
}

export interface MarketRow {
  symbol: string;
  baseAsset: string;
  marketType: string;
  marketCapUsd?: number;
  marketCapRank?: number;
  marketSizeTier: "large" | "mid" | "small";
  last: number;
  change24h: number;
  volume24h: number;
  funding?: number;
  spreadBps: number;
  venues: string[];
  updatedAt: string;
  stale?: boolean;
  venueSnapshots?: PairVenueSnapshot[];
  bestBidExchange?: string;
  bestAskExchange?: string;
  bestBidPrice?: number;
  bestAskPrice?: number;
  respondedExchangeCount?: number;
  fdvUsd?: number;
}

export interface AShareSymbolSearchResult {
  symbol: string;
  name: string;
  market: string;
}

export interface SpreadOpportunity {
  symbol: string;
  buyExchange: string;
  sellExchange: string;
  netSpreadPct: number;
  funding: string;
  liquidity: string;
}

export type ArbitrageTypeFilter = "all" | "spot" | "perpetual" | "cross_market";

export interface ArbitrageOpportunity {
  symbol: string;
  opportunityType: string;
  primaryMarketType: "spot" | "perpetual";
  secondaryMarketType?: "spot" | "perpetual";
  buyExchange: string;
  buyMarketType: "spot" | "perpetual";
  buyPrice: number;
  sellExchange: string;
  sellMarketType: "spot" | "perpetual";
  sellPrice: number;
  feeAdjustedNetSpreadPct: number;
  simulatedCarryPct: number;
  simulatedTotalYieldPct: number;
  liquidity24h: number;
  marketCapUsd?: number;
  fundingRate?: number;
  borrowRateDaily?: number;
  recommendationScore: number;
  updatedAt: string;
  stale: boolean;
}

export interface ArbitrageOpportunityPage {
  items: ArbitrageOpportunity[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface PairVenueSnapshot {
  exchange: string;
  last: number;
  bid: number;
  ask: number;
  changePct?: number;
  volume24h: number;
  funding?: number;
  mark?: number;
  index?: number;
  openInterest?: string;
  nextFundingAt?: string;
  updatedAt: string;
}

export interface PriceLevel {
  price: number;
  size: number;
}

export interface OrderBookSourceSnapshot {
  exchange: string;
  bids: PriceLevel[];
  asks: PriceLevel[];
  updatedAt: string;
}

export interface RecentTradeRow {
  exchange: string;
  side: string;
  price: number;
  size: number;
  timestamp: string;
}

export interface CoinInfo {
  name: string;
  symbol: string;
  summary: string;
  website?: string;
  whitepaper?: string;
  explorer?: string;
  ecosystem: string;
  marketCap?: string;
  fdv?: string;
  circulatingSupply?: string;
  totalSupply?: string;
  maxSupply?: string;
  volume24h?: string;
  listedExchanges: string[];
  riskTags: string[];
  github?: string;
}

export interface PairDetailSnapshot {
  symbol: string;
  marketType: string;
  thesis: string;
  sourceNote?: string;
  coinInfo: CoinInfo;
  venues: PairVenueSnapshot[];
  orderbooks: OrderBookSourceSnapshot[];
  recentTrades: RecentTradeRow[];
  spreads: SpreadOpportunity[];
}

export interface CandleBar {
  openTime: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  turnover?: number;
}

export interface CandleSeries {
  exchange: string;
  symbol: string;
  marketType: string;
  interval: string;
  updatedAt: string;
  bars: CandleBar[];
}

export interface PortfolioOverview {
  totalEquity: number;
  totalMarketValue: number;
  totalPnl: number;
  todayPnl: number;
  todayPnlPct: number;
  riskSummary: string;
  exchanges: Array<{
    name: string;
    equity: number;
    weight: number;
  }>;
}

export interface PositionRow {
  positionId: string;
  accountId: string;
  exchange: string;
  symbol: string;
  side: "Long" | "Short" | "Holding";
  quantity: number;
  size: string;
  entry: number;
  mark: number;
  pnlPct: number;
  leverage: string;
}

export interface OrderRow {
  id: string;
  exchange: string;
  symbol: string;
  type: string;
  status: string;
  quantity: string;
  fillPrice?: number;
  realizedPnl?: number;
  updatedAt?: string;
}

export interface RiskCheck {
  name: string;
  status: string;
  detail?: string;
}

export interface RiskDecision {
  status: string;
  riskScore: number;
  maxLossEstimate?: string;
  checks: RiskCheck[];
  modifications: string[];
  blockReasons: string[];
}

export interface RecommendationRun {
  id: string;
  status: string;
  triggerType?: string;
  hasTrade: boolean;
  symbol: string;
  stockName?: string;
  marketType: string;
  direction: string;
  venues?: string[];
  confidence: number;
  riskStatus: string;
  thesis: string;
  symbolRecommendations?: SymbolRecommendation[];
  entryLow?: number;
  entryHigh?: number;
  stopLoss?: number;
  takeProfit?: string;
  leverage?: number;
  amountCny?: number;
  invalidation?: string;
  maxLossCny?: number;
  riskDetails: RiskDecision;
  dataSnapshotAt?: string;
  modelProvider?: string;
  modelName?: string;
  promptVersion?: string;
  userPreferenceVersion?: string;
  generatedAt: string;
}

export interface SymbolRecommendation {
  symbol: string;
  stockName?: string;
  direction: string;
  thesis: string;
  riskStatus: string;
  hasTrade: boolean;
}

export interface RecommendationHistoryRow {
  id: string;
  createdAt: string;
  triggerType?: string;
  symbol: string;
  stockName?: string;
  shortlist?: string[];
  exchange: string;
  marketType: string;
  direction: string;
  rationale?: string;
  risk: string;
  result: string;
  entryLow?: number;
  entryHigh?: number;
  stopLoss?: number;
  takeProfit?: string;
  leverage?: number;
  confidence?: number;
  modelName?: string;
  promptVersion?: string;
  executed?: boolean;
  modified?: boolean;
  pnl5m: number;
  pnl10m: number;
  pnl30m: number;
  pnl60m: number;
  pnl24h: number;
  pnl7d: number;
  outcome: string;
}

export interface RecommendationAudit {
  recommendationId: string;
  triggerType: string;
  symbol: string;
  exchange: string;
  marketType: string;
  createdAt: string;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  userPreferenceVersion: string;
  aiRawOutput: string;
  aiStructuredOutput: string;
  riskResult: string;
  marketSnapshot: string;
  accountSnapshot: string;
}

export interface BacktestDataset {
  datasetId: string;
  name: string;
  status: string;
  symbols: string[];
  startDate: string;
  endDate: string;
  intervalMinutes: number;
  totalSnapshots: number;
  fetchedCount: number;
  estimatedLlmCalls: number;
  errorMessage?: string;
  createdAt: string;
  completedAt?: string;
}

export interface BacktestFetchFailure {
  failureId: string;
  datasetId: string;
  symbol: string;
  capturedAt?: string;
  timeframe: string;
  stage: string;
  reason: string;
  errorDetail?: string;
  createdAt: string;
}

export interface BacktestFetchProgress {
  datasetId: string;
  status: string;
  totalSnapshots: number;
  fetchedCount: number;
  failureCount: number;
  errorMessage?: string;
  recentFailures: BacktestFetchFailure[];
}

export interface FinancialReportRow {
  stockCode: string;
  reportDate?: string | null;
  stockName?: string | null;
  raw: Record<string, unknown>;
}

export interface FinancialReportSection {
  section: string;
  label: string;
  source: string;
  rows: FinancialReportRow[];
  error?: string | null;
}

export interface FinancialReportAnalysis {
  stockCode: string;
  sourceRevision: string;
  keySummary: string;
  positiveFactors: string;
  negativeFactors: string;
  fraudRiskPoints: string;
  modelProvider?: string | null;
  modelName?: string | null;
  generatedAt: string;
  stale: boolean;
}

export interface FinancialReportSnapshot {
  stockCode: string;
  sections: FinancialReportSection[];
  sourceRevision: string;
  refreshedAt?: string | null;
  analysis?: FinancialReportAnalysis | null;
}

export interface FinancialReportSectionSummary {
  section: string;
  label: string;
  source: string;
  rowCount: number;
}

export interface FinancialReportOverview {
  stockCount: number;
  rowCount: number;
  refreshedAt?: string | null;
  sections: FinancialReportSectionSummary[];
  analyses: FinancialReportAnalysis[];
}

export interface FinancialReportFetchProgress {
  stockCode: string;
  status: string;
  completedSections: number;
  totalSections: number;
  message: string;
  errorMessage?: string | null;
}

export interface BacktestRun {
  backtestId: string;
  datasetId: string;
  name: string;
  status: string;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  maxHoldingDays: number;
  totalAiCalls: number;
  processedAiCalls: number;
  totalTimepoints: number;
  processedTimepoints: number;
  totalSignals: number;
  tradeSignals: number;
  openTrades: number;
  winCount: number;
  lossCount: number;
  flatCount: number;
  totalPnlCny: number;
  totalPnlPercent: number;
  maxDrawdownPercent: number;
  profitFactor?: number;
  errorMessage?: string;
  createdAt: string;
  completedAt?: string;
}

export interface BacktestSignal {
  signalId: string;
  backtestId: string;
  symbol: string;
  stockName?: string;
  capturedAt: string;
  hasTrade: boolean;
  direction?: string;
  confidenceScore?: number;
  riskStatus?: string;
  entryLow?: number;
  entryHigh?: number;
  stopLoss?: number;
  takeProfit?: string;
  amountCny?: number;
  maxLossCny?: number;
  rationale?: string;
  result: string;
}

export interface BacktestTrade {
  tradeId: string;
  backtestId: string;
  signalId?: string;
  symbol: string;
  stockName?: string;
  direction: string;
  entryPrice: number;
  entryAt: string;
  exitPrice: number;
  exitAt: string;
  exitReason: string;
  stopLoss?: number;
  takeProfit?: number;
  amountCny?: number;
  holdingPeriods: number;
  pnlCny: number;
  pnlPercent: number;
}

export interface BacktestSummary {
  backtestId: string;
  totalSignals: number;
  tradeCount: number;
  winRate: number;
  totalPnlCny: number;
  totalPnlPercent: number;
  maxDrawdownPercent: number;
  profitFactor?: number;
  equityCurve: Array<{ capturedAt: string; cumulativePnlPercent: number }>;
}

export interface PaperOrderDraft {
  orderId: string;
  accountId: string;
  exchange: string;
  symbol: string;
  side: string;
  quantity: number;
  estimatedFillPrice: number;
  stopLoss?: number;
  takeProfit?: number;
}

export interface ManualPaperOrderRequest {
  accountId: string;
  symbol: string;
  marketType: string;
  side: "buy" | "sell";
  quantity: number;
  entryPrice?: number;
  leverage: number;
  stopLoss?: number;
  takeProfit?: number;
}

export interface PaperAccountSummary {
  accountId: string;
  exchange: string;
  availableUsdt: number;
}

export interface AssistantMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolsUsed?: string[];
  citedAt?: string;
}

export interface AssistantRun {
  sessionId: string;
  answer: string;
  toolsUsed: string[];
  citedAt: string;
  messages: AssistantMessage[];
}

export interface SettingsTab {
  id: SettingsTabId;
  label: string;
  blurb: string;
}

export interface UnifiedSignal {
  signalId: string;
  symbol: string;
  marketType: string;
  direction: "Buy" | "Sell" | "Neutral";
  score: number;
  strength: number;
  categoryBreakdown: Record<string, number>;
  contributors: string[];
  entryZoneLow: number;
  entryZoneHigh: number;
  stopLoss: number;
  takeProfit: number;
  reasonSummary: string;
  riskStatus: string;
  riskResult?: RiskDecision;
  executed: boolean;
  modified: boolean;
  generatedAt: string;
}

export interface SignalHistoryPage {
  items: UnifiedSignal[];
  total: number;
  page: number;
  pageSize: number;
}

export interface StrategyMeta {
  strategyId: string;
  name: string;
  category: string;
  applicableMarkets: string[];
  description: string;
  defaultParams: Record<string, number>;
}

export interface StrategyConfig {
  strategyId: string;
  enabled: boolean;
  params: Record<string, number>;
}

export interface StrategyStats {
  strategyId: string;
  totalSignals: number;
  buyCount: number;
  sellCount: number;
  neutralCount: number;
  avgScore: number;
  lastGeneratedAt: string | null;
}

export interface ScanRunRow {
  id: number;
  startedAt: string;
  endedAt: string | null;
  symbolsScanned: number;
  signalsFound: number;
  durationMs: number | null;
  status: string;
}

export interface ScanRunHistoryPage {
  items: ScanRunRow[];
  total: number;
  page: number;
  pageSize: number;
}
