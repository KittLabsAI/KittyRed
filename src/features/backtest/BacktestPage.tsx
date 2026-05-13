import * as echarts from "echarts";
import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Activity, BarChart3, Database, Play, RotateCw, Square } from "lucide-react";
import { WatchlistSelectionModal } from "../../components/WatchlistSelectionModal";
import { formatCurrency, formatDateTime, formatPercent } from "../../lib/format";
import {
  createBacktest,
  createBacktestDataset,
  cancelBacktest,
  deleteBacktest,
  deleteBacktestDataset,
  getBacktestFetchProgress,
  getBacktestSummary,
  getFinancialReportAnalysis,
  getSentimentAnalysisResults,
  listBacktestFetchFailures,
  listBacktestDatasets,
  listMarkets,
  listBacktestRuns,
  listBacktestSignals,
  listBacktestTrades,
  startGenerateBacktestSignals,
  startFetchSnapshots,
  startReplayBacktest,
} from "../../lib/tauri";
import type {
  BacktestDataset,
  BacktestFetchFailure,
  BacktestFetchProgress,
  BacktestRun,
  MarketRow,
} from "../../lib/types";

type MissingAnalysisRow = {
  symbol: string;
  name: string;
  missingFinancial: boolean;
  missingSentiment: boolean;
};

type BacktestTab = "data" | "signal" | "replay" | "analysis";

const tabs: Array<{ id: BacktestTab; label: string; helper: string; icon: typeof Database }> = [
  { id: "data", label: "拉取数据", helper: "准备历史快照", icon: Database },
  { id: "signal", label: "生成AI信号", helper: "20 路并发评估", icon: Activity },
  { id: "replay", label: "回放交易", helper: "按时间顺序撮合", icon: RotateCw },
  { id: "analysis", label: "回测结果分析", helper: "查看信号和交易", icon: BarChart3 },
];

function statusLabel(status: string) {
  const normalized = status.toLowerCase();
  if (normalized === "pending") return "待开始";
  if (normalized === "fetching") return "拉取中";
  if (normalized === "ready") return "就绪";
  if (normalized === "running") return "运行中";
  if (normalized === "generating_signals") return "生成信号中";
  if (normalized === "signals_ready") return "信号已生成";
  if (normalized === "replaying") return "回放交易中";
  if (normalized === "completed") return "已完成";
  if (normalized === "failed") return "失败";
  if (normalized === "cancelled") return "已取消";
  return status;
}

function riskLabel(value?: string) {
  if (!value) return "无";
  if (value === "approved") return "通过";
  if (value === "blocked") return "拦截";
  if (value === "watch") return "观察";
  return value;
}

function exitReasonLabel(value: string) {
  if (value === "take_profit") return "止盈";
  if (value === "stop_loss") return "止损";
  if (value === "timeout") return "超时";
  if (value === "sell_signal") return "卖出信号";
  if (value === "backtest_end") return "结束强平";
  return value;
}

function parseSymbols(value: string) {
  return value
    .split(/[\s,，]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function datasetProgress(dataset: BacktestDataset) {
  if (dataset.totalSnapshots > 0) return `${dataset.fetchedCount}/${dataset.totalSnapshots}`;
  if (dataset.fetchedCount > 0) return `${dataset.fetchedCount}`;
  return "待拉取";
}

function runProgress(run: BacktestRun) {
  if (run.status === "generating_signals") {
    if (run.totalAiCalls <= 0) return "待开始";
    return `${run.processedAiCalls}/${run.totalAiCalls}`;
  }
  if (run.totalTimepoints <= 0) return "待回放";
  return `${run.processedTimepoints}/${run.totalTimepoints}`;
}

function formatSignedPercent(value: number) {
  return formatPercent(value);
}

function buildEquityCurveOption(points: Array<{ capturedAt: string; cumulativePnlPercent: number }>) {
  const values = points.length > 0 ? points : [{ capturedAt: "", cumulativePnlPercent: 0 }];
  return {
    animation: false,
    grid: {
      left: 18,
      right: 18,
      top: 24,
      bottom: 28,
      containLabel: true,
    },
    tooltip: {
      trigger: "axis",
      valueFormatter: (value: number) => formatSignedPercent(value),
    },
    xAxis: {
      type: "category",
      boundaryGap: false,
      data: values.map((point) => formatDateTime(point.capturedAt)),
      axisLabel: {
        color: "rgba(237, 243, 255, 0.58)",
        formatter: (value: string) => value.slice(5, 16),
      },
      axisLine: {
        lineStyle: {
          color: "rgba(237, 243, 255, 0.12)",
        },
      },
    },
    yAxis: {
      type: "value",
      axisLabel: {
        color: "rgba(237, 243, 255, 0.58)",
        formatter: (value: number) => `${value}%`,
      },
      splitLine: {
        lineStyle: {
          color: "rgba(237, 243, 255, 0.08)",
        },
      },
    },
    series: [
      {
        type: "line",
        smooth: true,
        showSymbol: values.length <= 1,
        data: values.map((point) => point.cumulativePnlPercent),
        lineStyle: {
          width: 3,
          color: "#6cf4cb",
        },
        itemStyle: {
          color: "#8fdcff",
        },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: "rgba(108, 244, 203, 0.28)" },
            { offset: 1, color: "rgba(108, 244, 203, 0.02)" },
          ]),
        },
      },
    ],
  };
}

type TradeCardRow = {
  key: string;
  symbol: string;
  stockName: string;
  entryAt: string;
  entryPrice: number;
  exitAt?: string;
  exitPrice?: number;
  exitReason?: string;
  amountCny: number;
  pnlCny: number;
  pnlPercent: number;
  isOpen: boolean;
};

type FinalHoldingRow = {
  symbol: string;
  stockName: string;
  entryAt: string;
  holdingCost: number;
  latestPrice: number;
  pnlPercent: number;
  pnlCny: number;
  holdingAmountCny: number;
  totalBuyAmountCny: number;
  totalSellAmountCny: number;
};

function symbolTradeKey(symbol: string) {
  return symbol;
}

function formatMaybeDateTime(value?: string) {
  return value ? formatDateTime(value) : "无";
}

function buildTradeCardRows(
  trades: Awaited<ReturnType<typeof listBacktestTrades>>,
  openPositions: Awaited<ReturnType<typeof getBacktestSummary>>["openPositions"],
) {
  const closedRows = trades.map(
    (trade): TradeCardRow => ({
      key: trade.tradeId,
      symbol: trade.symbol,
      stockName: trade.stockName ?? trade.symbol,
      entryAt: trade.entryAt,
      entryPrice: trade.entryPrice,
      exitAt: trade.exitAt,
      exitPrice: trade.exitPrice,
      exitReason: trade.exitReason,
      amountCny: trade.amountCny ?? 0,
      pnlCny: trade.pnlCny,
      pnlPercent: trade.pnlPercent,
      isOpen: false,
    }),
  );
  const openRows = openPositions.map(
    (position): TradeCardRow => ({
      key: position.signalId,
      symbol: position.symbol,
      stockName: position.stockName ?? position.symbol,
      entryAt: position.entryAt,
      entryPrice: position.entryPrice,
      amountCny: position.amountCny,
      pnlCny: position.unrealizedPnlCny,
      pnlPercent: position.unrealizedPnlPercent,
      isOpen: true,
    }),
  );

  return [...closedRows, ...openRows].sort((left, right) => {
    const leftTime = left.exitAt ?? left.entryAt;
    const rightTime = right.exitAt ?? right.entryAt;
    if (leftTime === rightTime) return left.symbol.localeCompare(right.symbol);
    return leftTime.localeCompare(rightTime);
  });
}

function buildFinalHoldingRows(
  trades: Awaited<ReturnType<typeof listBacktestTrades>>,
  openPositions: Awaited<ReturnType<typeof getBacktestSummary>>["openPositions"],
) {
  const rowsBySymbol = new Map<string, FinalHoldingRow & {
    totalBuyQuantity: number;
    latestSeenAt: string;
  }>();

  const ensureRow = (symbol: string, stockName: string, entryAt: string) => {
    const existing = rowsBySymbol.get(symbol);
    if (existing) {
      existing.stockName = existing.stockName || stockName;
      if (entryAt < existing.entryAt) {
        existing.entryAt = entryAt;
      }
      return existing;
    }
    const row = {
      symbol,
      stockName,
      entryAt,
      holdingCost: 0,
      latestPrice: 0,
      pnlPercent: 0,
      pnlCny: 0,
      holdingAmountCny: 0,
      totalBuyAmountCny: 0,
      totalSellAmountCny: 0,
      totalBuyQuantity: 0,
      latestSeenAt: entryAt,
    };
    rowsBySymbol.set(symbol, row);
    return row;
  };

  for (const trade of trades) {
    const row = ensureRow(trade.symbol, trade.stockName ?? trade.symbol, trade.entryAt);
    const buyAmount = trade.amountCny ?? 0;
    const quantity = buyAmount / Math.max(trade.entryPrice, 0.01);
    row.totalBuyAmountCny += buyAmount;
    row.totalBuyQuantity += quantity;
    row.totalSellAmountCny += buyAmount + trade.pnlCny;
    row.pnlCny += trade.pnlCny;
    row.latestPrice = trade.exitPrice;
    row.latestSeenAt = trade.exitAt;
  }

  for (const position of openPositions) {
    const row = ensureRow(position.symbol, position.stockName ?? position.symbol, position.entryAt);
    row.totalBuyAmountCny += position.amountCny;
    row.totalBuyQuantity += position.amountCny / Math.max(position.entryPrice, 0.01);
    row.holdingAmountCny += position.amountCny + position.unrealizedPnlCny;
    row.pnlCny += position.unrealizedPnlCny;
    row.latestPrice = position.markPrice;
    row.latestSeenAt = position.entryAt;
  }

  return Array.from(rowsBySymbol.values())
    .map((row) => {
      const holdingCost = row.totalBuyQuantity > 0 ? row.totalBuyAmountCny / row.totalBuyQuantity : 0;
      const pnlPercent = row.totalBuyAmountCny > 0 ? (row.pnlCny / row.totalBuyAmountCny) * 100 : 0;
      return {
        symbol: row.symbol,
        stockName: row.stockName,
        entryAt: row.entryAt,
        holdingCost,
        latestPrice: row.latestPrice,
        pnlPercent,
        pnlCny: row.pnlCny,
        holdingAmountCny: row.holdingAmountCny,
        totalBuyAmountCny: row.totalBuyAmountCny,
        totalSellAmountCny: row.totalSellAmountCny,
      } satisfies FinalHoldingRow;
    })
    .sort((left, right) => left.entryAt.localeCompare(right.entryAt));
}

export function BacktestPage() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<BacktestTab>("data");
  const [datasetName, setDatasetName] = useState("5月A股数据");
  const [symbolText, setSymbolText] = useState("SHSE.600000");
  const [selectedSymbols, setSelectedSymbols] = useState<string[]>([]);
  const [activeFetchDatasetId, setActiveFetchDatasetId] = useState("");
  const [failureDatasetId, setFailureDatasetId] = useState("");
  const [startDate, setStartDate] = useState("2026-05-01");
  const [endDate, setEndDate] = useState("2026-05-07");
  const [intervalMinutes, setIntervalMinutes] = useState(30);
  const [runName, setRunName] = useState("5月AI回测");
  const [selectedDatasetId, setSelectedDatasetId] = useState("");
  const [selectedRunId, setSelectedRunId] = useState("");
  const [signalStockFilter, setSignalStockFilter] = useState("all");
  const [signalDirectionFilter, setSignalDirectionFilter] = useState("all");
  const [signalSelectionOpen, setSignalSelectionOpen] = useState(false);
  const [pendingSignalSymbols, setPendingSignalSymbols] = useState<string[] | null>(null);
  const [missingAnalyses, setMissingAnalyses] = useState<MissingAnalysisRow[]>([]);

  const datasetsQuery = useQuery({
    queryKey: ["backtest-datasets"],
    queryFn: listBacktestDatasets,
    refetchInterval: 5_000,
  });
  const runsQuery = useQuery({
    queryKey: ["backtest-runs"],
    queryFn: listBacktestRuns,
    refetchInterval: 5_000,
  });
  const watchlistQuery = useQuery({
    queryKey: ["backtest-watchlist"],
    queryFn: listMarkets,
  });

  const datasets = datasetsQuery.data ?? [];
  const runs = runsQuery.data ?? [];
  const readyDatasets = datasets.filter((dataset) => dataset.status === "ready");
  const activeDatasetId = selectedDatasetId || readyDatasets[0]?.datasetId || datasets[0]?.datasetId || "";
  const completedRuns = runs.filter((run) => run.status === "completed");
  const activeRunId = selectedRunId || completedRuns[0]?.backtestId || runs[0]?.backtestId || "";
  const activeRun = runs.find((run) => run.backtestId === activeRunId);
  const replayableRuns = runs.filter((run) =>
    ["signals_ready", "replaying", "completed"].includes(run.status),
  );
  const replayRunId =
    replayableRuns.some((run) => run.backtestId === selectedRunId)
      ? selectedRunId
      : replayableRuns[0]?.backtestId || activeRunId;
  const replayRun = runs.find((run) => run.backtestId === replayRunId);
  const watchlistRows = watchlistQuery.data ?? [];
  const selectedDataset = datasets.find((dataset) => dataset.datasetId === activeFetchDatasetId);
  const failureDataset = datasets.find((dataset) => dataset.datasetId === failureDatasetId);
  const datasetSymbols = selectedSymbols.length > 0 ? selectedSymbols : parseSymbols(symbolText);

  useEffect(() => {
    if (selectedSymbols.length === 0 && watchlistRows.length > 0) {
      setSelectedSymbols(watchlistRows.map((row) => row.symbol));
    }
  }, [selectedSymbols.length, watchlistRows]);

  const summaryQuery = useQuery({
    queryKey: ["backtest-summary", activeRunId],
    queryFn: () => getBacktestSummary(activeRunId),
    enabled: activeRunId.length > 0,
    refetchInterval: activeRun && activeRun.status !== "completed" ? 5_000 : false,
  });
  const signalsQuery = useQuery({
    queryKey: ["backtest-signals", activeRunId],
    queryFn: () => listBacktestSignals(activeRunId),
    enabled: activeRunId.length > 0,
    refetchInterval:
      activeRun && ["generating_signals", "signals_ready", "replaying"].includes(activeRun.status)
        ? 5_000
        : false,
  });
  const tradesQuery = useQuery({
    queryKey: ["backtest-trades", activeRunId],
    queryFn: () => listBacktestTrades(activeRunId),
    enabled: activeRunId.length > 0,
    refetchInterval: activeRun && activeRun.status === "replaying" ? 5_000 : false,
  });
  const fetchProgressQuery = useQuery({
    queryKey: ["backtest-fetch-progress", activeFetchDatasetId],
    queryFn: () => getBacktestFetchProgress(activeFetchDatasetId),
    enabled: activeFetchDatasetId.length > 0,
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      if (status && ["ready", "failed", "cancelled"].includes(status)) {
        return false;
      }
      return 1_500;
    },
  });
  const fetchFailuresQuery = useQuery({
    queryKey: ["backtest-fetch-failures", failureDatasetId],
    queryFn: () => listBacktestFetchFailures(failureDatasetId),
    enabled: failureDatasetId.length > 0,
  });

  const createDatasetMutation = useMutation({
    mutationFn: () =>
      createBacktestDataset({
        name: datasetName,
        symbols: datasetSymbols,
        startDate,
        endDate,
        intervalMinutes,
      }),
    onSuccess: async (dataset) => {
      setSelectedDatasetId(dataset.datasetId);
      await queryClient.invalidateQueries({ queryKey: ["backtest-datasets"] });
    },
  });
  const fetchMutation = useMutation({
    mutationFn: async (datasetId: string) => {
      setActiveFetchDatasetId(datasetId);
      await startFetchSnapshots(datasetId);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backtest-datasets"] });
      await queryClient.invalidateQueries({ queryKey: ["backtest-fetch-progress"] });
    },
  });
  const deleteDatasetMutation = useMutation({
    mutationFn: deleteBacktestDataset,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backtest-datasets"] });
    },
  });
  const generateSignalsMutation = useMutation({
    mutationFn: async (selectedSymbols: string[]) => {
      const run = await createBacktest({
        datasetId: activeDatasetId,
        name: runName,
        maxHoldingDays: 7,
      });
      await startGenerateBacktestSignals(run.backtestId, selectedSymbols);
      return run;
    },
    onSuccess: async (run) => {
      setSelectedRunId(run.backtestId);
      await queryClient.invalidateQueries({ queryKey: ["backtest-runs"] });
    },
  });

  function stockDisplayName(symbol: string) {
    return watchlistRows.find((row) => row.symbol === symbol)?.baseAsset ?? symbol;
  }

  async function confirmSignalSymbols(symbols: string[]) {
    const sentimentSymbols = new Set(
      (await getSentimentAnalysisResults().catch(() => [])).map((item) => item.stockCode),
    );
    const financialEntries = await Promise.all(
      symbols.map(async (symbol) => [symbol, await getFinancialReportAnalysis(symbol).catch(() => null)] as const),
    );
    const financialSymbols = new Set(
      financialEntries
        .filter(([, analysis]) => analysis !== null)
        .map(([symbol]) => symbol),
    );
    const missing = symbols
      .map((symbol) => ({
        symbol,
        name: stockDisplayName(symbol),
        missingFinancial: !financialSymbols.has(symbol),
        missingSentiment: !sentimentSymbols.has(symbol),
      }))
      .filter((item) => item.missingFinancial || item.missingSentiment);
    if (missing.length > 0) {
      setPendingSignalSymbols(symbols);
      setMissingAnalyses(missing);
      return;
    }
    setSignalSelectionOpen(false);
    generateSignalsMutation.mutate(symbols);
  }

  function continueSignalGeneration() {
    if (!pendingSignalSymbols) return;
    const symbols = pendingSignalSymbols;
    setPendingSignalSymbols(null);
    setMissingAnalyses([]);
    setSignalSelectionOpen(false);
    generateSignalsMutation.mutate(symbols);
  }
  const replayMutation = useMutation({
    mutationFn: async (backtestId: string) => {
      await startReplayBacktest(backtestId);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backtest-runs"] });
    },
  });
  const cancelRunMutation = useMutation({
    mutationFn: cancelBacktest,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backtest-runs"] });
    },
  });
  const deleteRunMutation = useMutation({
    mutationFn: deleteBacktest,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["backtest-runs"] });
    },
  });

  const summary = summaryQuery.data;
  const signals = signalsQuery.data ?? [];
  const trades = tradesQuery.data ?? [];

  useEffect(() => {
    if (!activeRunId) return;
    void queryClient.invalidateQueries({ queryKey: ["backtest-summary", activeRunId] });
    void queryClient.invalidateQueries({ queryKey: ["backtest-signals", activeRunId] });
    void queryClient.invalidateQueries({ queryKey: ["backtest-trades", activeRunId] });
  }, [
    activeRunId,
    activeRun?.status,
    activeRun?.processedAiCalls,
    activeRun?.processedTimepoints,
    activeRun?.totalSignals,
    activeRun?.winCount,
    activeRun?.lossCount,
    activeRun?.flatCount,
    queryClient,
  ]);

  return (
    <section className="page-stack backtest-page">
      <section className="hero-panel backtest-hero-panel">
        <div className="backtest-hero-copy">
          <span className="section-label">AI回测工作台</span>
          <h2>用历史 A 股行情重放 AI 推荐</h2>
          <p>
            先准备 AKShare 历史快照，再用当前模型和风控设置生成信号，最后顺序回放本地模拟交易。
          </p>
          <div className="backtest-hero-badges" aria-label="回测边界">
            <span>仅模拟账户</span>
            <span>AKShare 数据</span>
            <span>四阶段流程</span>
          </div>
        </div>
        <div className="backtest-hero-ledger" aria-label="回测概览">
          <span>数据集 <strong>{datasets.length}</strong></span>
          <span>回测 <strong>{runs.length}</strong></span>
          <span>完成 <strong>{completedRuns.length}</strong></span>
        </div>
      </section>

      <div aria-label="回测步骤" className="backtest-stage-tabs" role="tablist">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              aria-label={tab.label}
              aria-selected={activeTab === tab.id}
              className={`backtest-stage-tab${activeTab === tab.id ? " backtest-stage-tab--active" : ""}`}
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              role="tab"
              type="button"
            >
              <Icon aria-hidden="true" size={18} />
              <span>
                <strong>{tab.label}</strong>
                <small>{tab.helper}</small>
              </span>
            </button>
          );
        })}
      </div>

      {activeTab === "data" ? (
        <section className="panel panel--wide">
          <div className="panel__header">
            <div>
              <span className="section-label">拉取数据</span>
              <h2>历史快照数据集</h2>
            </div>
          </div>
          <div className="backtest-stage-stack">
            <div className="backtest-stage-main">
              <div className="backtest-form-grid backtest-form-grid--data">
                <label className="backtest-field">
                  <span>数据集名称</span>
                  <input value={datasetName} onChange={(event) => setDatasetName(event.target.value)} />
                </label>
                <label className="backtest-field">
                  <span>开始日期</span>
                  <input type="date" value={startDate} onChange={(event) => setStartDate(event.target.value)} />
                </label>
                <label className="backtest-field">
                  <span>结束日期</span>
                  <input type="date" value={endDate} onChange={(event) => setEndDate(event.target.value)} />
                </label>
                <label className="backtest-field">
                  <span>采样间隔</span>
                  <select value={intervalMinutes} onChange={(event) => setIntervalMinutes(Number(event.target.value))}>
                    <option value={15}>15 分钟</option>
                    <option value={30}>30 分钟</option>
                    <option value={60}>60 分钟</option>
                  </select>
                </label>
              </div>
              <label className="backtest-field backtest-field--full">
                <span>自选股票池</span>
                <WatchlistSelector
                  rows={watchlistRows}
                  selectedSymbols={selectedSymbols}
                  onChange={setSelectedSymbols}
                />
              </label>
              {watchlistRows.length === 0 ? (
                <label className="backtest-field backtest-field--manual">
                  <span>手动标的代码</span>
                  <input
                    value={symbolText}
                    onChange={(event) => setSymbolText(event.target.value)}
                    placeholder="SHSE.600000, SZSE.000001"
                  />
                </label>
              ) : null}
              <div className="backtest-action-row">
                <button
                  className="sidebar__button backtest-icon-button"
                  disabled={createDatasetMutation.isPending || datasetSymbols.length === 0}
                  onClick={() => createDatasetMutation.mutate()}
                  type="button"
                >
                  <Database aria-hidden="true" size={17} />
                  {createDatasetMutation.isPending ? "创建中..." : "创建数据集"}
                </button>
              </div>
            </div>
            <div className="backtest-progress-row" aria-label="拉取摘要">
              {activeFetchDatasetId ? (
                <FetchProgressCard
                  dataset={selectedDataset}
                  progress={fetchProgressQuery.data}
                  isLoading={fetchProgressQuery.isLoading}
                  onViewFailures={(id) => setFailureDatasetId(id)}
                />
              ) : (
                <div className="backtest-empty-state">
                  <strong>还没有正在拉取的数据集</strong>
                  <span>创建数据集后，在下方表格点击“拉取”。</span>
                </div>
              )}
            </div>
          </div>
          {failureDatasetId ? (
            <FetchFailurePanel
              dataset={failureDataset}
              failures={fetchFailuresQuery.data ?? []}
              isLoading={fetchFailuresQuery.isLoading}
              onClose={() => setFailureDatasetId("")}
            />
          ) : null}
          <DatasetTable
            datasets={datasets}
            onDelete={(id) => deleteDatasetMutation.mutate(id)}
            onFetch={(id) => fetchMutation.mutate(id)}
            onViewFailures={(id) => setFailureDatasetId(id)}
          />
        </section>
      ) : null}

      {activeTab === "signal" ? (
        <section className="panel panel--wide">
          <div className="panel__header">
            <div>
              <span className="section-label">生成AI信号</span>
              <h2>并发生成历史信号</h2>
            </div>
          </div>
          <div className="backtest-stage-stack">
            <div className="backtest-form-grid backtest-form-grid--signal">
                <label className="backtest-field">
                  <span>数据集</span>
                  <select value={activeDatasetId} onChange={(event) => setSelectedDatasetId(event.target.value)}>
                    {readyDatasets.map((dataset) => (
                      <option key={dataset.datasetId} value={dataset.datasetId}>
                        {dataset.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="backtest-field">
                  <span>回测名称</span>
                  <input value={runName} onChange={(event) => setRunName(event.target.value)} />
                </label>
                <button
                  className="sidebar__button backtest-icon-button backtest-form-action"
                  disabled={!activeDatasetId || generateSignalsMutation.isPending}
                  onClick={() => setSignalSelectionOpen(true)}
                  type="button"
                >
                  <Play aria-hidden="true" size={17} />
                  {generateSignalsMutation.isPending ? "启动中..." : "生成AI信号"}
                </button>
            </div>
            <div className="backtest-progress-row">
              {activeRun ? (
                <BacktestProgressCard
                  run={activeRun}
                  phase="signal"
                  onCancel={(id) => cancelRunMutation.mutate(id)}
                />
              ) : (
                <div className="backtest-empty-state">
                  <strong>等待生成信号</strong>
                  <span>选择已就绪数据集后启动 AI 信号任务。</span>
                </div>
              )}
            </div>
          </div>
          <RunTable runs={runs} datasets={datasets} onDelete={(id) => deleteRunMutation.mutate(id)} onView={(id) => {
            setSelectedRunId(id);
            setActiveTab("analysis");
          }} />
        </section>
      ) : null}
      <WatchlistSelectionModal
        confirmLabel="开始生成AI信号"
        description="从当前自选股池中勾选要进入本次 AI 回测信号生成的股票。"
        onClose={() => setSignalSelectionOpen(false)}
        onConfirm={(symbols) => void confirmSignalSymbols(symbols)}
        open={signalSelectionOpen}
        title="选择参与 AI 回测信号生成的股票"
        watchlist={watchlistRows}
      />
      {missingAnalyses.length > 0 ? (
        <MissingAnalysisConfirmModal
          items={missingAnalyses}
          onCancel={() => {
            setPendingSignalSymbols(null);
            setMissingAnalyses([]);
          }}
          onConfirm={continueSignalGeneration}
          title="确认继续生成 AI 信号？"
        />
      ) : null}

      {activeTab === "replay" ? (
        <section className="panel panel--wide">
          <div className="panel__header">
            <div>
              <span className="section-label">回放交易</span>
              <h2>按时间顺序模拟开平仓</h2>
            </div>
            <div className="backtest-header-actions">
              <label className="backtest-header-select">
                <span>回测任务</span>
                <select value={replayRunId} onChange={(event) => setSelectedRunId(event.target.value)}>
                  {replayableRuns.map((run) => (
                    <option key={run.backtestId} value={run.backtestId}>
                      {run.name}
                    </option>
                  ))}
                </select>
              </label>
                <button
                  className="sidebar__button backtest-icon-button backtest-header-button"
                  disabled={!replayRunId || replayMutation.isPending}
                  onClick={() => replayMutation.mutate(replayRunId)}
                  type="button"
                >
                  <RotateCw aria-hidden="true" size={17} />
                  {replayMutation.isPending ? "启动中..." : "开始回放交易"}
                </button>
            </div>
          </div>
          <div className="backtest-stage-stack">
            <div className="backtest-progress-row">
              {replayRun ? (
                <BacktestProgressCard
                  run={replayRun}
                  phase="replay"
                  onCancel={(id) => cancelRunMutation.mutate(id)}
                />
              ) : (
                <div className="backtest-empty-state">
                  <strong>暂无可回放的 AI 信号</strong>
                  <span>先在上一阶段生成信号。</span>
                </div>
              )}
            </div>
          </div>
          <RunTable runs={runs} datasets={datasets} onDelete={(id) => deleteRunMutation.mutate(id)} onView={(id) => {
            setSelectedRunId(id);
            setActiveTab("analysis");
          }} />
        </section>
      ) : null}

      {activeTab === "analysis" ? (
        <section className="panel panel--wide">
          <div className="panel__header">
            <div>
              <span className="section-label">回测结果分析</span>
              <h2>{activeRun?.name ?? "选择一个回测"}</h2>
            </div>
            <label className="search-shell">
              <select aria-label="选择回测" value={activeRunId} onChange={(event) => setSelectedRunId(event.target.value)}>
                {runs.map((run) => (
                  <option key={run.backtestId} value={run.backtestId}>
                    {run.name}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {summary ? (
            <>
              <div className="backtest-summary-strip">
                <Metric label="总信号" value={summary.totalSignals} hint="AI 输出数量" />
                <Metric label="交易" value={summary.tradeCount} hint="开仓 + 平仓" />
                <Metric label="胜率" value={formatPercent(summary.winRate)} hint="盈利交易占比" />
                <Metric
                  label="总收益"
                  value={formatPercent(summary.totalPnlPercent)}
                  hint={formatCurrency(summary.totalPnlCny)}
                  valueClassName={summary.totalPnlPercent >= 0 ? "positive-text" : "negative-text"}
                />
                <Metric label="最大回撤" value={formatPercent(-summary.maxDrawdownPercent)} hint="按交易曲线估算" />
                <Metric label="盈亏比" value={summary.profitFactor?.toFixed(2) ?? "无"} hint="盈利 / 亏损" />
              </div>
              <EquityCurve points={summary.equityCurve} />
              <FinalHoldingsTable
                openPositions={summary.openPositions}
                totalPnlCny={summary.totalPnlCny}
                trades={trades}
              />
              <TradeTable openPositions={summary.openPositions} trades={trades} />
              <SignalTable
                directionFilter={signalDirectionFilter}
                onDirectionFilterChange={setSignalDirectionFilter}
                onStockFilterChange={setSignalStockFilter}
                signals={signals}
                stockFilter={signalStockFilter}
              />
            </>
          ) : (
            <p className="panel__meta">暂无可分析的回测结果。</p>
          )}
        </section>
      ) : null}
    </section>
  );
}

function Metric({
  label,
  value,
  hint,
  valueClassName,
}: {
  label: string;
  value: string | number;
  hint: string;
  valueClassName?: string;
}) {
  return (
    <article className="metric-card">
      <p>{label}</p>
      <strong className={valueClassName}>{value}</strong>
      <small>{hint}</small>
    </article>
  );
}

function WatchlistSelector({
  rows,
  selectedSymbols,
  onChange,
}: {
  rows: MarketRow[];
  selectedSymbols: string[];
  onChange: (symbols: string[]) => void;
}) {
  if (rows.length === 0) {
    return (
      <div className="backtest-watchlist-empty">
        <strong>暂无自选股缓存</strong>
        <span>可以先在行情页添加自选股，或使用下方手动标的代码。</span>
      </div>
    );
  }

  const selected = new Set(selectedSymbols);
  const toggle = (symbol: string) => {
    const next = selected.has(symbol)
      ? selectedSymbols.filter((item) => item !== symbol)
      : [...selectedSymbols, symbol];
    onChange(next);
  };

  return (
    <div className="backtest-watchlist-select" aria-label="选择自选股票">
      {rows.map((row) => (
        <label className="backtest-watchlist-option" key={row.symbol}>
          <input
            checked={selected.has(row.symbol)}
            onChange={() => toggle(row.symbol)}
            type="checkbox"
          />
          <span>
            <strong>{row.baseAsset || row.symbol}</strong>
            <small>{row.symbol}</small>
          </span>
        </label>
      ))}
    </div>
  );
}

function FetchProgressCard({
  dataset,
  progress,
  isLoading,
  onViewFailures,
}: {
  dataset?: BacktestDataset;
  progress?: BacktestFetchProgress;
  isLoading: boolean;
  onViewFailures: (id: string) => void;
}) {
  const total = progress?.totalSnapshots ?? dataset?.totalSnapshots ?? 0;
  const estimatedTotal = dataset?.estimatedLlmCalls ?? 0;
  const fetched = progress?.fetchedCount ?? dataset?.fetchedCount ?? 0;
  const failures = progress?.failureCount ?? 0;
  const progressTotal = estimatedTotal > 0 ? estimatedTotal : total;
  const processed = Math.min(progressTotal, fetched + failures);
  const percent = progressTotal > 0 ? Math.round((processed / progressTotal) * 100) : 0;
  const datasetId = progress?.datasetId ?? dataset?.datasetId ?? "";

  return (
    <section className="backtest-progress-card" aria-label="拉取进度详情">
      <div className="backtest-progress-card__header">
        <div>
          <span className="section-label">拉取进度</span>
          <h3>{dataset?.name ?? "历史快照数据集"}</h3>
        </div>
        <strong>{isLoading ? "读取中" : `${percent}%`}</strong>
      </div>
      <div className="backtest-progress-bar" aria-label="拉取进度条" aria-valuemax={100} aria-valuemin={0} aria-valuenow={percent} role="progressbar">
        <span style={{ width: `${percent}%` }} />
      </div>
      <div className="backtest-progress-meta">
        <span>已拉取 {fetched}/{total || "待计算"}</span>
        <span>失败 {failures}</span>
        <span>{statusLabel(progress?.status ?? dataset?.status ?? "pending")}</span>
      </div>
      {progress?.errorMessage ? <p className="backtest-status-note">{progress.errorMessage}</p> : null}
      {progress?.recentFailures.length ? (
        <FailureList failures={progress.recentFailures} compact />
      ) : (
        <p className="panel__meta">暂无失败记录。</p>
      )}
      {failures > 0 && datasetId ? (
        <button className="table-action-button" onClick={() => onViewFailures(datasetId)} type="button">
          查看失败股票-时间点
        </button>
      ) : null}
    </section>
  );
}

function BacktestProgressCard({
  run,
  phase,
  onCancel,
}: {
  run: BacktestRun;
  phase: "signal" | "replay";
  onCancel: (id: string) => void;
}) {
  const isSignal = phase === "signal";
  const total = isSignal ? run.totalAiCalls : run.totalTimepoints;
  const processed = isSignal ? run.processedAiCalls : run.processedTimepoints;
  const percent = total > 0 ? Math.round((Math.min(processed, total) / total) * 100) : 0;
  const active = isSignal ? run.status === "generating_signals" : run.status === "replaying";

  return (
    <section className="backtest-progress-card" aria-label={isSignal ? "AI信号进度详情" : "回放交易进度详情"}>
      <div className="backtest-progress-card__header">
        <div>
          <span className="section-label">{isSignal ? "AI信号进度" : "回放交易进度"}</span>
          <h3>{run.name}</h3>
        </div>
        <strong>{percent}%</strong>
      </div>
      <div className="backtest-progress-bar" aria-label={isSignal ? "AI信号进度条" : "回放交易进度条"} aria-valuemax={100} aria-valuemin={0} aria-valuenow={percent} role="progressbar">
        <span style={{ width: `${percent}%` }} />
      </div>
      <div className="backtest-progress-meta">
        <span>AI 调用 {run.processedAiCalls}/{run.totalAiCalls || "待计算"}</span>
        <span>信号 {run.totalSignals}</span>
        <span>模拟交易 {run.winCount + run.lossCount + run.flatCount}</span>
        <span>回放 {run.processedTimepoints}/{run.totalTimepoints || "待开始"}</span>
        <span>{statusLabel(run.status)}</span>
        <span>交易信号 {run.tradeSignals}</span>
        <span>持仓 {run.openTrades}</span>
      </div>
      {run.errorMessage ? <p className="backtest-status-note">{run.errorMessage}</p> : null}
      {active ? (
        <button className="table-action-button" onClick={() => onCancel(run.backtestId)} type="button">
          <Square aria-hidden="true" size={14} />
          中断
        </button>
      ) : null}
    </section>
  );
}

function FetchFailurePanel({
  dataset,
  failures,
  isLoading,
  onClose,
}: {
  dataset?: BacktestDataset;
  failures: BacktestFetchFailure[];
  isLoading: boolean;
  onClose: () => void;
}) {
  return (
    <section className="backtest-failure-panel" aria-label="失败股票-时间点">
      <div className="panel__header">
        <div>
          <span className="section-label">失败记录</span>
          <h2>{dataset?.name ?? "数据集"}</h2>
        </div>
        <button className="table-action-button" onClick={onClose} type="button">
          收起
        </button>
      </div>
      {isLoading ? (
        <p className="panel__meta">正在读取失败记录。</p>
      ) : failures.length > 0 ? (
        <FailureList failures={failures} />
      ) : (
        <p className="panel__meta">没有失败股票-时间点记录。</p>
      )}
    </section>
  );
}

function FailureList({ failures, compact = false }: { failures: BacktestFetchFailure[]; compact?: boolean }) {
  const items = compact ? failures.slice(0, 3) : failures;
  return (
    <div className="backtest-failure-list">
      {items.map((failure) => (
        <article className="backtest-failure-row" key={failure.failureId}>
          <div>
            <strong>{failure.symbol}</strong>
            <span>{failure.capturedAt ? formatDateTime(failure.capturedAt) : "初始化阶段"}</span>
          </div>
          <div>
            <small>{failure.timeframe} / {failure.stage}</small>
            <p>{failure.reason}</p>
          </div>
        </article>
      ))}
    </div>
  );
}

function DatasetTable({
  datasets,
  onDelete,
  onFetch,
  onViewFailures,
}: {
  datasets: BacktestDataset[];
  onDelete: (id: string) => void;
  onFetch: (id: string) => void;
  onViewFailures: (id: string) => void;
}) {
  return (
    <div className="table-shell table-shell--visible-scrollbar backtest-table-shell">
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>时间范围</th>
            <th>标的</th>
            <th>快照</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {datasets.map((dataset) => (
            <tr key={dataset.datasetId}>
              <td>{dataset.name}</td>
              <td>{dataset.startDate} 至 {dataset.endDate}</td>
              <td>{dataset.symbols.join("、")}</td>
              <td>{datasetProgress(dataset)}</td>
              <td>
                <div className="backtest-status-cell">
                  <strong>{statusLabel(dataset.status)}</strong>
                  {dataset.errorMessage ? <span>{dataset.errorMessage}</span> : null}
                </div>
              </td>
              <td>
                <div className="backtest-table-actions">
                  <button className="table-action-button" onClick={() => onFetch(dataset.datasetId)} type="button">
                    拉取
                  </button>
                  <button className="table-action-button" onClick={() => onDelete(dataset.datasetId)} type="button">
                    删除
                  </button>
                  {dataset.errorMessage ? (
                    <button className="table-action-button" onClick={() => onViewFailures(dataset.datasetId)} type="button">
                      失败记录
                    </button>
                  ) : null}
                </div>
              </td>
            </tr>
          ))}
          {datasets.length === 0 ? (
            <tr>
              <td className="table-empty-cell" colSpan={6}>暂无数据集。</td>
            </tr>
          ) : null}
        </tbody>
      </table>
    </div>
  );
}

function RunTable({
  runs,
  datasets,
  onDelete,
  onView,
}: {
  runs: BacktestRun[];
  datasets: BacktestDataset[];
  onDelete: (id: string) => void;
  onView: (id: string) => void;
}) {
  return (
    <div className="table-shell table-shell--visible-scrollbar backtest-table-shell">
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>数据集</th>
            <th>状态</th>
            <th>进度</th>
            <th>信号</th>
            <th>胜率</th>
            <th>收益</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {runs.map((run) => {
            const dataset = datasets.find((item) => item.datasetId === run.datasetId);
            const tradeCount = run.winCount + run.lossCount + run.flatCount;
            const winRate = tradeCount === 0 ? 0 : (run.winCount / tradeCount) * 100;
            return (
              <tr key={run.backtestId}>
                <td>{run.name}</td>
                <td>{dataset?.name ?? run.datasetId}</td>
                <td>{statusLabel(run.status)}</td>
                <td>{runProgress(run)}</td>
                <td>{run.totalSignals}</td>
                <td>{formatPercent(winRate)}</td>
                <td>{formatPercent(run.totalPnlPercent)}</td>
                <td>
                  <div className="backtest-table-actions">
                    <button className="table-action-button" onClick={() => onView(run.backtestId)} type="button">
                      查看
                    </button>
                    <button className="table-action-button" onClick={() => onDelete(run.backtestId)} type="button">
                      删除
                    </button>
                  </div>
                </td>
              </tr>
            );
          })}
          {runs.length === 0 ? (
            <tr>
              <td className="table-empty-cell" colSpan={8}>暂无回测。</td>
            </tr>
          ) : null}
        </tbody>
      </table>
    </div>
  );
}

function EquityCurve({
  points,
}: {
  points: Array<{ capturedAt: string; cumulativePnlPercent: number }>;
}) {
  return (
    <section className="backtest-curve">
      <div className="panel__header">
        <div>
          <span className="section-label">收益曲线</span>
          <h2>累计 PnL%</h2>
        </div>
      </div>
      <ChartSurface ariaLabel="收益曲线" className="backtest-curve__chart" option={buildEquityCurveOption(points)} />
    </section>
  );
}

function FinalHoldingsTable({
  trades,
  openPositions,
  totalPnlCny,
}: {
  trades: Awaited<ReturnType<typeof listBacktestTrades>>;
  openPositions: Awaited<ReturnType<typeof getBacktestSummary>>["openPositions"];
  totalPnlCny: number;
}) {
  const rows = buildFinalHoldingRows(trades, openPositions);
  const totalMarketValue = openPositions.reduce((sum, position) => sum + position.amountCny + position.unrealizedPnlCny, 0);
  const totalAssets = 1_000_000 + totalPnlCny;
  const totalCash = totalAssets - totalMarketValue;

  if (rows.length === 0) {
    return null;
  }

  return (
    <section className="backtest-result-section">
      <div className="panel__header">
        <div>
          <span className="section-label">最终持仓</span>
          <h2>资产总览与持仓明细</h2>
        </div>
      </div>
      <div className="backtest-summary-strip backtest-summary-strip--holdings" aria-label="资产总览">
        <Metric label="总资产" value={formatCurrency(totalAssets)} hint="初始资金 + 总盈亏" />
        <Metric label="总现金" value={formatCurrency(totalCash)} hint="总资产 - 总市值" />
        <Metric label="总市值" value={formatCurrency(totalMarketValue)} hint="当前持仓市值" />
        <Metric
          label="总盈亏"
          value={formatCurrency(totalPnlCny)}
          hint="回测结束结果"
          valueClassName={totalPnlCny >= 0 ? "positive-text" : "negative-text"}
        />
      </div>
      <div className="table-shell table-shell--visible-scrollbar backtest-table-shell backtest-holdings-table">
        <table>
          <thead>
            <tr>
              <th>代码</th>
              <th>名称</th>
              <th>开仓时间</th>
              <th>持仓成本</th>
              <th>最新价</th>
              <th>盈亏比例</th>
              <th>盈亏金额</th>
              <th>持仓金额</th>
              <th>总买入金额</th>
              <th>总卖出金额</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={symbolTradeKey(row.symbol)}>
                <td>{row.symbol}</td>
                <td>{row.stockName}</td>
                <td>{formatMaybeDateTime(row.entryAt)}</td>
                <td>{formatCurrency(row.holdingCost)}</td>
                <td>{formatCurrency(row.latestPrice)}</td>
                <td className={row.pnlPercent >= 0 ? "positive-text" : "negative-text"}>
                  <span>{formatPercent(row.pnlPercent)}</span>
                </td>
                <td className={row.pnlCny >= 0 ? "positive-text" : "negative-text"}>
                  {formatCurrency(row.pnlCny)}
                </td>
                <td>{formatCurrency(row.holdingAmountCny)}</td>
                <td>{formatCurrency(row.totalBuyAmountCny)}</td>
                <td>{formatCurrency(row.totalSellAmountCny)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function ChartSurface({
  ariaLabel,
  className,
  option,
}: {
  ariaLabel: string;
  className: string;
  option: unknown;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, undefined, { renderer: "canvas" });
    chart.setOption(option);
    const resize = () => chart.resize();
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
    };
  }, [option]);

  return <div aria-label={ariaLabel} className={className} ref={containerRef} role="img" />;
}

function SignalTable({
  signals,
  stockFilter,
  directionFilter,
  onStockFilterChange,
  onDirectionFilterChange,
}: {
  signals: Awaited<ReturnType<typeof listBacktestSignals>>;
  stockFilter: string;
  directionFilter: string;
  onStockFilterChange: (value: string) => void;
  onDirectionFilterChange: (value: string) => void;
}) {
  const [page, setPage] = useState(1);
  const stockOptions = Array.from(new Set(signals.map((signal) => signal.symbol))).sort();
  const directionOptions = Array.from(
    new Set(signals.map((signal) => signal.direction ?? "观望")),
  ).sort();
  const filteredSignals = signals.filter((signal) => {
    const direction = signal.direction ?? "观望";
    return (
      (stockFilter === "all" || signal.symbol === stockFilter) &&
      (directionFilter === "all" || direction === directionFilter)
    );
  });
  const pageSize = 10;
  const totalPages = Math.max(1, Math.ceil(filteredSignals.length / pageSize));
  const currentPage = Math.min(page, totalPages);
  const pagedSignals = filteredSignals.slice(
    (currentPage - 1) * pageSize,
    currentPage * pageSize,
  );

  return (
    <section className="backtest-result-section">
      <div className="panel__header">
        <div>
          <span className="section-label">信号明细</span>
          <h2>AI 历史信号</h2>
        </div>
        <div className="backtest-filter-row">
          <select aria-label="筛选个股" value={stockFilter} onChange={(event) => { onStockFilterChange(event.target.value); setPage(1); }}>
            <option value="all">全部个股</option>
            {stockOptions.map((symbol) => (
              <option key={symbol} value={symbol}>
                {symbol}
              </option>
            ))}
          </select>
          <select aria-label="筛选方向" value={directionFilter} onChange={(event) => { onDirectionFilterChange(event.target.value); setPage(1); }}>
            <option value="all">全部方向</option>
            {directionOptions.map((direction) => (
              <option key={direction} value={direction}>
                {direction}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="table-shell table-shell--visible-scrollbar backtest-table-shell">
        <table>
          <thead>
            <tr>
              <th>时间</th>
              <th>代码</th>
              <th>名称</th>
              <th>方向</th>
              <th>置信度</th>
              <th>风险</th>
              <th>建议原因</th>
            </tr>
          </thead>
          <tbody>
            {pagedSignals.map((signal) => (
              <tr key={signal.signalId}>
                <td>{formatDateTime(signal.capturedAt)}</td>
                <td>{signal.symbol}</td>
                <td>{signal.stockName ?? signal.symbol}</td>
                <td>{signal.direction ?? "观望"}</td>
                <td>{signal.confidenceScore ?? "无"}</td>
                <td>{riskLabel(signal.riskStatus)}</td>
                <td>{signal.rationale ?? "无"}</td>
              </tr>
            ))}
            {filteredSignals.length === 0 ? (
              <tr>
                <td className="table-empty-cell" colSpan={7}>暂无匹配信号。</td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
      {filteredSignals.length > pageSize ? (
        <div className="pagination-bar">
          <span>
            第 {currentPage} / {totalPages} 页，每页 {pageSize} 条
          </span>
          <div className="pagination-bar__actions">
            <button
              className="ghost-button table-action-button"
              disabled={currentPage <= 1}
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              type="button"
            >
              上一页
            </button>
            <button
              className="ghost-button table-action-button"
              disabled={currentPage >= totalPages}
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              type="button"
            >
              下一页
            </button>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function TradeTable({
  trades,
  openPositions,
}: {
  trades: Awaited<ReturnType<typeof listBacktestTrades>>;
  openPositions: Awaited<ReturnType<typeof getBacktestSummary>>["openPositions"];
}) {
  const [page, setPage] = useState(1);
  const [stockFilter, setStockFilter] = useState("all");
  const rows = buildTradeCardRows(trades, openPositions);
  const stockOptions = Array.from(new Set(rows.map((row) => row.symbol))).sort();
  const filteredRows = rows.filter((row) => stockFilter === "all" || row.symbol === stockFilter);
  const pageSize = 10;
  const totalPages = Math.max(1, Math.ceil(filteredRows.length / pageSize));
  const currentPage = Math.min(page, totalPages);
  const pagedRows = filteredRows.slice((currentPage - 1) * pageSize, currentPage * pageSize);

  useEffect(() => {
    setPage(1);
  }, [stockFilter, trades, openPositions]);

  return (
    <section className="backtest-result-section">
      <div className="panel__header">
        <div>
          <span className="section-label">交易记录</span>
          <h2>模拟开平仓</h2>
        </div>
        <div className="backtest-filter-row">
          <select aria-label="筛选个股" value={stockFilter} onChange={(event) => setStockFilter(event.target.value)}>
            <option value="all">全部个股</option>
            {stockOptions.map((symbol) => (
              <option key={symbol} value={symbol}>
                {symbol}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="table-shell table-shell--visible-scrollbar backtest-table-shell backtest-trade-table">
        <table>
          <thead>
            <tr>
              <th>代码</th>
              <th>名称</th>
              <th>入场时间</th>
              <th>入场</th>
              <th>出场时间</th>
              <th>出场</th>
              <th>原因</th>
              <th>收益</th>
              <th>金额</th>
            </tr>
          </thead>
          <tbody>
            {pagedRows.map((row) => (
              <tr key={row.key}>
                <td>{row.symbol}</td>
                <td>{row.stockName}</td>
                <td>{formatDateTime(row.entryAt)}</td>
                <td>{formatCurrency(row.entryPrice)}</td>
                <td>{formatMaybeDateTime(row.exitAt)}</td>
                <td>{row.isOpen ? "无" : formatCurrency(row.exitPrice ?? 0)}</td>
                <td>{row.isOpen ? "无" : exitReasonLabel(row.exitReason ?? "")}</td>
                <td className={row.pnlPercent >= 0 ? "positive-text" : "negative-text"}>{formatPercent(row.pnlPercent)}</td>
                <td>{formatCurrency(row.amountCny)}</td>
              </tr>
            ))}
            {filteredRows.length === 0 ? (
              <tr>
                <td className="table-empty-cell" colSpan={9}>暂无匹配交易。</td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
      {filteredRows.length > pageSize ? (
        <div className="pagination-bar">
          <span>
            第 {currentPage} / {totalPages} 页，每页 {pageSize} 条
          </span>
          <div className="pagination-bar__actions">
            <button
              className="ghost-button table-action-button"
              disabled={currentPage <= 1}
              onClick={() => setPage((value) => Math.max(1, value - 1))}
              type="button"
            >
              上一页
            </button>
            <button
              className="ghost-button table-action-button"
              disabled={currentPage >= totalPages}
              onClick={() => setPage((value) => Math.min(totalPages, value + 1))}
              type="button"
            >
              下一页
            </button>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function MissingAnalysisConfirmModal({
  items,
  onCancel,
  onConfirm,
  title,
}: {
  items: MissingAnalysisRow[];
  onCancel: () => void;
  onConfirm: () => void;
  title: string;
}) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <section
        aria-label={title}
        aria-modal="true"
        className="modal-content analysis-missing-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="modal-header">
          <div>
            <p className="section-label">分析数据缺失</p>
            <h2>{title}</h2>
            <p className="panel__meta">以下股票缺少 AI财报分析或 AI舆情分析结果，继续后 LLM 将收到空值。</p>
          </div>
        </div>
        <div className="analysis-missing-list">
          {items.map((item) => (
            <div key={item.symbol}>
              <strong>{item.name}</strong>
              <span>{item.symbol}</span>
              <em>
                {[
                  item.missingFinancial ? "缺少 AI财报分析" : null,
                  item.missingSentiment ? "缺少 AI舆情分析" : null,
                ].filter(Boolean).join("、")}
              </em>
            </div>
          ))}
        </div>
        <div className="modal-actions">
          <button className="ghost-button" onClick={onCancel} type="button">取消</button>
          <button className="sidebar__button" onClick={onConfirm} type="button">继续分析</button>
        </div>
      </section>
    </div>
  );
}
