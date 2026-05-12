import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Select } from "../../components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableShell } from "../../components/ui/table";
import { formatCompact, formatPercent, formatStockPrice } from "../../lib/format";
import type { MarketRow } from "../../lib/types";
import {
  listMarkets,
  refreshWatchlistTickers,
  searchAShareSymbols,
} from "../../lib/tauri";
import { appendWatchlistSymbol, removeWatchlistSymbol } from "../../lib/settings";

function isAShare(row: MarketRow) {
  return row.symbol.startsWith("SHSE.") || row.symbol.startsWith("SZSE.");
}

function marketLabel(row: MarketRow) {
  if (row.symbol.startsWith("SHSE.")) return "沪市A股";
  if (row.symbol.startsWith("SZSE.")) return "深市A股";
  return row.marketType;
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

  async function handleRemoveWatchlistSymbol(symbol: string) {
    await removeWatchlistSymbol(symbol);
    await refreshWatchlistTickers();
    await queryClient.invalidateQueries({ queryKey: ["markets"], refetchType: "active" });
  }

  return (
    <section className="page-stack">
      <Card className="watchlist-add-card overflow-hidden">
        <CardHeader className="border-b border-border px-6 py-6">
          <div>
            <span className="section-label text-xs font-semibold uppercase tracking-[0.1em] text-accent">自选股票池</span>
            <CardTitle className="mt-2 text-[1.3rem]">添加 A 股标的</CardTitle>
          </div>
          <span className="panel__meta text-sm text-muted-foreground">添加后立即刷新自选行情</span>
        </CardHeader>
        <CardContent className="grid gap-4 px-6 py-6">
          <span className="sr-only">搜索并添加自选股票</span>
          <Input
            aria-label="搜索并添加自选股票"
            onChange={(event) => setWatchlistSearch(event.target.value)}
            placeholder="输入股票名称或代码，例如 茅台、600519"
            value={watchlistSearch}
          />
          {symbolSearchQuery.isFetching ? (
            <p className="panel__meta" role="status">正在匹配 A 股股票...</p>
          ) : null}
          {symbolSearchQuery.isError ? (
            <p className="panel__meta panel__meta--danger text-[color:var(--signal-danger-text)]" role="alert">
              股票搜索失败：{messageFromError(symbolSearchQuery.error)}
            </p>
          ) : null}
          {symbolSearchQuery.data?.length ? (
            <div className="watchlist-search-results">
              {symbolSearchQuery.data.map((item) => (
                <button
                  aria-label={`添加 ${item.symbol} ${item.name}`}
                  className="watchlist-search-result grid w-full gap-1 rounded-xl border border-border bg-white/4 px-4 py-3 text-left transition-colors hover:bg-white/8"
                  key={item.symbol}
                  onClick={() => void handleAddWatchlistSymbol(item.symbol)}
                  type="button"
                >
                  <strong>{item.symbol}</strong>
                  <span className="text-sm text-foreground/90">{item.name}</span>
                  <small className="text-xs text-muted-foreground">{item.market}</small>
                </button>
              ))}
            </div>
          ) : null}
          {deferredWatchlistSearch && !symbolSearchQuery.isFetching && symbolSearchQuery.data?.length === 0 ? (
            <p className="panel__meta" role="status">未找到匹配股票</p>
          ) : null}
          {addStatus ? <p className="panel__meta" role="status">{addStatus}</p> : null}
        </CardContent>
      </Card>

      <Card className="overflow-hidden">
        <CardHeader className="flex flex-col items-stretch justify-between gap-4 border-b border-border px-6 py-6 lg:flex-row lg:items-end">
          <div className="panel__header-copy">
            <span className="section-label text-xs font-semibold uppercase tracking-[0.1em] text-accent">行情</span>
            <CardTitle className="mt-2 text-[1.7rem]">A股行情</CardTitle>
          </div>
          <div className="panel__header-controls panel__header-controls--markets flex flex-col gap-2 md:flex-row md:items-center">
            <label className="search-shell search-shell--markets block">
              <span className="sr-only">搜索股票</span>
              <Input
                aria-label="搜索股票"
                onChange={(event) => setSearch(event.target.value)}
                placeholder="搜索代码或名称，例如 600000、平安银行"
                value={search}
              />
            </label>
            <label className="search-shell">
              <span className="sr-only">市场筛选</span>
              <Select
                aria-label="市场筛选"
                className="control-select"
                onChange={(event) => setMarketFilter(event.target.value)}
                value={marketFilter}
              >
                <option value="all">全部市场</option>
                <option value="shse">沪市A股</option>
                <option value="szse">深市A股</option>
              </Select>
            </label>
          </div>
        </CardHeader>
        <CardContent className="px-6 py-6">
          <p className="panel__meta">
            {marketsQuery.isFetching ? "正在刷新 AKShare 行情..." : "展示沪深 A 股行情，非 A 股标的不会出现在列表中。"}
          </p>
          <TableShell className="table-shell">
            <Table>
              <TableHeader>
                <TableRow className="border-t-0">
                  <TableHead className="w-[100px]">代码</TableHead>
                  <TableHead>名称</TableHead>
                  <TableHead>市场</TableHead>
                  <TableHead>最新价</TableHead>
                  <TableHead>涨跌幅</TableHead>
                  <TableHead>成交额</TableHead>
                  <TableHead>更新时间</TableHead>
                  <TableHead className="w-[60px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((row) => (
                  <TableRow
                  className="market-row"
                  key={row.symbol}
                >
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{row.symbol}</TableCell>
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{row.baseAsset}</TableCell>
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{marketLabel(row)}</TableCell>
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{formatStockPrice(row.last)}</TableCell>
                  <TableCell className={`cursor-pointer ${row.change24h >= 0 ? "positive-text" : "negative-text"}`} onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>
                    {formatPercent(row.change24h)}
                  </TableCell>
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{formatCompact(row.volume24h)}</TableCell>
                  <TableCell className="cursor-pointer" onClick={() => navigate(`/pair-detail?symbol=${encodeURIComponent(row.symbol)}`)}>{new Date(row.updatedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}</TableCell>
                  <TableCell>
                    <button
                      aria-label={`删除 ${row.symbol}`}
                      className="ghost-button table-action-button"
                      onClick={(event) => { event.stopPropagation(); handleRemoveWatchlistSymbol(row.symbol); }}
                      type="button"
                    >
                      删除
                    </button>
                  </TableCell>
                </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableShell>
        </CardContent>
      </Card>
    </section>
  );
}
