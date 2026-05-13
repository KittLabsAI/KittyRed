import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  loadSettingsFormData,
  saveSettingsFormData,
  testAkshareConnectionItem,
  type SettingsFormData,
} from "./settings";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(async (..._args: unknown[]): Promise<unknown> => undefined),
  storeGet: vi.fn(async (..._args: unknown[]) => null),
  storeSet: vi.fn(async () => undefined),
  storeSave: vi.fn(async () => undefined),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  LazyStore: class {
    get = mocks.storeGet;
    set = mocks.storeSet;
    save = mocks.storeSave;
  },
}));

vi.mock("@tauri-apps/api/path", () => ({
  appLocalDataDir: vi.fn(async () => "/tmp"),
  join: vi.fn(async (...parts: string[]) => parts.join("/")),
}));

vi.mock("@tauri-apps/plugin-stronghold", () => ({
  Stronghold: {
    load: vi.fn(),
  },
}));

function buildSettings(): SettingsFormData {
  return {
    exchanges: [
      {
        exchange: "akshare",
        enabled: true,
        apiKey: "",
        apiSecret: "",
        extraPassphrase: "",
        hasStoredApiKey: false,
        hasStoredApiSecret: false,
        hasStoredExtraPassphrase: false,
        connectionStatus: "market_data_only",
        permissionRead: false,
        permissionTrade: false,
        permissionWithdraw: false,
      },
    ],
    modelPreset: "Custom",
    modelProvider: "OpenAI-compatible",
    modelName: "gpt-5.5",
    modelBaseUrl: "",
    modelApiKey: "",
    xueqiuToken: "",
    intradayDataSource: "sina",
    historicalDataSource: "eastmoney",
    recommendationModel: {
      temperature: 0.2,
      maxTokens: 900,
      maxContext: 16000,
      effortLevel: "off" as const,
    },
    assistantModel: {
      temperature: 0.7,
      maxTokens: 16000,
      maxContext: 128000,
      effortLevel: "off" as const,
    },
    financialReportModel: {
      temperature: 0.2,
      maxTokens: 4096,
      maxContext: 64000,
      effortLevel: "off" as const,
    },
    sentimentAnalysisModel: {
      temperature: 0.2,
      maxTokens: 4096,
      maxContext: 64000,
      effortLevel: "off" as const,
    },
    sentimentPlatformPriority: ["xueqiu", "zhihu", "weibo", "baidu"],
    sentimentFetchRecentDays: 21,
    sentimentItemMaxChars: 800,
    sentimentSamplingOrder: "time_first",
    hasStoredModelApiKey: false,
    hasStoredXueqiuToken: false,
    autoAnalyzeEnabled: true,
    autoAnalyzeFrequency: "10m",
    scanScope: "all_markets",
    watchlistSymbols: ["SHSE.600000"],
    dailyMaxAiCalls: 24,
    useFinancialReportData: true,
    aiKlineBarCount: 60,
    aiKlineFrequencies: ["5m", "1h", "1d", "1w"],
    pauseAfterConsecutiveLosses: 3,
    minConfidenceScore: 60,
    allowedMarkets: "all",
    allowedDirection: "long_short",
    maxLeverage: 3,
    maxLossPerTradePercent: 1,
    maxDailyLossPercent: 3,
    minRiskRewardRatio: 1.5,
    min24hVolume: 25000000,
    maxSpreadBps: 12,
    allowMemeCoins: true,
    whitelistSymbols: ["SHSE.600000"],
    blacklistSymbols: [],
    promptExtension: "",
    assistantSystemPrompt: "Assistant 系统提示词",
    recommendationSystemPrompt: "AI 推荐系统提示词",
    financialReportSystemPrompt: "AI 财报分析系统提示词",
    sentimentAnalysisSystemPrompt: "AI 舆情分析系统提示词",
    accountMode: "paper",
    autoPaperExecution: false,
    notifications: {
      recommendations: true,
      spreads: true,
      paperOrders: true,
    },
    signalsEnabled: false,
    signalScanFrequency: "15m",
    signalMinScore: 30,
    signalCooldownMinutes: 15,
    signalDailyMax: 50,
    signalAutoExecute: false,
    signalNotifications: false,
    signalWatchlistSymbols: [],
  };
}

describe("saveSettingsFormData", () => {
  beforeEach(() => {
    mocks.invoke.mockClear();
    mocks.storeGet.mockClear();
    mocks.storeSet.mockClear();
    mocks.storeSave.mockClear();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("maps min24hVolume to minVolume24h for the tauri runtime payload", async () => {
    await saveSettingsFormData(buildSettings(), "");

    expect(mocks.invoke).toHaveBeenCalledWith(
      "save_runtime_settings",
      expect.objectContaining({
        settings: expect.objectContaining({
          minVolume24h: 25000000,
          intradayDataSource: "sina",
          historicalDataSource: "eastmoney",
          useFinancialReportData: true,
          sentimentPlatformPriority: ["xueqiu", "zhihu", "weibo", "baidu"],
          sentimentFetchRecentDays: 21,
          sentimentItemMaxChars: 800,
          sentimentSamplingOrder: "time_first",
          financialReportSystemPrompt: "AI 财报分析系统提示词",
          sentimentAnalysisSystemPrompt: "AI 舆情分析系统提示词",
        }),
      }),
    );

    const runtimeCalls = mocks.invoke.mock.calls as unknown as Array<
      [string, { settings?: Record<string, unknown> }]
    >;
    const runtimeCall = runtimeCalls.find(([command]) => command === "save_runtime_settings");
    const runtimePayload = runtimeCall?.[1];
    expect(runtimePayload?.settings).not.toHaveProperty("min24hVolume");
    expect(runtimePayload?.settings).not.toHaveProperty("modelApiKey");
    expect(runtimePayload?.settings).not.toHaveProperty("modelPreset");
  });

  it("persists the model API key through the backend secret store even without a vault password", async () => {
    await saveSettingsFormData(
      {
        ...buildSettings(),
        modelApiKey: "sk-test",
      },
      "",
    );

    expect(mocks.invoke).toHaveBeenCalledWith(
      "sync_runtime_secrets",
      expect.objectContaining({
        payload: expect.objectContaining({
          persist: true,
          modelApiKey: "sk-test",
        }),
      }),
    );
    expect(mocks.storeSet).toHaveBeenCalledWith(
      "settings",
      expect.objectContaining({
        hasStoredModelApiKey: true,
      }),
    );
  });

  it("persists the xueqiu token through the backend secret store without storing the plaintext in metadata", async () => {
    await saveSettingsFormData(
      {
        ...buildSettings(),
        xueqiuToken: "xq-token-test",
      },
      "",
    );

    expect(mocks.invoke).toHaveBeenCalledWith(
      "sync_runtime_secrets",
      {
        payload: expect.objectContaining({
          xueqiuToken: "xq-token-test",
        }),
      },
    );
    expect(mocks.storeSet).toHaveBeenCalledWith(
      "settings",
      expect.objectContaining({
        hasStoredXueqiuToken: true,
        xueqiuToken: "",
      }),
    );
  });

  it("keeps an existing stored xueqiu token when the write-only field stays blank", async () => {
    await saveSettingsFormData(
      {
        ...buildSettings(),
        hasStoredXueqiuToken: true,
        xueqiuToken: "",
      },
      "",
    );

    expect(mocks.invoke).toHaveBeenCalledWith(
      "sync_runtime_secrets",
      {
        payload: expect.objectContaining({
          xueqiuToken: null,
        }),
      },
    );
  });

  it("keeps an existing stored model API key when the write-only field stays blank", async () => {
    await saveSettingsFormData(
      {
        ...buildSettings(),
        hasStoredModelApiKey: true,
        modelApiKey: "",
      },
      "",
    );

    expect(mocks.invoke).toHaveBeenCalledWith(
      "sync_runtime_secrets",
      expect.objectContaining({
        payload: expect.objectContaining({
          modelApiKey: null,
        }),
      }),
    );
  });

  it("returns the updated default model settings when no metadata has been stored yet", async () => {
    const loaded = await loadSettingsFormData();

    expect(loaded.modelPreset).toBe("Custom");
    expect(loaded.modelProvider).toBe("OpenAI-compatible");
    expect(loaded.intradayDataSource).toBe("sina");
    expect(loaded.historicalDataSource).toBe("eastmoney");
    expect(loaded.recommendationModel).toEqual({
      temperature: 0.2,
      maxTokens: 900,
      maxContext: 16000,
      effortLevel: "off",
    });
    expect(loaded.assistantModel.temperature).toBe(0.7);
    expect(loaded.financialReportModel.temperature).toBe(0.2);
    expect(loaded.financialReportSystemPrompt).toContain("财报分析助手");
    expect(loaded.financialReportSystemPrompt).toContain("输出示例");
    expect(loaded.financialReportSystemPrompt).toContain("\"收入质量\":7");
    expect(loaded.sentimentAnalysisSystemPrompt).toContain("舆情分析助手");
    expect(loaded.sentimentAnalysisSystemPrompt).toContain("输出示例");
    expect(loaded.sentimentAnalysisSystemPrompt).toContain("\"情感倾向\":{\"score\":62");
    expect(loaded.sentimentPlatformPriority).toEqual([
      "xueqiu",
      "zhihu",
      "weibo",
      "xiaohongshu",
      "douyin",
      "bilibili",
      "wechat",
      "baidu",
      "toutiao",
    ]);
    expect(loaded.sentimentFetchRecentDays).toBe(30);
    expect(loaded.sentimentItemMaxChars).toBe(420);
    expect(loaded.sentimentSamplingOrder).toBe("time_first");
  });

  it("upgrades previously saved short analysis prompt defaults to the full prompts with examples", async () => {
    mocks.storeGet.mockResolvedValueOnce(({
      financialReportSystemPrompt:
        "你是 KittyRed 的沪深 A 股财报分析助手。只输出一个 JSON 对象，不要 Markdown、代码块或任何前后缀。前十个字段是整数评分，后四个字段是文本分析。字段名必须完全一致：收入质量、毛利水平、净利与回报、盈利调节、偿债能力、现金流状况、业绩增速、研发及资本投入、营运效率、资产质量、关键信息总结、财报正向因素、财报负向因素、财报造假嫌疑点。分数字段上限依次为 8、10、12、5、15、15、12、8、10、5。分数越高越好，请根据财报数据客观打分。不要输出财报综合评分，综合分由系统计算。不要给实盘交易指令，不要提及其他市场。",
      sentimentAnalysisSystemPrompt:
        "你是 KittyRed 的沪深 A 股舆情分析助手。只输出一个 JSON 对象，不要 Markdown、代码块或任何前后缀。任务：基于用户提供的真实社媒讨论，分别给出六个维度的 0-100 整数分数和判断原因。原因必须尽量引用输入中的平台、作者、标题、原文摘要或链接；不要编造不存在的来源、观点或数据。字段名必须完全一致：情感倾向、关注热度、传播动能、信息影响力、来源可靠性、舆论共识度。每个字段的值必须是对象，包含 score 和 reason。不要输出总分，总分由系统按六个 score 的平均值计算。不要给实盘交易指令，不要提及其他市场。",
    } as unknown) as null);

    const loaded = await loadSettingsFormData();

    expect(loaded.financialReportSystemPrompt).toContain("输出示例");
    expect(loaded.financialReportSystemPrompt).toContain("\"收入质量\":7");
    expect(loaded.sentimentAnalysisSystemPrompt).toContain("输出示例");
    expect(loaded.sentimentAnalysisSystemPrompt).toContain("\"情感倾向\":{\"score\":62");
  });

  it("loads stored model api key and xueqiu token back into the form from the backend", async () => {
    mocks.storeGet.mockResolvedValueOnce(({
      ...buildSettings(),
      hasStoredModelApiKey: true,
      hasStoredXueqiuToken: true,
    } as unknown) as null);
    mocks.invoke.mockImplementation((async (...args: unknown[]) => {
      const command = String(args[0] ?? "");
      if (command === "get_settings_snapshot") {
        return {
          exchange_credentials: [],
        };
      }
      if (command === "get_settings_secrets") {
        return {
          modelApiKey: "sk-restored",
          xueqiuToken: "xq-restored",
        };
      }
      return undefined;
    }) as (...args: unknown[]) => Promise<undefined>);

    const loaded = await loadSettingsFormData();

    expect(loaded.modelApiKey).toBe("sk-restored");
    expect(loaded.xueqiuToken).toBe("xq-restored");
  });

  it("passes current AKShare source selections and xueqiu token overrides to the single-item test command", async () => {
    mocks.invoke.mockImplementationOnce(async () => ({
      itemId: "intraday",
      ok: true,
      message: "分时数据测试成功",
    }));

    await testAkshareConnectionItem("intraday", {
      intradayDataSource: "eastmoney",
      historicalDataSource: "tencent",
      xueqiuToken: "xq-live-token",
    });

    expect(mocks.invoke).toHaveBeenCalledWith("test_akshare_connection_item", {
      payload: {
        itemId: "intraday",
        intradayDataSource: "eastmoney",
        historicalDataSource: "tencent",
        xueqiuToken: "xq-live-token",
      },
    });
  });

  it("keeps using the stored xueqiu token when the in-page field stays blank", async () => {
    mocks.invoke.mockImplementationOnce(async () => ({
      itemId: "quote",
      ok: true,
      message: "个股实时行情测试成功",
    }));

    await testAkshareConnectionItem("quote", {
      intradayDataSource: "sina",
      historicalDataSource: "eastmoney",
      xueqiuToken: "",
    });

    expect(mocks.invoke).toHaveBeenCalledWith("test_akshare_connection_item", {
      payload: {
        itemId: "quote",
        intradayDataSource: "sina",
        historicalDataSource: "eastmoney",
        xueqiuToken: null,
      },
    });
  });
});
