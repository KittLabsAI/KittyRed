import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelFinancialReportFetch,
  getFinancialReportAnalysis,
  getFinancialReportFetchProgress,
  getFinancialReportOverview,
  getFinancialReportSnapshot,
  listArbitrageOpportunities,
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

  it("invokes financial report fetch and cancel commands without stock selection", async () => {
    mocks.invoke.mockResolvedValue(undefined);

    await startFinancialReportFetch();
    await cancelFinancialReportFetch();
    await startFinancialReportAnalysis();

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_financial_report_fetch");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "cancel_financial_report_fetch");
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "start_financial_report_analysis");
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
