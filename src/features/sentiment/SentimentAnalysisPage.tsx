import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as echarts from "echarts";
import { Check, Circle, CircleAlert, RefreshCw, Square, Wand2, X } from "lucide-react";
import { WatchlistSelectionModal } from "../../components/WatchlistSelectionModal";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import {
  cancelSentimentDiscussionFetch,
  getSentimentAnalysisProgress,
  getSentimentAnalysisResults,
  getSentimentDiscussionSnapshot,
  getSentimentFetchProgress,
  listMarkets,
  startSentimentAnalysis,
  startSentimentDiscussionFetch,
} from "../../lib/tauri";
import type {
  SentimentAnalysisProgressItem,
  SentimentAnalysisResult,
  SentimentDimensionScore,
  SentimentFetchProgressItem,
  SentimentPlatformFetchStatus,
} from "../../lib/types";

const PLATFORM_LABELS: Record<string, string> = {
  weibo: "微博",
  xiaohongshu: "小红书",
  bilibili: "B站",
  zhihu: "知乎",
  douyin: "抖音",
  wechat: "微信公众号",
  baidu: "百度",
  toutiao: "今日头条",
  xueqiu: "雪球",
};

const DIMENSION_ITEMS: Array<{ key: keyof Pick<SentimentAnalysisResult, "sentiment" | "attention" | "momentum" | "impact" | "reliability" | "consensus">; label: string }> = [
  { key: "sentiment", label: "情感倾向" },
  { key: "attention", label: "关注热度" },
  { key: "momentum", label: "传播动能" },
  { key: "impact", label: "信息影响力" },
  { key: "reliability", label: "来源可靠性" },
  { key: "consensus", label: "舆论共识度" },
];

const DIMENSION_SCORE_STANDARDS: Record<string, string> = {
  sentiment: "50 分为中性，>50 偏正面，<50 偏负面；整体讨论越乐观得分越高",
  attention: "讨论量越多、跨平台覆盖越广、互动越高，关注热度得分越高",
  momentum: "近期集中出现、多平台同步扩散、互动增长明显时传播动能得分越高",
  impact: "涉及业绩、并购、监管、政策、重大订单等高影响事件时得分越高",
  reliability: "官方、权威媒体、认证分析师、含数据和原始链接的来源占比越高得分越高",
  consensus: "观点越一致得分越高，多空激烈冲突且证据分散时得分越低",
};

type SelectionMode = "fetch" | "analysis";

function progressPercent(completed: number, total: number) {
  if (total <= 0) return 0;
  return Math.min(100, Math.round((completed / total) * 100));
}

export function SentimentAnalysisPage() {
  const queryClient = useQueryClient();
  const [selectionMode, setSelectionMode] = useState<SelectionMode | null>(null);
  const [activeResult, setActiveResult] = useState<SentimentAnalysisResult | null>(null);
  const [analysisEligibleSymbols, setAnalysisEligibleSymbols] = useState<Set<string> | null>(null);

  const watchlistQuery = useQuery({
    queryKey: ["sentiment-watchlist"],
    queryFn: listMarkets,
  });
  const fetchProgressQuery = useQuery({
    queryKey: ["sentiment-fetch-progress"],
    queryFn: getSentimentFetchProgress,
    refetchInterval: (query) => (query.state.data?.status === "running" ? 1200 : false),
  });
  const analysisProgressQuery = useQuery({
    queryKey: ["sentiment-analysis-progress"],
    queryFn: getSentimentAnalysisProgress,
    refetchInterval: (query) => (query.state.data?.status === "running" ? 1200 : false),
  });
  const resultsQuery = useQuery({
    queryKey: ["sentiment-analysis-results"],
    queryFn: getSentimentAnalysisResults,
  });

  const fetchMutation = useMutation({
    mutationFn: (selectedSymbols: string[]) => startSentimentDiscussionFetch(selectedSymbols),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["sentiment-fetch-progress"] });
    },
  });
  const cancelMutation = useMutation({
    mutationFn: cancelSentimentDiscussionFetch,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["sentiment-fetch-progress"] });
    },
  });
  const analyzeMutation = useMutation({
    mutationFn: (selectedSymbols: string[]) => startSentimentAnalysis(selectedSymbols),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["sentiment-analysis-progress"] });
      await queryClient.invalidateQueries({ queryKey: ["sentiment-analysis-results"] });
    },
  });

  const fetchProgress = fetchProgressQuery.data;
  const analysisProgress = analysisProgressQuery.data;
  const fetchPercent = progressPercent(fetchProgress?.completedCount ?? 0, fetchProgress?.totalCount ?? 0);
  const analysisPercent = progressPercent(analysisProgress?.completedCount ?? 0, analysisProgress?.totalCount ?? 0);
  const isFetching = fetchProgress?.status === "running";
  const watchlistRows = watchlistQuery.data ?? [];
  const watchlistNameBySymbol = useMemo(
    () => new Map(watchlistRows.map((row) => [row.symbol, row.baseAsset])),
    [watchlistRows],
  );
  const analysisWatchlistRows = useMemo(
    () => (
      analysisEligibleSymbols
        ? watchlistRows.filter((row) => analysisEligibleSymbols.has(row.symbol))
        : watchlistRows
    ),
    [analysisEligibleSymbols, watchlistRows],
  );
  const results = useMemo(
    () => (resultsQuery.data ?? []).slice().sort((left, right) => right.totalScore - left.totalScore),
    [resultsQuery.data],
  );
  const selectionCopy = useMemo(() => {
    if (selectionMode === "analysis") {
      return {
        title: "选择参与 AI 舆情分析的股票",
        description: "从当前自选股池中勾选要进入本次 AI 舆情分析的股票。请先完成社媒平台讨论拉取。",
        confirmLabel: "开始分析",
      };
    }
    return {
      title: "选择拉取社媒平台讨论的股票",
      description: "从当前自选股池中勾选要拉取社媒平台讨论的股票。",
      confirmLabel: "开始拉取",
    };
  }, [selectionMode]);

  async function openAnalysisSelection() {
    const entries = await Promise.all(
      watchlistRows.map(async (row) => ({
        symbol: row.symbol,
        snapshot: await getSentimentDiscussionSnapshot(row.symbol),
      })),
    );
    setAnalysisEligibleSymbols(
      new Set(
        entries
          .filter((entry) => (entry.snapshot?.items.length ?? 0) > 0)
          .map((entry) => entry.symbol),
      ),
    );
    setSelectionMode("analysis");
  }

  useEffect(() => {
    if (analysisProgress?.status !== "running") return;
    const timer = window.setInterval(() => {
      void queryClient.invalidateQueries({ queryKey: ["sentiment-analysis-results"] });
    }, 1200);
    return () => window.clearInterval(timer);
  }, [analysisProgress?.status, queryClient]);

  useEffect(() => {
    if (analysisProgress?.status === "completed") {
      void queryClient.invalidateQueries({ queryKey: ["sentiment-analysis-results"] });
    }
  }, [analysisProgress?.status, queryClient]);

  useEffect(() => {
    if (!activeResult) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setActiveResult(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeResult]);

  return (
    <section className="page financial-page sentiment-page">
      <div className="page__header">
        <div>
          <p className="eyebrow">舆情分析</p>
        </div>
      </div>

      <Card className="panel financial-workbench">
        <CardContent className="grid gap-5 px-6 py-6">
          <div className="financial-toolbar">
            <div className="financial-scope-copy">
              <span>分析范围</span>
              <strong>自选股票池社媒平台讨论</strong>
            </div>
            <div className="financial-actions">
              <Button
                disabled={isFetching || fetchMutation.isPending}
                onClick={() => setSelectionMode("fetch")}
                type="button"
              >
                <RefreshCw size={16} />
                拉取社媒平台讨论
              </Button>
              <Button
                disabled={!isFetching || cancelMutation.isPending}
                onClick={() => cancelMutation.mutate()}
                type="button"
                variant="ghost"
              >
                <Square size={15} />
                中断拉取
              </Button>
              <Button
                disabled={analyzeMutation.isPending}
                onClick={() => void openAnalysisSelection()}
                type="button"
                variant="ghost"
              >
                <Wand2 size={16} />
                AI舆情分析
              </Button>
            </div>
          </div>

          <ProgressBlock
            ariaLabel="社媒平台讨论拉取进度"
            message={fetchProgress?.message ?? "尚未开始社媒平台讨论拉取"}
            percent={fetchPercent}
          />

          {fetchProgress && fetchProgress.items.length > 0 ? (
            <section className="sentiment-fetch-grid" aria-label="社媒平台讨论拉取状态">
              {fetchProgress.items.map((item) => (
                <FetchProgressRow item={item} key={item.stockCode} stockName={watchlistNameBySymbol.get(item.stockCode)} />
              ))}
            </section>
          ) : (
            <p className="financial-analysis-empty">暂无社媒平台讨论拉取记录。请先从自选股票池选择股票。</p>
          )}
        </CardContent>
      </Card>

      <Card className="panel financial-analysis-panel">
        <CardHeader className="px-6 pb-4 pt-6">
          <div className="panel__header-copy">
            <CardTitle>AI舆情结论</CardTitle>
            <p className="panel__meta">基于已缓存的社媒平台讨论生成舆情评分，完成后按总分从高到低展示。</p>
          </div>
        </CardHeader>
        <CardContent className="px-6 pb-6 pt-0">
          <ProgressBlock
            ariaLabel="AI舆情分析进度"
            message={analysisProgress?.message ?? "尚未开始 AI 舆情分析"}
            percent={analysisPercent}
          />
          {analysisProgress && analysisProgress.items.length > 0 ? (
            <section className="financial-analysis-status-grid" aria-label="AI舆情分析状态">
              {analysisProgress.items.map((item) => (
                <AnalysisStatusChip item={item} key={item.stockCode} stockName={watchlistNameBySymbol.get(item.stockCode)} />
              ))}
            </section>
          ) : null}
          {results.length > 0 ? (
            <div className="financial-analysis-list sentiment-analysis-list">
              {results.map((result) => (
                <button
                  aria-label={`${result.stockName || result.stockCode} 舆情总分 ${result.totalScore}`}
                  className="financial-analysis-row"
                  key={result.stockCode}
                  onClick={() => setActiveResult(result)}
                  type="button"
                >
                  <div className="financial-analysis-row__identity">
                    <strong>{result.stockName || result.stockCode}</strong>
                    <span>{result.stockCode}</span>
                  </div>
                  <div className="financial-analysis-row__score">
                    <strong>{result.totalScore}</strong>
                    <span>舆情总分</span>
                  </div>
                  <div className="financial-analysis-row__time">
                    <span>{formatDateTime(result.generatedAt)}</span>
                  </div>
                </button>
              ))}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <WatchlistSelectionModal
        confirmLabel={selectionCopy.confirmLabel}
        description={selectionCopy.description}
        onClose={() => setSelectionMode(null)}
        onConfirm={(symbols) => {
          const mode = selectionMode;
          setSelectionMode(null);
          setAnalysisEligibleSymbols(null);
          if (mode === "analysis") {
            analyzeMutation.mutate(symbols);
            return;
          }
          fetchMutation.mutate(symbols);
        }}
        open={selectionMode !== null}
        title={selectionCopy.title}
        watchlist={selectionMode === "analysis" ? analysisWatchlistRows : watchlistRows}
      />
      {activeResult ? (
        <SentimentDetailModal onClose={() => setActiveResult(null)} result={activeResult} />
      ) : null}
    </section>
  );
}

function ProgressBlock({
  ariaLabel,
  message,
  percent,
}: {
  ariaLabel: string;
  message: string;
  percent: number;
}) {
  return (
    <div className="financial-progress" aria-live="polite">
      <div className="financial-progress__meta">
        <span>{message}</span>
        <strong>{percent}%</strong>
      </div>
      <div
        aria-label={ariaLabel}
        aria-valuemax={100}
        aria-valuemin={0}
        aria-valuenow={percent}
        className="financial-progress__track"
        role="progressbar"
      >
        <span style={{ width: `${percent}%` }} />
      </div>
    </div>
  );
}

function FetchProgressRow({ item, stockName }: { item: SentimentFetchProgressItem; stockName?: string }) {
  const displayName = stockName || item.shortName || item.stockCode;
  const totalItems = item.platformStatuses.reduce((sum, status) => sum + Math.max(0, status.itemCount), 0);
  return (
    <article className="sentiment-fetch-row">
      <div className="sentiment-fetch-row__stock">
        <div className="sentiment-fetch-row__title">
          <strong>{displayName}</strong>
          <span>共 {totalItems} 条舆论</span>
        </div>
        <span>{item.stockCode}</span>
      </div>
      <div className="sentiment-platform-statuses">
        {item.platformStatuses.map((status) => (
          <PlatformStatusBadge key={status.platform} status={status} />
        ))}
      </div>
    </article>
  );
}

function PlatformStatusBadge({ status }: { status: SentimentPlatformFetchStatus }) {
  const label = platformLabel(status.platform);
  const statusLabel = platformStatusLabel(status.status);
  return (
    <div className={`sentiment-platform-status sentiment-platform-status--${status.status}`}>
      <span aria-label={statusLabel} className="sentiment-platform-status__icon" title={status.errorMessage ?? statusLabel}>
        {statusIcon(status.status)}
      </span>
      <span>{label}</span>
    </div>
  );
}

function AnalysisStatusChip({ item, stockName }: { item: SentimentAnalysisProgressItem; stockName?: string }) {
  const statusLabel = platformStatusLabel(item.status);
  const displayName = stockName || item.shortName || item.stockCode;
  return (
    <article className={`financial-analysis-status financial-analysis-status--${item.status}`}>
      <span aria-label={statusLabel} className="financial-analysis-status__icon" title={item.errorMessage ?? statusLabel}>
        {statusIcon(item.status)}
      </span>
      <strong>{displayName}</strong>
    </article>
  );
}

function SentimentDetailModal({
  onClose,
  result,
}: {
  onClose: () => void;
  result: SentimentAnalysisResult;
}) {
  const dimensions = DIMENSION_ITEMS.map((item) => ({
    ...item,
    value: result[item.key] as SentimentDimensionScore,
  }));
  return (
    <div className="modal-overlay" onClick={onClose}>
      <section
        aria-label={`${result.stockName || result.stockCode} AI舆情详情`}
        aria-modal="true"
        className="modal-content financial-report-modal sentiment-detail-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="modal-header">
          <div>
            <p className="section-label">AI舆情详情</p>
            <h2>{result.stockName || result.stockCode}</h2>
            <p className="panel__meta">
              {result.stockCode} · {formatDateTime(result.generatedAt)}
            </p>
          </div>
          <div className="sentiment-detail-score">
            <span>总分</span>
            <strong>{result.totalScore}</strong>
          </div>
          <Button onClick={onClose} size="icon" variant="ghost">
            <X size={16} />
          </Button>
        </div>

        <div className="modal-body financial-report-modal__body">
          <section className="financial-report-chart-block sentiment-chart-block">
            <div className="financial-report-chart-block__item">
              <h3>维度条形图</h3>
              <DimensionBarChart dimensions={dimensions} />
            </div>
            <div className="financial-report-chart-block__item">
              <h3>舆情雷达</h3>
              <DimensionRadarChart dimensions={dimensions} />
            </div>
          </section>
          <section className="sentiment-dimension-grid">
            {dimensions.map((dimension) => (
              <article className="financial-report-text-section" key={dimension.key}>
                <h3>{dimension.label}</h3>
                <strong>{dimension.value.score}</strong>
                <p>{dimension.value.reason}</p>
              </article>
            ))}
          </section>
        </div>
      </section>
    </div>
  );
}

function DimensionBarChart({
  dimensions,
}: {
  dimensions: Array<{ key: string; label: string; value: SentimentDimensionScore }>;
}) {
  const option = useMemo(
    () => ({
      backgroundColor: "transparent",
      animation: false,
      grid: {
        left: 92,
        right: 28,
        top: 14,
        bottom: 14,
        containLabel: false,
      },
      xAxis: {
        type: "value",
        min: 0,
        max: 100,
        interval: 25,
        axisLabel: {
          color: "rgba(237, 243, 255, 0.6)",
          fontSize: 11,
        },
        splitLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.12)",
          },
        },
      },
      yAxis: {
        type: "category",
        data: dimensions.map((dimension) => dimension.label),
        axisLabel: {
          color: "rgba(237, 243, 255, 0.82)",
          fontSize: 12,
          margin: 14,
        },
        axisLine: { show: false },
        axisTick: { show: false },
      },
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "shadow" },
        backgroundColor: "rgba(5, 10, 17, 0.96)",
        borderColor: "rgba(143, 220, 255, 0.24)",
        textStyle: {
          color: "#edf3ff",
        },
        formatter: (params: Array<{ name: string; value: number; dataIndex: number }>) => {
          const point = params[0];
          const current = dimensions[point.dataIndex];
          if (!current) return point.name;
          return `${point.name}<br/>${current.value.score} / 100`;
        },
      },
      series: [
        {
          type: "bar",
          barWidth: 10,
          data: dimensions.map((dimension) => dimension.value.score),
          showBackground: true,
          backgroundStyle: {
            color: "rgba(255, 255, 255, 0.05)",
            borderRadius: 999,
          },
          itemStyle: {
            borderRadius: 999,
            color: new echarts.graphic.LinearGradient(1, 0, 0, 0, [
              { offset: 0, color: "#72f0cb" },
              { offset: 1, color: "#72beff" },
            ]),
          },
          label: {
            show: true,
            position: "right",
            color: "rgba(237, 243, 255, 0.82)",
            fontSize: 11,
            formatter: ({ value }: { value: number }) => `${value}`,
          },
        },
      ],
    }),
    [dimensions],
  );
  return (
    <ChartSurface ariaLabel="舆情维度条形图" className="financial-report-score-chart sentiment-bar-chart" option={option} />
  );
}

function DimensionRadarChart({
  dimensions,
}: {
  dimensions: Array<{ key: string; label: string; value: SentimentDimensionScore }>;
}) {
  const option = useMemo(
    () => ({
      backgroundColor: "transparent",
      tooltip: {
        show: false,
      },
      radar: {
        center: ["50%", "52%"],
        radius: "65%",
        splitNumber: 5,
        axisName: {
          color: "rgba(237, 243, 255, 0.84)",
          fontSize: 12,
          fontWeight: 600,
        },
        splitLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.16)",
          },
        },
        splitArea: {
          areaStyle: {
            color: [
              "rgba(143, 220, 255, 0.02)",
              "rgba(143, 220, 255, 0.035)",
              "rgba(143, 220, 255, 0.05)",
              "rgba(143, 220, 255, 0.07)",
              "rgba(143, 220, 255, 0.09)",
            ],
          },
        },
        axisLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.2)",
          },
        },
        indicator: dimensions.map((dimension) => ({
          name: dimension.label,
          max: 100,
        })),
      },
      series: [
        {
          type: "radar",
          symbol: "circle",
          symbolSize: 7,
          itemStyle: {
            color: "#7ee0ff",
          },
          lineStyle: {
            width: 2,
            color: "#7ee0ff",
          },
          areaStyle: {
            color: "rgba(126, 224, 255, 0.18)",
          },
          data: [
            {
              value: dimensions.map((dimension) => dimension.value.score),
              name: "舆情维度",
            },
          ],
        },
      ],
    }),
    [dimensions],
  );
  const hoverItems = dimensions.map((dimension) => ({
    label: dimension.label,
    max: 100,
    score: dimension.value.score,
    standard: DIMENSION_SCORE_STANDARDS[dimension.key],
  }));
  return (
    <ChartSurface ariaLabel="舆情维度雷达图" className="financial-report-radar sentiment-radar-chart" option={option} radarHoverItems={hoverItems} />
  );
}

function ChartSurface({
  ariaLabel,
  className,
  option,
  radarHoverItems,
}: {
  ariaLabel: string;
  className: string;
  option: unknown;
  radarHoverItems?: Array<{ label: string; max: number; score: number; standard: string }>;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [radarTooltip, setRadarTooltip] = useState<{
    item: { label: string; max: number; score: number; standard: string };
    x: number;
    y: number;
  } | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, undefined, { renderer: "canvas" });
    chart.setOption(option);
    const resize = () => chart.resize();
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
    };
  }, [option]);

  function handleRadarMouseMove(event: React.MouseEvent<HTMLDivElement>) {
    if (!radarHoverItems || radarHoverItems.length === 0) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const centerX = rect.width / 2;
    const centerY = rect.height * 0.52;
    const angle = Math.atan2(event.clientY - rect.top - centerY, event.clientX - rect.left - centerX);
    const normalized = (-angle - Math.PI / 2 + Math.PI * 2) % (Math.PI * 2);
    const index = Math.round((normalized / (Math.PI * 2)) * radarHoverItems.length) % radarHoverItems.length;
    setRadarTooltip({
      item: radarHoverItems[index],
      x: Math.min(rect.width - 180, Math.max(12, event.clientX - rect.left + 12)),
      y: Math.min(rect.height - 86, Math.max(12, event.clientY - rect.top + 12)),
    });
  }

  return (
    <div
      aria-label={ariaLabel}
      className={`${className}${radarHoverItems ? " chart-surface--radar-hover" : ""}`}
      onMouseLeave={radarHoverItems ? () => setRadarTooltip(null) : undefined}
      onMouseMove={radarHoverItems ? handleRadarMouseMove : undefined}
      ref={containerRef}
      role="img"
    >
      {radarTooltip ? (
        <div
          aria-label={`${radarTooltip.item.label}评分说明`}
          className="radar-hover-tooltip"
          style={{ left: radarTooltip.x, top: radarTooltip.y }}
        >
          <strong>{radarTooltip.item.label}</strong>
          <span>得分：{radarTooltip.item.score}/{radarTooltip.item.max}</span>
          <p>评分标准：{radarTooltip.item.standard}</p>
        </div>
      ) : null}
    </div>
  );
}

function statusIcon(status: string) {
  if (status === "succeeded") return <Check size={14} strokeWidth={2.6} />;
  if (status === "retrying") return <CircleAlert size={14} strokeWidth={2.4} />;
  if (status === "failed") return <X size={14} strokeWidth={2.4} />;
  return <Circle size={11} strokeWidth={2.2} />;
}

function platformStatusLabel(status: string) {
  if (status === "succeeded") return "成功";
  if (status === "retrying") return "重试中";
  if (status === "failed") return "最终失败";
  if (status === "running") return "进行中";
  return "等待中";
}

function platformLabel(platform: string) {
  return PLATFORM_LABELS[platform] ?? platform;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}
