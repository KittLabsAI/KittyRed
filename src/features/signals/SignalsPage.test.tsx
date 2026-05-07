import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { SignalsPage } from "./SignalsPage";
import * as tauri from "../../lib/tauri";

vi.mock("../../lib/tauri", () => ({
  getStrategyMeta: vi.fn().mockResolvedValue([]),
  getStrategyConfigs: vi.fn().mockResolvedValue([]),
  getStrategyStats: vi.fn().mockResolvedValue([]),
  scanSignals: vi.fn().mockResolvedValue([]),
  listSignalHistory: vi.fn().mockResolvedValue({ items: [], total: 0, page: 1, pageSize: 10 }),
  listMarkets: vi.fn().mockResolvedValue([]),
  updateStrategyConfig: vi.fn().mockResolvedValue({ strategyId: "", enabled: true, params: {} }),
  listScanRuns: vi.fn().mockResolvedValue({ items: [], total: 0, page: 1, pageSize: 25 }),
  executeSignal: vi.fn(),
  dismissSignal: vi.fn(),
}));

function renderSignalsPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SignalsPage />
    </QueryClientProvider>,
  );
}

describe("SignalsPage", () => {
  it("renders the scan button and strategy section", async () => {
    renderSignalsPage();
    expect(screen.getByText("策略信号")).toBeInTheDocument();
    expect(screen.getAllByText("策略列表").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("信号历史")).toBeInTheDocument();
    expect(screen.getByText("扫描记录")).toBeInTheDocument();
  });

  it("shows the empty signals message", async () => {
    renderSignalsPage();
    await waitFor(() => {
      expect(screen.getByText("暂无信号。运行扫描后会在这里显示结果。")).toBeInTheDocument();
    });
  });

  it("counts enabled strategies only from active metadata", async () => {
    vi.mocked(tauri.getStrategyMeta).mockResolvedValueOnce([
      {
        strategyId: "ma_cross",
        name: "均线交叉",
        category: "Trend",
        applicableMarkets: ["a_share"],
        description: "识别均线金叉和死叉",
        defaultParams: {},
      },
      {
        strategyId: "rsi_extreme",
        name: "RSI 超买超卖",
        category: "Momentum",
        applicableMarkets: ["a_share"],
        description: "识别 RSI 超买和超卖",
        defaultParams: {},
      },
    ]);
    vi.mocked(tauri.getStrategyConfigs).mockResolvedValueOnce([
      { strategyId: "ma_cross", enabled: true, params: {} },
      { strategyId: "rsi_extreme", enabled: true, params: {} },
      { strategyId: "spread_arbitrage", enabled: true, params: {} },
    ]);

    renderSignalsPage();

    expect(await screen.findByText(/2 个策略 · 2 个已启用/)).toBeInTheDocument();
  });

  it("uses reusable classes for signal history filters and wide table", async () => {
    vi.mocked(tauri.listSignalHistory).mockResolvedValueOnce({
      items: [
        {
          signalId: "sig-1",
          symbol: "SHSE.600000",
          marketType: "A 股",
          direction: "Buy",
          score: 85,
          strength: 0.85,
          categoryBreakdown: { Trend: 0.85 },
          contributors: ["ma_cross"],
          entryZoneLow: 62450,
          entryZoneHigh: 63200,
          stopLoss: 61800,
          takeProfit: 65000,
          reasonSummary: "Golden cross detected",
          riskStatus: "approved",
          executed: false,
          modified: false,
          generatedAt: "2026-05-05T10:30:00Z",
        },
      ],
      total: 1,
      page: 1,
      pageSize: 10,
    });
    vi.mocked(tauri.listMarkets).mockResolvedValueOnce([
      {
        symbol: "SHSE.600000",
        baseAsset: "浦发银行",
        marketType: "A 股",
        marketSizeTier: "large",
        last: 8.72,
        change24h: 0.81,
        volume24h: 1_260_000_000,
        spreadBps: 2,
        venues: ["akshare:xueqiu"],
        updatedAt: "2026-05-06T10:00:00+08:00",
      },
    ]);

    const { container } = renderSignalsPage();

    expect(await screen.findByText("SHSE.600000")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "名称" })).toBeInTheDocument();
    expect(screen.getByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "风险筛选" })).toHaveClass("signals-filter-select");
    expect(screen.getByRole("combobox", { name: "状态筛选" })).toHaveClass("signals-filter-select");
    expect(screen.queryByRole("columnheader", { name: "策略" })).not.toBeInTheDocument();
    expect(screen.queryAllByText("A 股")).toHaveLength(0);
    expect(container.querySelector(".signals-history-table-shell")).toHaveClass("table-shell--visible-scrollbar");
    expect(container.querySelector(".signals-history-table")).toBeInTheDocument();
  });
});
