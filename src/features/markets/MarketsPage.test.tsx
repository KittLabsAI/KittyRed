import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MarketsPage } from "./MarketsPage";

const {
  appendWatchlistSymbol,
  listMarkets,
  refreshWatchlistTickers,
  searchAShareSymbols,
} = vi.hoisted(() => ({
  appendWatchlistSymbol: vi.fn(async () => undefined),
  listMarkets: vi.fn(async () => [
    {
      symbol: "SHSE.600000",
      baseAsset: "浦发银行",
      marketType: "沪市A股",
      marketSizeTier: "large" as const,
      last: 8.88,
      change24h: 1.23,
      volume24h: 123000000,
      spreadBps: 0,
      venues: ["akshare"],
      updatedAt: "2026-05-07T10:30:00+08:00",
    },
  ]),
  refreshWatchlistTickers: vi.fn(async () => []),
  searchAShareSymbols: vi.fn(async () => [
    { symbol: "SHSE.600519", name: "贵州茅台", market: "沪市A股" },
  ]),
}));

vi.mock("../../lib/tauri", () => ({
  listMarkets,
  refreshWatchlistTickers,
  searchAShareSymbols,
}));

vi.mock("../../lib/settings", () => ({
  appendWatchlistSymbol,
}));

describe("MarketsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows A-share rows and removes crypto pairs and CEX venue copy", async () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter>
          <MarketsPage />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    expect(screen.getByText("SHSE.600000")).toBeInTheDocument();
    expect(screen.queryByText("BTC/USDT")).not.toBeInTheDocument();
    expect(screen.queryByText("akshare")).not.toBeInTheDocument();
    expect(screen.queryByText("akshare")).not.toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "代码" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "成交额" })).toBeInTheDocument();
    expect(screen.getByText("+1.23%")).toBeInTheDocument();
  });

  it("renders cached rows first and refreshes watchlist tickers in the background", async () => {
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter>
          <MarketsPage />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(await screen.findByText("浦发银行")).toBeInTheDocument();
    await waitFor(() => {
      expect(listMarkets).toHaveBeenCalled();
      expect(refreshWatchlistTickers).toHaveBeenCalled();
    });
  });

  it("shows two decimal places for high-priced A-share quotes", async () => {
    listMarkets.mockResolvedValueOnce([
      {
        symbol: "SHSE.688981",
        baseAsset: "中芯国际",
        marketType: "沪市A股",
        marketSizeTier: "large" as const,
        last: 102.4,
        change24h: 1.23,
        volume24h: 123000000,
        spreadBps: 0,
        venues: ["akshare"],
        updatedAt: "2026-05-07T10:30:00+08:00",
      },
    ]);

    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter>
          <MarketsPage />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    expect(await screen.findByText("中芯国际")).toBeInTheDocument();
    expect(screen.getByText("¥102.40")).toBeInTheDocument();
  });


  it("adds a searched A-share symbol to the watchlist and refreshes tickers", async () => {
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter>
          <MarketsPage />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(await screen.findByLabelText("搜索并添加自选股票"), "茅台");
    await user.click(await screen.findByRole("button", { name: "添加 SHSE.600519 贵州茅台" }));

    await waitFor(() => {
      expect(appendWatchlistSymbol).toHaveBeenCalledWith("SHSE.600519");
      expect(refreshWatchlistTickers).toHaveBeenCalled();
    });
  });

  it("shows string errors returned by the stock search command", async () => {
    searchAShareSymbols.mockRejectedValue("AKShare Python SDK 未安装");
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <MemoryRouter>
          <MarketsPage />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    await user.type(await screen.findByLabelText("搜索并添加自选股票"), "茅台");

    expect(await screen.findByText("股票搜索失败：AKShare Python SDK 未安装")).toBeInTheDocument();
  });
});
