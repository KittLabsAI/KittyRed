import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FinancialReportsPage } from "./FinancialReportsPage";
import type { FinancialReportFetchProgress, FinancialReportOverview } from "../../lib/types";

const {
  cancelFinancialReportFetchMock,
  getFinancialReportFetchProgressMock,
  getFinancialReportOverviewMock,
  startFinancialReportAnalysisMock,
  startFinancialReportFetchMock,
} = vi.hoisted(() => ({
  cancelFinancialReportFetchMock: vi.fn(async () => undefined),
  getFinancialReportFetchProgressMock: vi.fn<() => Promise<FinancialReportFetchProgress>>(),
  getFinancialReportOverviewMock: vi.fn<() => Promise<FinancialReportOverview>>(),
  startFinancialReportAnalysisMock: vi.fn(async () => undefined),
  startFinancialReportFetchMock: vi.fn(async () => undefined),
}));

vi.mock("../../lib/tauri", () => ({
  cancelFinancialReportFetch: cancelFinancialReportFetchMock,
  getFinancialReportFetchProgress: getFinancialReportFetchProgressMock,
  getFinancialReportOverview: getFinancialReportOverviewMock,
  startFinancialReportAnalysis: startFinancialReportAnalysisMock,
  startFinancialReportFetch: startFinancialReportFetchMock,
}));

const emptyOverview: FinancialReportOverview = {
  stockCount: 0,
  rowCount: 0,
  refreshedAt: null,
  sections: [],
  analyses: [],
};

const idleProgress: FinancialReportFetchProgress = {
  stockCode: "ALL",
  status: "idle",
  completedSections: 0,
  totalSections: 6,
  message: "尚未开始财报拉取",
  errorMessage: null,
};

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <FinancialReportsPage />
    </QueryClientProvider>,
  );
}

describe("FinancialReportsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getFinancialReportOverviewMock.mockResolvedValue(emptyOverview);
    getFinancialReportFetchProgressMock.mockResolvedValue(idleProgress);
  });

  it("shows the Chinese empty state and fetch action", async () => {
    renderPage();

    expect(await screen.findByText("全量财报分析")).toBeInTheDocument();
    expect(screen.getByText("暂无本地财报缓存。请先拉取近两年全量财报。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "拉取近两年全量财报" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
  });

  it("shows running progress and can cancel the fetch", async () => {
    const user = userEvent.setup();
    getFinancialReportFetchProgressMock.mockResolvedValue({
      stockCode: "ALL",
      status: "running",
      completedSections: 2,
      totalSections: 6,
      message: "正在缓存 利润表",
      errorMessage: null,
    });

    renderPage();

    expect(await screen.findByText("正在缓存 利润表")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "33");
    await user.click(screen.getByRole("button", { name: "中断拉取" }));

    await waitFor(() => {
      expect(cancelFinancialReportFetchMock).toHaveBeenCalled();
    });
  });

  it("shows cached report sections and the four AI analysis fields", async () => {
    const user = userEvent.setup();
    getFinancialReportOverviewMock.mockResolvedValue({
      stockCount: 2,
      rowCount: 5,
      refreshedAt: "2026-05-08T10:00:00+08:00",
      sections: [
        {
          section: "performance_report",
          label: "业绩报表",
          source: "akshare:stock_yjbb_em",
          rowCount: 3,
        },
        {
          section: "income_statement",
          label: "利润表",
          source: "akshare:stock_lrb_em",
          rowCount: 2,
        },
      ],
      analyses: [
        {
          stockCode: "SHSE.600000",
          sourceRevision: "rev-1",
          keySummary: "营收和利润保持改善。",
          positiveFactors: "现金流质量提升。",
          negativeFactors: "费用率仍需观察。",
          fraudRiskPoints: "暂未发现明显异常。",
          modelProvider: "OpenAI-compatible",
          modelName: "gpt-5.5",
          generatedAt: "2026-05-08T10:05:00+08:00",
          stale: false,
        },
      ],
    });

    renderPage();

    expect(await screen.findByText("业绩报表")).toBeInTheDocument();
    expect(screen.getByText("利润表")).toBeInTheDocument();
    expect(screen.queryByText("来源")).not.toBeInTheDocument();
    expect(screen.queryByText("akshare:stock_yjbb_em")).not.toBeInTheDocument();
    expect(screen.getByText("财报行数")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("关键信息总结")).toBeInTheDocument();
    expect(screen.getByText("营收和利润保持改善。")).toBeInTheDocument();
    expect(screen.getByText("财报正向因素")).toBeInTheDocument();
    expect(screen.getByText("现金流质量提升。")).toBeInTheDocument();
    expect(screen.getByText("财报负向因素")).toBeInTheDocument();
    expect(screen.getByText("费用率仍需观察。")).toBeInTheDocument();
    expect(screen.getByText("财报造假嫌疑点")).toBeInTheDocument();
    expect(screen.getByText("暂未发现明显异常。")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "分析自选股票池财报" }));
    await waitFor(() => {
      expect(startFinancialReportAnalysisMock).toHaveBeenCalled();
    });
  });
});
