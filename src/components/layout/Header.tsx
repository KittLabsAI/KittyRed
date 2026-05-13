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
  "/backtest": "AI回测",
  "/financial-reports": "财报分析",
  "/sentiment": "舆情分析",
  "/signals": "策略信号",
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
    <header className="app-header flex flex-col items-start justify-between gap-5 border-b border-border pb-5 lg:flex-row">
      <div>
        <span className="section-label text-xs font-semibold uppercase tracking-[0.1em] text-accent">A股模拟投资工作台</span>
        <h1 className="mt-2 text-[clamp(1.8rem,3vw,2.4rem)] font-semibold tracking-tight">{title}</h1>
      </div>
      <div className="app-header__meta flex flex-wrap justify-end gap-2">
        <span className="rounded-full border border-border bg-white/4 px-3 py-2 text-[0.74rem] uppercase tracking-[0.1em] text-muted-foreground">{modeLabels[accountMode]}</span>
        <span className="rounded-full border border-border bg-white/4 px-3 py-2 text-[0.74rem] uppercase tracking-[0.1em] text-muted-foreground">{syncedLabel}</span>
        <span className="rounded-full border border-border bg-white/4 px-3 py-2 text-[0.74rem] uppercase tracking-[0.1em] text-muted-foreground">{cadenceLabel}</span>
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
