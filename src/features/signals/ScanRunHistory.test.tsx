import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { ScanRunHistory } from "./ScanRunHistory";

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

const { listScanRunsMock } = vi.hoisted(() => ({
  listScanRunsMock: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  listScanRuns: listScanRunsMock,
}));

describe("ScanRunHistory", () => {
  it("shows empty state when no runs", async () => {
    listScanRunsMock.mockResolvedValueOnce({
      items: [],
      total: 0,
      page: 1,
      pageSize: 25,
    });

    render(
      <QueryClientProvider client={queryClient}>
        <ScanRunHistory />
      </QueryClientProvider>,
    );
    await waitFor(() => {
      expect(screen.getByText("暂无扫描记录。")).toBeInTheDocument();
    });
  });

  it("uses tokenized status badge classes", async () => {
    listScanRunsMock.mockResolvedValueOnce({
      items: [
        {
          id: "run-1",
          startedAt: "2026-05-05T10:30:00Z",
          symbolsScanned: 12,
          signalsFound: 3,
          durationMs: 1234,
          status: "done",
        },
      ],
      total: 1,
      page: 1,
      pageSize: 25,
    });

    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <ScanRunHistory />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("完成")).toHaveClass("badge--status-done");
    expect(screen.getByText("完成")).not.toHaveAttribute("style");
  });
});
