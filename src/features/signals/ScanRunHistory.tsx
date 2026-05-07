import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { formatDateTime } from "../../lib/format";
import { listScanRuns } from "../../lib/tauri";

const PAGE_SIZE = 25;

function statusBadge(status: string) {
  switch (status) {
    case "done":
      return <span className="badge badge--status-done">完成</span>;
    case "running":
      return <span className="badge badge--status-running">运行中</span>;
    case "failed":
      return <span className="badge badge--status-failed">失败</span>;
    default:
      return <span className="badge badge--status-neutral">{status}</span>;
  }
}

export function ScanRunHistory() {
  const [page, setPage] = useState(1);

  const pageQuery = useQuery({
    queryKey: ["scan-runs", page, PAGE_SIZE],
    queryFn: () => listScanRuns(page, PAGE_SIZE),
    refetchInterval: 30_000,
    staleTime: 30_000,
  });

  const scanPage = pageQuery.data;
  const items = scanPage?.items ?? [];
  const total = scanPage?.total ?? 0;
  const totalPages = scanPage ? Math.ceil(scanPage.total / scanPage.pageSize) : 1;
  const showEmpty = !pageQuery.isFetching && items.length === 0;

  return (
    <div>
      {showEmpty ? (
        <p className="panel__meta">暂无扫描记录。</p>
      ) : (
        <>
          <div className="table-shell">
            <table>
              <thead>
                <tr>
                  <th>开始时间</th>
                  <th>股票数</th>
                  <th>信号数</th>
                  <th>耗时</th>
                  <th>状态</th>
                </tr>
              </thead>
              <tbody>
                {items.map((run) => (
                  <tr key={run.id}>
                    <td>{formatDateTime(run.startedAt)}</td>
                    <td>{run.symbolsScanned}</td>
                    <td>{run.signalsFound}</td>
                    <td>{run.durationMs != null ? `${(run.durationMs / 1000).toFixed(1)}s` : "—"}</td>
                    <td>{statusBadge(run.status)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="panel__header">
            <p className="panel__meta">
              第 {scanPage?.page ?? page} / {totalPages} 页 · {total} 次扫描
            </p>
            <div className="hero-panel__actions">
              <button
                className="ghost-button"
                disabled={(scanPage?.page ?? page) <= 1}
                onClick={() => setPage((c) => Math.max(1, c - 1))}
                type="button"
              >
                上一页
              </button>
              <button
                className="ghost-button"
                disabled={(scanPage?.page ?? page) >= totalPages}
                onClick={() => setPage((c) => Math.min(totalPages, c + 1))}
                type="button"
              >
                下一页
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
