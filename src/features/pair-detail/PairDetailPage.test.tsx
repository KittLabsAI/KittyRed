import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PairDetailPage } from "./PairDetailPage";

const mocks = vi.hoisted(() => ({
  createManualPaperOrder: vi.fn(async () => ({
    id: 51,
    kind: "paper.order",
    status: "running",
    message: "模拟委托已加入后台任务",
    startedAt: "2026-05-07T10:00:00+08:00",
    updatedAt: "2026-05-07T10:00:00+08:00",
    endedAt: null,
    durationMs: null,
    inputParamsJson: "{\"symbol\":\"SHSE.600000\",\"quantity\":100}",
    resultSummary: null,
    errorDetails: null,
  })),
  getLatestRecommendation: vi.fn(async () => [{
    id: "rec-1",
    status: "completed",
    hasTrade: true,
    symbol: "SHSE.600000",
    marketType: "A 股",
    direction: "买入",
    confidence: 72,
    riskStatus: "通过",
    thesis: "银行板块修复，价格仍在模拟买入区间内。",
    symbolRecommendations: [
      {
        symbol: "SHSE.600000",
        direction: "买入",
        thesis: "subagent 返回的浦发银行最新建议。",
        riskStatus: "approved",
        hasTrade: true,
      },
      {
        symbol: "SZSE.000001",
        direction: "观望",
        thesis: "subagent 返回的平安银行最新建议。",
        riskStatus: "watch",
        hasTrade: false,
      },
    ],
    riskDetails: { status: "approved", riskScore: 30, checks: [], modifications: [], blockReasons: [] },
    generatedAt: "2026-05-03T19:02:00+08:00",
  }]),
  getPairCandles: vi.fn(async () => ({
    exchange: "akshare",
    symbol: "SHSE.600000",
    marketType: "A 股",
    interval: "1D",
    updatedAt: "2026-05-03T19:00:00+08:00",
    bars: [
      { openTime: "1714734000000", open: 8.68, high: 8.75, low: 8.64, close: 8.72, volume: 820000 },
      { openTime: "1714820400000", open: 8.72, high: 8.81, low: 8.7, close: 8.78, volume: 910000 },
    ],
  })),
  getPairDetail: vi.fn(async () => ({
    symbol: "SHSE.600000",
    marketType: "A 股",
    thesis: "A 股行情来自 AKShare。",
    coinInfo: {
      name: "浦发银行",
      symbol: "SHSE.600000",
      summary: "上海证券交易所 A 股。",
      ecosystem: "沪市 A 股",
      listedExchanges: ["上海证券交易所"],
      riskTags: [],
    },
    venues: [
      {
        exchange: "akshare",
        last: 8.72,
        bid: 8.71,
        ask: 8.73,
        changePct: 0.81,
        volume24h: 130000000,
        updatedAt: "2026-05-03T19:00:00+08:00",
      },
    ],
    orderbooks: [],
    recentTrades: [],
    spreads: [],
  })),
  listMarkets: vi.fn(async () => [
    {
      symbol: "SHSE.600000",
      baseAsset: "浦发银行",
      marketType: "沪市A股",
      marketSizeTier: "large" as const,
      last: 8.88,
      change24h: 1.23,
      volume24h: 123456789,
      spreadBps: 0,
      venues: ["akshare"],
      updatedAt: "2026-05-07T10:30:00+08:00",
    },
  ]),
  listPaperAccounts: vi.fn(async () => [{ accountId: "paper-cash", exchange: "模拟账户", availableUsdt: 1_000_000 }]),
  triggerRecommendation: vi.fn(async () => []),
}));

vi.mock("../../lib/tauri", () => ({
  createManualPaperOrder: mocks.createManualPaperOrder,
  getLatestRecommendation: mocks.getLatestRecommendation,
  getPairCandles: mocks.getPairCandles,
  getPairDetail: mocks.getPairDetail,
  listMarkets: mocks.listMarkets,
  listPaperAccounts: mocks.listPaperAccounts,
  triggerRecommendation: mocks.triggerRecommendation,
}));

vi.mock("./CandlestickChart", () => ({
  CandlestickChart: () => <div data-testid="candlestick-chart" />,
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={["/pair-detail?symbol=SHSE.600000"]}>
        <Routes>
          <Route element={<PairDetailPage />} path="/pair-detail" />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function renderPageWithoutSymbol() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <MemoryRouter initialEntries={["/pair-detail"]}>
        <Routes>
          <Route element={<PairDetailPage />} path="/pair-detail" />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("PairDetailPage", () => {
  const localStorageMap = new Map<string, string>();

  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMap.clear();
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: {
        getItem: vi.fn((key: string) => localStorageMap.get(key) ?? null),
        setItem: vi.fn((key: string, value: string) => {
          localStorageMap.set(key, value);
        }),
        removeItem: vi.fn((key: string) => {
          localStorageMap.delete(key);
        }),
      },
    });
  });

  it("renders an A-share detail page without CEX concepts", async () => {
    renderPage();
    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("SHSE.600000 · 沪深 A 股 · AKShare 行情")).toBeInTheDocument();
    expect(screen.getByText("模拟委托")).toBeInTheDocument();
    expect(screen.queryByText("行情摘要")).not.toBeInTheDocument();
    expect(screen.queryByText("数据边界")).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "账户" })).not.toBeInTheDocument();
    expect(screen.getByTestId("candlestick-chart")).toBeInTheDocument();
    expect(screen.queryByText(/USDT|Spot|Perpetual/i)).not.toBeInTheDocument();
  });

  it("submits a quantity-based CNY paper order as a background task", async () => {
    const user = userEvent.setup();
    renderPage();

    const submitButton = await screen.findByRole("button", { name: "生成模拟委托" });
    await waitFor(() => {
      expect(submitButton).toBeEnabled();
    });
    await user.click(submitButton);

    await waitFor(() => {
      expect(mocks.createManualPaperOrder).toHaveBeenCalledWith(
        expect.objectContaining({
          accountId: "paper-cash",
          symbol: "SHSE.600000",
          marketType: "ashare",
          quantity: 200,
          leverage: 1,
        }),
      );
    });
    expect(await screen.findByText(/模拟委托已加入后台任务/)).toBeInTheDocument();
  });

  it("does not reset to SHSE.600000 when no symbol is provided", async () => {
    renderPageWithoutSymbol();

    expect(await screen.findByText("请先从行情页选择股票")).toBeInTheDocument();
    expect(mocks.getPairDetail).not.toHaveBeenCalledWith("SHSE.600000", expect.anything(), expect.anything());
  });

  it("restores the last selected symbol when returning without a query string", async () => {
    window.localStorage.setItem("kittyred:last-pair-detail-symbol", "SHSE.600000");

    renderPageWithoutSymbol();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(mocks.getPairDetail).toHaveBeenCalledWith("SHSE.600000", "ashare", "akshare");
  });

  it("uses the market row for detail quote price volume and change percent", async () => {
    renderPage();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.getAllByText("¥8.88").length).toBeGreaterThan(0);
    expect(screen.getByText("+1.23%")).toBeInTheDocument();
    expect(screen.getByText("123,456,789")).toBeInTheDocument();
    expect(screen.getByText("涨跌幅")).toBeInTheDocument();
    expect(screen.queryByText("K 线涨跌")).not.toBeInTheDocument();
  });

  it("requests candles for the selected interval", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.selectOptions(await screen.findByRole("combobox"), "1H");

    await waitFor(() => {
      expect(mocks.getPairCandles).toHaveBeenCalledWith("SHSE.600000", "ashare", "1H", "akshare");
    });
  });

  it("shows only the latest per-symbol recommendation in the advice card", async () => {
    const { container } = renderPage();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("subagent 返回的浦发银行最新建议。")).toBeInTheDocument();
    expect(screen.queryByText("历史建议")).not.toBeInTheDocument();
    expect(screen.queryByText("等待下一交易K线：10分钟、60分钟、24小时、7天。")).not.toBeInTheDocument();
    const chartPanel = container.querySelector(".pair-detail-chart-panel");
    const orderPanel = container.querySelector(".paper-trade-card");
    const advicePanel = container.querySelector(".pair-detail-advice-panel");
    expect(chartPanel).toBeInTheDocument();
    expect(orderPanel).toBeInTheDocument();
    expect(advicePanel).toBeInTheDocument();
    expect(
      chartPanel?.compareDocumentPosition(advicePanel as Node) ?? 0,
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(
      orderPanel?.compareDocumentPosition(advicePanel as Node) ?? 0,
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });

  it("omits redundant labels from the aligned paper order card", async () => {
    const { container } = renderPage();

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    const orderPanel = container.querySelector(".paper-trade-card");
    expect(orderPanel).toBeInTheDocument();
    expect(within(orderPanel as HTMLElement).queryByText("SHSE.600000")).not.toBeInTheDocument();
    expect(within(orderPanel as HTMLElement).queryByText("仅模拟")).not.toBeInTheDocument();
    expect(within(orderPanel as HTMLElement).queryByText("资金账户")).not.toBeInTheDocument();
    expect(within(orderPanel as HTMLElement).getByText("可用资金")).toBeInTheDocument();
    expect(within(orderPanel as HTMLElement).getByText("参考价")).toBeInTheDocument();
  });
});
