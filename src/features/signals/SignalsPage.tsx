import { Fragment, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { formatCurrency, formatDateTime } from "../../lib/format";
import {
  getStrategyMeta,
  getStrategyConfigs,
  getStrategyStats,
  listMarkets,
  scanSignals,
  listSignalHistory,
  updateStrategyConfig,
} from "../../lib/tauri";
import { StrategyCard } from "./StrategyCard";
import { StrategyConfigModal } from "./StrategyConfigModal";
import { SignalDetailPanel } from "./SignalDetailPanel";
import { ScanRunHistory } from "./ScanRunHistory";
import type { UnifiedSignal } from "../../lib/types";

const PAGE_SIZE = 10;
const LATEST_PAGE_SIZE = 10;

type RiskFilter = "all" | "approved" | "blocked";
type StatusFilter = "all" | "Executed" | "Modified" | "Pending";

function signalStatus(signal: UnifiedSignal): string {
  if (signal.executed) return "已执行";
  if (signal.modified) return "已调整";
  return "待处理";
}

function signalStatusKey(signal: UnifiedSignal): StatusFilter {
  if (signal.executed) return "Executed";
  if (signal.modified) return "Modified";
  return "Pending";
}

function riskClass(status: string) {
  if (status === "approved") return "positive-text";
  if (status === "blocked") return "negative-text";
  return "";
}

function dirClass(direction: string) {
  if (direction === "Buy") return "positive-text";
  if (direction === "Sell") return "negative-text";
  return "";
}

function directionLabel(direction: string) {
  if (direction === "Buy") return "买入";
  if (direction === "Sell") return "卖出";
  return direction;
}

function riskLabel(status: string) {
  if (status === "approved") return "通过";
  if (status === "blocked") return "拦截";
  return status;
}

function stockName(symbol: string, markets: Array<{ symbol: string; baseAsset: string }>) {
  return markets.find((row) => row.symbol === symbol)?.baseAsset ?? "未知";
}

export function SignalsPage() {
  const queryClient = useQueryClient();
  const [page, setPage] = useState(1);
  const [activeTab, setActiveTab] = useState<"signals" | "scans">("signals");
  const [expandedSignalId, setExpandedSignalId] = useState<string | null>(null);
  const [editingMeta, setEditingMeta] = useState<typeof allMeta[number] | null>(null);

  // 最新扫描
  const [latestScanPage, setLatestScanPage] = useState(1);
  const [latestScanRiskFilter, setLatestScanRiskFilter] = useState<RiskFilter>("all");

  // 历史筛选
  const [historyRiskFilter, setHistoryRiskFilter] = useState<RiskFilter>("all");
  const [historyStatusFilter, setHistoryStatusFilter] = useState<StatusFilter>("all");

  const metaQuery = useQuery({
    queryKey: ["strategy-meta"],
    queryFn: getStrategyMeta,
    staleTime: 300_000,
  });
  const configsQuery = useQuery({
    queryKey: ["strategy-configs"],
    queryFn: getStrategyConfigs,
    refetchInterval: 30_000,
  });
  const statsQuery = useQuery({
    queryKey: ["strategy-stats"],
    queryFn: getStrategyStats,
    refetchInterval: 30_000,
  });
  const historyQuery = useQuery({
    queryKey: ["signal-history", page, PAGE_SIZE],
    queryFn: () => listSignalHistory(page, PAGE_SIZE),
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });

  const scanMutation = useMutation({
    mutationFn: scanSignals,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["strategy-stats"] });
      queryClient.invalidateQueries({ queryKey: ["signal-history"] });
      queryClient.invalidateQueries({ queryKey: ["scan-runs"] });
      setLatestScanPage(1);
      setExpandedSignalId(null);
    },
  });

  const updateConfigMutation = useMutation({
    mutationFn: (args: { id: string; enabled?: boolean; params?: Record<string, number> }) =>
      updateStrategyConfig(args.id, args.enabled, args.params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["strategy-configs"] });
      queryClient.invalidateQueries({ queryKey: ["strategy-stats"] });
    },
  });

  const allMeta = metaQuery.data ?? [];
  const configs = configsQuery.data ?? [];
  const stats = statsQuery.data ?? [];
  const activeStrategyIds = new Set(allMeta.map((meta) => meta.strategyId));
  const activeConfigs = configs.filter((config) => activeStrategyIds.has(config.strategyId));
  const signalPage = historyQuery.data;
  const markets = marketsQuery.data ?? [];
  const items = signalPage?.items ?? [];
  const total = signalPage?.total ?? 0;
  const totalPages = signalPage ? Math.ceil(signalPage.total / signalPage.pageSize) : 1;

  const editingConfig = editingMeta
    ? configs.find((c) => c.strategyId === editingMeta.strategyId)
    : undefined;

  // 最新扫描：筛选和分页
  const scanAll = scanMutation.data ?? [];
  const scanFiltered = latestScanRiskFilter === "all"
    ? scanAll
    : scanAll.filter((s) => s.riskStatus === latestScanRiskFilter);
  const scanTotalPages = Math.max(1, Math.ceil(scanFiltered.length / LATEST_PAGE_SIZE));
  const scanPageSlice = scanFiltered.slice(
    (latestScanPage - 1) * LATEST_PAGE_SIZE,
    latestScanPage * LATEST_PAGE_SIZE,
  );

  // 信号历史：筛选
  const historyFiltered = items.filter((s) => {
    if (historyRiskFilter !== "all" && s.riskStatus !== historyRiskFilter) return false;
    if (historyStatusFilter !== "all" && signalStatusKey(s) !== historyStatusFilter) return false;
    return true;
  });
  const showEmptyHistory = !historyQuery.isFetching && historyFiltered.length === 0 && items.length === 0;
  const showFilteredEmpty = !historyQuery.isFetching && historyFiltered.length === 0 && items.length > 0;

  return (
    <section className="page-stack">
      <section className="panel panel--wide">
        <div className="panel__header">
          <div>
            <span className="section-label">信号扫描</span>
            <h2>策略信号</h2>
          </div>
          <button
            className="primary-button"
            disabled={scanMutation.isPending}
            onClick={() => scanMutation.mutate()}
            type="button"
          >
            {scanMutation.isPending ? "扫描中..." : "扫描"}
          </button>
        </div>
        <p className="panel__meta">
          {scanMutation.isPending
            ? "正在扫描已启用的 A 股策略..."
            : scanMutation.data
              ? `最近一次扫描发现 ${scanMutation.data.length} 条信号。`
              : ""}
          {allMeta.length} 个策略 · {activeConfigs.filter((c) => c.enabled).length} 个已启用
        </p>
      </section>

      <section className="panel panel--wide">
        <div className="panel__header">
          <h3>策略列表</h3>
          <span className="panel__meta">
            累计生成 {stats.reduce((sum, s) => sum + s.totalSignals, 0)} 条信号
          </span>
        </div>
        <div className="strategy-grid">
          {allMeta.map((meta) => (
            <StrategyCard
              key={meta.strategyId}
              meta={meta}
              config={configs.find((c) => c.strategyId === meta.strategyId)}
              stats={stats.find((s) => s.strategyId === meta.strategyId)}
              onClick={() => setEditingMeta(meta)}
            />
          ))}
        </div>
      </section>

      {scanMutation.data && scanMutation.data.length > 0 && (
        <section className="panel panel--wide">
          <div className="panel__header">
            <h3>最新扫描 · {scanMutation.data.length} 条信号</h3>
            <select
              aria-label="最新扫描风险筛选"
              className="control-select control-select--compact signals-filter-select"
              value={latestScanRiskFilter}
              onChange={(e) => { setLatestScanRiskFilter(e.target.value as RiskFilter); setLatestScanPage(1); }}
            >
              <option value="all">全部风险</option>
              <option value="approved">通过</option>
              <option value="blocked">拦截</option>
            </select>
          </div>
          <div className="table-shell">
            <table>
              <thead>
                <tr>
                  <th>代码</th>
                  <th>名称</th>
                  <th>方向</th>
                  <th>评分</th>
                  <th>入场区间</th>
                  <th>止损</th>
                  <th>风险</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {scanPageSlice.map((signal) => (
                  <Fragment key={signal.signalId}>
                    <tr key={signal.signalId}>
                      <td>
                        <b>{signal.symbol}</b>
                      </td>
                      <td>{stockName(signal.symbol, markets)}</td>
                      <td className={dirClass(signal.direction)}>{directionLabel(signal.direction)}</td>
                      <td>{signal.score.toFixed(1)}</td>
                      <td>{formatCurrency(signal.entryZoneLow)} – {formatCurrency(signal.entryZoneHigh)}</td>
                      <td>{formatCurrency(signal.stopLoss)}</td>
                      <td><span className={riskClass(signal.riskStatus)}>{riskLabel(signal.riskStatus)}</span></td>
                      <td>
                        <button
                          className="ghost-button"
                          onClick={() =>
                            setExpandedSignalId(
                              expandedSignalId === signal.signalId ? null : signal.signalId,
                            )
                          }
                          type="button"
                        >
                          {expandedSignalId === signal.signalId ? "收起" : "详情"}
                        </button>
                      </td>
                    </tr>
                    {expandedSignalId === signal.signalId && (
                      <tr key={`${signal.signalId}-detail`}>
                        <td colSpan={8}>
                          <SignalDetailPanel signal={signal} />
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
                {scanPageSlice.length === 0 && (
                  <tr><td colSpan={8} className="panel__meta table-empty-cell">没有符合筛选条件的信号。</td></tr>
                )}
              </tbody>
            </table>
          </div>
          {scanFiltered.length > LATEST_PAGE_SIZE && (
            <div className="panel__header">
              <p className="panel__meta">
                第 {latestScanPage} / {scanTotalPages} 页 · {scanFiltered.length} 条信号
              </p>
              <div className="hero-panel__actions">
                <button className="ghost-button" disabled={latestScanPage <= 1} onClick={() => setLatestScanPage((c) => c - 1)} type="button">上一页</button>
                <button className="ghost-button" disabled={latestScanPage >= scanTotalPages} onClick={() => setLatestScanPage((c) => c + 1)} type="button">下一页</button>
              </div>
            </div>
          )}
        </section>
      )}

      <section className="panel panel--wide">
        <div className="panel__header">
          <div className="segmented-control" role="tablist">
            <button
              className={`segmented-control__button${activeTab === "signals" ? " segmented-control__button--active" : ""}`}
              onClick={() => setActiveTab("signals")}
              role="tab"
              aria-selected={activeTab === "signals"}
              type="button"
            >
              信号历史
            </button>
            <button
              className={`segmented-control__button${activeTab === "scans" ? " segmented-control__button--active" : ""}`}
              onClick={() => setActiveTab("scans")}
              role="tab"
              aria-selected={activeTab === "scans"}
              type="button"
            >
              扫描记录
            </button>
          </div>
        </div>

        {activeTab === "signals" ? (
          <>
            {showEmptyHistory ? (
              <p className="panel__meta">暂无信号。运行扫描后会在这里显示结果。</p>
            ) : (
              <>
                {/* Filters */}
                <div className="panel__header">
                  <p className="panel__meta">
                    第 {signalPage?.page ?? page} / {totalPages} 页 · {total} 条信号
                    {historyFiltered.length !== items.length && `（${historyFiltered.length} 条符合筛选）`}
                  </p>
                  <div className="signals-filter-row">
                    <select
                      aria-label="风险筛选"
                      className="control-select control-select--compact signals-filter-select"
                      value={historyRiskFilter}
                      onChange={(e) => { setHistoryRiskFilter(e.target.value as RiskFilter); setPage(1); }}
                    >
                      <option value="all">全部风险</option>
                      <option value="approved">通过</option>
                      <option value="blocked">拦截</option>
                    </select>
                    <select
                      aria-label="状态筛选"
                      className="control-select control-select--compact signals-filter-select"
                      value={historyStatusFilter}
                      onChange={(e) => { setHistoryStatusFilter(e.target.value as StatusFilter); setPage(1); }}
                    >
                      <option value="all">全部状态</option>
                      <option value="Executed">已执行</option>
                      <option value="Modified">已调整</option>
                      <option value="Pending">待处理</option>
                    </select>
                  </div>
                </div>

                <div className="table-shell table-shell--visible-scrollbar signals-history-table-shell">
                  <table className="signals-history-table">
                    <thead>
                      <tr>
                        <th>代码</th>
                        <th>名称</th>
                        <th>方向</th>
                        <th>评分</th>
                        <th>入场区间</th>
                        <th>止损</th>
                        <th>风险</th>
                        <th>状态</th>
                        <th>时间</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      {historyFiltered.map((signal) => (
                        <Fragment key={signal.signalId}>
                          <tr key={signal.signalId}>
                            <td>
                              <b>{signal.symbol}</b>
                            </td>
                            <td>{stockName(signal.symbol, markets)}</td>
                            <td className={dirClass(signal.direction)}>{directionLabel(signal.direction)}</td>
                            <td>{signal.score.toFixed(1)}</td>
                            <td>{formatCurrency(signal.entryZoneLow)} – {formatCurrency(signal.entryZoneHigh)}</td>
                            <td>{formatCurrency(signal.stopLoss)}</td>
                            <td><span className={riskClass(signal.riskStatus)}>{riskLabel(signal.riskStatus)}</span></td>
                            <td><span className={riskClass(signal.riskStatus)}>{signalStatus(signal)}</span></td>
                            <td>{formatDateTime(signal.generatedAt)}</td>
                            <td>
                              <button
                                className="ghost-button"
                                onClick={() =>
                                  setExpandedSignalId(
                                    expandedSignalId === signal.signalId ? null : signal.signalId,
                                  )
                                }
                                type="button"
                              >
                                {expandedSignalId === signal.signalId ? "收起" : "详情"}
                              </button>
                            </td>
                          </tr>
                          {expandedSignalId === signal.signalId && (
                            <tr key={`${signal.signalId}-detail`}>
                              <td colSpan={10}>
                                <SignalDetailPanel signal={signal} />
                              </td>
                            </tr>
                          )}
                        </Fragment>
                      ))}
                      {showFilteredEmpty && (
                        <tr><td colSpan={10} className="panel__meta table-empty-cell">没有符合当前筛选条件的信号。</td></tr>
                      )}
                    </tbody>
                  </table>
                </div>

                <div className="panel__header">
                  <p className="panel__meta" />
                  <div className="hero-panel__actions">
                    <button
                      className="ghost-button"
                      disabled={(signalPage?.page ?? page) <= 1}
                      onClick={() => setPage((c) => Math.max(1, c - 1))}
                      type="button"
                    >
                      上一页
                    </button>
                    <button
                      className="ghost-button"
                      disabled={(signalPage?.page ?? page) >= totalPages}
                      onClick={() => setPage((c) => Math.min(totalPages, c + 1))}
                      type="button"
                    >
                      下一页
                    </button>
                  </div>
                </div>
              </>
            )}
          </>
        ) : (
          <ScanRunHistory />
        )}
      </section>

      {editingMeta && (
        <StrategyConfigModal
          meta={editingMeta}
          config={editingConfig}
          onSave={(enabled, params) =>
            updateConfigMutation.mutate({
              id: editingMeta.strategyId,
              enabled,
              params: Object.keys(params).length > 0 ? params : undefined,
            })
          }
          onClose={() => setEditingMeta(null)}
        />
      )}
    </section>
  );
}
