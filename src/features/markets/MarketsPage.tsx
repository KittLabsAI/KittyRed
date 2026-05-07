import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { formatCompact, formatPercent } from "../../lib/format";
import type { MarketRow } from "../../lib/types";
import {
  listMarkets,
  refreshWatchlistTickers,
  searchAShareSymbols,
} from "../../lib/tauri";
import { appendWatchlistSymbol } from "../../lib/settings";

function isAShare(row: MarketRow) {
  return row.symbol.startsWith("SHSE.") || row.symbol.startsWith("SZSE.");
}

function marketLabel(row: MarketRow) {
  if (row.symbol.startsWith("SHSE.")) return "沪市A股";
  if (row.symbol.startsWith("SZSE.")) return "深市A股";
  return row.marketType;
}

function formatCny(value: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: value >= 100 ? 0 : 2,
  }).format(value);
}

function messageFromError(error: unknown) {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "未知错误";
}

export function MarketsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [watchlistSearch, setWatchlistSearch] = useState("");
  const [addStatus, setAddStatus] = useState("");
  const [marketFilter, setMarketFilter] = useState("all");
  const deferredSearch = useDeferredValue(search.trim().toLowerCase());
  const deferredWatchlistSearch = useDeferredValue(watchlistSearch.trim());
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const symbolSearchQuery = useQuery({
    queryKey: ["a-share-symbol-search", deferredWatchlistSearch],
    queryFn: () => searchAShareSymbols(deferredWatchlistSearch),
    enabled: deferredWatchlistSearch.length > 0,
    staleTime: 60_000,
  });

  const sourceRows = useMemo(() => {
    return (marketsQuery.data ?? []).filter(isAShare);
  }, [marketsQuery.data]);

  const rows = useMemo(() => {
    return sourceRows.filter((row) => {
      const name = row.baseAsset.toLowerCase();
      const symbol = row.symbol.toLowerCase();
      const matchesSearch =
        !deferredSearch ||
        name.includes(deferredSearch) ||
        symbol.includes(deferredSearch);
      const matchesMarket =
        marketFilter === "all" ||
        (marketFilter === "shse" && row.symbol.startsWith("SHSE.")) ||
        (marketFilter === "szse" && row.symbol.startsWith("SZSE."));
      return matchesSearch && matchesMarket;
    });
  }, [deferredSearch, marketFilter, sourceRows]);

  useEffect(() => {
    if (marketFilter !== "all" && !["shse", "szse"].includes(marketFilter)) {
      setMarketFilter("all");
    }
  }, [marketFilter]);

  useEffect(() => {
    let cancelled = false;
    void refreshWatchlistTickers()
      .then(() => {
        if (!cancelled) {
          void queryClient.invalidateQueries({ queryKey: ["markets"], refetchType: "active" });
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [queryClient]);

  async function handleAddWatchlistSymbol(symbol: string) {
    setAddStatus(`正在添加 ${symbol}...`);
    await appendWatchlistSymbol(symbol);
    await refreshWatchlistTickers();
    await queryClient.invalidateQueries({ queryKey: ["markets"], refetchType: "active" });
    setAddStatus(`已添加 ${symbol}，自选行情已刷新。`);
  }

  return (
    <section className="page-stack">
      <section className="panel watchlist-add-card">
        <div className="panel__header">
          <div>
            <span className="section-label">自选股票池</span>
            <h3>添加 A 股标的</h3>
          </div>
          <span className="panel__meta">添加后立即刷新自选行情</span>
        </div>
        <label className="search-shell search-shell--watchlist">
          <span className="sr-only">搜索并添加自选股票</span>
          <input
            aria-label="搜索并添加自选股票"
            onChange={(event) => setWatchlistSearch(event.target.value)}
            placeholder="输入股票名称或代码，例如 茅台、600519"
            value={watchlistSearch}
          />
        </label>
        {symbolSearchQuery.isFetching ? (
          <p className="panel__meta" role="status">正在匹配 A 股股票...</p>
        ) : null}
        {symbolSearchQuery.isError ? (
          <p className="panel__meta panel__meta--danger" role="alert">
            股票搜索失败：{messageFromError(symbolSearchQuery.error)}
          </p>
        ) : null}
        {symbolSearchQuery.data?.length ? (
          <div className="watchlist-search-results">
            {symbolSearchQuery.data.map((item) => (
              <button
                aria-label={`添加 ${item.symbol} ${item.name}`}
                className="watchlist-search-result"
                key={item.symbol}
                onClick={() => void handleAddWatchlistSymbol(item.symbol)}
                type="button"
              >
                <strong>{item.symbol}</strong>
                <span>{item.name}</span>
                <small>{item.market}</small>
              </button>
            ))}
          </div>
        ) : null}
        {deferredWatchlistSearch && !symbolSearchQuery.isFetching && symbolSearchQuery.data?.length === 0 ? (
          <p className="panel__meta" role="status">未找到匹配股票</p>
        ) : null}
        {addStatus ? <p className="panel__meta" role="status">{addStatus}</p> : null}
      </section>

      <section className="panel panel--wide">
        <div className="panel__header panel__header--markets">
          <div className="panel__header-copy">
            <span className="section-label">行情</span>
            <h2>A股行情</h2>
          </div>
          <div className="panel__header-controls panel__header-controls--markets">
            <label className="search-shell search-shell--markets">
              <span className="sr-only">搜索股票</span>
              <input
                aria-label="搜索股票"
                onChange={(event) => setSearch(event.target.value)}
                placeholder="搜索代码或名称，例如 600000、平安银行"
                value={search}
              />
            </label>
            <label className="search-shell">
              <span className="sr-only">市场筛选</span>
              <select
                aria-label="市场筛选"
                className="control-select"
                onChange={(event) => setMarketFilter(event.target.value)}
                value={marketFilter}
              >
                <option value="all">全部市场</option>
                <option value="shse">沪市A股</option>
                <option value="szse">深市A股</option>
              </select>
            </label>
          </div>
        </div>
        <p className="panel__meta">
          {marketsQuery.isFetching ? "正在刷新 AKShare 行情..." : "展示沪深 A 股行情，非 A 股标的不会出现在列表中。"}
        </p>

        <div className="table-shell">
          <table>
            <thead>
              <tr>
                <th>代码</th>
                <th>名称</th>
                <th>市场</th>
                <th>最新价</th>
                <th>涨跌幅</th>
                <th>成交额</th>
                <th>更新时间</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  className="market-row"
                  key={row.symbol}
                  onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}
                >
                  <td>{row.symbol}</td>
                  <td>{row.baseAsset}</td>
                  <td>{marketLabel(row)}</td>
                  <td>{formatCny(row.last)}</td>
                  <td className={row.change24h >= 0 ? "positive-text" : "negative-text"}>
                    {formatPercent(row.change24h)}
                  </td>
                  <td>{formatCompact(row.volume24h)}</td>
                  <td>{new Date(row.updatedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}
