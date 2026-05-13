import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SentimentAnalysisPage } from "./SentimentAnalysisPage";
import type {
  SentimentAnalysisProgress,
  SentimentAnalysisResult,
  SentimentDiscussionSnapshot,
  SentimentFetchProgress,
} from "../../lib/types";

const mocks = vi.hoisted(() => ({
  chartSetOption: vi.fn(),
  echartsInit: vi.fn(() => ({
    setOption: mocks.chartSetOption,
    resize: vi.fn(),
    dispose: vi.fn(),
  })),
  getSentimentFetchProgress: vi.fn<() => Promise<SentimentFetchProgress>>(),
  getSentimentAnalysisProgress: vi.fn<() => Promise<SentimentAnalysisProgress>>(),
  getSentimentAnalysisResults: vi.fn<() => Promise<SentimentAnalysisResult[]>>(),
  getSentimentDiscussionSnapshot: vi.fn<(_: string) => Promise<SentimentDiscussionSnapshot | null>>(async () => null),
  listMarkets: vi.fn(async () => [
    { symbol: "SHSE.600000", baseAsset: "浦发银行", marketType: "沪市A股", marketSizeTier: "large", last: 8.72, change24h: 0.81, volume24h: 1260000000, spreadBps: 0, venues: ["akshare"], updatedAt: "2026-05-08T10:00:00+08:00" },
    { symbol: "SHSE.600900", baseAsset: "长江电力", marketType: "沪市A股", marketSizeTier: "large", last: 28.1, change24h: 0.2, volume24h: 520000000, spreadBps: 0, venues: ["akshare"], updatedAt: "2026-05-08T10:00:00+08:00" },
  ]),
  startSentimentDiscussionFetch: vi.fn(async () => undefined),
  cancelSentimentDiscussionFetch: vi.fn(async () => undefined),
  startSentimentAnalysis: vi.fn(async () => undefined),
}));

vi.mock("echarts", () => ({
  default: undefined,
  init: mocks.echartsInit,
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

vi.mock("../../lib/tauri", () => ({
  getSentimentFetchProgress: mocks.getSentimentFetchProgress,
  getSentimentAnalysisProgress: mocks.getSentimentAnalysisProgress,
  getSentimentAnalysisResults: mocks.getSentimentAnalysisResults,
  getSentimentDiscussionSnapshot: mocks.getSentimentDiscussionSnapshot,
  listMarkets: mocks.listMarkets,
  startSentimentDiscussionFetch: mocks.startSentimentDiscussionFetch,
  cancelSentimentDiscussionFetch: mocks.cancelSentimentDiscussionFetch,
  startSentimentAnalysis: mocks.startSentimentAnalysis,
}));

const idleFetch: SentimentFetchProgress = {
  status: "idle",
  completedCount: 0,
  totalCount: 0,
  message: "尚未开始社媒平台讨论拉取",
  items: [],
};

const idleAnalysis: SentimentAnalysisProgress = {
  status: "idle",
  completedCount: 0,
  totalCount: 0,
  message: "尚未开始 AI 舆情分析",
  items: [],
};

function renderPage() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <SentimentAnalysisPage />
    </QueryClientProvider>,
  );
}

describe("SentimentAnalysisPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getSentimentFetchProgress.mockResolvedValue(idleFetch);
    mocks.getSentimentAnalysisProgress.mockResolvedValue(idleAnalysis);
    mocks.getSentimentAnalysisResults.mockResolvedValue([]);
    mocks.getSentimentDiscussionSnapshot.mockResolvedValue(null);
  });

  it("shows stock names in fetch progress and filters AI analysis choices to stocks with cached discussions", async () => {
    const user = userEvent.setup();
    mocks.getSentimentFetchProgress.mockResolvedValue({
      status: "completed",
      completedCount: 9,
      totalCount: 9,
      message: "社媒平台讨论拉取完成",
      items: [
        {
          stockCode: "SHSE.600900",
          shortName: "SHSE",
          platformStatuses: [{ platform: "xueqiu", status: "succeeded", itemCount: 2, errorMessage: null }],
        },
      ],
    });
    mocks.getSentimentDiscussionSnapshot.mockImplementation(async (stockCode: string) =>
      stockCode === "SHSE.600900"
        ? {
            stockCode,
            stockName: "长江电力",
            sourceRevision: "rev",
            items: [
              {
                platform: "xueqiu",
                title: "长江电力讨论",
                text: "长江电力分红稳定。",
                author: "投资者",
                publishedAt: null,
                url: null,
                engagement: {},
                fetchedAt: "2026-05-12T10:00:00+08:00",
                raw: {},
              },
            ],
            platformStatuses: [],
            fetchedAt: "2026-05-12T10:00:00+08:00",
          }
        : null,
    );
    renderPage();

    expect(await screen.findByText("长江电力")).toBeInTheDocument();
    expect(screen.queryByText("SHSE", { selector: "strong" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "AI舆情分析" }));

    const dialog = await screen.findByRole("dialog", { name: "选择参与 AI 舆情分析的股票" });
    expect(within(dialog).getByText("长江电力")).toBeInTheDocument();
    expect(within(dialog).queryByText("浦发银行")).not.toBeInTheDocument();
  });

  it("renders the two-step sentiment workflow and starts fetch from watchlist modal", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("舆情分析")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "拉取社媒平台讨论" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "中断拉取" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "AI舆情分析" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "拉取社媒平台讨论" }));
    await user.click(await screen.findByRole("button", { name: "开始拉取" }));

    expect(mocks.startSentimentDiscussionFetch).toHaveBeenCalledWith(["SHSE.600000", "SHSE.600900"]);
  });

  it("renders platform-by-stock fetch progress", async () => {
    mocks.getSentimentFetchProgress.mockResolvedValue({
      status: "running",
      completedCount: 1,
      totalCount: 9,
      message: "正在拉取社媒平台讨论",
      items: [
        {
          stockCode: "SHSE.600000",
          shortName: "浦发银行",
          platformStatuses: [
            { platform: "zhihu", status: "succeeded", itemCount: 2, errorMessage: null },
            { platform: "xueqiu", status: "retrying", itemCount: 0, errorMessage: "需要登录" },
          ],
        },
      ],
    });

    renderPage();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("知乎")).toBeInTheDocument();
    expect(screen.getByText("雪球")).toBeInTheDocument();
    expect(screen.getByLabelText("成功")).toBeInTheDocument();
    expect(screen.getByLabelText("重试中")).toBeInTheDocument();
  });

  it("shows total fetched discussion count beside each fetch status stock title", async () => {
    mocks.getSentimentFetchProgress.mockResolvedValue({
      status: "completed",
      completedCount: 2,
      totalCount: 2,
      message: "社媒平台讨论拉取完成",
      items: [
        {
          stockCode: "SHSE.600900",
          shortName: "长江电力",
          platformStatuses: [
            { platform: "zhihu", status: "succeeded", itemCount: 3, errorMessage: null },
            { platform: "xueqiu", status: "succeeded", itemCount: 2, errorMessage: null },
          ],
        },
      ],
    });

    renderPage();

    expect(await screen.findByText("长江电力")).toBeInTheDocument();
    expect(screen.getByText("共 5 条舆论")).toBeInTheDocument();
  });

  it("uses watchlist stock names in AI analysis status and hides the empty AI conclusion copy", async () => {
    mocks.getSentimentAnalysisProgress.mockResolvedValue({
      status: "running",
      completedCount: 0,
      totalCount: 1,
      message: "正在进行 AI 舆情分析",
      items: [
        {
          stockCode: "SHSE.600900",
          shortName: "SHSE",
          status: "running",
          attempt: 1,
          errorMessage: null,
        },
      ],
    });

    renderPage();

    expect(await screen.findByText("长江电力")).toBeInTheDocument();
    expect(screen.queryByText("SHSE", { selector: "strong" })).not.toBeInTheDocument();
    expect(screen.queryByText("暂无 AI 舆情结论。完成社媒平台讨论拉取后可以分析自选股票池。")).not.toBeInTheDocument();
  });

  it("sorts AI sentiment conclusions and opens detail modal", async () => {
    const user = userEvent.setup();
    mocks.getSentimentAnalysisResults.mockResolvedValue([
      sentimentResult("SZSE.000001", "平安银行", 61),
      sentimentResult("SHSE.600000", "浦发银行", 82),
    ]);

    renderPage();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    const rows = screen.getAllByRole("button", { name: /舆情总分/ });
    expect(rows[0]).toHaveTextContent("浦发银行");
    expect(rows[0]).toHaveTextContent("82");

    await user.click(rows[0]);

    expect(await screen.findByRole("dialog", { name: "浦发银行 AI舆情详情" })).toBeInTheDocument();
    expect(screen.getAllByText("情感倾向").length).toBeGreaterThan(0);
    expect(screen.getByText("雪球讨论偏正面，知乎引用了业绩改善。")).toBeInTheDocument();
    expect(screen.getByLabelText("舆情维度条形图")).toBeInTheDocument();
    expect(screen.getByLabelText("舆情维度雷达图")).toBeInTheDocument();
    expect(mocks.echartsInit).toHaveBeenCalledTimes(2);

    const radar = screen.getByLabelText("舆情维度雷达图");
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
    expect(screen.getByLabelText("情感倾向评分说明")).toHaveTextContent("得分：82/100");
    expect(screen.getByLabelText("情感倾向评分说明")).toHaveTextContent("评分标准：50 分为中性");
    fireEvent.mouseMove(radar, { clientX: 10, clientY: 55 });
    expect(screen.getByLabelText("关注热度评分说明")).toHaveTextContent("得分：70/100");
    expect(screen.getByLabelText("关注热度评分说明")).toHaveTextContent("评分标准：讨论量越多");
    fireEvent.mouseMove(radar, { clientX: 190, clientY: 55 });
    expect(screen.getByLabelText("舆论共识度评分说明")).toHaveTextContent("得分：74/100");
  });
});

function sentimentResult(stockCode: string, stockName: string, totalScore: number): SentimentAnalysisResult {
  return {
    stockCode,
    stockName,
    totalScore,
    sentiment: { score: totalScore, reason: "雪球讨论偏正面，知乎引用了业绩改善。" },
    attention: { score: 70, reason: "微博、雪球、百度均有讨论。" },
    momentum: { score: 60, reason: "近两日讨论增加但未爆发。" },
    impact: { score: 68, reason: "涉及业绩快报和行业政策。" },
    reliability: { score: 58, reason: "部分来源有链接，匿名评论较多。" },
    consensus: { score: 74, reason: "多数讨论方向一致。" },
    sourceRevision: "rev-1",
    modelProvider: "openai",
    modelName: "gpt",
    generatedAt: "2026-05-12T10:30:00+08:00",
  };
}
