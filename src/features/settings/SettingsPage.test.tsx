import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";

const mocks = vi.hoisted(() => ({
  openUrl: vi.fn(async () => undefined),
  testAkshareConnectionItem: vi.fn(async (itemId: string) => ({
    itemId,
    ok: true,
    message: `${itemId} ok`,
  })),
  testModelConnection: vi.fn(async () => ({
    ok: true,
    message: "模型连接正常",
  })),
  getSentimentPlatformAuthStatuses: vi.fn(async () => [
    { platform: "zhihu", hasLoginState: true, capturedAt: "2026-05-12T10:00:00+08:00" },
  ]),
  captureSentimentPlatformLoginState: vi.fn(async () => undefined),
  testSentimentPlatformConnections: vi.fn(async () => [
    { platform: "zhihu", ok: true, message: "知乎 连接测试可用" },
    { platform: "xueqiu", ok: false, message: "雪球 未登录" },
  ]),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: mocks.openUrl,
}));

vi.mock("../../lib/tauri", () => ({
  listNotificationEvents: vi.fn(async () => []),
  getSentimentPlatformAuthStatuses: mocks.getSentimentPlatformAuthStatuses,
  captureSentimentPlatformLoginState: mocks.captureSentimentPlatformLoginState,
  testSentimentPlatformConnections: mocks.testSentimentPlatformConnections,
}));

vi.mock("../../lib/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/settings")>();
  return {
      ...actual,
      loadSettingsFormData: vi.fn(async () => ({
      ...actual.createDefaultSettingsFormData(),
        hasStoredModelApiKey: true,
        hasStoredXueqiuToken: true,
        accountMode: "paper",
      })),
      testAkshareConnectionItem: mocks.testAkshareConnectionItem,
      testModelConnection: mocks.testModelConnection,
    };
  });

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.spyOn(window, "open").mockImplementation(() => null);
    mocks.openUrl.mockReset();
    mocks.openUrl.mockResolvedValue(undefined);
    mocks.testAkshareConnectionItem.mockReset();
    mocks.testAkshareConnectionItem.mockImplementation(async (itemId: string) => ({
      itemId,
      ok: true,
      message: `${itemId} ok`,
    }));
    mocks.testModelConnection.mockReset();
    mocks.testModelConnection.mockResolvedValue({
      ok: true,
      message: "模型连接正常",
    });
    mocks.getSentimentPlatformAuthStatuses.mockReset();
    mocks.getSentimentPlatformAuthStatuses.mockResolvedValue([
      { platform: "zhihu", hasLoginState: true, capturedAt: "2026-05-12T10:00:00+08:00" },
    ]);
    mocks.captureSentimentPlatformLoginState.mockReset();
    mocks.captureSentimentPlatformLoginState.mockResolvedValue(undefined);
    mocks.testSentimentPlatformConnections.mockReset();
    mocks.testSentimentPlatformConnections.mockResolvedValue([
      { platform: "zhihu", ok: true, message: "知乎 连接测试可用" },
      { platform: "xueqiu", ok: false, message: "雪球 未登录" },
    ]);
  });

  it("shows AKShare data-source settings without account credentials", async () => {
    render(<SettingsPage />);

    expect(await screen.findByLabelText("数据接口")).toHaveValue("AKShare Python SDK");
    expect(screen.getByLabelText("分时数据")).toHaveValue("sina");
    expect(screen.getByLabelText("历史行情数据")).toHaveValue("eastmoney");
    expect(screen.getByLabelText("雪球 Token")).toBeInTheDocument();
    expect(screen.getByText("新浪财经和腾讯证券的周线/月线由本地日线聚合生成。")).toBeInTheDocument();
    expect(screen.getByLabelText("雪球 Token")).toHaveAttribute("placeholder", "已保存雪球 Token，可直接覆盖");
    expect(screen.getByRole("tab", { name: "数据源" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "舆情分析" })).toBeInTheDocument();
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

  it("shows sentiment login state controls and social platform connection test", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "舆情分析" }));

    expect(screen.getByText("舆情抽样设置")).toBeInTheDocument();
    expect(screen.getByLabelText("拉取舆情的最近天数")).toHaveValue(30);
    expect(screen.getByLabelText("每条舆情的限制字数")).toHaveValue(420);
    expect(screen.getByLabelText("抽样优先规则")).toHaveValue("time_first");
    expect((await screen.findAllByText("知乎")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("小红书").length).toBeGreaterThan(0);
    expect(screen.getAllByText("抖音").length).toBeGreaterThan(0);
    expect(screen.getAllByText("雪球").length).toBeGreaterThan(0);
    expect(screen.getByText("已成功获取登录态")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新登录态" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "获取登录态" })).toHaveLength(3);

    await user.click(screen.getByRole("button", { name: "刷新登录态" }));
    expect(mocks.openUrl).toHaveBeenCalledWith("https://www.zhihu.com");
    const dialog = await screen.findByRole("dialog", { name: "知乎登录态获取" });
    expect(dialog).toHaveClass("modal-overlay");
    expect(within(dialog).getByText(/请在打开的浏览器中登录知乎/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "确定" }));
    expect(mocks.captureSentimentPlatformLoginState).toHaveBeenCalledWith("zhihu");

    await user.click(screen.getByRole("button", { name: "测试社媒平台连接" }));
    expect(mocks.testSentimentPlatformConnections).toHaveBeenCalled();
    expect((await screen.findAllByText("知乎")).length).toBeGreaterThan(1);
    expect(screen.getAllByText("雪球").length).toBeGreaterThan(1);
    expect(screen.getByText("失败")).toBeInTheDocument();
  });

  it("reorders sentiment platform priority by dragging platform chips", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "舆情分析" }));
    const priorityList = screen.getByLabelText("AI分析舆情时的平台优先级");
    let chips = within(priorityList).getAllByRole("button");
    expect(chips.map((chip) => chip.textContent)).toEqual([
      "雪球",
      "知乎",
      "微博",
      "小红书",
      "抖音",
      "B站",
      "微信公众号",
      "百度",
      "今日头条",
    ]);

    const originalElementsFromPoint = document.elementsFromPoint;
    document.elementsFromPoint = vi.fn(() => [chips[0]]);
    await user.pointer([
      { keys: "[MouseLeft>]", target: chips[2] },
      { target: chips[0] },
      { keys: "[/MouseLeft]" },
    ]);
    document.elementsFromPoint = originalElementsFromPoint;

    chips = within(priorityList).getAllByRole("button");
    expect(chips.map((chip) => chip.textContent)).toEqual([
      "微博",
      "雪球",
      "知乎",
      "小红书",
      "抖音",
      "B站",
      "微信公众号",
      "百度",
      "今日头条",
    ]);
  });

  it("shows a floating preview while dragging sentiment platform priority", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "舆情分析" }));
    const priorityList = screen.getByLabelText("AI分析舆情时的平台优先级");
    const chips = within(priorityList).getAllByRole("button");

    const originalElementsFromPoint = document.elementsFromPoint;
    document.elementsFromPoint = vi.fn(() => [chips[0]]);
    await user.pointer([{ keys: "[MouseLeft>]", target: chips[2] }]);
    await user.pointer([{ target: chips[0] }]);

    expect(document.querySelector(".sentiment-platform-drag-preview")).toHaveTextContent("微博");
    expect(chips[2]).toHaveClass("sentiment-platform-chip--dragging-source");
    expect(within(priorityList).getAllByRole("button").map((chip) => chip.textContent)[0]).toBe("微博");

    await user.pointer([{ keys: "[/MouseLeft]", target: chips[2] }]);
    document.elementsFromPoint = originalElementsFromPoint;
    expect(document.querySelector(".sentiment-platform-drag-preview")).toBeNull();
  });

  it("does not show sentiment login success unless backend status confirms it", async () => {
    const user = userEvent.setup();
    mocks.getSentimentPlatformAuthStatuses
      .mockResolvedValueOnce([
        { platform: "zhihu", hasLoginState: true, capturedAt: "2026-05-12T10:00:00+08:00" },
      ])
      .mockResolvedValueOnce([
        { platform: "zhihu", hasLoginState: false, capturedAt: "2026-05-12T10:01:00+08:00" },
      ]);
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "舆情分析" }));
    await user.click(await screen.findByRole("button", { name: "刷新登录态" }));
    await user.click(screen.getByRole("button", { name: "确定" }));

    expect(await screen.findByText("知乎 登录态获取失败：后端未确认有效登录态。")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "知乎登录态获取" })).toBeInTheDocument();
  });

  it("expands six AKShare test rows and uses the current source selections", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.selectOptions(await screen.findByLabelText("分时数据"), "eastmoney");
    await user.selectOptions(screen.getByLabelText("历史行情数据"), "tencent");
    await user.click(await screen.findByText("测试 AKShare 连接"));

    expect(mocks.testAkshareConnectionItem).toHaveBeenCalledTimes(6);
    expect(mocks.testAkshareConnectionItem).toHaveBeenCalledWith("intraday", {
      historicalDataSource: "tencent",
      intradayDataSource: "eastmoney",
      xueqiuToken: "",
    });
    expect(mocks.testAkshareConnectionItem).toHaveBeenCalledWith("historical", {
      historicalDataSource: "tencent",
      intradayDataSource: "eastmoney",
      xueqiuToken: "",
    });
    expect(await screen.findByText("个股实时行情 · 雪球接口")).toBeInTheDocument();
    expect(screen.getByText("分时数据 · 东方财富")).toBeInTheDocument();
    expect(screen.getByText("历史行情数据 · 腾讯证券")).toBeInTheDocument();
    expect(screen.getByText("财报数据 · 东方财富")).toBeInTheDocument();
    expect(screen.getByText("公司基础资料 · 雪球接口")).toBeInTheDocument();
    expect(screen.getByText("交易日历 · 新浪交易日历")).toBeInTheDocument();
    expect(await screen.findAllByText("成功")).toHaveLength(6);
  });

  it("keeps mixed AKShare test results visible together", async () => {
    mocks.testAkshareConnectionItem.mockImplementation(async (itemId: string) => ({
      itemId,
      ok: itemId !== "financial",
      message: itemId === "financial" ? "财报接口失败" : `${itemId} ok`,
    }));

    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByText("测试 AKShare 连接"));

    expect(await screen.findByText("失败")).toBeInTheDocument();
    expect(screen.getAllByText("成功").length).toBeGreaterThan(0);
    expect(screen.getByText(/AKShare 连接测试完成：5 项成功，1 项失败。/)).toBeInTheDocument();
  });

  it("keeps watchlist editing out of AI settings and exposes KittyAlpha-style model controls", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "AI交易" }));
    const panel = screen.getByRole("tabpanel");
    expect(within(panel).queryByText("自选股票")).not.toBeInTheDocument();
    expect(within(panel).getByText("AI分析")).toBeInTheDocument();
    expect(within(panel).getAllByText("使用财报数据")).toHaveLength(2);
    expect(within(panel).getByLabelText("使用财报数据").closest(".settings-ai-toggle-box")).toBeInTheDocument();
    expect(within(panel).getByLabelText("K线根数")).toHaveValue(60);
    expect(within(panel).getByLabelText("K线根数").closest(".settings-ai-row--inputs")).toBeInTheDocument();
    expect(within(panel).getByLabelText("K线级别")).toBeInTheDocument();
    expect(within(panel).getByLabelText("K线级别").closest(".settings-ai-row--levels")).toBeInTheDocument();
    expect(within(panel).getByText("风险")).toBeInTheDocument();
    expect(within(panel).getByText("策略信号")).toBeInTheDocument();
    expect(within(panel).getByLabelText("单笔最大亏损").tagName).toBe("SELECT");
    expect(within(panel).getByLabelText("日内最大亏损").tagName).toBe("SELECT");
    expect(within(panel).getByLabelText("最低置信度").tagName).toBe("SELECT");

    await user.click(screen.getByRole("tab", { name: "模型" }));
    expect(screen.getByLabelText("模型服务商")).toBeInTheDocument();
    expect(screen.getByLabelText("模型 API Key")).toHaveAttribute("placeholder", "已保存 API Key，可直接覆盖");
    expect(screen.getByLabelText("接口类型")).toBeInTheDocument();
    expect(screen.getByText("AI推荐/回测")).toBeInTheDocument();
    expect(screen.getByText("AI助手")).toBeInTheDocument();
    expect(screen.getByText("AI财报分析")).toBeInTheDocument();
    expect(screen.getByText("AI舆情分析")).toBeInTheDocument();
    expect(screen.getAllByLabelText("温度")).toHaveLength(4);
    expect(screen.getAllByLabelText("最大输出 Token")).toHaveLength(4);
    expect(screen.getAllByLabelText("最大上下文 Token")).toHaveLength(4);
    expect(screen.getAllByLabelText("思考深度")).toHaveLength(4);
    expect(screen.getByLabelText("推荐模型思考深度")).toHaveValue("off");
    expect(screen.getByLabelText("助手模型思考深度")).toHaveValue("off");
    expect(screen.getByLabelText("财报模型思考深度")).toHaveValue("off");
    expect(screen.getByLabelText("舆情模型思考深度")).toHaveValue("off");

    await user.click(screen.getByRole("button", { name: "测试模型连接" }));

    expect(mocks.testModelConnection).toHaveBeenCalled();
    expect(await screen.findByText("模型连接正常")).toBeInTheDocument();
  });

  it("edits full Assistant and AI analysis system prompts", async () => {
    const user = userEvent.setup();
    render(<SettingsPage />);

    await user.click(await screen.findByRole("tab", { name: "提示词" }));

    expect(screen.queryByLabelText("提示词扩展")).not.toBeInTheDocument();
    const assistantPrompt = screen.getByLabelText("Assistant 系统提示词");
    const recommendationPrompt = screen.getByLabelText("AI推荐/回测系统提示词");
    const financialReportPrompt = screen.getByLabelText("AI财报分析系统提示词");
    const sentimentPrompt = screen.getByLabelText("AI舆情分析系统提示词");
    expect((assistantPrompt as HTMLTextAreaElement).value).toContain("KittyRed Assistant");
    expect((recommendationPrompt as HTMLTextAreaElement).value).toContain("沪深 A 股模拟投资助手");
    expect((financialReportPrompt as HTMLTextAreaElement).value).toContain("财报分析助手");
    expect((financialReportPrompt as HTMLTextAreaElement).value).toContain("输出示例");
    expect((financialReportPrompt as HTMLTextAreaElement).value).toContain("\"收入质量\":7");
    expect((sentimentPrompt as HTMLTextAreaElement).value).toContain("舆情分析助手");
    expect((sentimentPrompt as HTMLTextAreaElement).value).toContain("输出示例");
    expect((sentimentPrompt as HTMLTextAreaElement).value).toContain("\"情感倾向\":{\"score\":62");

    await user.clear(assistantPrompt);
    await user.type(assistantPrompt, "新的 Assistant 系统提示词");

    expect(assistantPrompt).toHaveValue("新的 Assistant 系统提示词");
  });
});
