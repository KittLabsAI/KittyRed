import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RecommendationsPage } from "./RecommendationsPage";

const { triggerRecommendationMock, deleteRecommendationMock, getRecommendationAuditMock } = vi.hoisted(() => ({
  triggerRecommendationMock: vi.fn(async () => [{
    id: "rec-new",
    status: "completed",
    hasTrade: false,
    symbol: "市场扫描",
    direction: "观望",
    marketType: "A 股",
    confidence: 55,
    riskStatus: "watch",
    thesis: "暂无符合风险收益比的 A 股机会。",
    riskDetails: { status: "watch", riskScore: 0, checks: [], modifications: [], blockReasons: [] },
    generatedAt: "2026-05-06T10:00:00+08:00",
  }]),
  deleteRecommendationMock: vi.fn(async () => undefined),
  getRecommendationAuditMock: vi.fn(async () => ({
    recommendationId: "rec-1",
    triggerType: "manual",
    symbol: "SHSE.600000",
    exchange: "模拟账户",
    marketType: "A 股",
    createdAt: "2026-05-03T19:00:00+08:00",
    modelProvider: "OpenAI-compatible",
    modelName: "gpt-5.5",
    promptVersion: "recommendation-system-v2",
    userPreferenceVersion: "prefs-test",
    aiRawOutput: "{\"has_trade\":true}",
    aiStructuredOutput: "{\"input_stock_code\":\"SHSE.600000\"}",
    riskResult: "{\"status\":\"approved\"}",
    marketSnapshot: "{\"rows\":[]}",
    accountSnapshot: "{\"account_mode\":\"paper\"}",
  })),
}));

vi.mock("../../lib/tauri", () => ({
  getLatestRecommendation: vi.fn(async () => [{
    id: "rec-live-7",
    status: "completed",
    hasTrade: true,
    symbol: "SHSE.600000",
    direction: "买入",
    marketType: "A 股",
    confidence: 74,
    riskStatus: "approved",
    thesis: "量价改善，模拟账户小仓位观察。",
    entryLow: 8.68,
    entryHigh: 8.76,
    stopLoss: 8.45,
    takeProfit: "9.10 / 9.35",
    amountCny: 20_000,
    maxLossCny: 700,
    riskDetails: { status: "approved", riskScore: 42, checks: [], modifications: [], blockReasons: [] },
    generatedAt: "2026-05-06T10:00:00+08:00",
  }]),
  listRecommendationHistory: vi.fn(async () => [
    {
      id: "rec-1",
      createdAt: "2026-05-03T19:00:00+08:00",
      symbol: "SHSE.600000",
      stockName: "输入浦发银行",
      shortlist: ["SHSE.600000"],
      exchange: "模拟账户",
      marketType: "A 股",
      direction: "买入",
      rationale: "日线趋势向上，1h 回踩后重新放量，5m 结构确认。",
      risk: "approved",
      result: "Win",
      entryLow: 8.68,
      entryHigh: 8.72,
      stopLoss: 8.42,
      confidence: 72,
      executed: true,
      pnl5m: 0.1,
      pnl10m: 0.2,
      pnl30m: 0.3,
      pnl60m: 0.4,
      pnl24h: 1.2,
      pnl7d: 2.4,
      outcome: "Live evaluation complete through 7d using market candles.",
    },
    {
      id: "rec-2",
      createdAt: "2026-05-03T20:00:00+08:00",
      symbol: "SZSE.000001",
      stockName: "输入平安银行",
      shortlist: ["SZSE.000001"],
      exchange: "模拟账户",
      marketType: "A 股",
      direction: "观望",
      rationale: "成交额低于阈值，1h K 线没有确认突破。",
      risk: "blocked",
      result: "Blocked",
      confidence: 52,
      executed: false,
      pnl5m: 0,
      pnl10m: 0,
      pnl30m: 0,
      pnl60m: 0,
      pnl24h: 0,
      pnl7d: 0,
      outcome: "等待下一交易K线：10分钟、60分钟、24小时、7天。",
    },
    ...Array.from({ length: 10 }, (_, index) => ({
      id: `rec-extra-${index + 1}`,
      createdAt: `2026-05-04T${String(9 + index).padStart(2, "0")}:00:00+08:00`,
      symbol: "SHSE.600000",
      stockName: `输入浦发银行${index + 1}`,
      shortlist: ["SHSE.600000"],
      exchange: "模拟账户",
      marketType: "A 股",
      direction: "观望",
      rationale: `分页测试建议 ${index + 1}`,
      risk: "watch",
      result: "Pending",
      confidence: 50 + index,
      executed: false,
      pnl5m: 0,
      pnl10m: 0,
      pnl30m: 0,
      pnl60m: 0,
      pnl24h: 0,
      pnl7d: 0,
      outcome: "Queued for 10m / 60m / 24h / 7d evaluation windows.",
    })),
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
  triggerRecommendation: triggerRecommendationMock,
  deleteRecommendation: deleteRecommendationMock,
  getRecommendationAudit: getRecommendationAuditMock,
}));

function renderPage() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RecommendationsPage />
    </QueryClientProvider>,
  );
}

describe("RecommendationsPage", () => {
  it("merges latest AI recommendation and history into one Chinese page", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("AI投资建议")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "生成AI建议" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "咨询AI助手" })).toBeInTheDocument();
    expect(screen.queryByText("交易计划")).not.toBeInTheDocument();
    expect(await screen.findByText("输入浦发银行")).toBeInTheDocument();
    expect(screen.getByText("输入平安银行")).toBeInTheDocument();
    expect(screen.getByText("10分钟")).toBeInTheDocument();
    expect(screen.getByText("60分钟")).toBeInTheDocument();
    expect(screen.getByText("24小时")).toBeInTheDocument();
    expect(screen.getByText("7天")).toBeInTheDocument();
    expect(screen.getByText("建议原因")).toBeInTheDocument();
    expect(screen.getByText("日线趋势向上，1h 回踩后重新放量，5m 结构确认。")).toBeInTheDocument();
    expect(screen.queryByText("股票筛选")).not.toBeInTheDocument();
    expect(screen.queryByText("交易方向筛选")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "交易方向筛选" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "股票筛选" })).toBeInTheDocument();
    expect(screen.queryByText("10M")).not.toBeInTheDocument();
    expect(screen.queryByText("60M")).not.toBeInTheDocument();
    expect(screen.queryByText("24H")).not.toBeInTheDocument();
    expect(screen.queryByText("5M")).not.toBeInTheDocument();
    expect(screen.queryByText("30M")).not.toBeInTheDocument();
    expect(screen.getByText("已使用行情 K 线完成 7 天评估。")).toBeInTheDocument();
    expect(screen.getByText("第 1 / 2 页，每页 10 条")).toBeInTheDocument();
    expect(screen.queryByText("分页测试建议 9")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "下一页" }));
    expect(await screen.findByText("分页测试建议 9")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "生成AI建议" }));
    expect(triggerRecommendationMock).toHaveBeenCalled();
  });

  it("filters recommendation history by trade direction", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("输入浦发银行")).toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "交易方向筛选" }), "买入");

    expect(screen.getByText("输入浦发银行")).toBeInTheDocument();
    expect(screen.queryByText("输入平安银行")).not.toBeInTheDocument();
    expect(screen.getByText("当前显示 1 / 12 条建议。")).toBeInTheDocument();
  });

  it("shows a dash outcome for no-trade and blocked recommendations", async () => {
    renderPage();

    expect(await screen.findByText("输入浦发银行")).toBeInTheDocument();
    expect(screen.getByText("成交额低于阈值，1h K 线没有确认突破。")).toBeInTheDocument();
    expect(screen.getAllByText("-").length).toBeGreaterThan(0);
  });

  it("opens the audit drawer and can delete a recommendation", async () => {
    const user = userEvent.setup();
    renderPage();

    const auditButtons = await screen.findAllByRole("button", { name: "查看 SHSE.600000 的审计详情" });
    await user.click(auditButtons[0]);
    expect(await screen.findByText("AI 推荐详情")).toBeInTheDocument();
    expect(screen.getAllByText("审查").length).toBeGreaterThan(0);
    expect(getRecommendationAuditMock).toHaveBeenCalledWith("rec-1");

    const deleteButtons = screen.getAllByRole("button", { name: "删除 SHSE.600000 的建议" });
    await user.click(deleteButtons[0]);
    await waitFor(() => expect(deleteRecommendationMock).toHaveBeenCalled());
    const firstDeleteCall = deleteRecommendationMock.mock.calls[0] as unknown[] | undefined;
    expect(firstDeleteCall?.[0]).toBe("rec-1");
  });
});
