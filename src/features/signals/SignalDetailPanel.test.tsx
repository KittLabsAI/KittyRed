import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SignalDetailPanel } from "./SignalDetailPanel";
import type { UnifiedSignal } from "../../lib/types";

const tauriMocks = vi.hoisted(() => ({
  dismissSignal: vi.fn(async () => undefined),
  executeSignal: vi.fn(async () => ({
    orderId: "PO-0001",
    accountId: "paper-cny",
    exchange: "模拟账户",
    symbol: "SHSE.600000",
    side: "buy",
    quantity: 0.016,
    estimatedFillPrice: 62825,
    stopLoss: 61800,
    takeProfit: 65000,
  })),
}));

vi.mock("../../lib/tauri", () => ({
  dismissSignal: tauriMocks.dismissSignal,
  executeSignal: tauriMocks.executeSignal,
}));

const mockSignal: UnifiedSignal = {
  signalId: "sig-1",
  symbol: "SHSE.600000",
  marketType: "A 股",
  direction: "Buy",
  score: 85.3,
  strength: 0.85,
  categoryBreakdown: { Trend: 0.85, Momentum: 0.78 },
  contributors: ["ma_cross", "rsi_extreme", "bollinger_break"],
  entryZoneLow: 62450,
  entryZoneHigh: 63200,
  stopLoss: 61800,
  takeProfit: 65000,
  reasonSummary: "Golden cross detected",
  riskStatus: "approved",
  riskResult: undefined,
  executed: false,
  modified: false,
  generatedAt: "2026-05-05T10:30:00Z",
};

function wrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

describe("SignalDetailPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders reason summary", () => {
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });
    expect(screen.getByText("检测到均线金叉")).toBeInTheDocument();
  });

  it("shows entry zone and stop loss", () => {
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });
    expect(screen.getByText(/62,450/)).toBeInTheDocument();
    expect(screen.getByText(/61,800/)).toBeInTheDocument();
  });

  it("shows contributors", () => {
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });
    expect(screen.getByText("均线交叉")).toBeInTheDocument();
    expect(screen.getByText("RSI 超买超卖")).toBeInTheDocument();
  });

  it("localizes risk check names details and unknown contributor labels", () => {
    render(
      <SignalDetailPanel
        signal={{
          ...mockSignal,
          contributors: ["volume_breakout"],
          reasonSummary: "Volume breakout confirmed | single contributor only",
          riskResult: {
            status: "blocked",
            riskScore: 70,
            checks: [
              {
                name: "signal_contributor_minimum",
                status: "failed",
                detail: "1 contributors, min 2",
              },
            ],
            modifications: [],
            blockReasons: ["contributor_count_below_minimum"],
          },
        }}
      />,
      { wrapper },
    );

    expect(screen.getByText("成交量突破确认 | 只有单一策略触发")).toBeInTheDocument();
    expect(screen.getByText("信号来源数量")).toBeInTheDocument();
    expect(screen.getByText("当前 1 个，最低 2 个")).toBeInTheDocument();
    expect(screen.getByText("成交量突破")).toBeInTheDocument();
    expect(screen.queryByText(/contributors|min|Volume breakout|single contributor|volume_breakout/i)).not.toBeInTheDocument();
  });

  it("uses semantic classes instead of inline bar colors", () => {
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });

    const bars = document.querySelectorAll(".signal-detail__category-bar");

    expect(bars).toHaveLength(2);
    expect(bars[0]).toHaveClass("signal-detail__category-bar--strong");
    expect(bars[0]).not.toHaveStyle({ background: "#22c55e" });
  });

  it("shows Execute button when not executed", () => {
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });
    expect(screen.getByText("执行模拟交易")).toBeInTheDocument();
  });

  it("executes approved signals against the default paper account and shows immediate feedback", async () => {
    const user = userEvent.setup();
    render(<SignalDetailPanel signal={mockSignal} />, { wrapper });

    await user.click(screen.getByRole("button", { name: "执行模拟交易" }));

    await waitFor(() => {
      expect(tauriMocks.executeSignal).toHaveBeenCalledWith("sig-1", "paper-cny");
    });
    expect(await screen.findByRole("button", { name: "已执行" })).toBeDisabled();
    expect(screen.getByText("已在 模拟账户 生成模拟成交，估算价格 ¥62,825")).toBeInTheDocument();
  });
});
