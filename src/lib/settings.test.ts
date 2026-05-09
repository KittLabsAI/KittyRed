import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadSettingsFormData, saveSettingsFormData, type SettingsFormData } from "./settings";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(async () => undefined),
  storeGet: vi.fn(async () => null),
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
    modelTemperature: 0,
    modelMaxTokens: 4096,
    modelMaxContext: 128000,
    hasStoredModelApiKey: false,
    autoAnalyzeEnabled: true,
    autoAnalyzeFrequency: "10m",
    scanScope: "all_markets",
    watchlistSymbols: ["SHSE.600000"],
    dailyMaxAiCalls: 24,
    useBidAskData: true,
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
          useFinancialReportData: true,
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
    expect(loaded.modelTemperature).toBe(0);
    expect(loaded.modelMaxTokens).toBe(4096);
    expect(loaded.modelMaxContext).toBe(128000);
  });
});
