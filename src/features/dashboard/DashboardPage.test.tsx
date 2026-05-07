import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { HashRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";

vi.mock("../../lib/tauri", () => ({
  listMarkets: vi.fn(async () => []),
  getPortfolioOverview: vi.fn(async () => ({
    totalEquity: 1_002_400,
    totalMarketValue: 82_400,
    totalPnl: 2_400,
    todayPnl: 800,
    todayPnlPct: 0.08,
    riskSummary: "模拟账户当前有 1 个持仓。",
    exchanges: [{ name: "人民币现金", equity: 1_002_400, weight: 100 }],
  })),
  triggerRecommendation: vi.fn(async () => [{
    id: "rec-1",
    status: "completed",
    hasTrade: false,
    symbol: "SHSE.600000",
    direction: "观察",
    confidence: 0,
    riskStatus: "watch",
    thesis: "暂无建议",
    riskDetails: { status: "watch", riskScore: 0, checks: [], modifications: [], blockReasons: [] },
    generatedAt: "2026-05-06T10:00:00+08:00",
  }]),
  listAnalyzeJobs: vi.fn(async () => []),
}));

describe("DashboardPage", () => {
  it("renders the Chinese A-share dashboard without crypto or CEX copy", async () => {
    const { container } = render(
      <QueryClientProvider client={new QueryClient()}>
        <HashRouter>
          <DashboardPage />
        </HashRouter>
      </QueryClientProvider>,
    );

    expect(screen.getByRole("button", { name: "AI 分析" })).toBeInTheDocument();
    expect(await screen.findByText("总资产")).toBeInTheDocument();
    expect(screen.getByText("总市值")).toBeInTheDocument();
    expect(screen.getByText("总盈亏")).toBeInTheDocument();
    expect(screen.getByText("当日盈亏")).toBeInTheDocument();
    expect(await screen.findByText("¥1,002,400")).toBeInTheDocument();
    expect(screen.getByText("¥82,400")).toBeInTheDocument();
    expect(screen.getByText("+¥2,400")).toBeInTheDocument();
    expect(screen.getByText("+¥800 / +0.08%")).toBeInTheDocument();
    expect(container.querySelector(".dashboard-workbench-ledger")).toBeInTheDocument();
    expect(container.querySelectorAll(".dashboard-workbench-ledger__item")).toHaveLength(4);
    expect(screen.queryByText("行情来源")).not.toBeInTheDocument();
    expect(screen.queryByText("账户模式")).not.toBeInTheDocument();
    expect(screen.queryByText("最新建议")).not.toBeInTheDocument();
    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.queryByText(/BTC\/USDT|USDT|akshare|akshare|券商模拟|策略账户|风控账户|现金账户/)).not.toBeInTheDocument();
  });
});
