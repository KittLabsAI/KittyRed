import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Select } from "../../components/ui/select";
import { formatCurrency, formatDateTime, formatPercent, formatStockPrice } from "../../lib/format";
import {
  createManualPaperOrder,
  getLatestRecommendation,
  getPairCandles,
  getPairDetail,
  listMarkets,
  listPaperAccounts,
  listenToMarketEvents,
  triggerRecommendation,
} from "../../lib/tauri";
import { useAppStore } from "../../store/appStore";
import { useRecommendationStore } from "../../store/recommendationStore";
import { CandlestickChart } from "./CandlestickChart";

const candleIntervals = ["1m", "5m", "15m", "30m", "1H", "1D", "1W"] as const;
const lastPairDetailSymbolKey = "kittyred:last-pair-detail-symbol";

function readLastPairDetailSymbol() {
  try {
    return window.localStorage?.getItem(lastPairDetailSymbolKey) ?? "";
  } catch {
    return "";
  }
}

function rememberPairDetailSymbol(symbol: string) {
  try {
    window.localStorage?.setItem(lastPairDetailSymbolKey, symbol);
  } catch {
    // Browser storage can be unavailable in hardened desktop contexts.
  }
}

export function PairDetailPage() {
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const querySymbol = searchParams.get("symbol") ?? "";
  const [rememberedSymbol] = useState(readLastPairDetailSymbol);
  const symbol = querySymbol || rememberedSymbol;
  const [candleInterval, setCandleInterval] = useState<(typeof candleIntervals)[number]>("1D");
  const [paperAccountId, setPaperAccountId] = useState("paper-cash");
  const [paperSide, setPaperSide] = useState<"buy" | "sell">("buy");
  const [paperQuantity, setPaperQuantity] = useState("200");
  const openAssistant = useAppStore((state) => state.openAssistant);
  const setAssistantDraft = useRecommendationStore((state) => state.setAssistantDraft);

  const pairDetailQuery = useQuery({
    queryKey: ["pair-detail", symbol, "ashare"],
    queryFn: () => getPairDetail(symbol, "ashare", "akshare"),
    enabled: symbol.length > 0,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const candlesQuery = useQuery({
    queryKey: ["pair-candles", symbol, "ashare", candleInterval],
    queryFn: () => getPairCandles(symbol, "ashare", candleInterval, "akshare"),
    enabled: symbol.length > 0,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const latestRecommendationQuery = useQuery({
    queryKey: ["latest-recommendation"],
    queryFn: getLatestRecommendation,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });
  const paperAccountsQuery = useQuery({
    queryKey: ["paper-accounts"],
    queryFn: listPaperAccounts,
    staleTime: 30_000,
  });
  const marketsQuery = useQuery({
    queryKey: ["markets"],
    queryFn: listMarkets,
    refetchInterval: 30_000,
    staleTime: 30_000,
  });

  const pairDetail = pairDetailQuery.data;
  const candles = candlesQuery.data;
  const latestRecommendations = latestRecommendationQuery.data ?? [];
  const latestRecommendation = latestRecommendations.find((item) => item.symbol === symbol);
  const symbolRecommendation = latestRecommendations
    .flatMap((item) => item.symbolRecommendations ?? [])
    .find((item) => item.symbol === symbol);
  const marketRow = marketsQuery.data?.find((row) => row.symbol === symbol);
  const referencePrice = marketRow?.last ?? pairDetail?.venues[0]?.last ?? 0;
  const paperAccounts = paperAccountsQuery.data ?? [];
  const paperAccount = paperAccounts[0];
  const availableCash = paperAccount?.availableUsdt ?? 0;
  const venueRows = useMemo(() => pairDetail?.venues ?? [], [pairDetail?.venues]);
  const bestVenue = venueRows[0];
  const quoteUpdatedAt = marketRow?.updatedAt ?? bestVenue?.updatedAt;
  const quoteVolume = marketRow?.volume24h ?? bestVenue?.volume24h;
  const latestBars = candles?.bars ?? [];
  const candleChangePct =
    marketRow?.change24h ?? bestVenue?.changePct ?? 0;

  const analyzeMutation = useMutation({
    mutationFn: () => triggerRecommendation(symbol),
    onSuccess: async (recommendations) => {
      queryClient.setQueryData(["latest-recommendation"], recommendations);
      await queryClient.invalidateQueries({ queryKey: ["recommendation-history"], refetchType: "active" });
    },
  });
  const manualOrderMutation = useMutation({
    mutationFn: () =>
      createManualPaperOrder({
        accountId: paperAccountId,
        symbol,
        marketType: "ashare",
        side: paperSide,
        quantity: Number(paperQuantity),
        entryPrice: referencePrice || undefined,
        leverage: 1,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["paper-accounts"], refetchType: "active" });
      await queryClient.invalidateQueries({ queryKey: ["analyze-jobs"], refetchType: "active" });
    },
  });

  useEffect(() => {
    if (paperAccount) {
      setPaperAccountId(paperAccount.accountId);
    }
  }, [paperAccount]);

  useEffect(() => {
    if (querySymbol) {
      rememberPairDetailSymbol(querySymbol);
    }
  }, [querySymbol]);

  useEffect(() => {
    if (!symbol) {
      return undefined;
    }

    let disposed = false;
    let cleanup: (() => void) | undefined;

    void listenToMarketEvents((event) => {
      if (event.symbol !== symbol || event.interval !== candleInterval) {
        return;
      }
      void queryClient.invalidateQueries({
        queryKey: ["pair-candles", symbol, "ashare", candleInterval],
        refetchType: "active",
      });
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
        return;
      }
      cleanup = unlisten;
    });

    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [candleInterval, queryClient, symbol]);

  function handleAskAssistant() {
    const refreshedAt = quoteUpdatedAt ?? candles?.updatedAt ?? "最新行情";
    setAssistantDraft(`请用中文解释 ${symbol} 的行情变化、交易计划和主要风险。参考数据时间：${refreshedAt}。`);
    openAssistant();
  }

  if (!symbol) {
    return (
      <section className="page-stack pair-detail-page">
        <section className="panel panel--wide empty-state-panel">
          <span className="section-label">个股详情</span>
          <h2>请先从行情页选择股票</h2>
          <p className="panel__meta">离开详情页后不会再自动重置到 SHSE.600000。</p>
        </section>
      </section>
    );
  }

  return (
    <section className="page-stack pair-detail-page">
      <Card className="pair-detail-hero overflow-hidden">
        <CardContent className="grid gap-5 p-6 lg:grid-cols-[220px_minmax(620px,1fr)_max-content] lg:items-stretch">
        <div className="pair-detail-hero__identity">
          <span className="section-label">个股详情</span>
          <h2 className="mt-2 text-[1.7rem] font-semibold leading-tight">{pairDetail?.coinInfo.name || symbol}</h2>
          <p>{symbol} · 沪深 A 股 · AKShare 行情</p>
        </div>
        <div className="pair-detail-quote-strip rounded-xl border border-white/8 bg-white/[0.025]">
          <div>
            <span>最新价</span>
            <strong>{referencePrice > 0 ? formatStockPrice(referencePrice) : "--"}</strong>
          </div>
          <div>
            <span>涨跌幅</span>
            <strong className={candleChangePct >= 0 ? "positive-text" : "negative-text"}>
              {marketRow || bestVenue ? formatPercent(candleChangePct) : "--"}
            </strong>
          </div>
          <div>
            <span>成交量</span>
            <strong>{quoteVolume !== undefined ? quoteVolume.toLocaleString("zh-CN") : "--"}</strong>
          </div>
          <div>
            <span>数据时间</span>
            <strong>{quoteUpdatedAt ? formatDateTime(quoteUpdatedAt) : "等待刷新"}</strong>
          </div>
        </div>
        <div className="pair-detail-actions lg:col-start-3 lg:justify-self-end">
          <Button className="w-full min-w-[116px]" disabled={analyzeMutation.isPending} onClick={() => analyzeMutation.mutate()} variant="ghost">
            {analyzeMutation.isPending ? "分析中..." : "AI 分析"}
          </Button>
          <Button className="w-full min-w-[116px]" onClick={handleAskAssistant} variant="ghost">
            询问助手
          </Button>
        </div>
        </CardContent>
      </Card>

      <section className="pair-detail-workspace">
        <Card className="panel panel--wide pair-detail-chart-panel overflow-hidden">
          <CardHeader className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <span className="section-label">K 线</span>
              <CardTitle className="mt-1 text-lg">{symbol}</CardTitle>
            </div>
            <Select className="control-select control-select--compact h-10 w-full lg:w-[110px]" onChange={(event) => setCandleInterval(event.target.value as (typeof candleIntervals)[number])} value={candleInterval}>
              {candleIntervals.map((interval) => (
                <option key={interval} value={interval}>{interval}</option>
              ))}
            </Select>
          </CardHeader>
          <CardContent className="pt-0">
          {candlesQuery.isFetching ? <p className="panel__meta" role="status">K 线加载中...</p> : null}
          {candlesQuery.isError ? (
            <p className="panel__meta panel__meta--danger" role="alert">
              K 线加载失败：{String(candlesQuery.error)}
            </p>
          ) : null}
          <CandlestickChart bars={latestBars} />
          </CardContent>
        </Card>

        <Card className="panel paper-trade-card overflow-hidden">
            <CardHeader className="pb-4">
              <div>
                <span className="section-label">模拟委托</span>
              </div>
            </CardHeader>
            <CardContent className="space-y-5 pt-0">
            <div className="paper-trade-summary">
              <div>
                <span>可用资金</span>
                <strong>{formatCurrency(availableCash)}</strong>
              </div>
              <div>
                <span>参考价</span>
                <strong>{referencePrice > 0 ? formatStockPrice(referencePrice) : "--"}</strong>
              </div>
            </div>
            <div className="paper-trade-grid">
              <div className="paper-side-switch" aria-label="方向">
                <Button
                  className={paperSide === "buy" ? "paper-side-switch__item paper-side-switch__item--active" : "paper-side-switch__item"}
                  onClick={() => setPaperSide("buy")}
                  size="sm"
                  variant="ghost"
                >
                  买入
                </Button>
                <Button
                  className={paperSide === "sell" ? "paper-side-switch__item paper-side-switch__item--active" : "paper-side-switch__item"}
                  onClick={() => setPaperSide("sell")}
                  size="sm"
                  variant="ghost"
                >
                  卖出
                </Button>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="paper-quantity">委托数量（股）</Label>
                <Input
                  id="paper-quantity"
                  className="control-input"
                  inputMode="decimal"
                  onChange={(event) => setPaperQuantity(event.target.value)}
                  value={paperQuantity}
                />
              </div>
            </div>
            <Button
              className="w-full"
              disabled={!paperAccount || manualOrderMutation.isPending || Number(paperQuantity) <= 0}
              onClick={() => manualOrderMutation.mutate()}
            >
              {manualOrderMutation.isPending ? "提交中..." : "生成模拟委托"}
            </Button>
            {manualOrderMutation.data ? (
              <p className="panel__meta">
                {manualOrderMutation.data.message}
              </p>
            ) : null}
            </CardContent>
        </Card>

        <Card className="panel pair-detail-advice-panel overflow-hidden">
          <CardHeader className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <CardTitle className="text-lg">最新建议</CardTitle>
              <CardDescription>优先展示该股票最近一次可用建议。</CardDescription>
            </div>
            <span className="panel__meta">{latestRecommendation?.generatedAt ? formatDateTime(latestRecommendation.generatedAt) : "暂无"}</span>
          </CardHeader>
          <CardContent className="pt-0">
          {symbolRecommendation ? (
            <div className="recommendation-card__body">
              <strong>{symbolRecommendation.direction}</strong>
              <p>{symbolRecommendation.thesis}</p>
              <p className="panel__meta">风险状态：{symbolRecommendation.riskStatus}</p>
            </div>
          ) : latestRecommendation?.symbol === symbol ? (
            <div className="recommendation-card__body">
              <strong>{latestRecommendation.direction}</strong>
              <p>{latestRecommendation.thesis}</p>
              <p className="panel__meta">风险状态：{latestRecommendation.riskStatus}</p>
            </div>
          ) : (
            <p className="panel__meta">暂无该股票的投资建议。</p>
          )}
          </CardContent>
        </Card>
      </section>
    </section>
  );
}
