import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelFinancialReportFetch,
  getRecommendationGenerationProgress,
  getFinancialReportAnalysis,
  getFinancialReportFetchProgress,
  getFinancialReportOverview,
  getFinancialReportSnapshot,
  getSentimentAnalysisProgress,
  getSentimentAnalysisResults,
  getSentimentDiscussionSnapshot,
  getSentimentFetchProgress,
  getSentimentPlatformAuthStatuses,
  testSentimentPlatformConnections,
  captureSentimentPlatformLoginState,
  startSentimentAnalysis,
  startSentimentDiscussionFetch,
  listArbitrageOpportunities,
  startRecommendationGeneration,
  startFinancialReportAnalysis,
  startFinancialReportFetch,
} from "./tauri";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

describe("listArbitrageOpportunities", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("maps paginated arbitrage candidates from the tauri runtime", async () => {
    mocks.invoke.mockResolvedValue({
      items: [
        {
          symbol: "BTC/USDT",
          opportunity_type: "spot_long_perp_short_cross_exchange",
          primary_market_type: "spot",
          secondary_market_type: "perpetual",
          buy_exchange: "akshare",
          buy_market_type: "spot",
          buy_price: 68_420,
          sell_exchange: "模拟账户",
          sell_market_type: "perpetual",
          sell_price: 68_719,
          fee_adjusted_net_spread_pct: 0.18,
          simulated_carry_pct: -0.0492,
          simulated_total_yield_pct: 0.1308,
          liquidity_usdt_24h: 220_000_000,
          market_cap_usd: 1_300_000_000_000,
          funding_rate: 0.0008,
          borrow_rate_daily: null,
          recommendation_score: 91.2,
          updated_at: "2026-05-04T18:30:00+08:00",
          stale: false,
        },
      ],
      total: 1,
      page: 1,
      page_size: 25,
      total_pages: 1,
    });

    const page = await listArbitrageOpportunities(1, 25, "cross_market");

    expect(mocks.invoke).toHaveBeenCalledWith("list_arbitrage_opportunities", {
      page: 1,
      pageSize: 25,
      typeFilter: "cross_market",
    });
    expect(page).toEqual({
      items: [
        {
          symbol: "BTC/USDT",
          opportunityType: "spot_long_perp_short_cross_exchange",
          primaryMarketType: "spot",
          secondaryMarketType: "perpetual",
          buyExchange: "akshare",
          buyMarketType: "spot",
          buyPrice: 68_420,
          sellExchange: "模拟账户",
          sellMarketType: "perpetual",
          sellPrice: 68_719,
          feeAdjustedNetSpreadPct: 0.18,
          simulatedCarryPct: -0.0492,
          simulatedTotalYieldPct: 0.1308,
          liquidity24h: 220_000_000,
          marketCapUsd: 1_300_000_000_000,
          fundingRate: 0.0008,
          borrowRateDaily: undefined,
          recommendationScore: 91.2,
          updatedAt: "2026-05-04T18:30:00+08:00",
          stale: false,
        },
      ],
      total: 1,
      page: 1,
      pageSize: 25,
      totalPages: 1,
    });
  });
});

describe("financial report tauri bridge", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("invokes financial report fetch and cancel commands with selected watchlist symbols", async () => {
    mocks.invoke.mockResolvedValue(undefined);

    await startFinancialReportFetch();
    await cancelFinancialReportFetch();
    await startFinancialReportAnalysis(["SHSE.600000"]);

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_financial_report_fetch");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "cancel_financial_report_fetch");
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "start_financial_report_analysis", {
      selectedSymbols: ["SHSE.600000"],
    });
  });

  it("reads financial report progress, overview, snapshot, and cached analysis", async () => {
    mocks.invoke
      .mockResolvedValueOnce({
        stockCode: "ALL",
        status: "running",
        completedSections: 3,
        totalSections: 6,
        message: "正在缓存 现金流量表",
        errorMessage: null,
      })
      .mockResolvedValueOnce({
        stockCount: 2,
        rowCount: 10,
        refreshedAt: "2026-05-08T10:00:00+08:00",
        sections: [],
        analyses: [],
      })
      .mockResolvedValueOnce({
        stockCode: "SHSE.600000",
        sections: [],
        sourceRevision: "rev-1",
        refreshedAt: "2026-05-08T10:00:00+08:00",
        metricSeries: [],
        analysis: null,
      })
      .mockResolvedValueOnce({
        stockCode: "SHSE.600000",
        sourceRevision: "rev-1",
        keySummary: "经营稳定。",
        positiveFactors: "现金流改善。",
        negativeFactors: "费用承压。",
        fraudRiskPoints: "暂无明显异常。",
        generatedAt: "2026-05-08T10:05:00+08:00",
        stale: false,
      });

    await expect(getFinancialReportFetchProgress()).resolves.toMatchObject({
      status: "running",
      completedSections: 3,
    });
    await expect(getFinancialReportOverview()).resolves.toMatchObject({
      stockCount: 2,
      rowCount: 10,
    });
    await expect(getFinancialReportSnapshot("SHSE.600000")).resolves.toMatchObject({
      stockCode: "SHSE.600000",
      sourceRevision: "rev-1",
    });
    await expect(getFinancialReportAnalysis("SHSE.600000")).resolves.toMatchObject({
      keySummary: "经营稳定。",
      fraudRiskPoints: "暂无明显异常。",
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "get_financial_report_fetch_progress");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "get_financial_report_overview");
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "get_financial_report_snapshot", {
      stockCode: "SHSE.600000",
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "get_financial_report_analysis", {
      stockCode: "SHSE.600000",
    });
  });
});

describe("sentiment analysis tauri bridge", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("invokes sentiment commands and maps backend payloads without secret values", async () => {
    mocks.invoke
      .mockResolvedValueOnce([
        {
          platform: "zhihu",
          hasLoginState: true,
          capturedAt: "2026-05-12T10:00:00+08:00",
        },
      ])
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([{ platform: "zhihu", ok: true, message: "知乎 连接测试可用" }])
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
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
            ],
          },
        ],
      })
      .mockResolvedValueOnce({
        stockCode: "SHSE.600000",
        stockName: "浦发银行",
        sourceRevision: "rev-1",
        fetchedAt: "2026-05-12T10:00:00+08:00",
        platformStatuses: [{ platform: "zhihu", status: "succeeded", itemCount: 2, errorMessage: null }],
        items: [
          {
            platform: "zhihu",
            title: "浦发银行讨论",
            text: "讨论正文",
            author: "用户A",
            publishedAt: "2026-05-12T09:00:00+08:00",
            url: "https://example.com/1",
            engagement: { likes: 8 },
            fetchedAt: "2026-05-12T10:00:00+08:00",
            raw: { id: "1" },
          },
        ],
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        status: "idle",
        completedCount: 0,
        totalCount: 0,
        message: "尚未开始 AI 舆情分析",
        items: [],
      })
      .mockResolvedValueOnce([
        {
          stockCode: "SHSE.600000",
          stockName: "浦发银行",
          totalScore: 66,
          sentiment: { score: 62, reason: "雪球讨论偏正面。" },
          attention: { score: 70, reason: "多平台有讨论。" },
          momentum: { score: 60, reason: "扩散平稳。" },
          impact: { score: 68, reason: "涉及业绩。" },
          reliability: { score: 58, reason: "来源一般。" },
          consensus: { score: 78, reason: "观点较一致。" },
          sourceRevision: "rev-1",
          modelProvider: "openai",
          modelName: "gpt",
          generatedAt: "2026-05-12T10:30:00+08:00",
        },
      ]);

    const authStatuses = await getSentimentPlatformAuthStatuses();
    expect(authStatuses).toEqual([
      { platform: "zhihu", hasLoginState: true, capturedAt: "2026-05-12T10:00:00+08:00" },
    ]);
    await captureSentimentPlatformLoginState("zhihu");
    await expect(testSentimentPlatformConnections()).resolves.toEqual([
      { platform: "zhihu", ok: true, message: "知乎 连接测试可用" },
    ]);
    await startSentimentDiscussionFetch(["SHSE.600000"]);
    await expect(getSentimentFetchProgress()).resolves.toMatchObject({
      completedCount: 1,
      items: [{ stockCode: "SHSE.600000" }],
    });
    await expect(getSentimentDiscussionSnapshot("SHSE.600000")).resolves.toMatchObject({
      stockCode: "SHSE.600000",
      items: [{ platform: "zhihu", text: "讨论正文" }],
    });
    await startSentimentAnalysis(["SHSE.600000"]);
    await expect(getSentimentAnalysisProgress()).resolves.toMatchObject({
      status: "idle",
      message: "尚未开始 AI 舆情分析",
    });
    await expect(getSentimentAnalysisResults()).resolves.toMatchObject([
      {
        stockCode: "SHSE.600000",
        totalScore: 66,
        sentiment: { score: 62 },
      },
    ]);

    const serializedStatuses = JSON.stringify(authStatuses);
    expect(serializedStatuses).not.toContain("cookie");
    expect(serializedStatuses).not.toContain("secret");
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "get_sentiment_platform_auth_statuses");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "capture_sentiment_platform_login_state", {
      platform: "zhihu",
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "test_sentiment_platform_connections");
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "start_sentiment_discussion_fetch", {
      selectedSymbols: ["SHSE.600000"],
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "get_sentiment_fetch_progress");
    expect(mocks.invoke).toHaveBeenNthCalledWith(6, "get_sentiment_discussion_snapshot", {
      stockCode: "SHSE.600000",
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(7, "start_sentiment_analysis", {
      selectedSymbols: ["SHSE.600000"],
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(8, "get_sentiment_analysis_progress");
    expect(mocks.invoke).toHaveBeenNthCalledWith(9, "get_sentiment_analysis_results");
  });
});

describe("recommendation tauri bridge", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("starts recommendation generation and maps progress payloads", async () => {
    mocks.invoke
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        status: "running",
        completed_count: 1,
        total_count: 3,
        message: "正在生成 AI 建议",
        items: [
          {
            stock_code: "SHSE.600000",
            short_name: "浦发银行",
            status: "running",
            attempt: 1,
            error_message: null,
          },
        ],
      });

    await startRecommendationGeneration(["SHSE.600000", "SZSE.000001"]);
    await expect(getRecommendationGenerationProgress()).resolves.toEqual({
      status: "running",
      completedCount: 1,
      totalCount: 3,
      message: "正在生成 AI 建议",
      items: [
        {
          stockCode: "SHSE.600000",
          shortName: "浦发银行",
          status: "running",
          attempt: 1,
          errorMessage: undefined,
        },
      ],
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_recommendation_generation", {
      selectedSymbols: ["SHSE.600000", "SZSE.000001"],
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "get_recommendation_generation_progress");
  });
});
