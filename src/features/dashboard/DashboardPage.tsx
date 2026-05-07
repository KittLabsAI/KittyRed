import { Link } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { AnalyzeJobsPanel } from "../../components/jobs/AnalyzeJobsPanel";
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
      <section className="hero-panel dashboard-workbench-panel">
        <div className="dashboard-workbench-heading">
          <span className="section-label">A股工作台</span>
          <strong>模拟账户</strong>
        </div>
        <div className="dashboard-workbench-ledger">
          <dl className="dashboard-workbench-ledger__item dashboard-workbench-ledger__item--primary">
            <dt>总资产</dt>
            <dd>{formatCny(overview?.totalEquity ?? 0)}</dd>
          </dl>
          <dl className="dashboard-workbench-ledger__item">
            <dt>总市值</dt>
            <dd>{formatCny(overview?.totalMarketValue ?? 0)}</dd>
          </dl>
          <dl className="dashboard-workbench-ledger__item">
            <dt>总盈亏</dt>
            <dd className={(overview?.totalPnl ?? 0) >= 0 ? "positive-text" : "negative-text"}>
              {formatSignedCny(overview?.totalPnl ?? 0)}
            </dd>
          </dl>
          <dl className="dashboard-workbench-ledger__item">
            <dt>当日盈亏</dt>
            <dd className={(overview?.todayPnl ?? 0) >= 0 ? "positive-text" : "negative-text"}>
              {formatSignedCny(overview?.todayPnl ?? 0)} / {formatPercent(overview?.todayPnlPct ?? 0)}
            </dd>
          </dl>
        </div>
        <div className="dashboard-workbench-actions">
          <button
            className="primary-button dashboard-workbench-button"
            disabled={triggerRecommendationMutation.isPending}
            onClick={() => triggerRecommendationMutation.mutate()}
            type="button"
          >
            {triggerRecommendationMutation.isPending ? "分析中..." : "AI 分析"}
          </button>
        </div>
      </section>

      <section className="panel panel--wide">
        <div className="panel__header">
          <div>
            <span className="section-label">行情</span>
            <h2>重点 A 股</h2>
          </div>
          <Link className="ghost-button" to="/markets">查看全部</Link>
        </div>
        <div className="table-shell">
          <table>
            <thead>
              <tr>
                <th>代码</th>
                <th>名称</th>
                <th>最新价</th>
                <th>涨跌幅</th>
                <th>成交额</th>
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((row) => (
                <tr key={row.symbol}>
                  <td>{row.symbol}</td>
                  <td>{row.baseAsset}</td>
                  <td>{formatCny(row.last)}</td>
                  <td className={row.change24h >= 0 ? "positive-text" : "negative-text"}>{formatPercent(row.change24h)}</td>
                  <td>{formatCompact(row.volume24h)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <AnalyzeJobsPanel />
    </section>
  );
}
