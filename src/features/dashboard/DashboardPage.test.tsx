import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";

const { triggerRecommendationMock } = vi.hoisted(() => ({
  triggerRecommendationMock: vi.fn(async () => []),
}));

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
  triggerRecommendation: triggerRecommendationMock,
  listAnalyzeJobs: vi.fn(async () => []),
}));

describe("DashboardPage", () => {
  it("renders the Chinese A-share dashboard and routes AI analysis to recommendations", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={["/"]}>
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/recommendations" element={<div>推荐页已打开</div>} />
          </Routes>
        </MemoryRouter>
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
    expect(container.querySelectorAll("dl")).toHaveLength(4);
    expect(screen.getByRole("button", { name: "AI 分析" })).toBeInTheDocument();
    expect(screen.queryByText("行情来源")).not.toBeInTheDocument();
    expect(screen.queryByText("账户模式")).not.toBeInTheDocument();
    expect(screen.queryByText("最新建议")).not.toBeInTheDocument();
    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.queryByText(/BTC\/USDT|USDT|akshare|akshare|券商模拟|策略账户|风控账户|现金账户/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "AI 分析" }));
    expect(await screen.findByText("推荐页已打开")).toBeInTheDocument();
    expect(triggerRecommendationMock).not.toHaveBeenCalled();
  });
});
