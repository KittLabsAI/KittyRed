import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FinancialReportsPage } from "./FinancialReportsPage";
import type { FinancialReportAnalysisProgress, FinancialReportFetchProgress, FinancialReportOverview } from "../../lib/types";

const echartsMocks = vi.hoisted(() => ({
  chartSetOption: vi.fn(),
}));

vi.mock("echarts", () => ({
  default: undefined,
  init: () => ({
    setOption: echartsMocks.chartSetOption,
    resize: vi.fn(),
    dispose: vi.fn(),
  }),
  graphic: {
    LinearGradient: class LinearGradientMock {
      constructor(
        public x0: number,
        public y0: number,
        public x1: number,
        public y1: number,
        public colorStops: Array<{ offset: number; color: string }>,
      ) {}
    },
  },
}));

const {
  cancelFinancialReportFetchMock,
  getFinancialReportAnalysisProgressMock,
  getFinancialReportFetchProgressMock,
  getFinancialReportOverviewMock,
  getFinancialReportSnapshotMock,
  listMarketsMock,
  startFinancialReportAnalysisMock,
  startFinancialReportFetchMock,
} = vi.hoisted(() => ({
  cancelFinancialReportFetchMock: vi.fn(async () => undefined),
  getFinancialReportAnalysisProgressMock: vi.fn<() => Promise<FinancialReportAnalysisProgress>>(),
  getFinancialReportFetchProgressMock: vi.fn<() => Promise<FinancialReportFetchProgress>>(),
  getFinancialReportOverviewMock: vi.fn<() => Promise<FinancialReportOverview>>(),
  listMarketsMock: vi.fn(async () => [
    { symbol: "SHSE.600000", baseAsset: "浦发银行", marketType: "沪市A股", marketSizeTier: "large", last: 8.72, change24h: 0.81, volume24h: 1260000000, spreadBps: 0, venues: ["akshare"], updatedAt: "2026-05-08T10:00:00+08:00" },
    { symbol: "SZSE.000001", baseAsset: "平安银行", marketType: "深市A股", marketSizeTier: "large", last: 11.34, change24h: -0.31, volume24h: 1050000000, spreadBps: 0, venues: ["akshare"], updatedAt: "2026-05-08T10:00:00+08:00" },
  ]),
  getFinancialReportSnapshotMock: vi.fn(async () => ({
    stockCode: "SHSE.600000",
    stockName: "浦发银行",
    sections: [
      {
        section: "income_statement",
        label: "利润表",
        source: "akshare:stock_lrb_em",
        error: null,
        rows: [
          {
            stockCode: "SHSE.600000",
            reportDate: "2026-03-31",
            stockName: "浦发银行",
            raw: {
              序号: 1,
              股票代码: "600000",
              股票简称: "浦发银行",
              公告日期: "2026-04-29",
              报告期: "2026-03-31",
              营业收入: 1234567.89,
              净利润: 20250.25,
              营业收入同比: 12.5,
              研发费用占比: 8.88,
              净利润环比: -3.25,
              其他收益: null,
            },
          },
        ],
      },
    ],
    sourceRevision: "rev-1",
    refreshedAt: "2026-05-08T10:00:00+08:00",
    metricSeries: [
      {
        metricKey: "营业收入",
        metricLabel: "营收",
        unit: "亿元",
        points: [
          { reportDate: "2025-03-31", value: 80, yoy: null, qoq: null },
          { reportDate: "2025-06-30", value: 90, yoy: 12.5, qoq: 12.5 },
        ],
      },
    ],
    analysis: {
      stockCode: "SHSE.600000",
      stockName: "浦发银行",
      financialScore: 88,
      categoryScores: {
        revenueQuality: 7,
        grossMargin: 8,
        netProfitReturn: 10,
        earningsManipulation: 4,
        solvency: 12,
        cashFlow: 13,
        growth: 9,
        researchCapital: 7,
        operatingEfficiency: 8,
        assetQuality: 4,
      },
      radarScores: {
        profitability: 8.18,
        authenticity: 8.46,
        cashGeneration: 8.67,
        safety: 8,
        growthPotential: 8,
        operatingEfficiency: 8,
      },
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
  })),
  startFinancialReportAnalysisMock: vi.fn(async () => undefined),
  startFinancialReportFetchMock: vi.fn(async () => undefined),
}));

vi.mock("../../lib/tauri", () => ({
  cancelFinancialReportFetch: cancelFinancialReportFetchMock,
  getFinancialReportAnalysisProgress: getFinancialReportAnalysisProgressMock,
  getFinancialReportFetchProgress: getFinancialReportFetchProgressMock,
  getFinancialReportOverview: getFinancialReportOverviewMock,
  getFinancialReportSnapshot: getFinancialReportSnapshotMock,
  listMarkets: listMarketsMock,
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

const idleAnalysisProgress: FinancialReportAnalysisProgress = {
  status: "idle",
  completedCount: 0,
  totalCount: 0,
  message: "尚未开始财报 AI 分析",
  items: [],
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
    getFinancialReportAnalysisProgressMock.mockResolvedValue(idleAnalysisProgress);
  });

  it("shows the Chinese empty state and fetch action", async () => {
    renderPage();

    expect(screen.queryByText("全量财报分析")).not.toBeInTheDocument();
    expect(screen.queryByText("拉取近两年 AKShare 全量财报数据，缓存后为自选股票池生成 AI 财报结论。")).not.toBeInTheDocument();
    expect(screen.getByText("暂无本地财报缓存。请先拉取近两年全量财报。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "拉取近两年全量财报" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "财报拉取进度" })).toHaveAttribute("aria-valuenow", "0");
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
    expect(screen.getByRole("progressbar", { name: "财报拉取进度" })).toHaveAttribute("aria-valuenow", "33");
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
          stockName: "浦发银行",
          financialScore: 88,
          categoryScores: {
            revenueQuality: 7,
            grossMargin: 8,
            netProfitReturn: 10,
            earningsManipulation: 4,
            solvency: 12,
            cashFlow: 13,
            growth: 9,
            researchCapital: 7,
            operatingEfficiency: 8,
            assetQuality: 4,
          },
          radarScores: {
            profitability: 8.18,
            authenticity: 8.46,
            cashGeneration: 8.67,
            safety: 8,
            growthPotential: 8,
            operatingEfficiency: 8,
          },
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
    expect(screen.getByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("88")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /浦发银行/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /浦发银行/ }));

    expect(await screen.findByRole("dialog", { name: "浦发银行 财报详情" })).toBeInTheDocument();
    expect(screen.getByText("关键信息总结")).toBeInTheDocument();
    expect(screen.getByText("营收和利润保持改善。")).toBeInTheDocument();
    expect(screen.getByText("财报正向因素")).toBeInTheDocument();
    expect(screen.getByText("现金流质量提升。")).toBeInTheDocument();
    expect(screen.getByText("财报负向因素")).toBeInTheDocument();
    expect(screen.getByText("费用率仍需观察。")).toBeInTheDocument();
    expect(screen.getByText("财报造假嫌疑点")).toBeInTheDocument();
    expect(screen.getByText("暂未发现明显异常。")).toBeInTheDocument();
    expect(screen.queryByText("公告日期")).not.toBeInTheDocument();
    expect(screen.queryByText("股票代码")).not.toBeInTheDocument();
    expect(screen.queryByText("序号")).not.toBeInTheDocument();
    expect(screen.queryByText("股票简称")).not.toBeInTheDocument();
    expect(screen.getByText("营业收入同比")).toBeInTheDocument();
    expect(screen.getByText("12.5%")).toBeInTheDocument();
    expect(screen.getByText("研发费用占比")).toBeInTheDocument();
    expect(screen.getByText("8.88%")).toBeInTheDocument();
    expect(screen.getByText("净利润环比")).toBeInTheDocument();
    expect(screen.getByText("-3.25%")).toBeInTheDocument();
    expect(screen.getByText("其他收益")).toBeInTheDocument();
    expect(screen.getByText("无")).toBeInTheDocument();
    expect(screen.getByText("1,234,567.89")).toBeInTheDocument();
    expect(screen.getByText("20,250.25")).toBeInTheDocument();
    expect(screen.queryByText("600000")).not.toBeInTheDocument();
    expect(screen.getByText("子维度评分")).toBeInTheDocument();
    expect(screen.getByText("能力雷达")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "财报评分雷达图" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "财报子维度评分条形图" })).toBeInTheDocument();

    const radar = screen.getByRole("img", { name: "财报评分雷达图" });
    vi.spyOn(radar, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 200,
      bottom: 200,
      width: 200,
      height: 200,
      toJSON: () => ({}),
    });
    fireEvent.mouseMove(radar, { clientX: 100, clientY: 0 });
    expect(screen.getByLabelText("盈利性评分说明")).toHaveTextContent("得分：8.18/10");
    expect(screen.getByLabelText("盈利性评分说明")).toHaveTextContent("评分标准：净利与回报");
    fireEvent.mouseMove(radar, { clientX: 10, clientY: 55 });
    expect(screen.getByLabelText("真实性评分说明")).toHaveTextContent("得分：8.46/10");
    expect(screen.getByLabelText("真实性评分说明")).toHaveTextContent("评分标准：盈利调节");
    fireEvent.mouseMove(radar, { clientX: 190, clientY: 55 });
    expect(screen.getByLabelText("运转效率评分说明")).toHaveTextContent("得分：8/10");

    await user.click(screen.getByRole("button", { name: "AI财报分析" }));
    expect(await screen.findByRole("dialog", { name: "选择参与财报 AI 分析的股票" })).toBeInTheDocument();
    await user.click(screen.getAllByRole("checkbox")[0]);
    await user.click(screen.getByRole("button", { name: "开始财报分析（1）" }));
    await waitFor(() => {
      expect(startFinancialReportAnalysisMock).toHaveBeenCalledWith(["SZSE.000001"]);
    });
  });

  it("shows in-page financial analysis progress states", async () => {
    getFinancialReportOverviewMock.mockResolvedValue({
      stockCount: 1,
      rowCount: 1,
      refreshedAt: "2026-05-08T02:00:00Z",
      sections: [
        {
          section: "income_statement",
          label: "利润表",
          source: "akshare:stock_lrb_em",
          rowCount: 1,
        },
      ],
      analyses: [],
    });
    getFinancialReportAnalysisProgressMock.mockResolvedValue({
      status: "running",
      completedCount: 1,
      totalCount: 4,
      message: "正在分析自选股票池财报",
      items: [
        { stockCode: "SHSE.600000", shortName: "浦发银行", status: "succeeded", attempt: 1, errorMessage: null },
        { stockCode: "SZSE.000001", shortName: "平安银行", status: "running", attempt: 1, errorMessage: null },
        { stockCode: "SZSE.000858", shortName: "五粮液", status: "retrying", attempt: 2, errorMessage: "JSON 解析失败" },
        { stockCode: "SHSE.600519", shortName: "贵州茅台", status: "failed", attempt: 3, errorMessage: "超时" },
      ],
    });

    renderPage();

    expect(await screen.findByText("正在分析自选股票池财报")).toBeInTheDocument();
    expect(screen.getByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("平安银行")).toBeInTheDocument();
    expect(screen.getByText("五粮液")).toBeInTheDocument();
    expect(screen.getByText("贵州茅台")).toBeInTheDocument();
    expect(screen.getByText("2026-05-08 10:00:00")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "财报分析进度" })).toBeInTheDocument();
  });

  it("blocks empty financial analysis selection", async () => {
    const user = userEvent.setup();
    getFinancialReportOverviewMock.mockResolvedValue({
      ...emptyOverview,
      stockCount: 1,
      rowCount: 1,
    });
    renderPage();

    await user.click(screen.getByRole("button", { name: "AI财报分析" }));
    expect(await screen.findByRole("dialog", { name: "选择参与财报 AI 分析的股票" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "清空" }));
    expect(screen.getByText("至少选择 1 只股票才能继续。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始财报分析（0）" })).toBeDisabled();
  });
});
