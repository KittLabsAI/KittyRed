import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as echarts from "echarts";
import { BarChart3, Check, ChevronDown, Circle, CircleAlert, FileText, RefreshCw, Square, Wand2, X, X as XIcon } from "lucide-react";
import { WatchlistSelectionModal } from "../../components/WatchlistSelectionModal";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import {
  cancelFinancialReportFetch,
  getFinancialReportAnalysisProgress,
  getFinancialReportFetchProgress,
  getFinancialReportOverview,
  getFinancialReportSnapshot,
  listMarkets,
  startFinancialReportAnalysis,
  startFinancialReportFetch,
} from "../../lib/tauri";
import { formatDateTime } from "../../lib/format";
import type {
  FinancialReportAnalysis,
  FinancialReportAnalysisProgressItem,
  FinancialReportMetricSeries,
  FinancialReportSection,
  FinancialReportSnapshot,
} from "../../lib/types";

function progressPercent(completed: number, total: number) {
  if (total <= 0) return 0;
  return Math.min(100, Math.round((completed / total) * 100));
}

function sortByScore(analyses: FinancialReportAnalysis[]) {
  return analyses.slice().sort((a, b) => b.financialScore - a.financialScore);
}

export function FinancialReportsPage() {
  const queryClient = useQueryClient();
  const [activeStockCode, setActiveStockCode] = useState<string | null>(null);
  const [selectionOpen, setSelectionOpen] = useState(false);

  const overviewQuery = useQuery({
    queryKey: ["financial-report-overview"],
    queryFn: getFinancialReportOverview,
  });
  const watchlistQuery = useQuery({
    queryKey: ["financial-report-watchlist"],
    queryFn: listMarkets,
  });
  const progressQuery = useQuery({
    queryKey: ["financial-report-progress"],
    queryFn: getFinancialReportFetchProgress,
    refetchInterval: (query) => (query.state.data?.status === "running" ? 1200 : false),
  });
  const analysisProgressQuery = useQuery({
    queryKey: ["financial-report-analysis-progress"],
    queryFn: getFinancialReportAnalysisProgress,
    refetchInterval: (query) => (query.state.data?.status === "running" ? 1200 : false),
  });
  const detailQuery = useQuery({
    queryKey: ["financial-report-snapshot", activeStockCode],
    queryFn: () => getFinancialReportSnapshot(activeStockCode ?? ""),
    enabled: Boolean(activeStockCode),
  });

  const fetchMutation = useMutation({
    mutationFn: startFinancialReportFetch,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["financial-report-progress"] });
      await queryClient.invalidateQueries({ queryKey: ["financial-report-overview"] });
    },
  });
  const cancelMutation = useMutation({
    mutationFn: cancelFinancialReportFetch,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["financial-report-progress"] });
    },
  });
  const analyzeMutation = useMutation({
    mutationFn: (selectedSymbols: string[]) => startFinancialReportAnalysis(selectedSymbols),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["financial-report-analysis-progress"] });
      await queryClient.invalidateQueries({ queryKey: ["financial-report-overview"] });
    },
  });

  const overview = overviewQuery.data;
  const progress = progressQuery.data;
  const isRunning = progress?.status === "running";
  const percent = progressPercent(progress?.completedSections ?? 0, progress?.totalSections ?? 6);
  const analysisProgress = analysisProgressQuery.data;
  const analysisPercent = progressPercent(analysisProgress?.completedCount ?? 0, analysisProgress?.totalCount ?? 0);
  const sections = overview?.sections ?? [];
  const analyses = useMemo(() => sortByScore(overview?.analyses ?? []), [overview?.analyses]);
  const rowsCount = overview?.rowCount ?? 0;
  const snapshot = detailQuery.data;
  const watchlistRows = watchlistQuery.data ?? [];

  useEffect(() => {
    if (!activeStockCode) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setActiveStockCode(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeStockCode]);

  return (
    <section className="page financial-page">
      <div className="page__header">
        <div>
          <p className="eyebrow">财报分析</p>
        </div>
      </div>

      <Card className="panel financial-workbench">
        <CardContent className="grid gap-5 px-6 py-6">
          <div className="financial-toolbar">
            <div className="financial-scope-copy">
              <span>拉取范围</span>
              <strong>沪深 A 股近两年全量财报</strong>
            </div>
            <div className="financial-actions">
              <Button disabled={isRunning || fetchMutation.isPending} onClick={() => fetchMutation.mutate()} type="button">
                <RefreshCw size={16} />
                拉取近两年全量财报
              </Button>
              <Button
                disabled={!isRunning || cancelMutation.isPending}
                onClick={() => cancelMutation.mutate()}
                type="button"
                variant="ghost"
              >
                <Square size={15} />
                中断拉取
              </Button>
              <Button
                disabled={rowsCount === 0 || analyzeMutation.isPending}
                onClick={() => setSelectionOpen(true)}
                type="button"
                variant="ghost"
              >
                <Wand2 size={16} />
                AI财报分析
              </Button>
            </div>
          </div>

          <div className="financial-progress" aria-live="polite">
            <div className="financial-progress__meta">
              <span>{progress?.message ?? "尚未开始财报拉取"}</span>
              <strong>{percent}%</strong>
            </div>
            <div
              aria-label="财报拉取进度"
              className="financial-progress__track"
              aria-valuemax={100}
              aria-valuemin={0}
              aria-valuenow={percent}
              role="progressbar"
            >
              <span style={{ width: `${percent}%` }} />
            </div>
            {progress?.errorMessage ? <p>{progress.errorMessage}</p> : null}
          </div>
        </CardContent>
      </Card>

      <section className="financial-summary-band">
        <div>
          <span>缓存股票</span>
          <strong>{overview?.stockCount ?? 0}</strong>
        </div>
        <div>
          <span>财报行数</span>
          <strong>{rowsCount}</strong>
        </div>
        <div>
          <span>刷新时间</span>
          <strong>{overview?.refreshedAt ? formatDateTime(overview.refreshedAt) : "未刷新"}</strong>
        </div>
      </section>

      {sections.length > 0 ? (
        <section className="financial-section-grid" aria-label="财报缓存">
          {sections.map((section) => (
            <article className="financial-section-card" key={section.section}>
              <div>
                <FileText size={16} />
                <h2>{section.label}</h2>
              </div>
              <dl>
                <div>
                  <dt>缓存行数</dt>
                  <dd>{section.rowCount}</dd>
                </div>
              </dl>
            </article>
          ))}
        </section>
      ) : (
        <section className="panel financial-empty">
          <BarChart3 size={22} />
          <p>暂无本地财报缓存。请先拉取近两年全量财报。</p>
        </section>
      )}

      <Card className="panel financial-analysis-panel">
        <CardHeader className="px-6 pb-4 pt-6">
          <div className="panel__header-copy">
            <CardTitle>AI财报结论</CardTitle>
            <p className="panel__meta">按财报综合评分从高到低展示，点击可查看原始财报和历史图表。</p>
          </div>
        </CardHeader>
        <CardContent className="px-6 pb-6 pt-0">
          <div className="financial-progress financial-progress--analysis" aria-live="polite">
            <div className="financial-progress__meta">
              <span>{analysisProgress?.message ?? "尚未开始财报 AI 分析"}</span>
              <strong>{analysisPercent}%</strong>
            </div>
            <div
              aria-label="财报分析进度"
              className="financial-progress__track"
              aria-valuemax={100}
              aria-valuemin={0}
              aria-valuenow={analysisPercent}
              role="progressbar"
            >
              <span style={{ width: `${analysisPercent}%` }} />
            </div>
          </div>

          {analysisProgress && analysisProgress.items.length > 0 ? (
            <section className="financial-analysis-status-grid" aria-label="财报分析进度">
              {analysisProgress.items.map((item) => (
                <AnalysisStatusChip item={item} key={item.stockCode} />
              ))}
            </section>
          ) : null}

          {analyses.length > 0 ? (
            <div className="financial-analysis-list">
              {analyses.map((analysis) => (
                <button
                  className="financial-analysis-row"
                  key={analysis.stockCode}
                  onClick={() => setActiveStockCode(analysis.stockCode)}
                  type="button"
                >
                  <div className="financial-analysis-row__identity">
                    <strong>{analysis.stockName || analysis.stockCode}</strong>
                    <span>{analysis.stockCode}</span>
                  </div>
                  <div className="financial-analysis-row__score">
                    <strong>{analysis.financialScore}</strong>
                    <span>财报评分</span>
                  </div>
                  <div className="financial-analysis-row__time">
                    <span>{formatDateTime(analysis.generatedAt)}</span>
                    {analysis.stale ? <em>数据已过期</em> : null}
                  </div>
                  <ChevronDown size={16} />
                </button>
              ))}
            </div>
          ) : (
            <p className="financial-analysis-empty">暂无 AI 财报结论。完成全量财报拉取后可以分析自选股票池。</p>
          )}
        </CardContent>
      </Card>

      {activeStockCode && snapshot ? (
        <FinancialReportModal
          onClose={() => setActiveStockCode(null)}
          snapshot={snapshot}
        />
      ) : null}
      <WatchlistSelectionModal
        confirmLabel="开始财报分析"
        description="从当前自选股池中勾选要进入本次 AI 财报分析的股票。"
        onClose={() => setSelectionOpen(false)}
        onConfirm={(symbols) => {
          setSelectionOpen(false);
          analyzeMutation.mutate(symbols);
        }}
        open={selectionOpen}
        title="选择参与财报 AI 分析的股票"
        watchlist={watchlistRows}
      />
    </section>
  );
}

function FinancialReportModal({
  onClose,
  snapshot,
}: {
  onClose: () => void;
  snapshot: FinancialReportSnapshot;
}) {
  const latestStatements = [
    { title: "资产负债表", section: findSection(snapshot.sections, ["资产负债表", "balance_sheet"]) },
    { title: "利润表", section: findSection(snapshot.sections, ["利润表", "income_statement"]) },
    { title: "现金流量表", section: findSection(snapshot.sections, ["现金流量表", "cash_flow"]) },
  ];
  const analysis = snapshot.analysis;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <section
        aria-label={`${snapshot.stockName || snapshot.stockCode} 财报详情`}
        aria-modal="true"
        className="modal-content financial-report-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="modal-header">
          <div>
            <p className="section-label">财报 AI 详情</p>
            <h2>{snapshot.stockName || snapshot.stockCode}</h2>
            <p className="panel__meta">
              {snapshot.stockCode} · {analysis?.generatedAt ? formatDateTime(analysis.generatedAt) : "暂无分析时间"}
            </p>
          </div>
          <Button onClick={onClose} size="icon" variant="ghost">
            <X size={16} />
          </Button>
        </div>

        <div className="modal-body financial-report-modal__body">
          {analysis ? (
            <section className="financial-report-ai-block">
              <div className="financial-report-score">
                <span>财报综合评分</span>
                <strong>{analysis.financialScore}</strong>
              </div>
              <section className="financial-report-chart-block">
                <div className="financial-report-chart-block__item">
                  <h3>子维度评分</h3>
                  <CategoryScoreBarChart scores={analysis.categoryScores} />
                </div>
                <div className="financial-report-chart-block__item">
                  <h3>能力雷达</h3>
                  <RadarChart scores={analysis.radarScores} />
                </div>
              </section>
              <TextSection title="关键信息总结" text={analysis.keySummary} />
              <TextSection title="财报正向因素" text={analysis.positiveFactors} />
              <TextSection title="财报负向因素" text={analysis.negativeFactors} />
              <TextSection title="财报造假嫌疑点" text={analysis.fraudRiskPoints} />
            </section>
          ) : null}

          <section className="financial-report-statement-grid">
            {latestStatements.map(({ title, section }) => (
              <article className="financial-report-statement-card" key={title}>
                <div className="financial-report-statement-card__header">
                  <h3>{title}</h3>
                  <span>{section?.rows[0]?.reportDate ?? "暂无"}</span>
                </div>
                {section ? (
                  <FinancialRawTable row={section.rows[0]} />
                ) : (
                  <p className="panel__meta">暂无缓存数据。</p>
                )}
              </article>
            ))}
          </section>

          <section className="financial-report-chart-grid">
            {snapshot.metricSeries.length > 0 ? snapshot.metricSeries.map((series) => <MetricBarChart key={series.metricKey} series={series} />) : <p className="panel__meta">暂无关键指标历史数据。</p>}
          </section>
        </div>
      </section>
    </div>
  );
}

function TextSection({ title, text }: { title: string; text: string }) {
  return (
    <section className="financial-report-text-section">
      <h3>{title}</h3>
      <p>{highlightDigits(text)}</p>
    </section>
  );
}

function AnalysisStatusChip({ item }: { item: FinancialReportAnalysisProgressItem }) {
  const marker = analysisMarker(item.status);
  return (
    <article className={`financial-analysis-status financial-analysis-status--${item.status}`}>
      <span aria-hidden="true" className="financial-analysis-status__icon">
        {marker}
      </span>
      <strong>{shortenStockName(item.shortName)}</strong>
    </article>
  );
}

function FinancialRawTable({ row }: { row?: FinancialReportSection["rows"][number] }) {
  if (!row) return <p className="panel__meta">暂无缓存数据。</p>;
  const entries = Object.entries(row.raw ?? {})
    .filter(([key]) => !HIDDEN_FINANCIAL_RAW_FIELDS.has(key))
    .sort(([left], [right]) => left.localeCompare(right, "zh-CN"))
    .slice(0, 18);
  return (
    <dl className="financial-report-raw-table">
      {entries.map(([key, value]) => (
        <div key={key}>
          <dt>{key}</dt>
          <dd>{formatFinancialRawValue(key, value)}</dd>
        </div>
      ))}
    </dl>
  );
}

function findSection(sections: FinancialReportSection[], keywords: string[]) {
  return sections.find((section) => keywords.some((keyword) => section.label.includes(keyword) || section.section.includes(keyword)));
}

function highlightDigits(text: string) {
  const parts = text.split(/(\d+(?:\.\d+)?%?)/g);
  return parts.map((part, index) =>
    /^\d/.test(part) ? (
      <span className="financial-report-number" key={`${part}-${index}`}>
        {part}
      </span>
    ) : (
      <span key={`${part}-${index}`}>{part}</span>
    ),
  );
}

function RadarChart({
  scores,
}: {
  scores: FinancialReportAnalysis["radarScores"];
}) {
  const labels = [
    { key: "profitability", label: "盈利性", standard: "净利与回报、毛利水平等盈利表现越强得分越高" },
    { key: "authenticity", label: "真实性", standard: "盈利调节和资产质量异常越少，财务数据一致性越好得分越高" },
    { key: "cashGeneration", label: "造血力", standard: "经营现金流、现金流稳定性和利润含金量越强得分越高" },
    { key: "safety", label: "安全性", standard: "偿债能力越强、负债压力和流动性风险越低得分越高" },
    { key: "growthPotential", label: "成长性", standard: "业绩增速、研发及资本投入能支撑未来增长时得分越高" },
    { key: "operatingEfficiency", label: "运转效率", standard: "营运效率、周转表现和资源利用效率越好得分越高" },
  ] as const;
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
        indicator: labels.map((item) => ({
          name: item.label,
          max: 10,
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
              value: labels.map((item) => Number(scores[item.key].toFixed(2))),
              name: "财报能力",
            },
          ],
        },
      ],
    }),
    [scores],
  );
  const hoverItems = labels.map((item) => ({
    label: item.label,
    max: 10,
    score: Number(scores[item.key].toFixed(2)),
    standard: item.standard,
  }));
  return (
    <ChartSurface ariaLabel="财报评分雷达图" className="financial-report-radar" option={option} radarHoverItems={hoverItems} />
  );
}

function analysisMarker(status: FinancialReportAnalysisProgressItem["status"]) {
  if (status === "succeeded") return <Check size={14} strokeWidth={2.6} />;
  if (status === "retrying") return <CircleAlert size={14} strokeWidth={2.4} />;
  if (status === "failed") return <XIcon size={14} strokeWidth={2.4} />;
  return <Circle size={11} strokeWidth={2.2} />;
}

function shortenStockName(value: string) {
  return Array.from(value).slice(0, 4).join("");
}

const CATEGORY_SCORE_ITEMS: Array<{
  key: keyof FinancialReportAnalysis["categoryScores"];
  label: string;
  max: number;
}> = [
  { key: "revenueQuality", label: "收入质量", max: 8 },
  { key: "grossMargin", label: "毛利水平", max: 10 },
  { key: "netProfitReturn", label: "净利与回报", max: 12 },
  { key: "earningsManipulation", label: "盈利调节", max: 5 },
  { key: "solvency", label: "偿债能力", max: 15 },
  { key: "cashFlow", label: "现金流状况", max: 15 },
  { key: "growth", label: "业绩增速", max: 12 },
  { key: "researchCapital", label: "研发及资本投入", max: 8 },
  { key: "operatingEfficiency", label: "营运效率", max: 10 },
  { key: "assetQuality", label: "资产质量", max: 5 },
];

function CategoryScoreBarChart({
  scores,
}: {
  scores: FinancialReportAnalysis["categoryScores"];
}) {
  const scoreRatios = useMemo(
    () =>
      CATEGORY_SCORE_ITEMS.map((item) => {
        const score = scores[item.key];
        return {
          label: item.label,
          score,
          max: item.max,
          ratio: Number(((score / item.max) * 100).toFixed(2)),
        };
      }),
    [scores],
  );
  const option = useMemo(
    () => ({
      backgroundColor: "transparent",
      animation: false,
      grid: {
        left: 100,
        right: 26,
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
          formatter: "{value}%",
        },
        splitLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.12)",
          },
        },
      },
      yAxis: {
        type: "category",
        data: scoreRatios.map((item) => item.label),
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
          const current = scoreRatios[point.dataIndex];
          if (!current) return point.name;
          return `${point.name}<br/>${current.score} / ${current.max}<br/>占比：${current.ratio}%`;
        },
      },
      series: [
        {
          type: "bar",
          barWidth: 10,
          data: scoreRatios.map((item) => item.ratio),
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
            formatter: ({ dataIndex }: { dataIndex: number; value: number }) => {
              const current = scoreRatios[dataIndex];
              if (!current) return "";
              return `${current.score}/${current.max}`;
            },
          },
        },
      ],
    }),
    [scoreRatios],
  );
  return <ChartSurface ariaLabel="财报子维度评分条形图" className="financial-report-score-chart" option={option} />;
}

function MetricBarChart({ series }: { series: FinancialReportMetricSeries }) {
  const option = useMemo(
    () => ({
      backgroundColor: "transparent",
      animation: false,
      grid: {
        left: 42,
        right: 12,
        top: 18,
        bottom: 44,
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: "rgba(5, 10, 17, 0.96)",
        borderColor: "rgba(143, 220, 255, 0.24)",
        textStyle: {
          color: "#edf3ff",
        },
        formatter: (params: Array<{ axisValue: string; value: number; dataIndex: number }>) => {
          const point = params[0];
          const detail = series.points[point.dataIndex];
          return [
            point.axisValue,
            `数值：${point.value} ${series.unit}`,
            `同比：${detail?.yoy === null || detail?.yoy === undefined ? "无" : `${detail.yoy.toFixed(2)}%`}`,
            `环比：${detail?.qoq === null || detail?.qoq === undefined ? "无" : `${detail.qoq.toFixed(2)}%`}`,
          ].join("<br/>");
        },
      },
      xAxis: {
        type: "category",
        data: series.points.map((point) => point.reportDate),
        axisLabel: {
          color: "rgba(237, 243, 255, 0.58)",
          fontSize: 10,
          interval: 0,
          rotate: 32,
        },
        axisLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.16)",
          },
        },
      },
      yAxis: {
        type: "value",
        axisLabel: {
          color: "rgba(237, 243, 255, 0.58)",
          fontSize: 10,
        },
        splitLine: {
          lineStyle: {
            color: "rgba(143, 220, 255, 0.1)",
          },
        },
      },
      series: [
        {
          type: "bar",
          data: series.points.map((point) => point.value),
          barWidth: "42%",
          itemStyle: {
            borderRadius: [10, 10, 4, 4],
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: "rgba(114, 240, 203, 0.96)" },
              { offset: 1, color: "rgba(114, 190, 255, 0.28)" },
            ]),
          },
        },
      ],
    }),
    [series],
  );

  return (
    <article className="financial-report-chart-card">
      <div className="financial-report-chart-card__header">
        <h3>{series.metricLabel}</h3>
        <span>{series.unit}</span>
      </div>
      <ChartSurface ariaLabel={series.metricLabel} className="financial-report-metric-chart" option={option} />
    </article>
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

function formatFinancialRawValue(key: string, value: unknown) {
  if (value === null || value === undefined) {
    return "无";
  }
  if (typeof value === "number" && (key.includes("同比") || key.includes("环比") || key.includes("占比"))) {
    return `${value.toLocaleString("en-US", {
      maximumFractionDigits: 2,
      minimumFractionDigits: 0,
    })}%`;
  }
  if (typeof value !== "number" || key.includes("代码")) {
    return String(value);
  }
  return value.toLocaleString("en-US", {
    maximumFractionDigits: Number.isInteger(value) ? 0 : 2,
    minimumFractionDigits: 0,
  });
}

const HIDDEN_FINANCIAL_RAW_FIELDS = new Set(["公告日期", "股票代码", "序号", "股票简称"]);
