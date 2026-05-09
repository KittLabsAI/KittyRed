import { Fragment, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { Select } from "../../components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableShell } from "../../components/ui/table";
import { cn } from "../../lib/utils";
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

function riskBadgeClass(status: string) {
  if (status === "approved") {
    return "bg-[color:var(--signal-success-bg)] text-[color:var(--signal-success-text)]";
  }
  if (status === "blocked") {
    return "bg-[color:var(--signal-danger-bg)] text-[color:var(--signal-danger-text)]";
  }
  return "bg-[color:var(--signal-neutral-bg)] text-[color:var(--signal-neutral-text)]";
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
      <Card className="panel panel--wide overflow-hidden">
        <CardHeader className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="space-y-2">
            <span className="section-label">信号扫描</span>
            <CardTitle>策略信号</CardTitle>
            <CardDescription className="max-w-3xl leading-6">
              {scanMutation.isPending
                ? "正在扫描已启用的 A 股策略..."
                : scanMutation.data
                  ? `最近一次扫描发现 ${scanMutation.data.length} 条信号。`
                  : "策略结果、执行状态和最近扫描都会在这里集中查看。"}
            </CardDescription>
          </div>
          <Button
            className="w-full lg:w-auto"
            disabled={scanMutation.isPending}
            onClick={() => scanMutation.mutate()}
          >
            {scanMutation.isPending ? "扫描中..." : "扫描"}
          </Button>
        </CardHeader>
        <CardContent className="pt-0">
          <p className="panel__meta">
            {allMeta.length} 个策略 · {activeConfigs.filter((c) => c.enabled).length} 个已启用
          </p>
        </CardContent>
      </Card>

      <Card className="panel panel--wide overflow-hidden">
        <CardHeader className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg">策略列表</CardTitle>
            <CardDescription>
              累计生成 {stats.reduce((sum, s) => sum + s.totalSignals, 0)} 条信号
            </CardDescription>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
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
        </CardContent>
      </Card>

      {scanMutation.data && scanMutation.data.length > 0 && (
        <Card className="panel panel--wide overflow-hidden">
          <CardHeader className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <CardTitle className="text-lg">最新扫描 · {scanMutation.data.length} 条信号</CardTitle>
            <Select
              aria-label="最新扫描风险筛选"
              className="control-select control-select--compact signals-filter-select h-10 w-full lg:w-[140px]"
              value={latestScanRiskFilter}
              onChange={(e) => { setLatestScanRiskFilter(e.target.value as RiskFilter); setLatestScanPage(1); }}
            >
              <option value="all">全部风险</option>
              <option value="approved">通过</option>
              <option value="blocked">拦截</option>
            </Select>
          </CardHeader>
          <CardContent className="pt-0">
            <TableShell className="table-shell rounded-xl border border-white/6 bg-black/10 p-4">
              <Table>
                <TableHeader>
                  <TableRow className="border-t-0">
                    <TableHead>代码</TableHead>
                    <TableHead>名称</TableHead>
                    <TableHead>方向</TableHead>
                    <TableHead>评分</TableHead>
                    <TableHead>入场区间</TableHead>
                    <TableHead>止损</TableHead>
                    <TableHead>风险</TableHead>
                    <TableHead className="w-[92px]" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                {scanPageSlice.map((signal) => (
                  <Fragment key={signal.signalId}>
                    <TableRow key={signal.signalId}>
                      <TableCell>
                        <b>{signal.symbol}</b>
                      </TableCell>
                      <TableCell>{stockName(signal.symbol, markets)}</TableCell>
                      <TableCell className={dirClass(signal.direction)}>{directionLabel(signal.direction)}</TableCell>
                      <TableCell>{signal.score.toFixed(1)}</TableCell>
                      <TableCell>{formatCurrency(signal.entryZoneLow)} – {formatCurrency(signal.entryZoneHigh)}</TableCell>
                      <TableCell>{formatCurrency(signal.stopLoss)}</TableCell>
                      <TableCell>
                        <span className={cn("inline-flex rounded-full px-2.5 py-1 text-xs font-medium", riskBadgeClass(signal.riskStatus))}>
                          {riskLabel(signal.riskStatus)}
                        </span>
                      </TableCell>
                      <TableCell>
                        <Button
                          className="w-full"
                          size="sm"
                          variant="ghost"
                          onClick={() =>
                            setExpandedSignalId(
                              expandedSignalId === signal.signalId ? null : signal.signalId,
                            )
                          }
                          type="button"
                        >
                          {expandedSignalId === signal.signalId ? "收起" : "详情"}
                        </Button>
                      </TableCell>
                    </TableRow>
                    {expandedSignalId === signal.signalId && (
                      <TableRow key={`${signal.signalId}-detail`}>
                        <TableCell colSpan={8} className="py-3">
                          <SignalDetailPanel signal={signal} />
                        </TableCell>
                      </TableRow>
                    )}
                  </Fragment>
                ))}
                {scanPageSlice.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={8} className="table-empty-cell py-6 text-center text-sm text-muted-foreground">
                      没有符合筛选条件的信号。
                    </TableCell>
                  </TableRow>
                )}
                </TableBody>
              </Table>
            </TableShell>
          {scanFiltered.length > LATEST_PAGE_SIZE && (
            <div className="mt-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <p className="panel__meta">
                第 {latestScanPage} / {scanTotalPages} 页 · {scanFiltered.length} 条信号
              </p>
              <div className="hero-panel__actions flex gap-3">
                <Button disabled={latestScanPage <= 1} onClick={() => setLatestScanPage((c) => c - 1)} size="sm" variant="ghost">上一页</Button>
                <Button disabled={latestScanPage >= scanTotalPages} onClick={() => setLatestScanPage((c) => c + 1)} size="sm" variant="ghost">下一页</Button>
              </div>
            </div>
          )}
          </CardContent>
        </Card>
      )}

      <Card className="panel panel--wide overflow-hidden">
        <CardHeader className="pb-4">
          <div className="segmented-control" role="tablist">
            <Button
              className={`segmented-control__button${activeTab === "signals" ? " segmented-control__button--active" : ""}`}
              onClick={() => setActiveTab("signals")}
              role="tab"
              aria-selected={activeTab === "signals"}
              size="sm"
              variant="ghost"
            >
              信号历史
            </Button>
            <Button
              className={`segmented-control__button${activeTab === "scans" ? " segmented-control__button--active" : ""}`}
              onClick={() => setActiveTab("scans")}
              role="tab"
              aria-selected={activeTab === "scans"}
              size="sm"
              variant="ghost"
            >
              扫描记录
            </Button>
          </div>
        </CardHeader>

        {activeTab === "signals" ? (
          <CardContent className="pt-0">
            {showEmptyHistory ? (
              <p className="panel__meta">暂无信号。运行扫描后会在这里显示结果。</p>
            ) : (
              <>
                <div className="flex flex-col gap-3 pb-4 lg:flex-row lg:items-center lg:justify-between">
                  <p className="panel__meta">
                    第 {signalPage?.page ?? page} / {totalPages} 页 · {total} 条信号
                    {historyFiltered.length !== items.length && `（${historyFiltered.length} 条符合筛选）`}
                  </p>
                  <div className="signals-filter-row">
                    <Select
                      aria-label="风险筛选"
                      className="control-select control-select--compact signals-filter-select h-10"
                      value={historyRiskFilter}
                      onChange={(e) => { setHistoryRiskFilter(e.target.value as RiskFilter); setPage(1); }}
                    >
                      <option value="all">全部风险</option>
                      <option value="approved">通过</option>
                      <option value="blocked">拦截</option>
                    </Select>
                    <Select
                      aria-label="状态筛选"
                      className="control-select control-select--compact signals-filter-select h-10"
                      value={historyStatusFilter}
                      onChange={(e) => { setHistoryStatusFilter(e.target.value as StatusFilter); setPage(1); }}
                    >
                      <option value="all">全部状态</option>
                      <option value="Executed">已执行</option>
                      <option value="Modified">已调整</option>
                      <option value="Pending">待处理</option>
                    </Select>
                  </div>
                </div>

                <TableShell className="table-shell table-shell--visible-scrollbar signals-history-table-shell rounded-xl border border-white/6 bg-black/10 p-4">
                  <Table className="signals-history-table">
                    <TableHeader>
                      <TableRow className="border-t-0">
                        <TableHead>代码</TableHead>
                        <TableHead>名称</TableHead>
                        <TableHead>方向</TableHead>
                        <TableHead>评分</TableHead>
                        <TableHead>入场区间</TableHead>
                        <TableHead>止损</TableHead>
                        <TableHead>风险</TableHead>
                        <TableHead>状态</TableHead>
                        <TableHead>时间</TableHead>
                        <TableHead className="w-[92px]" />
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {historyFiltered.map((signal) => (
                        <Fragment key={signal.signalId}>
                          <TableRow key={signal.signalId}>
                            <TableCell>
                              <b>{signal.symbol}</b>
                            </TableCell>
                            <TableCell>{stockName(signal.symbol, markets)}</TableCell>
                            <TableCell className={dirClass(signal.direction)}>{directionLabel(signal.direction)}</TableCell>
                            <TableCell>{signal.score.toFixed(1)}</TableCell>
                            <TableCell>{formatCurrency(signal.entryZoneLow)} – {formatCurrency(signal.entryZoneHigh)}</TableCell>
                            <TableCell>{formatCurrency(signal.stopLoss)}</TableCell>
                            <TableCell>
                              <span className={cn("inline-flex rounded-full px-2.5 py-1 text-xs font-medium", riskBadgeClass(signal.riskStatus))}>
                                {riskLabel(signal.riskStatus)}
                              </span>
                            </TableCell>
                            <TableCell>
                              <span className={cn("inline-flex rounded-full px-2.5 py-1 text-xs font-medium", riskBadgeClass(signal.riskStatus))}>
                                {signalStatus(signal)}
                              </span>
                            </TableCell>
                            <TableCell>{formatDateTime(signal.generatedAt)}</TableCell>
                            <TableCell>
                              <Button
                                className="w-full"
                                size="sm"
                                variant="ghost"
                                onClick={() =>
                                  setExpandedSignalId(
                                    expandedSignalId === signal.signalId ? null : signal.signalId,
                                  )
                                }
                                type="button"
                              >
                                {expandedSignalId === signal.signalId ? "收起" : "详情"}
                              </Button>
                            </TableCell>
                          </TableRow>
                          {expandedSignalId === signal.signalId && (
                            <TableRow key={`${signal.signalId}-detail`}>
                              <TableCell colSpan={10} className="py-3">
                                <SignalDetailPanel signal={signal} />
                              </TableCell>
                            </TableRow>
                          )}
                        </Fragment>
                      ))}
                      {showFilteredEmpty && (
                        <TableRow>
                          <TableCell colSpan={10} className="table-empty-cell py-6 text-center text-sm text-muted-foreground">
                            没有符合当前筛选条件的信号。
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </TableShell>

                <div className="mt-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                  <p className="panel__meta" />
                  <div className="hero-panel__actions flex gap-3">
                    <Button
                      disabled={(signalPage?.page ?? page) <= 1}
                      onClick={() => setPage((c) => Math.max(1, c - 1))}
                      size="sm"
                      variant="ghost"
                    >
                      上一页
                    </Button>
                    <Button
                      disabled={(signalPage?.page ?? page) >= totalPages}
                      onClick={() => setPage((c) => Math.min(totalPages, c + 1))}
                      size="sm"
                      variant="ghost"
                    >
                      下一页
                    </Button>
                  </div>
                </div>
              </>
            )}
          </CardContent>
        ) : (
          <CardContent className="pt-0">
            <ScanRunHistory />
          </CardContent>
        )}
      </Card>

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
