import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";

const mocks = vi.hoisted(() => ({
  getAkshareCurrentQuote: vi.fn(async () => ({
    ok: true,
    data: {
      symbol: "SHSE.600000",
      last: 8.72,
      open: 8.7,
      high: 8.8,
      low: 8.6,
      volume: 1000,
      amount: 8720,
      updated_at: "2026-05-06T10:00:00+08:00",
      source: "akshare" as const,
    },
  })),
  testModelConnection: vi.fn(async () => ({
    ok: true,
    message: "模型连接正常",
  })),
}));

vi.mock("../../lib/tauri", () => ({
  listNotificationEvents: vi.fn(async () => []),
}));

vi.mock("../../lib/akshare", () => ({
  getAkshareCurrentQuote: mocks.getAkshareCurrentQuote,
}));

vi.mock("../../lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/settings")>();
  return {
      ...actual,
      loadSettingsFormData: vi.fn(async () => ({
      ...actual.createDefaultSettingsFormData(),
        accountMode: "paper",
      })),
      testModelConnection: mocks.testModelConnection,
    };
  });

describe("SettingsPage", () => {
  it("shows AKShare data-source settings without account credentials", async () => {
    render(<SettingsPage />);

    expect(await screen.findByLabelText("数据接口")).toHaveValue("AKShare Python SDK");
    expect(screen.getByRole("tab", { name: "数据源" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "模型" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "AI交易" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "提示词" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "通知" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "安全与数据" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "账户模式" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "交易偏好" })).not.toBeInTheDocument();
    expect(screen.queryByText("仅模拟账号模式")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存设置" })).toHaveClass("settings-save-button");
    expect(screen.queryByText("真实只读模式")).not.toBeInTheDocument();
    expect(screen.queryByText("交易所 API")).not.toBeInTheDocument();
    expect(screen.queryByText(/Crypto/i)).not.toBeInTheDocument();
  });

  it("tests the AKShare connection without credentials", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByText("测试 AKShare 连接"));

    expect(mocks.getAkshareCurrentQuote).toHaveBeenCalledWith("SHSE.600000");
    expect(await screen.findByText(/AKShare 连接正常/)).toBeInTheDocument();
  });

  it("keeps watchlist editing out of AI settings and exposes KittyAlpha-style model controls", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "AI交易" }));
    const panel = screen.getByRole("tabpanel");
    expect(within(panel).queryByText("自选股票")).not.toBeInTheDocument();
    expect(within(panel).getByText("AI分析")).toBeInTheDocument();
    expect(within(panel).getByText("风险")).toBeInTheDocument();
    expect(within(panel).getByText("策略信号")).toBeInTheDocument();
    expect(within(panel).getByLabelText("单笔最大亏损").tagName).toBe("SELECT");
    expect(within(panel).getByLabelText("日内最大亏损").tagName).toBe("SELECT");
    expect(within(panel).getByLabelText("最低置信度").tagName).toBe("SELECT");

    await user.click(screen.getByRole("tab", { name: "模型" }));
    expect(screen.getByLabelText("模型服务商")).toBeInTheDocument();
    expect(screen.getByLabelText("接口类型")).toBeInTheDocument();
    expect(screen.getByLabelText("温度")).toBeInTheDocument();
    expect(screen.getByLabelText("最大输出 Token")).toBeInTheDocument();
    expect(screen.getByLabelText("最大上下文 Token")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "测试模型连接" }));

    expect(mocks.testModelConnection).toHaveBeenCalled();
    expect(await screen.findByText("模型连接正常")).toBeInTheDocument();
  });

  it("edits full Assistant and recommendation system prompts", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "提示词" }));

    expect(screen.queryByLabelText("提示词扩展")).not.toBeInTheDocument();
    const assistantPrompt = screen.getByLabelText("Assistant 系统提示词");
    const recommendationPrompt = screen.getByLabelText("AI 推荐系统提示词");
    expect((assistantPrompt as HTMLTextAreaElement).value).toContain("KittyRed Assistant");
    expect((recommendationPrompt as HTMLTextAreaElement).value).toContain("沪深 A 股模拟投资助手");

    await user.clear(assistantPrompt);
    await user.type(assistantPrompt, "新的 Assistant 系统提示词");

    expect(assistantPrompt).toHaveValue("新的 Assistant 系统提示词");
  });
});
