import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { Header } from "./Header";
import { useAppStore } from "../../store/appStore";

vi.mock("../../lib/tauri", () => ({
  listAnalyzeJobs: vi.fn(async () => [
    {
      id: 1,
      kind: "recommendation_auto_cycle",
      status: "done",
      message: "completed",
      updatedAt: "2026-05-03T23:10:00+08:00",
    },
  ]),
}));

vi.mock("../../lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/settings")>();
  return {
    ...actual,
    loadSettingsFormData: vi.fn(async () => ({
      exchanges: [],
      modelPreset: "Custom",
      modelProvider: "OpenAI-compatible",
      modelName: "gpt-5.5",
      modelBaseUrl: "",
      modelApiKey: "",
      modelTemperature: 0,
      modelMaxTokens: 4096,
      modelMaxContext: 128000,
      hasStoredModelApiKey: false,
      autoAnalyzeEnabled: true,
      autoAnalyzeFrequency: "10m",
      scanScope: "all_markets",
      watchlistSymbols: [],
      dailyMaxAiCalls: 24,
      pauseAfterConsecutiveLosses: 3,
      minConfidenceScore: 60,
      allowedMarkets: "all",
      allowedDirection: "long_short",
      maxLeverage: 3,
      maxLossPerTradePercent: 1,
      maxDailyLossPercent: 3,
      minRiskRewardRatio: 1.5,
      min24hVolume: 20000000,
      maxSpreadBps: 12,
      allowMemeCoins: true,
      whitelistSymbols: [],
      blacklistSymbols: [],
      promptExtension: "",
      accountMode: "paper",
      autoPaperExecution: false,
      notifications: {
        recommendations: true,
        spreads: true,
        paperOrders: true,
      },
    })),
  };
});

describe("Header", () => {
  it("shows live cadence and latest backend activity instead of hardcoded metadata", async () => {
    useAppStore.setState({ accountMode: "paper" });

    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter initialEntries={["/positions"]}>
          <Header />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(screen.getByRole("heading", { name: "持仓" })).toBeInTheDocument();
    expect(await screen.findByText("AI 扫描 10m")).toBeInTheDocument();
    expect(screen.getByText("同步于 23:10")).toBeInTheDocument();
    expect(screen.getByText("模拟账号")).toBeInTheDocument();
  });
});
