import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { formatCurrency, formatDateTime, formatPercent } from "../../lib/format";
import {
  createManualPaperOrder,
  getLatestRecommendation,
  getPairCandles,
  getPairDetail,
  listMarkets,
  listPaperAccounts,
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
      <section className="pair-detail-hero">
        <div className="pair-detail-hero__identity">
          <span className="section-label">个股详情</span>
          <h2>{pairDetail?.coinInfo.name || symbol}</h2>
          <p>{symbol} · 沪深 A 股 · AKShare 行情</p>
        </div>
        <div className="pair-detail-quote-strip">
          <div>
            <span>最新价</span>
            <strong>{referencePrice > 0 ? formatCurrency(referencePrice) : "--"}</strong>
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
        <div className="pair-detail-actions">
          <button className="ghost-button" disabled={analyzeMutation.isPending} onClick={() => analyzeMutation.mutate()} type="button">
            {analyzeMutation.isPending ? "分析中..." : "AI 分析"}
          </button>
          <button className="ghost-button" onClick={handleAskAssistant} type="button">
            询问助手
          </button>
        </div>
      </section>

      <section className="pair-detail-workspace">
        <section className="panel panel--wide pair-detail-chart-panel">
          <div className="panel__header">
            <div>
              <span className="section-label">K 线</span>
              <h3>{symbol}</h3>
            </div>
            <select className="control-select control-select--compact" onChange={(event) => setCandleInterval(event.target.value as (typeof candleIntervals)[number])} value={candleInterval}>
              {candleIntervals.map((interval) => (
                <option key={interval} value={interval}>{interval}</option>
              ))}
            </select>
          </div>
          {candlesQuery.isFetching ? <p className="panel__meta" role="status">K 线加载中...</p> : null}
          {candlesQuery.isError ? (
            <p className="panel__meta panel__meta--danger" role="alert">
              K 线加载失败：{String(candlesQuery.error)}
            </p>
          ) : null}
          <CandlestickChart bars={latestBars} />
        </section>

        <section className="panel paper-trade-card">
            <div className="panel__header">
              <div>
                <span className="section-label">模拟委托</span>
              </div>
            </div>
            <div className="paper-trade-summary">
              <div>
                <span>可用资金</span>
                <strong>{formatCurrency(paperAccount?.availableUsdt ?? 0)}</strong>
              </div>
              <div>
                <span>参考价</span>
                <strong>{referencePrice > 0 ? formatCurrency(referencePrice) : "--"}</strong>
              </div>
            </div>
            <div className="paper-trade-grid">
              <div className="paper-side-switch" aria-label="方向">
                <button
                  className={paperSide === "buy" ? "paper-side-switch__item paper-side-switch__item--active" : "paper-side-switch__item"}
                  onClick={() => setPaperSide("buy")}
                  type="button"
                >
                  买入
                </button>
                <button
                  className={paperSide === "sell" ? "paper-side-switch__item paper-side-switch__item--active" : "paper-side-switch__item"}
                  onClick={() => setPaperSide("sell")}
                  type="button"
                >
                  卖出
                </button>
              </div>
              <label>
                <span>委托数量（股）</span>
                <input className="control-input" inputMode="decimal" onChange={(event) => setPaperQuantity(event.target.value)} value={paperQuantity} />
              </label>
            </div>
            <button
              className="primary-button"
              disabled={!paperAccount || manualOrderMutation.isPending || Number(paperQuantity) <= 0}
              onClick={() => manualOrderMutation.mutate()}
              type="button"
            >
              {manualOrderMutation.isPending ? "提交中..." : "生成模拟委托"}
            </button>
            {manualOrderMutation.data ? (
              <p className="panel__meta">
                {manualOrderMutation.data.message}
              </p>
            ) : null}
        </section>

        <article className="panel pair-detail-advice-panel">
          <div className="panel__header">
            <h3>最新建议</h3>
            <span className="panel__meta">{latestRecommendation?.generatedAt ? formatDateTime(latestRecommendation.generatedAt) : "暂无"}</span>
          </div>
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
        </article>
      </section>
    </section>
  );
}
