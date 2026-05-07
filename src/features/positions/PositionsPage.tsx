import { useQuery } from "@tanstack/react-query";
import { formatDateTime, formatPercent } from "../../lib/format";
import { getPortfolioOverview, listMarkets, listOrders, listPositions } from "../../lib/tauri";
import type { MarketRow } from "../../lib/types";

function formatCny(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: 0,
  }).format(value);
}

function formatSignedCny(value: number) {
  const sign = value > 0 ? "+" : "";
  return `${sign}${formatCny(value)}`;
}

function stockName(symbol: string, markets: MarketRow[]) {
  return markets.find((row) => row.symbol === symbol)?.baseAsset ?? fallbackStockNames[symbol] ?? "未知";
}

function isAShareSymbol(symbol: string) {
  return symbol.startsWith("SHSE.") || symbol.startsWith("SZSE.");
}

const fallbackStockNames: Record<string, string> = {
  "SHSE.600000": "浦发银行",
  "SZSE.000001": "平安银行",
  "SHSE.600519": "贵州茅台",
  "SHSE.601318": "中国平安",
  "SZSE.300750": "宁德时代",
};

export function PositionsPage() {
  const overviewQuery = useQuery({
    queryKey: ["portfolio-overview"],
    queryFn: getPortfolioOverview,
    refetchInterval: 15_000,
    staleTime: 30_000,
  });
  const positionsQuery = useQuery({
    queryKey: ["positions"],
    queryFn: listPositions,
    refetchInterval: 15_000,
    staleTime: 30_000,
  });
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const ordersQuery = useQuery({
    queryKey: ["orders"],
    queryFn: listOrders,
    refetchInterval: 15_000,
    staleTime: 15_000,
  });
  const positions = positionsQuery.data ?? [];
  const orders = (ordersQuery.data ?? []).filter((order) => isAShareSymbol(order.symbol));
  const overview = overviewQuery.data;
  const markets = marketsQuery.data ?? [];

  return (
    <section className="page-stack">
      <section className="panel panel--wide">
        <div className="panel__header">
          <div>
            <span className="section-label">资产总览</span>
            <h2>模拟账户资产</h2>
          </div>
        </div>
        <div className="metric-grid metric-grid--four">
          <article className="metric-card">
            <span className="section-label">总资产</span>
            <strong>{formatCny(overview?.totalEquity ?? 0)}</strong>
          </article>
          <article className="metric-card">
            <span className="section-label">总市值</span>
            <strong>{formatCny(overview?.totalMarketValue ?? 0)}</strong>
          </article>
          <article className="metric-card">
            <span className="section-label">总盈亏</span>
            <strong className={(overview?.totalPnl ?? 0) >= 0 ? "positive-text" : "negative-text"}>
              {formatSignedCny(overview?.totalPnl ?? 0)}
            </strong>
          </article>
          <article className="metric-card">
            <span className="section-label">当日盈亏</span>
            <strong className={(overview?.todayPnl ?? 0) >= 0 ? "positive-text" : "negative-text"}>
              {formatSignedCny(overview?.todayPnl ?? 0)} / {formatPercent(overview?.todayPnlPct ?? 0)}
            </strong>
          </article>
        </div>
      </section>

      <section className="panel panel--wide">
        <div className="panel__header">
          <div>
            <span className="section-label">持仓风险</span>
            <h2>A股模拟持仓</h2>
          </div>
        </div>
        <div className="table-shell">
          <table>
            <thead>
              <tr>
                <th>代码</th>
                <th>名称</th>
                <th>方向</th>
                <th>数量</th>
                <th>成本价</th>
                <th>最新价</th>
                <th>盈亏</th>
                <th>类型</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((position) => (
                <tr key={`${position.exchange}-${position.symbol}-${position.side}`}>
                  <td>{position.symbol}</td>
                  <td>{stockName(position.symbol, markets)}</td>
                  <td>{position.side === "Short" ? "观察" : "持有"}</td>
                  <td>{position.size}</td>
                  <td>¥{position.entry.toLocaleString("zh-CN")}</td>
                  <td>¥{position.mark.toLocaleString("zh-CN")}</td>
                  <td className={position.pnlPct >= 0 ? "positive-text" : "negative-text"}>{formatPercent(position.pnlPct)}</td>
                  <td>模拟</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="panel panel--wide">
        <div className="panel__header">
          <div>
            <span className="section-label">模拟委托</span>
            <h2>A股模拟委托</h2>
          </div>
        </div>
        <div className="table-shell">
          <table>
            <thead>
              <tr>
                <th>编号</th>
                <th>代码</th>
                <th>名称</th>
                <th>类型</th>
                <th>状态</th>
                <th>数量</th>
                <th>成交价</th>
                <th>已实现盈亏</th>
                <th>更新时间</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((order) => (
                <tr key={order.id}>
                  <td>{order.id}</td>
                  <td>{order.symbol}</td>
                  <td>{stockName(order.symbol, markets)}</td>
                  <td>{order.type}</td>
                  <td>{order.status}</td>
                  <td>{order.quantity}</td>
                  <td>{order.fillPrice !== undefined ? `¥${order.fillPrice.toLocaleString("zh-CN")}` : "无"}</td>
                  <td
                    className={
                      order.realizedPnl === undefined
                        ? undefined
                        : order.realizedPnl >= 0
                          ? "positive-text"
                          : "negative-text"
                    }
                  >
                    {order.realizedPnl !== undefined ? `¥${order.realizedPnl.toLocaleString("zh-CN")}` : "无"}
                  </td>
                  <td>{formatDateTime(order.updatedAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}
