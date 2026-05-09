import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BarChart3, FileText, RefreshCw, Square, Wand2 } from "lucide-react";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import {
  cancelFinancialReportFetch,
  getFinancialReportFetchProgress,
  getFinancialReportOverview,
  startFinancialReportAnalysis,
  startFinancialReportFetch,
} from "../../lib/tauri";

function progressPercent(completed: number, total: number) {
  if (total <= 0) return 0;
  return Math.min(100, Math.round((completed / total) * 100));
}

export function FinancialReportsPage() {
  const queryClient = useQueryClient();

  const overviewQuery = useQuery({
    queryKey: ["financial-report-overview"],
    queryFn: getFinancialReportOverview,
  });
  const progressQuery = useQuery({
    queryKey: ["financial-report-progress"],
    queryFn: getFinancialReportFetchProgress,
    refetchInterval: (query) => (query.state.data?.status === "running" ? 1200 : false),
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
    mutationFn: startFinancialReportAnalysis,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["financial-report-overview"] });
    },
  });

  const overview = overviewQuery.data;
  const progress = progressQuery.data;
  const isRunning = progress?.status === "running";
  const percent = progressPercent(
    progress?.completedSections ?? 0,
    progress?.totalSections ?? 6,
  );
  const sections = overview?.sections ?? [];
  const analyses = overview?.analyses ?? [];
  const rowsCount = overview?.rowCount ?? 0;

  return (
    <section className="page financial-page">
      <div className="page__header">
        <div>
          <p className="eyebrow">财报分析</p>
          <h1>全量财报分析</h1>
          <p className="page__intro">拉取近两年 AKShare 全量财报数据，缓存后为自选股票池生成 AI 财报结论。</p>
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
            <Button
              disabled={isRunning || fetchMutation.isPending}
              onClick={() => fetchMutation.mutate()}
              type="button"
            >
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
              onClick={() => analyzeMutation.mutate()}
              type="button"
              variant="ghost"
            >
              <Wand2 size={16} />
              分析自选股票池财报
            </Button>
          </div>
        </div>

        <div className="financial-progress" aria-live="polite">
          <div className="financial-progress__meta">
            <span>{progress?.message ?? "尚未开始财报拉取"}</span>
            <strong>{percent}%</strong>
          </div>
          <div className="financial-progress__track" role="progressbar" aria-valuemax={100} aria-valuemin={0} aria-valuenow={percent}>
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
          <strong>{overview?.refreshedAt ?? "未刷新"}</strong>
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
            <p className="panel__meta">推荐和 Assistant 只会读取这里缓存的四项财报结论。</p>
          </div>
        </CardHeader>
        <CardContent className="px-6 pb-6 pt-0">
        {analyses.length > 0 ? (
          <div className="financial-analysis-grid">
            {analyses.map((analysis) => (
              <section key={analysis.stockCode}>
                <span>{analysis.stockCode}</span>
                <h3>关键信息总结</h3>
                <p>{analysis.keySummary}</p>
                <h3>财报正向因素</h3>
                <p>{analysis.positiveFactors}</p>
                <h3>财报负向因素</h3>
                <p>{analysis.negativeFactors}</p>
                <h3>财报造假嫌疑点</h3>
                <p>{analysis.fraudRiskPoints}</p>
              </section>
            ))}
          </div>
        ) : (
          <p className="financial-analysis-empty">暂无 AI 财报结论。完成全量财报拉取后可以分析自选股票池。</p>
        )}
        </CardContent>
      </Card>
    </section>
  );
}
