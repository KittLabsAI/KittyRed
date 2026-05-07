import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AnalyzeJobsPanel } from "./AnalyzeJobsPanel";

const { cancelAnalyzeJob, listAnalyzeJobs } = vi.hoisted(() => ({
  cancelAnalyzeJob: vi.fn(async () => undefined),
  listAnalyzeJobs: vi.fn(async () => [
    {
      id: 42,
      kind: "recommendation.generate",
      status: "running",
      message: "正在分析 SHSE.600000",
      startedAt: "2026-05-04T11:05:00+08:00",
      updatedAt: "epoch:1777864020",
      endedAt: null,
      durationMs: null,
      inputParamsJson: "{\"symbol\":\"SHSE.600000\",\"triggerSource\":\"manual\"}",
      resultSummary: null,
      errorDetails: null,
    },
    {
      id: 43,
      kind: "recommendation.generate",
      status: "done",
      message: "完成 SZSE.000001 买入分析，置信度 66%。",
      startedAt: "2026-05-03T19:01:00+08:00",
      updatedAt: "2026-05-03T19:01:06+08:00",
      endedAt: "2026-05-03T19:01:06+08:00",
      durationMs: 6000,
      inputParamsJson: "{\"symbol\":\"SZSE.000001\",\"triggerSource\":\"manual\"}",
      resultSummary: "价格回到均线附近，成交额温和放大。",
      errorDetails: null,
    },
    {
      id: 46,
      kind: "market.refresh_tickers",
      status: "done",
      message: "Refreshed cached tickers",
      startedAt: "2026-05-03T18:59:56+08:00",
      updatedAt: "2026-05-03T19:00:00+08:00",
      endedAt: "2026-05-03T19:00:00+08:00",
      durationMs: 4200,
      inputParamsJson: "{\"market\":\"ashare\"}",
      resultSummary: "刷新 3000 条 A 股缓存行情",
      errorDetails: null,
    },
    {
      id: 47,
      kind: "signal.scan",
      status: "done",
      message: "Manual signal scan completed with 3 signals.",
      startedAt: "2026-05-03T19:02:00+08:00",
      updatedAt: "2026-05-03T19:02:04+08:00",
      endedAt: "2026-05-03T19:02:04+08:00",
      durationMs: 4000,
      inputParamsJson: "{\"triggerSource\":\"manual\"}",
      resultSummary: "3 signals found",
      errorDetails: null,
    },
    {
      id: 44,
      kind: "recommendation.generate",
      status: "failed",
      message: "Recommendation run failed",
      startedAt: "2026-05-03T19:04:55+08:00",
      updatedAt: "2026-05-03T19:05:00+08:00",
      endedAt: "2026-05-03T19:05:00+08:00",
      durationMs: 5000,
      inputParamsJson: "{\"symbol\":\"SHSE.600519\",\"triggerSource\":\"auto\"}",
      resultSummary: null,
      errorDetails: "AI recommendation did not return a plan",
    },
  ]),
}));

vi.mock("../../lib/tauri", () => ({
  cancelAnalyzeJob,
  listAnalyzeJobs,
}));

describe("AnalyzeJobsPanel", () => {
  it("shows persisted job history details and cancels the active running job", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <QueryClientProvider client={new QueryClient()}>
        <AnalyzeJobsPanel />
      </QueryClientProvider>,
    );

    expect(container.querySelector(".job-list")).toHaveClass("job-list--scrollable");
    expect(screen.getByRole("heading", { name: "后台任务" })).toBeInTheDocument();
    expect(container.querySelector(".jobs-filter-row")).toBeInTheDocument();
    await screen.findByText("2026-05-04 11:07:00");
    expect(screen.getByLabelText("任务概览")).toHaveTextContent("显示 5");
    expect(screen.getByLabelText("任务概览")).toHaveTextContent("运行 1");
    expect(screen.getByLabelText("任务概览")).toHaveTextContent("失败 1");
    expect(screen.getByText("2026-05-04 11:07:00")).toBeInTheDocument();
    expect(screen.getByText("输入：{\"symbol\":\"SHSE.600000\",\"triggerSource\":\"manual\"}")).toBeInTheDocument();
    expect(screen.queryByText("结束：2026-05-03 19:00:00")).not.toBeInTheDocument();
    expect(screen.getByText("耗时：4.2s")).toBeInTheDocument();
    expect(screen.getByText("结果：刷新 3000 条 A 股缓存行情")).toBeInTheDocument();
    expect(screen.getByText("Manual signal scan completed with 3 signals.")).toBeInTheDocument();
    expect(
      screen.getByText(
        "原因：价格回到均线附近，成交额温和放大。",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("错误：AI recommendation did not return a plan")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "任务状态筛选" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "任务类型筛选" })).toBeInTheDocument();

    const recommendationCards = screen.getAllByText("recommendation.generate", { selector: "strong" });
    const runningCard = recommendationCards[0]?.closest("article");
    expect(runningCard).not.toBeNull();

    await user.click(
      within(runningCard as HTMLElement).getByRole("button", { name: "停止 recommendation.generate" }),
    );

    await waitFor(() => {
      expect(cancelAnalyzeJob).toHaveBeenCalledWith(42);
    });
    expect(screen.getByRole("button", { name: "停止 market.refresh_tickers" })).toBeDisabled();

    await user.selectOptions(screen.getByRole("combobox", { name: "任务状态筛选" }), "failed");
    expect(screen.getByText("Recommendation run failed")).toBeInTheDocument();
    expect(screen.queryByText("正在分析 SHSE.600000")).not.toBeInTheDocument();

    await user.selectOptions(screen.getByRole("combobox", { name: "任务状态筛选" }), "all");
    await user.selectOptions(screen.getByRole("combobox", { name: "任务类型筛选" }), "signal.scan");
    expect(screen.getByText("Manual signal scan completed with 3 signals.")).toBeInTheDocument();
    expect(screen.queryByText("Refreshed cached tickers")).not.toBeInTheDocument();
  });
});
