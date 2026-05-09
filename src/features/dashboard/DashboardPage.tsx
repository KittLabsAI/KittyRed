import { Link } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { AnalyzeJobsPanel } from "../../components/jobs/AnalyzeJobsPanel";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableShell } from "../../components/ui/table";
import { formatCompact, formatPercent } from "../../lib/format";
import { getPortfolioOverview, listMarkets, triggerRecommendation } from "../../lib/tauri";

function formatCny(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: value >= 100 ? 0 : 2,
  }).format(value);
}

function formatSignedCny(value: number) {
  const sign = value > 0 ? "+" : "";
  return `${sign}${formatCny(value)}`;
}

function isAShareSymbol(symbol: string) {
  return symbol.startsWith("SHSE.") || symbol.startsWith("SZSE.");
}

export function DashboardPage() {
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const triggerRecommendationMutation = useMutation({
    mutationFn: () => triggerRecommendation(),
  });
  const overviewQuery = useQuery({
    queryKey: ["portfolio-overview"],
    queryFn: getPortfolioOverview,
    refetchInterval: 15_000,
    staleTime: 30_000,
  });
  const markets = (marketsQuery.data ?? []).filter((row) => isAShareSymbol(row.symbol));
  const visibleRows = markets.length > 0 ? markets.slice(0, 5) : [
    {
      symbol: "SHSE.600000",
      baseAsset: "浦发银行",
      last: 8.72,
      change24h: 0.81,
      volume24h: 1_260_000_000,
    },
    {
      symbol: "SZSE.000001",
      baseAsset: "平安银行",
      last: 11.34,
      change24h: -0.35,
      volume24h: 1_850_000_000,
    },
  ];
  const overview = overviewQuery.data;

  return (
    <section className="page-stack">
      <Card className="overflow-hidden">
        <CardContent className="dashboard-workbench-panel px-6 py-5">
          <div className="dashboard-workbench-heading min-w-0">
            <span className="section-label text-xs font-semibold uppercase tracking-[0.1em] text-accent">A股工作台</span>
            <strong className="block text-xl font-semibold">模拟账户</strong>
          </div>
          <div className="dashboard-workbench-ledger">
            <dl className="dashboard-workbench-ledger__item dashboard-workbench-ledger__item--primary rounded-none border-0 bg-transparent p-0">
              <dt className="text-xs uppercase tracking-[0.08em] text-muted-foreground">总资产</dt>
              <dd className="mt-2 text-2xl font-semibold tabular-nums">{formatCny(overview?.totalEquity ?? 0)}</dd>
            </dl>
            <dl className="dashboard-workbench-ledger__item rounded-none border-0 bg-transparent p-0">
              <dt className="text-xs uppercase tracking-[0.08em] text-muted-foreground">总市值</dt>
              <dd className="mt-2 text-2xl font-semibold tabular-nums">{formatCny(overview?.totalMarketValue ?? 0)}</dd>
            </dl>
            <dl className="dashboard-workbench-ledger__item rounded-none border-0 bg-transparent p-0">
              <dt className="text-xs uppercase tracking-[0.08em] text-muted-foreground">总盈亏</dt>
              <dd className={(overview?.totalPnl ?? 0) >= 0 ? "positive-text mt-2 text-2xl font-semibold tabular-nums" : "negative-text mt-2 text-2xl font-semibold tabular-nums"}>
                {formatSignedCny(overview?.totalPnl ?? 0)}
              </dd>
            </dl>
            <dl className="dashboard-workbench-ledger__item rounded-none border-0 bg-transparent p-0">
              <dt className="text-xs uppercase tracking-[0.08em] text-muted-foreground">当日盈亏</dt>
              <dd className={(overview?.todayPnl ?? 0) >= 0 ? "positive-text mt-2 text-2xl font-semibold tabular-nums" : "negative-text mt-2 text-2xl font-semibold tabular-nums"}>
                {formatSignedCny(overview?.todayPnl ?? 0)} / {formatPercent(overview?.todayPnlPct ?? 0)}
              </dd>
            </dl>
          </div>
          <div className="dashboard-workbench-actions">
            <Button className="dashboard-workbench-button" disabled={triggerRecommendationMutation.isPending} onClick={() => triggerRecommendationMutation.mutate()} type="button">
              {triggerRecommendationMutation.isPending ? "分析中..." : "AI 分析"}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card className="overflow-hidden">
        <CardHeader className="flex-row items-start justify-between gap-4 border-b border-border px-6 py-6">
          <div className="min-w-0">
            <span className="section-label text-xs font-semibold uppercase tracking-[0.1em] text-accent">行情</span>
            <CardTitle className="mt-2 text-left text-[1.7rem]">重点 A 股</CardTitle>
          </div>
          <Link className="ghost-button inline-flex h-12 shrink-0 items-center justify-center rounded-lg border border-border bg-white/8 px-4 text-sm text-foreground transition-colors hover:bg-white/12" to="/markets">查看全部</Link>
        </CardHeader>
        <CardContent className="px-6 py-6">
          <TableShell className="table-shell">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>代码</TableHead>
                  <TableHead>名称</TableHead>
                  <TableHead>最新价</TableHead>
                  <TableHead>涨跌幅</TableHead>
                  <TableHead>成交额</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {visibleRows.map((row) => (
                  <TableRow key={row.symbol}>
                    <TableCell>{row.symbol}</TableCell>
                    <TableCell>{row.baseAsset}</TableCell>
                    <TableCell>{formatCny(row.last)}</TableCell>
                    <TableCell className={row.change24h >= 0 ? "positive-text" : "negative-text"}>{formatPercent(row.change24h)}</TableCell>
                    <TableCell>{formatCompact(row.volume24h)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableShell>
        </CardContent>
      </Card>

      <AnalyzeJobsPanel />
    </section>
  );
}
