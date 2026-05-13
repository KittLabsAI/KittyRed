import { useEffect, useMemo, useRef, useState } from "react";
import { Check, Circle, CircleAlert, XIcon } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { WatchlistSelectionModal } from "../../components/WatchlistSelectionModal";
import { formatCurrency, formatDateTime, formatPercent } from "../../lib/format";
import {
  deleteRecommendation,
  getFinancialReportAnalysis,
  getLatestRecommendation,
  getRecommendationGenerationProgress,
  getRecommendationAudit,
  getSentimentAnalysisResults,
  listMarkets,
  listRecommendationHistory,
  startRecommendationGeneration,
} from "../../lib/tauri";
import { useAppStore } from "../../store/appStore";
import type {
  MarketRow,
  RecommendationAudit,
  RecommendationGenerationProgressItem,
  RecommendationHistoryRow,
  RecommendationRun,
} from "../../lib/types";

type MissingAnalysisRow = {
  symbol: string;
  name: string;
  missingFinancial: boolean;
  missingSentiment: boolean;
};

function formatEntryRange(low?: number, high?: number) {
  if (low === undefined && high === undefined) return "无";
  if (low === undefined) return String(high);
  if (high === undefined || high === low) return String(low);
  return `${low} - ${high}`;
}

function stockName(symbol: string, markets: MarketRow[]) {
  return markets.find((row) => row.symbol === symbol)?.baseAsset ?? fallbackStockNames[symbol] ?? "未知";
}

function historyStockName(row: RecommendationHistoryRow, markets: MarketRow[]) {
  return row.stockName ?? stockName(row.symbol, markets);
}

function historyResultLabel(value: string) {
  const normalized = value.trim().toLowerCase();
  if (normalized === "win") return "盈利";
  if (normalized === "loss") return "亏损";
  if (normalized === "flat") return "持平";
  if (normalized === "pending") return "待评估";
  if (normalized === "blocked") return "已拦截";
  if (normalized === "no trade" || normalized === "no_trade") return "未交易";
  return value || "无";
}

function riskLabel(value: string) {
  const normalized = value.trim().toLowerCase();
  if (normalized === "approved") return "通过";
  if (normalized === "blocked") return "拦截";
  if (normalized === "watch") return "观察";
  if (normalized === "failed") return "失败";
  return value || "无";
}

function historyOutcomeLabel(value: string) {
  return value
    .replace("Queued for 10m / 60m / 24h / 7d evaluation windows.", "等待 10 分钟、60 分钟、24 小时和 7 天评估窗口。")
    .replace("Live evaluation complete through 7d using persisted ledger records.", "已使用持久化记录完成 7 天评估。")
    .replace("Live evaluation complete through 7d using market candles.", "已使用行情 K 线完成 7 天评估。")
    .replace("Live evaluation complete through 24h using persisted ledger records.", "已使用持久化记录完成 24 小时评估。")
    .replace("Live evaluation complete through 24h using market candles.", "已使用行情 K 线完成 24 小时评估。")
    .replace("Live evaluation updated through", "实时评估已更新至")
    .replace("waiting for", "等待")
    .replace("windows.", "评估窗口。");
}

function shouldShowOutcome(row: RecommendationHistoryRow) {
  return row.result.trim().toLowerCase() !== "blocked" && row.result.trim().toLowerCase() !== "no trade";
}

const fallbackStockNames: Record<string, string> = {
  "SHSE.600000": "浦发银行",
  "SZSE.000001": "平安银行",
  "SHSE.600519": "贵州茅台",
  "SHSE.601318": "中国平安",
  "SZSE.300750": "宁德时代",
};

const comparisonColumns: Array<{ key: keyof RecommendationHistoryRow; label: string }> = [
  { key: "pnl10m", label: "10分钟" },
  { key: "pnl60m", label: "60分钟" },
  { key: "pnl24h", label: "24小时" },
  { key: "pnl7d", label: "7天" },
];

function summarizeLatestRuns(runs: RecommendationRun[], markets: MarketRow[]) {
  const groups: Record<"buy" | "sell" | "watch", string[]> = {
    buy: [],
    sell: [],
    watch: [],
  };

  for (const run of runs) {
    const name = run.stockName ?? stockName(run.symbol, markets);
    if (run.hasTrade && run.direction === "买入") {
      groups.buy.push(name);
    } else if (run.hasTrade && run.direction === "卖出") {
      groups.sell.push(name);
    } else {
      groups.watch.push(name);
    }
  }

  return [
    `买入：${groups.buy.length > 0 ? groups.buy.join("、") : "无"}`,
    `卖出：${groups.sell.length > 0 ? groups.sell.join("、") : "无"}`,
    `观察：${groups.watch.length > 0 ? groups.watch.join("、") : "无"}`,
  ];
}

function recommendationProgressMarker(status: RecommendationGenerationProgressItem["status"]) {
  if (status === "succeeded") return <Check size={14} strokeWidth={2.6} />;
  if (status === "retrying") return <CircleAlert size={14} strokeWidth={2.4} />;
  if (status === "failed") return <XIcon size={14} strokeWidth={2.4} />;
  return <Circle size={11} strokeWidth={2.2} />;
}

function shortenStockName(value: string) {
  return Array.from(value).slice(0, 4).join("");
}

function parseAuditPrompt(snapshot: string) {
  try {
    const value = JSON.parse(snapshot) as { system_prompt?: unknown; user_prompt?: unknown };
    return {
      systemPrompt: typeof value.system_prompt === "string" ? value.system_prompt : "无",
      userPrompt: typeof value.user_prompt === "string" ? value.user_prompt : "无",
    };
  } catch {
    return {
      systemPrompt: "无",
      userPrompt: "无",
    };
  }
}

export function RecommendationsPage() {
  const queryClient = useQueryClient();
  const openAssistant = useAppStore((state) => state.openAssistant);
  const [directionFilter, setDirectionFilter] = useState("all");
  const [symbolFilter, setSymbolFilter] = useState("all");
  const [selectedAuditId, setSelectedAuditId] = useState<string | null>(null);
  const [historyPage, setHistoryPage] = useState(1);
  const [selectionOpen, setSelectionOpen] = useState(false);
  const [pendingSymbols, setPendingSymbols] = useState<string[] | null>(null);
  const [missingAnalyses, setMissingAnalyses] = useState<MissingAnalysisRow[]>([]);
  const lastProgressStatusRef = useRef<string>("idle");

  const latestRecommendationQuery = useQuery({
    queryKey: ["latest-recommendation"],
    queryFn: getLatestRecommendation,
    refetchInterval: 15_000,
    staleTime: 30_000,
  });
  const historyQuery = useQuery({
    queryKey: ["recommendation-history"],
    queryFn: listRecommendationHistory,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const auditQuery = useQuery({
    queryKey: ["recommendation-audit", selectedAuditId],
    queryFn: () => (selectedAuditId ? getRecommendationAudit(selectedAuditId) : Promise.resolve(null)),
    enabled: selectedAuditId !== null,
    staleTime: 30_000,
  });
  const recommendationProgressQuery = useQuery({
    queryKey: ["recommendation-generation-progress"],
    queryFn: getRecommendationGenerationProgress,
    refetchInterval: 1_000,
    staleTime: 0,
  });
  const triggerRecommendationMutation = useMutation({
    mutationFn: (selectedSymbols: string[]) => startRecommendationGeneration(selectedSymbols),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["recommendation-generation-progress"] });
    },
  });
  const deleteRecommendationMutation = useMutation({
    mutationFn: deleteRecommendation,
    onSuccess: async (_, recommendationId) => {
      if (selectedAuditId === recommendationId) setSelectedAuditId(null);
      await queryClient.invalidateQueries({ queryKey: ["recommendation-history"] });
      await queryClient.invalidateQueries({ queryKey: ["recommendation-audit", recommendationId] });
    },
  });

  const latestRuns = latestRecommendationQuery.data ?? [];
  const history = historyQuery.data ?? [];
  const markets = marketsQuery.data ?? [];
  const recommendationProgress = recommendationProgressQuery.data;
  const symbolOptions = useMemo(
    () => Array.from(new Set(history.map((row) => row.symbol))).sort(),
    [history],
  );
  const directionOptions = useMemo(
    () => Array.from(new Set(history.map((row) => row.direction).filter(Boolean))).sort(),
    [history],
  );
  const filteredHistory = history.filter(
    (row) =>
      (directionFilter === "all" || row.direction === directionFilter) &&
      (symbolFilter === "all" || row.symbol === symbolFilter),
  );
  const historyPageSize = 10;
  const totalHistoryPages = Math.max(1, Math.ceil(filteredHistory.length / historyPageSize));
  const currentHistoryPage = Math.min(historyPage, totalHistoryPages);
  const pagedHistory = filteredHistory.slice(
    (currentHistoryPage - 1) * historyPageSize,
    currentHistoryPage * historyPageSize,
  );
  const executedCount = filteredHistory.filter((row) => row.executed).length;
  const avgPnl24h =
    filteredHistory.length === 0
      ? 0
      : filteredHistory.reduce((sum, row) => sum + row.pnl24h, 0) / filteredHistory.length;
  const selectedAudit = auditQuery.data;
  const latestSummaryLines = useMemo(
    () => summarizeLatestRuns(latestRuns, markets),
    [latestRuns, markets],
  );
  const progressPercent =
    recommendationProgress && recommendationProgress.totalCount > 0
      ? Math.min(
          100,
          Math.round((recommendationProgress.completedCount / recommendationProgress.totalCount) * 100),
        )
      : 0;

  useEffect(() => {
    const nextStatus = recommendationProgress?.status ?? "idle";
    const previousStatus = lastProgressStatusRef.current;
    if (
      nextStatus !== previousStatus &&
      (nextStatus === "completed" || nextStatus === "failed") &&
      previousStatus === "running"
    ) {
      void queryClient.invalidateQueries({ queryKey: ["latest-recommendation"] });
      void queryClient.invalidateQueries({ queryKey: ["recommendation-history"] });
    }
    lastProgressStatusRef.current = nextStatus;
  }, [recommendationProgress?.status, queryClient]);

  async function confirmRecommendationSymbols(symbols: string[]) {
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
        name: stockName(symbol, markets),
        missingFinancial: !financialSymbols.has(symbol),
        missingSentiment: !sentimentSymbols.has(symbol),
      }))
      .filter((item) => item.missingFinancial || item.missingSentiment);
    if (missing.length > 0) {
      setPendingSymbols(symbols);
      setMissingAnalyses(missing);
      return;
    }
    setSelectionOpen(false);
    triggerRecommendationMutation.mutate(symbols);
  }

  function continueRecommendationGeneration() {
    if (!pendingSymbols) return;
    const symbols = pendingSymbols;
    setPendingSymbols(null);
    setMissingAnalyses([]);
    setSelectionOpen(false);
    triggerRecommendationMutation.mutate(symbols);
  }

  return (
    <section className="page-stack">
      <section className="hero-panel recommendation-hero-panel">
        <div className="recommendation-hero-panel__header">
          <div>
            <span className="section-label">AI投资建议</span>
            <h2>{latestRuns.length > 0 ? `最新生成 ${latestRuns.length} 条个股建议` : "暂无 AI 建议"}</h2>
            <p className="recommendation-summary-text">
              {latestRuns.length > 0
                ? latestSummaryLines.join("\n")
                : "点击生成 AI 建议，基于自选股行情、K 线、持仓和风控设置生成买入、卖出或观望结论。"}
            </p>
          </div>
          <div className="hero-panel__actions recommendation-hero-actions">
            <button
              className="sidebar__button"
              disabled={triggerRecommendationMutation.isPending}
              onClick={() => setSelectionOpen(true)}
              type="button"
            >
              {triggerRecommendationMutation.isPending ? "生成中..." : "生成AI建议"}
            </button>
            <button className="ghost-button" onClick={openAssistant} type="button">
              咨询AI助手
            </button>
          </div>
        </div>
        <div className="financial-progress financial-progress--analysis recommendation-progress" aria-live="polite">
          <div className="financial-progress__meta">
            <span>{recommendationProgress?.message ?? "尚未开始 AI 建议生成"}</span>
            <strong>{progressPercent}%</strong>
          </div>
          <div
            aria-label="AI投资建议进度"
            className="financial-progress__track"
            aria-valuemax={100}
            aria-valuemin={0}
            aria-valuenow={progressPercent}
            role="progressbar"
          >
            <span style={{ width: `${progressPercent}%` }} />
          </div>
          {recommendationProgress && recommendationProgress.items.length > 0 ? (
            <section className="financial-analysis-status-grid" aria-label="AI投资建议进度">
              {recommendationProgress.items.map((item) => (
                <RecommendationStatusChip item={item} key={item.stockCode} />
              ))}
            </section>
          ) : null}
        </div>
      </section>

      <section className="panel panel--wide recommendation-history-panel">
        <div className="panel__header">
          <div>
            <span className="section-label">历史评估</span>
            <h2>投资建议历史</h2>
          </div>
          <div className="recommendation-history-filters">
            <label className="search-shell recommendation-history-filter">
              <select
                aria-label="交易方向筛选"
                onChange={(event) => {
                  setDirectionFilter(event.target.value);
                  setHistoryPage(1);
                }}
                value={directionFilter}
              >
                <option value="all">全部方向</option>
                {directionOptions.map((direction) => (
                  <option key={direction} value={direction}>
                    {direction}
                  </option>
                ))}
              </select>
            </label>
            <label className="search-shell recommendation-history-filter">
              <select
                aria-label="股票筛选"
                onChange={(event) => {
                  setSymbolFilter(event.target.value);
                  setHistoryPage(1);
                }}
                value={symbolFilter}
              >
                <option value="all">全部股票</option>
                {symbolOptions.map((symbol) => (
                  <option key={symbol} value={symbol}>
                    {symbol} {history.find((row) => row.symbol === symbol)?.stockName ?? stockName(symbol, markets)}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>
        <p className="panel__meta">
          {historyQuery.isFetching
            ? "正在刷新历史建议..."
            : `当前显示 ${filteredHistory.length} / ${history.length} 条建议。`}
        </p>
        <div className="recommendation-history-summary">
          <article className="metric-card">
            <p>建议数量</p>
            <strong>{filteredHistory.length}</strong>
            <small>按当前筛选统计</small>
          </article>
          <article className="metric-card">
            <p>模拟执行</p>
            <strong>{executedCount}</strong>
            <small>只统计本地模拟</small>
          </article>
          <article className="metric-card">
            <p>24 小时表现</p>
            <strong>{formatPercent(avgPnl24h)}</strong>
            <small>历史样本均值</small>
          </article>
        </div>
        <div className="table-shell table-shell--visible-scrollbar recommendation-history-table-shell">
          <table className="recommendation-history-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>代码</th>
                <th>名称</th>
                <th>方向</th>
                <th>结果</th>
                <th>入场</th>
                <th>止损</th>
                <th>建议金额</th>
                <th>置信度</th>
                <th>风险</th>
                {comparisonColumns.map((column) => (
                  <th key={column.key}>{column.label}</th>
                ))}
                <th className="recommendation-history-table__col--rationale">建议原因</th>
                <th className="recommendation-history-table__col--outcome">结论</th>
                <th>审查</th>
                <th>删除</th>
              </tr>
            </thead>
            <tbody>
              {pagedHistory.map((row) => (
                <tr key={row.id}>
                  <td className="recommendation-history-table__cell--nowrap">{formatDateTime(row.createdAt)}</td>
                  <td className="recommendation-history-table__cell--nowrap">{row.symbol}</td>
                  <td className="recommendation-history-table__cell--nowrap">{historyStockName(row, markets)}</td>
                  <td className="recommendation-history-table__cell--nowrap">{row.direction}</td>
                  <td className="recommendation-history-table__cell--nowrap">{historyResultLabel(row.result)}</td>
                  <td className="recommendation-history-table__cell--nowrap">{formatEntryRange(row.entryLow, row.entryHigh)}</td>
                  <td className="recommendation-history-table__cell--nowrap">{row.stopLoss ?? "无"}</td>
                  <td className="recommendation-history-table__cell--nowrap">{row.amountCny != null ? formatCurrency(row.amountCny) : "无"}</td>
                  <td className="recommendation-history-table__cell--nowrap">{row.confidence ?? "无"}</td>
                  <td className="recommendation-history-table__cell--nowrap">{riskLabel(row.risk)}</td>
                  {comparisonColumns.map((column) => {
                    const value = Number(row[column.key] ?? 0);
                    return (
                      <td
                        className={`recommendation-history-table__cell--nowrap ${value >= 0 ? "positive-text" : "negative-text"}`}
                        key={column.key}
                      >
                        {formatPercent(value)}
                      </td>
                    );
                  })}
                  <td className="recommendation-history-table__cell--rationale">{row.rationale ?? "无"}</td>
                  <td className="recommendation-history-table__cell--outcome">
                    {shouldShowOutcome(row) ? historyOutcomeLabel(row.outcome) : "-"}
                  </td>
                  <td className="recommendation-history-table__cell--nowrap">
                    <button
                      aria-label={`查看 ${row.symbol} 的审计详情`}
                      className="ghost-button table-action-button"
                      onClick={() => setSelectedAuditId(row.id)}
                      type="button"
                    >
                      审查
                    </button>
                  </td>
                  <td className="recommendation-history-table__cell--nowrap">
                    <button
                      aria-label={`删除 ${row.symbol} 的建议`}
                      className="ghost-button table-action-button"
                      disabled={deleteRecommendationMutation.isPending}
                      onClick={() => deleteRecommendationMutation.mutate(row.id)}
                      type="button"
                    >
                      {deleteRecommendationMutation.isPending && deleteRecommendationMutation.variables === row.id
                        ? "删除中..."
                        : "删除"}
                    </button>
                  </td>
                </tr>
              ))}
              {filteredHistory.length === 0 ? (
                <tr>
                  <td className="table-empty-cell" colSpan={18}>
                    暂无符合筛选条件的建议。
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
        <div className="pagination-bar recommendation-history-pagination">
          <span>
            第 {currentHistoryPage} / {totalHistoryPages} 页，每页 10 条
          </span>
          <div className="pagination-bar__actions">
            <button
              className="ghost-button table-action-button"
              disabled={currentHistoryPage <= 1}
              onClick={() => setHistoryPage((page) => Math.max(1, page - 1))}
              type="button"
            >
              上一页
            </button>
            <button
              className="ghost-button table-action-button"
              disabled={currentHistoryPage >= totalHistoryPages}
              onClick={() => setHistoryPage((page) => Math.min(totalHistoryPages, page + 1))}
              type="button"
            >
              下一页
            </button>
          </div>
        </div>
      </section>

      {selectedAuditId ? (
        <AuditDrawer
          audit={selectedAudit}
          isLoading={auditQuery.isFetching}
          onClose={() => setSelectedAuditId(null)}
        />
      ) : null}
      <WatchlistSelectionModal
        confirmLabel="开始生成AI建议"
        description="从当前自选股池中勾选要进入本次 AI 建议分析的股票。"
        onClose={() => setSelectionOpen(false)}
        onConfirm={(symbols) => void confirmRecommendationSymbols(symbols)}
        open={selectionOpen}
        title="选择参与 AI 建议分析的股票"
        watchlist={markets}
      />
      {missingAnalyses.length > 0 ? (
        <MissingAnalysisConfirmModal
          items={missingAnalyses}
          onCancel={() => {
            setPendingSymbols(null);
            setMissingAnalyses([]);
          }}
          onConfirm={continueRecommendationGeneration}
          title="确认继续生成 AI 建议？"
        />
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

function AuditDrawer({
  audit,
  isLoading,
  onClose,
}: {
  audit?: RecommendationAudit | null;
  isLoading: boolean;
  onClose: () => void;
}) {
  const prompts = audit ? parseAuditPrompt(audit.marketSnapshot) : null;
  return (
    <section aria-modal="true" aria-label="AI 推荐审计详情" className="recommendation-audit-drawer" role="dialog">
      <div className="recommendation-audit-drawer__header">
        <div>
          <span className="section-label">审查</span>
          <h2>AI 推荐详情</h2>
        </div>
        <button className="ghost-button" onClick={onClose} type="button">
          关闭
        </button>
      </div>
      <div className="recommendation-audit-drawer__body">
        {isLoading ? (
          <p className="panel__meta">正在读取本次推荐详情...</p>
        ) : audit ? (
          <>
            <dl className="detail-grid">
              <div>
                <dt>代码</dt>
                <dd>{audit.symbol}</dd>
              </div>
              <div>
                <dt>市场</dt>
                <dd>{audit.marketType}</dd>
              </div>
              <div>
                <dt>触发方式</dt>
                <dd>{audit.triggerType}</dd>
              </div>
              <div>
                <dt>生成时间</dt>
                <dd>{formatDateTime(audit.createdAt)}</dd>
              </div>
              <div>
                <dt>模型</dt>
                <dd>{audit.modelName}</dd>
              </div>
              <div>
                <dt>提示词版本</dt>
                <dd>{audit.promptVersion}</dd>
              </div>
            </dl>
            <div className="form-stack">
              <AuditBlock label="AI 系统提示词" value={prompts?.systemPrompt ?? "无"} />
              <AuditBlock label="AI 用户提示词" value={prompts?.userPrompt ?? "无"} />
              <AuditBlock label="风控结果" value={audit.riskResult} />
              <AuditBlock label="AI 原始输出" value={audit.aiRawOutput} />
              <AuditBlock label="AI 结构化输出" value={audit.aiStructuredOutput} />
              <AuditBlock label="行情快照" value={audit.marketSnapshot} />
              <AuditBlock label="账户快照" value={audit.accountSnapshot} />
            </div>
          </>
        ) : (
          <p className="panel__meta">没有找到这条建议的审计记录。</p>
        )}
      </div>
    </section>
  );
}

function RecommendationStatusChip({ item }: { item: RecommendationGenerationProgressItem }) {
  return (
    <article
      aria-label={`${item.shortName} 建议状态`}
      className={`financial-analysis-status financial-analysis-status--${item.status}`}
    >
      <span aria-hidden="true" className="financial-analysis-status__icon">
        {recommendationProgressMarker(item.status)}
      </span>
      <strong>{shortenStockName(item.shortName)}</strong>
    </article>
  );
}

function AuditBlock({ label, value }: { label: string; value: string }) {
  return (
    <label className="field">
      <span>{label}</span>
      <pre className="code-shell">{value}</pre>
    </label>
  );
}
