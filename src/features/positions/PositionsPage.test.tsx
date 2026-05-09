import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PositionsPage } from "./PositionsPage";

const { closePaperPositionMock, resetPaperAccountMock } = vi.hoisted(() => ({
  closePaperPositionMock: vi.fn(async () => ({
    orderId: "PO-0002",
    accountId: "paper-cash",
    exchange: "模拟账户",
    symbol: "SHSE.600000",
    side: "sell",
    quantity: 100,
    estimatedFillPrice: 8.72,
  })),
  resetPaperAccountMock: vi.fn(async () => undefined),
}));

vi.mock("../../lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/tauri")>();
  return {
    ...actual,
    getPortfolioOverview: vi.fn(async () => ({
      totalEquity: 1_002_400,
      totalMarketValue: 82_400,
      totalPnl: 2_400,
      todayPnl: 800,
      todayPnlPct: 0.08,
      riskSummary: "模拟账户当前有 1 个持仓。",
      exchanges: [{ name: "人民币现金", equity: 1_002_400, weight: 100 }],
    })),
    listPositions: vi.fn(async () => [
      {
        positionId: "PO-0001",
        accountId: "paper-cash",
        exchange: "模拟账户",
        symbol: "SHSE.600000",
        side: "Long",
        quantity: 100,
        size: "100 股",
        entry: 8.68,
        mark: 8.72,
        pnlPct: 0.83,
        leverage: "模拟",
      },
    ]),
    closePaperPosition: closePaperPositionMock,
    resetPaperAccount: resetPaperAccountMock,
    listOrders: vi.fn(async () => [
      {
        id: "BN-1092",
        exchange: "akshare",
        symbol: "DOGE/USDT",
        type: "Spot Limit Buy",
        status: "Filled",
        quantity: "15000 DOGE",
        fillPrice: 0.1812,
        realizedPnl: 84.25,
        updatedAt: "2026-05-03T20:30:00+08:00",
      },
      {
        id: "PO-0001",
        exchange: "模拟账户",
        symbol: "SHSE.600000",
        type: "买入",
        status: "Filled",
        quantity: "100 股",
        fillPrice: 9.18,
        realizedPnl: 12.5,
        updatedAt: "2026-05-06T15:00:00+08:00",
      },
    ]),
    listMarkets: vi.fn(async () => [
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
    ]),
  };
});

describe("PositionsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders simulated positions from the backend feed", async () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <PositionsPage />
      </QueryClientProvider>,
    );

    expect((await screen.findAllByText("SHSE.600000")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("浦发银行").length).toBeGreaterThan(0);
    expect(screen.queryByRole("columnheader", { name: "账户" })).not.toBeInTheDocument();
    expect(screen.getByText("资产总览")).toBeInTheDocument();
    expect(screen.getByText("总市值")).toBeInTheDocument();
    expect(screen.getByText("总盈亏")).toBeInTheDocument();
    expect(screen.getByText("当日盈亏")).toBeInTheDocument();
    expect(screen.getByText("¥82,400")).toBeInTheDocument();
    expect(screen.getByText("+¥2,400")).toBeInTheDocument();
    expect(screen.getByText("+¥800 / +0.08%")).toBeInTheDocument();
    expect(screen.getAllByText("100 股").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "重置" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "清仓" })).toBeInTheDocument();
    expect(await screen.findByText("A股模拟委托")).toBeInTheDocument();
    expect(screen.getByText("PO-0001")).toBeInTheDocument();
    expect(screen.getByText("2026-05-06 15:00:00")).toBeInTheDocument();
    expect(screen.queryByText("BN-1092")).not.toBeInTheDocument();
    expect(screen.queryByText(/BTC\/USDT|ETH\/USDT|akshare|风控账户|akshare/)).not.toBeInTheDocument();
  });

  it("closes a simulated position from the position row", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <PositionsPage />
      </QueryClientProvider>,
    );

    await user.click(await screen.findByRole("button", { name: "清仓" }));

    await waitFor(() => {
      const firstCall = closePaperPositionMock.mock.calls[0] as unknown[] | undefined;
      expect(firstCall?.[0]).toBe("PO-0001");
    });
  });

  it("resets the simulated account from the overview card", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <PositionsPage />
      </QueryClientProvider>,
    );

    await user.click(await screen.findByRole("button", { name: "重置" }));

    await waitFor(() => {
      expect(resetPaperAccountMock).toHaveBeenCalledTimes(1);
    });
  });
});
