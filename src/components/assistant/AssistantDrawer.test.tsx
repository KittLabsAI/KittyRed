import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AssistantEvent } from "../../lib/assistantTypes";
import App from "../../App";
import { AssistantDrawer } from "./AssistantDrawer";

const tauriMocks = vi.hoisted(() => {
  let listener: ((event: AssistantEvent) => void) | null = null;

  return {
    startAssistantRun: vi.fn(async () => undefined),
    stopAssistantRun: vi.fn(async () => undefined),
    clearAssistantSession: vi.fn(async () => undefined),
    listenToAssistantEvents: vi.fn(async (callback: (event: AssistantEvent) => void) => {
      listener = callback;
      return () => {
        if (listener === callback) {
          listener = null;
        }
      };
    }),
    emitAssistantEvent: (event: AssistantEvent) => listener?.(event),
    resetListener: () => {
      listener = null;
    },
  };
});

vi.mock("../../lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/tauri")>();
  return {
    ...actual,
    startAssistantRun: tauriMocks.startAssistantRun,
    stopAssistantRun: tauriMocks.stopAssistantRun,
    clearAssistantSession: tauriMocks.clearAssistantSession,
    listenToAssistantEvents: tauriMocks.listenToAssistantEvents,
  };
});

describe("Assistant drawer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
    tauriMocks.resetListener();
  });

  it("opens and closes from the sidebar button", async () => {
    const user = userEvent.setup();

    render(<App />);

    const assistantButton = screen.getByRole("button", { name: "智能助手" });
    await user.click(assistantButton);
    expect(
      screen.getByRole("dialog", { name: "AI 助手抽屉" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "AI 助手抽屉" })).toHaveAttribute("aria-modal", "true");

    await user.click(screen.getByRole("button", { name: "关闭助手" }));
    expect(
      screen.queryByRole("dialog", { name: "AI 助手抽屉" }),
    ).not.toBeInTheDocument();
    expect(assistantButton).toHaveFocus();
  });

  it("streams markdown replies and shows thinking and tool call blocks", async () => {
    const user = userEvent.setup();
    const thinkingMarker = "VISIBLE_ONLY_AFTER_THINKING_EXPAND";
    const toolMarker = "VISIBLE_ONLY_AFTER_TOOL_EXPAND";

    render(<AssistantDrawer onClose={() => {}} open />);

    await waitFor(() => {
      expect(tauriMocks.listenToAssistantEvents).toHaveBeenCalled();
    });

    await user.type(
      screen.getByRole("textbox", { name: "助手消息" }),
      "查看浦发银行",
    );
    await user.click(screen.getByRole("button", { name: "发送" }));

    const sessionId = latestSessionId();
    expect(tauriMocks.startAssistantRun).toHaveBeenCalledWith(
      sessionId,
      "查看浦发银行",
    );

    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "status",
      status: "running",
      context: {
        usedTokens: 4000,
        maxTokens: 16000,
        remainingTokens: 12000,
        thinkingTokens: 600,
        breakdown: {
          system: 900,
          user: 500,
          assistant: 1800,
          tool: 800,
        },
      },
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "thinking_status",
      status: "running",
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "thinking_delta",
      delta: `Checking cached market and recommendation context before final answer ${thinkingMarker}`,
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "tool_start",
      toolCallId: "tool-1",
      name: "market_data",
      summary: "载入浦发银行行情",
      arguments: { symbol: "SHSE.600000", marketType: "A 股", limit: 1 },
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "tool_output",
      toolCallId: "tool-1",
      delta: `{"ok":true,"rows":[{"symbol":"SHSE.600000","detailMarker":"${toolMarker}"}]}`,
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "tool_end",
      toolCallId: "tool-1",
      name: "market_data",
      status: "done",
      resultPreview: "1 cached row",
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "token",
      delta: "这是 **浦发银行** 的上下文。\n\n- 已载入缓存行情",
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "done",
      reply: "这是 **浦发银行** 的上下文。\n\n- 已载入缓存行情",
    });

    expect(await screen.findByText("查看浦发银行")).toBeInTheDocument();
    expect(screen.getByText("思考")).toBeInTheDocument();
    expect(screen.getByText("market_data")).toBeInTheDocument();
    expect(screen.getByText("浦发银行", { selector: "strong" })).toBeInTheDocument();
    expect(screen.getByText("25%")).toBeInTheDocument();
    expect(screen.queryByText(new RegExp(thinkingMarker))).not.toBeInTheDocument();
    expect(screen.queryByText(new RegExp(toolMarker))).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /思考/i }));
    expect(screen.getByText(new RegExp(thinkingMarker))).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /market_data/i }));
    expect(screen.getByText(/"SHSE.600000"/)).toBeInTheDocument();
    expect(screen.getByText(new RegExp(toolMarker))).toBeInTheDocument();
  });

  it("supports stop and returns to send state after cancellation", async () => {
    const user = userEvent.setup();

    render(<AssistantDrawer onClose={() => {}} open />);

    await user.type(
      screen.getByRole("textbox", { name: "助手消息" }),
      "Will this stop?",
    );
    await user.click(screen.getByRole("button", { name: "发送" }));

    const sessionId = latestSessionId();
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "status",
      status: "running",
    });

    await user.click(screen.getByRole("button", { name: "停止" }));
    expect(tauriMocks.stopAssistantRun).toHaveBeenCalledWith(sessionId);

    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "cancelled",
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "发送" })).toBeInTheDocument();
    });
  });

  it("keeps session state after close and clears it on refresh", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<AssistantDrawer onClose={() => {}} open />);

    await user.type(
      screen.getByRole("textbox", { name: "助手消息" }),
      "Keep this session",
    );
    await user.click(screen.getByRole("button", { name: "发送" }));

    const sessionId = latestSessionId();
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "status",
      status: "running",
      context: {
        usedTokens: 2000,
        maxTokens: 16000,
        remainingTokens: 14000,
        thinkingTokens: 200,
        breakdown: {
          system: 500,
          user: 300,
          assistant: 900,
          tool: 300,
        },
      },
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "token",
      delta: "Saved reply",
    });
    tauriMocks.emitAssistantEvent({
      sessionId,
      type: "done",
      reply: "Saved reply",
    });

    expect(await screen.findByText("Saved reply")).toBeInTheDocument();
    expect(screen.getByText("13%")).toBeInTheDocument();

    rerender(<AssistantDrawer onClose={() => {}} open={false} />);
    expect(
      screen.queryByRole("dialog", { name: "AI 助手抽屉" }),
    ).not.toBeInTheDocument();

    rerender(<AssistantDrawer onClose={() => {}} open />);
    expect(screen.getByText("Saved reply")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "刷新助手" }));
    expect(tauriMocks.clearAssistantSession).toHaveBeenCalledWith(sessionId);

    await waitFor(() => {
      expect(screen.queryByText("Saved reply")).not.toBeInTheDocument();
    });
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("shows a visible error if the assistant start call fails", async () => {
    const user = userEvent.setup();
    tauriMocks.startAssistantRun.mockRejectedValueOnce(
      new Error("request timed out"),
    );

    render(<AssistantDrawer onClose={() => {}} open />);

    await user.type(
      screen.getByRole("textbox", { name: "助手消息" }),
      "Why did the run fail?",
    );
    await user.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("request timed out");
    expect(screen.getByRole("button", { name: "发送" })).toBeInTheDocument();
  });
});

function latestSessionId() {
  const calls = tauriMocks.startAssistantRun.mock.calls as unknown[][];
  const sessionId = calls[calls.length - 1]?.[0];
  expect(typeof sessionId).toBe("string");
  return sessionId as string;
}
