import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { BacktestPage } from "./BacktestPage";
import * as tauri from "../../lib/tauri";

vi.mock("echarts", () => ({
  default: undefined,
  init: () => ({
    setOption: vi.fn(),
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
  createBacktestDatasetMock,
  createBacktestMock,
  startGenerateBacktestSignalsMock,
  startReplayBacktestMock,
  startFetchSnapshotsMock,
} = vi.hoisted(() => ({
  createBacktestDatasetMock: vi.fn(async () => ({
    datasetId: "dataset-2",
    name: "5月浦发数据",
    status: "pending",
    symbols: ["SHSE.600000"],
    startDate: "2026-05-01",
    endDate: "2026-05-07",
    intervalMinutes: 30,
    totalSnapshots: 0,
    fetchedCount: 0,
    estimatedLlmCalls: 48,
    createdAt: "2026-05-07T10:00:00+08:00",
  })),
  createBacktestMock: vi.fn(async () => ({
    backtestId: "bt-2",
    datasetId: "dataset-1",
    name: "5月AI回测",
    status: "pending",
    modelProvider: "OpenAI-compatible",
    modelName: "gpt-5.5",
    promptVersion: "recommendation-system-v2",
    maxHoldingDays: 7,
    totalAiCalls: 0,
    processedAiCalls: 0,
    totalTimepoints: 0,
    processedTimepoints: 0,
    totalSignals: 0,
    tradeSignals: 0,
    openTrades: 0,
    winCount: 0,
    lossCount: 0,
    flatCount: 0,
    totalPnlCny: 0,
    totalPnlPercent: 0,
    maxDrawdownPercent: 0,
    createdAt: "2026-05-07T10:00:00+08:00",
  })),
  startGenerateBacktestSignalsMock: vi.fn(async () => undefined),
  startReplayBacktestMock: vi.fn(async () => undefined),
  startFetchSnapshotsMock: vi.fn(async () => undefined),
}));

vi.mock("../../lib/tauri", () => ({
  createBacktestDataset: createBacktestDatasetMock,
  startFetchSnapshots: startFetchSnapshotsMock,
  cancelFetchSnapshots: vi.fn(async () => undefined),
  listMarkets: vi.fn(async () => [
    {
      symbol: "SHSE.600000",
      baseAsset: "浦发银行",
      marketType: "沪市A股",
      marketSizeTier: "large",
      last: 9.16,
      change24h: -0.22,
      volume24h: 245736322,
      spreadBps: 0,
      venues: ["akshare"],
      updatedAt: "2026-05-07 11:19:19",
    },
    {
      symbol: "SZSE.000001",
      baseAsset: "平安银行",
      marketType: "深市A股",
      marketSizeTier: "large",
      last: 11.32,
      change24h: -0.35,
      volume24h: 539031999,
      spreadBps: 0,
      venues: ["akshare"],
      updatedAt: "2026-05-07 11:19:21",
    },
  ]),
  listBacktestDatasets: vi.fn(async () => [
    {
      datasetId: "dataset-1",
      name: "4月A股数据",
      status: "ready",
      symbols: ["SHSE.600000"],
      startDate: "2026-04-01",
      endDate: "2026-04-03",
      intervalMinutes: 30,
      totalSnapshots: 48,
      fetchedCount: 48,
      estimatedLlmCalls: 48,
      errorMessage: "拉取完成，46 个快照可用，2 个股票-时间点失败，可在失败记录中查看。",
      createdAt: "2026-05-07T09:00:00+08:00",
      completedAt: "2026-05-07T09:05:00+08:00",
    },
  ]),
  getBacktestFetchProgress: vi.fn(async () => ({
    datasetId: "dataset-1",
    status: "fetching",
    totalSnapshots: 48,
    fetchedCount: 18,
    failureCount: 1,
    errorMessage: "拉取中，已记录 1 个失败股票-时间点。",
    recentFailures: [
      {
        failureId: "failure-1",
        datasetId: "dataset-1",
        symbol: "SHSE.600000",
        capturedAt: "2026-04-01T10:00:00+08:00",
        timeframe: "1h",
        stage: "history_bars",
        reason: "SHSE.600000 1h K 线拉取失败",
        errorDetail: "sina ssl disconnected",
        createdAt: "2026-05-07T09:01:00+08:00",
      },
    ],
  })),
  listBacktestFetchFailures: vi.fn(async () => [
    {
      failureId: "failure-1",
      datasetId: "dataset-1",
      symbol: "SHSE.600000",
      capturedAt: "2026-04-01T10:00:00+08:00",
      timeframe: "1h",
      stage: "history_bars",
      reason: "SHSE.600000 1h K 线拉取失败",
      errorDetail: "sina ssl disconnected",
      createdAt: "2026-05-07T09:01:00+08:00",
    },
  ]),
  deleteBacktestDataset: vi.fn(async () => undefined),
  createBacktest: createBacktestMock,
  startBacktest: startGenerateBacktestSignalsMock,
  startGenerateBacktestSignals: startGenerateBacktestSignalsMock,
  startReplayBacktest: startReplayBacktestMock,
  cancelBacktest: vi.fn(async () => undefined),
  listBacktestRuns: vi.fn(async () => [
    {
      backtestId: "bt-1",
      datasetId: "dataset-1",
      name: "4月AI回测",
      status: "completed",
      modelProvider: "OpenAI-compatible",
      modelName: "gpt-5.5",
      promptVersion: "recommendation-system-v2",
      maxHoldingDays: 7,
      totalAiCalls: 48,
      processedAiCalls: 48,
      totalTimepoints: 24,
      processedTimepoints: 24,
      totalSignals: 48,
      tradeSignals: 8,
      openTrades: 0,
      winCount: 5,
      lossCount: 3,
      flatCount: 0,
      totalPnlCny: 1260,
      totalPnlPercent: 6.3,
      maxDrawdownPercent: 1.8,
      profitFactor: 1.7,
      createdAt: "2026-05-07T09:10:00+08:00",
      completedAt: "2026-05-07T09:20:00+08:00",
    },
  ]),
  deleteBacktest: vi.fn(async () => undefined),
  listBacktestSignals: vi.fn(async () => [
    {
      signalId: "sig-1",
      backtestId: "bt-1",
      symbol: "SHSE.600000",
      stockName: "浦发银行",
      capturedAt: "2026-04-01T10:00:00+08:00",
      hasTrade: true,
      direction: "买入",
      confidenceScore: 72,
      riskStatus: "approved",
      rationale: "历史 K 线回踩后放量。",
      result: "opened",
    },
  ]),
  listBacktestTrades: vi.fn(async () => [
    {
      tradeId: "trade-1",
      backtestId: "bt-1",
      signalId: "sig-1",
      symbol: "SHSE.600000",
      stockName: "浦发银行",
      direction: "long",
      entryPrice: 8.7,
      entryAt: "2026-04-01T10:00:00+08:00",
      exitPrice: 8.95,
      exitAt: "2026-04-02T10:00:00+08:00",
      exitReason: "take_profit",
      amountCny: 20000,
      holdingPeriods: 8,
      pnlCny: 534.7,
      pnlPercent: 2.67,
    },
  ]),
  getBacktestSummary: vi.fn(async () => ({
    backtestId: "bt-1",
    totalSignals: 48,
    tradeCount: 8,
    winRate: 62.5,
    totalPnlCny: 1260,
    totalPnlPercent: 6.3,
    maxDrawdownPercent: 1.8,
    profitFactor: 1.7,
    equityCurve: [
      { capturedAt: "2026-04-01T10:00:00+08:00", cumulativePnlPercent: 0 },
      { capturedAt: "2026-04-02T10:00:00+08:00", cumulativePnlPercent: 2.67 },
    ],
    openPositions: [
      {
        signalId: "sig-open-1",
        symbol: "SZSE.000001",
        stockName: "平安银行",
        entryPrice: 11.2,
        entryAt: "2026-04-03T10:00:00+08:00",
        markPrice: 11.45,
        amountCny: 18000,
        holdingPeriods: 3,
        unrealizedPnlCny: 369.64,
        unrealizedPnlPercent: 2.05,
      },
    ],
  })),
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <BacktestPage />
    </QueryClientProvider>,
  );
}

describe("BacktestPage", () => {
  it("renders the Chinese four-step backtest workflow", async () => {
    renderPage();

    expect(await screen.findByText("AI回测工作台")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "拉取数据" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "生成AI信号" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "回放交易" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "回测结果分析" })).toBeInTheDocument();
    expect(await screen.findByText("4月A股数据")).toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "AI 调用" })).not.toBeInTheDocument();
  });

  it("creates a dataset and starts a backtest from ready data", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.clear(await screen.findByLabelText("数据集名称"));
    await user.type(screen.getByLabelText("数据集名称"), "5月浦发数据");
    await waitFor(() => expect(screen.getByText("浦发银行")).toBeInTheDocument());
    await user.click(screen.getAllByLabelText(/浦发银行/)[0]);
    await user.click(screen.getAllByLabelText(/平安银行/)[0]);
    await user.click(screen.getByRole("button", { name: "创建数据集" }));
    await waitFor(() => expect(createBacktestDatasetMock).toHaveBeenCalled());
    const datasetCalls = createBacktestDatasetMock.mock.calls as unknown as Array<[{ symbols: string[] }]>;
    const request = datasetCalls[0][0];
    expect(request.symbols).toEqual(expect.arrayContaining(["SHSE.600000", "SZSE.000001"]));
    expect(request.symbols).toHaveLength(2);

    await user.click(screen.getByRole("tab", { name: "生成AI信号" }));
    await user.click(screen.getByRole("button", { name: "生成AI信号" }));
    await waitFor(() => expect(createBacktestMock).toHaveBeenCalled());
    expect(startGenerateBacktestSignalsMock).toHaveBeenCalledWith("bt-2");
  });

  it("shows fetch progress and completed failure records", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "拉取" }));
    await waitFor(() => expect(startFetchSnapshotsMock).toHaveBeenCalledWith("dataset-1"));

    expect(await screen.findByText("拉取进度")).toBeInTheDocument();
    expect(screen.getByText("已拉取 18/48")).toBeInTheDocument();
    expect(screen.getByText("失败 1")).toBeInTheDocument();
    expect(screen.getAllByText("SHSE.600000")[0]).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "查看失败股票-时间点" }));
    expect(await screen.findByLabelText("失败股票-时间点")).toBeInTheDocument();
    expect(screen.getAllByText("SHSE.600000 1h K 线拉取失败")[0]).toBeInTheDocument();
  });

  it("shows completed summary, signal detail, and trade result", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("tab", { name: "回测结果分析" }));

    expect(await screen.findByText("总收益")).toBeInTheDocument();
    expect(screen.getByText("+6.30%")).toBeInTheDocument();
    expect(screen.getByText("开仓 + 平仓")).toBeInTheDocument();
    expect(screen.getByLabelText("收益曲线")).toBeInTheDocument();
    expect(screen.getByText("历史 K 线回踩后放量。")).toBeInTheDocument();
    expect(screen.getByText("止盈")).toBeInTheDocument();
    expect(screen.getAllByText("+2.67%").length).toBeGreaterThan(0);
    expect(screen.getByText("最终持仓")).toBeInTheDocument();
    expect(screen.getByText("总资产")).toBeInTheDocument();
    expect(screen.getByText("总现金")).toBeInTheDocument();
    expect(screen.getByText("总市值")).toBeInTheDocument();
    expect(screen.getByText("总盈亏")).toBeInTheDocument();
    expect(screen.getAllByText("平安银行").length).toBeGreaterThan(0);
    expect(screen.getAllByText("无").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+2.05%").length).toBeGreaterThan(0);
    expect(screen.getByText("+6.30%")).toHaveClass("positive-text");
    expect(screen.getAllByText("¥1,260").find((node) => node.tagName === "STRONG")).toHaveClass("positive-text");
    expect(
      screen.getByText("模拟开平仓").compareDocumentPosition(screen.getByText("信号明细")),
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(screen.queryByText("未实现盈亏")).not.toBeInTheDocument();
  });

  it("paginates trade cards and filters by stock", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.listBacktestTrades).mockResolvedValueOnce(
      Array.from({ length: 11 }, (_, index) => ({
        tradeId: `trade-${index + 1}`,
        backtestId: "bt-1",
        signalId: `sig-${index + 1}`,
        symbol: `SHSE.600${String(index + 1).padStart(3, "0")}`,
        stockName: `股票${index + 1}`,
        direction: "long",
        entryPrice: 8 + index * 0.1,
        entryAt: `2026-04-${String(index + 1).padStart(2, "0")}T10:00:00+08:00`,
        exitPrice: 8.5 + index * 0.1,
        exitAt: `2026-04-${String(index + 1).padStart(2, "0")}T15:00:00+08:00`,
        exitReason: "take_profit",
        amountCny: 10000 + index * 100,
        holdingPeriods: 2,
        pnlCny: 100 + index,
        pnlPercent: 1 + index * 0.1,
      })),
    );
    vi.mocked(tauri.getBacktestSummary).mockResolvedValueOnce({
      backtestId: "bt-1",
      totalSignals: 48,
      tradeCount: 8,
      winRate: 62.5,
      totalPnlCny: 1260,
      totalPnlPercent: 6.3,
      maxDrawdownPercent: 1.8,
      profitFactor: 1.7,
      equityCurve: [
        { capturedAt: "2026-04-01T10:00:00+08:00", cumulativePnlPercent: 0 },
        { capturedAt: "2026-04-02T10:00:00+08:00", cumulativePnlPercent: 2.67 },
      ],
      openPositions: [
        {
          signalId: "sig-open-1",
          symbol: "SZSE.000001",
          stockName: "平安银行",
          entryPrice: 11.2,
          entryAt: "2026-05-01T10:00:00+08:00",
          markPrice: 11.45,
          amountCny: 18000,
          holdingPeriods: 3,
          unrealizedPnlCny: 369.64,
          unrealizedPnlPercent: 2.05,
        },
      ],
    });

    renderPage();

    await user.click(await screen.findByRole("tab", { name: "回测结果分析" }));
    expect(await screen.findByText("第 1 / 2 页，每页 10 条")).toBeInTheDocument();
    const tradeStockFilter = screen.getAllByRole("combobox", { name: "筛选个股" })[1];
    await user.click(screen.getByRole("button", { name: "下一页" }));
    await waitFor(() => expect(screen.getByText("第 2 / 2 页，每页 10 条")).toBeInTheDocument());
    expect(screen.getAllByText("无").length).toBeGreaterThan(0);

    await user.selectOptions(tradeStockFilter, "SHSE.600000");
    expect(screen.getAllByText("浦发银行").length).toBeGreaterThan(0);
    expect(screen.queryByText(/第 1 \/ 1 页/)).not.toBeInTheDocument();
  });
});
