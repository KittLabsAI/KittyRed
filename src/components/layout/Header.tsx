import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useLocation } from "react-router-dom";
import { loadSettingsFormData } from "../../lib/settings";
import { listAnalyzeJobs } from "../../lib/tauri";
import { useAppStore } from "../../store/appStore";

const routeTitles: Record<string, string> = {
  "/": "总览",
  "/markets": "行情",
  "/pair-detail": "个股详情",
  "/positions": "持仓",
  "/recommendations": "AI投资建议",
  "/recommendations/history": "AI投资建议",
  "/settings": "设置",
};

const modeLabels = {
  paper: "模拟账号",
  real_read_only: "模拟账号",
  dual: "模拟账号",
} as const;

export function Header() {
  const location = useLocation();
  const accountMode = useAppStore((state) => state.accountMode);
  const title = useMemo(() => routeTitles[location.pathname] ?? "KittyRed", [location.pathname]);
  const settingsQuery = useQuery({
    queryKey: ["header-settings"],
    queryFn: loadSettingsFormData,
    staleTime: 30_000,
  });
  const jobsQuery = useQuery({
    queryKey: ["header-jobs"],
    queryFn: listAnalyzeJobs,
    refetchInterval: 15_000,
    staleTime: 15_000,
  });
  const latestJob = jobsQuery.data?.[0];
  const syncedLabel = latestJob?.updatedAt ? `同步于 ${formatHeaderTime(latestJob.updatedAt)}` : "等待同步";
  const cadenceLabel = settingsQuery.data?.autoAnalyzeEnabled
    ? `AI 扫描 ${settingsQuery.data.autoAnalyzeFrequency}`
    : "AI 扫描关闭";

  return (
    <header className="app-header">
      <div>
        <span className="section-label">A股模拟投资工作台</span>
        <h1>{title}</h1>
      </div>
      <div className="app-header__meta">
        <span>{modeLabels[accountMode]}</span>
        <span>{syncedLabel}</span>
        <span>{cadenceLabel}</span>
      </div>
    </header>
  );
}

function formatHeaderTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}
