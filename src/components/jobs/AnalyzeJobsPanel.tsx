import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { formatDateTime } from "../../lib/format";
import { cancelAnalyzeJob, listAnalyzeJobs } from "../../lib/tauri";

export function AnalyzeJobsPanel() {
  const queryClient = useQueryClient();
  const [typeFilter, setTypeFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const jobsQuery = useQuery({
    queryKey: ["analyze-jobs"],
    queryFn: listAnalyzeJobs,
    refetchInterval: 15_000,
    staleTime: 15_000,
  });
  const cancelMutation = useMutation({
    mutationFn: (id: number) => cancelAnalyzeJob(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["analyze-jobs"], refetchType: "active" });
    },
  });
  const jobs = useMemo(() => {
    return (jobsQuery.data ?? []).filter((job) => {
      if (typeFilter !== "all" && job.kind !== typeFilter) {
        return false;
      }
      return statusFilter === "all" || job.status === statusFilter;
    });
  }, [jobsQuery.data, statusFilter, typeFilter]);
  const jobTypes = useMemo(() => {
    return Array.from(new Set((jobsQuery.data ?? []).map((job) => job.kind))).sort();
  }, [jobsQuery.data]);
  const runningCount = (jobsQuery.data ?? []).filter((job) => job.status === "running").length;
  const failedCount = (jobsQuery.data ?? []).filter((job) => job.status === "failed").length;

  return (
    <section aria-labelledby="analyze-jobs-title" className="panel analyze-jobs-panel">
      <div className="panel__header analyze-jobs-panel__header">
        <div>
          <span className="section-label">任务</span>
          <h2 id="analyze-jobs-title">后台任务</h2>
        </div>
        <div className="analyze-jobs-panel__summary" aria-label="任务概览">
          <span>显示 {jobs.length}</span>
          <span>运行 {runningCount}</span>
          <span>失败 {failedCount}</span>
        </div>
        <div className="jobs-filter-row">
          <label className="search-shell">
            <span className="sr-only">任务类型筛选</span>
            <select
              aria-label="任务类型筛选"
              className="control-select"
              onChange={(event) => setTypeFilter(event.target.value)}
              value={typeFilter}
            >
              <option value="all">全部类型</option>
              {jobTypes.map((kind) => (
                <option key={kind} value={kind}>
                  {kind}
                </option>
              ))}
            </select>
          </label>
          <label className="search-shell">
            <span className="sr-only">任务状态筛选</span>
            <select
              aria-label="任务状态筛选"
              className="control-select"
              onChange={(event) => setStatusFilter(event.target.value)}
              value={statusFilter}
            >
              <option value="all">全部状态</option>
              <option value="running">运行中</option>
              <option value="done">已完成</option>
              <option value="failed">失败</option>
              <option value="blocked">已拦截</option>
              <option value="cancelled">已取消</option>
            </select>
          </label>
        </div>
      </div>
      <div className="job-list job-list--scrollable">
        {jobs.map((job) => (
          <article className="job-card" key={job.id}>
            <div className="job-card__identity">
              <strong>{job.kind}</strong>
              <time>{formatDateTime(job.updatedAt)}</time>
              {job.durationMs !== null && job.durationMs !== undefined ? (
                <span className="job-card__duration">耗时：{formatDuration(job.durationMs)}</span>
              ) : null}
            </div>
            <div className="job-card__body">
              <p>{job.message}</p>
              {job.inputParamsJson ? <p className="job-card__detail">输入：{job.inputParamsJson}</p> : null}
              {job.resultSummary ? (
                <p className="job-card__detail">
                  {isRecommendationJob(job.kind) ? "原因" : "结果"}：{job.resultSummary}
                </p>
              ) : null}
              {job.errorDetails ? <p className="job-card__detail">错误：{job.errorDetails}</p> : null}
            </div>
            <div className="job-card__actions">
              <span className={`status-pill status-pill--${job.status}`}>{job.status}</span>
              <button
                aria-label={`停止 ${job.kind}`}
                className="job-card__stop"
                disabled={job.status !== "running" || cancelMutation.isPending}
                onClick={() => cancelMutation.mutate(job.id)}
                type="button"
              >
                停止
              </button>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function formatDuration(durationMs: number) {
  return `${(durationMs / 1000).toFixed(1)}s`;
}

function isRecommendationJob(kind: string) {
  return kind === "recommendation.generate" || kind === "recommendation_generate";
}
