import { invoke } from "@tauri-apps/api/core";
import { appLocalDataDir, join } from "@tauri-apps/api/path";
import { LazyStore } from "@tauri-apps/plugin-store";
import { Stronghold } from "@tauri-apps/plugin-stronghold";

export type AnalyzeFrequency = "5m" | "10m" | "30m" | "1h";
export type SignalScanFrequency = "5m" | "10m" | "15m" | "30m" | "1h";
export type AiKlineFrequency = "1m" | "5m" | "30m" | "1h" | "1d" | "1w" | "1M";
export type ScanScopeSetting = "all_markets" | "watchlist_only";
export type AccountModeSetting = "paper" | "real_read_only" | "dual";
export type AllowedMarketsSetting = "all" | "spot" | "perpetual";
export type AllowedDirectionSetting =
  | "long_short"
  | "long_only"
  | "observe_only";
export type ModelInterfaceSetting =
  | "OpenAI-compatible"
  | "Anthropic-compatible";
export type ExchangeConnectionStatus =
  | "disabled"
  | "market_data_only"
  | "connected_read_only"
  | "configured_unverified"
  | "blocked_region"
  | "auth_error";

export type ExchangeSecretDraft = {
  exchange: string;
  enabled: boolean;
  apiKey: string;
  apiSecret: string;
  extraPassphrase: string;
  hasStoredApiKey: boolean;
  hasStoredApiSecret: boolean;
  hasStoredExtraPassphrase: boolean;
  connectionStatus: ExchangeConnectionStatus;
  permissionRead: boolean;
  permissionTrade: boolean;
  permissionWithdraw: boolean;
};

export type SettingsFormData = {
  exchanges: ExchangeSecretDraft[];
  modelPreset: string;
  modelProvider: string;
  modelName: string;
  modelBaseUrl: string;
  modelApiKey: string;
  modelTemperature: number;
  modelMaxTokens: number;
  modelMaxContext: number;
  hasStoredModelApiKey: boolean;
  autoAnalyzeEnabled: boolean;
  autoAnalyzeFrequency: AnalyzeFrequency;
  scanScope: ScanScopeSetting;
  watchlistSymbols: string[];
  dailyMaxAiCalls: number;
  useBidAskData: boolean;
  useFinancialReportData: boolean;
  aiKlineBarCount: number;
  aiKlineFrequencies: AiKlineFrequency[];
  pauseAfterConsecutiveLosses: number;
  minConfidenceScore: number;
  allowedMarkets: AllowedMarketsSetting;
  allowedDirection: AllowedDirectionSetting;
  maxLeverage: number;
  maxLossPerTradePercent: number;
  maxDailyLossPercent: number;
  minRiskRewardRatio: number;
  min24hVolume: number;
  maxSpreadBps: number;
  allowMemeCoins: boolean;
  whitelistSymbols: string[];
  blacklistSymbols: string[];
  promptExtension: string;
  assistantSystemPrompt: string;
  recommendationSystemPrompt: string;
  accountMode: AccountModeSetting;
  autoPaperExecution: boolean;
  notifications: {
    recommendations: boolean;
    spreads: boolean;
    paperOrders: boolean;
  };
  signalsEnabled: boolean;
  signalScanFrequency: SignalScanFrequency;
  signalMinScore: number;
  signalCooldownMinutes: number;
  signalDailyMax: number;
  signalAutoExecute: boolean;
  signalNotifications: boolean;
  signalWatchlistSymbols: string[];
};

export type ModelProviderPreset = {
  provider: string;
  baseUrl: string;
  interface: ModelInterfaceSetting;
};

export type ModelConnectionDraft = Pick<
  SettingsFormData,
  | "modelProvider"
  | "modelName"
  | "modelBaseUrl"
  | "modelApiKey"
  | "modelTemperature"
  | "modelMaxTokens"
  | "modelMaxContext"
>;

export type ConnectionTestResult = {
  ok: boolean;
  message: string;
};

export type ExchangeConnectionTestResult = {
  status: ExchangeConnectionStatus;
  permissionRead: boolean;
  permissionTrade: boolean;
  permissionWithdraw: boolean;
  message: string;
};

type PersistedSettings = Omit<SettingsFormData, "modelApiKey" | "exchanges"> & {
  exchanges: Array<
    Omit<
      ExchangeSecretDraft,
      | "apiKey"
      | "apiSecret"
      | "extraPassphrase"
      | "connectionStatus"
      | "permissionRead"
      | "permissionTrade"
      | "permissionWithdraw"
    >
  >;
};

type RuntimeSettingsDto = Omit<
  PersistedSettings,
  "min24hVolume" | "modelPreset"
> & {
  minVolume24h: number;
};

type RuntimeSecretsSyncDto = {
  persist: boolean;
  modelApiKey: string | null;
  exchanges: Array<{
    exchange: string;
    apiKey: string | null;
    apiSecret: string | null;
    extraPassphrase: string | null;
  }>;
};

type SettingsSnapshotDto = {
  exchange_credentials: Array<{
    exchange: string;
    status: ExchangeConnectionStatus;
    permission_read: boolean;
    permission_trade: boolean;
    permission_withdraw: boolean;
  }>;
};

const SETTINGS_STORAGE_PATH = "kittyalpha.settings.json";
const SETTINGS_STORAGE_KEY = "settings";
const FALLBACK_METADATA_KEY = "kittyalpha.settings.metadata";
const FALLBACK_SECRET_PREFIX = "kittyalpha.secret.";
export const CUSTOM_MODEL_PROVIDER = "Custom";
export const MODEL_PROVIDER_PRESETS: ModelProviderPreset[] = [
  {
    provider: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Zhipu GLM",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Zhipu GLM en",
    baseUrl: "https://api.z.ai/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Bailian",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Kimi",
    baseUrl: "https://api.moonshot.cn/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Kimi For Coding",
    baseUrl: "https://api.kimi.com/coding",
    interface: "Anthropic-compatible",
  },
  {
    provider: "StepFun",
    baseUrl: "https://api.stepfun.ai/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Minimax",
    baseUrl: "https://api.minimaxi.com/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Minimax en",
    baseUrl: "https://platform.minimax.io",
    interface: "OpenAI-compatible",
  },
  {
    provider: "DouBaoSeed",
    baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Xiaomi MiMo",
    baseUrl: "https://api.xiaomimimo.com/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "ModelScope",
    baseUrl: "https://api-inference.modelscope.cn/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "OpenRouter",
    baseUrl: "https://openrouter.ai/api/v1",
    interface: "OpenAI-compatible",
  },
  {
    provider: "Ollama",
    baseUrl: "http://localhost:11434/v1",
    interface: "OpenAI-compatible",
  },
];
const DEFAULT_EXCHANGES: string[] = [];
export const DEFAULT_ASSISTANT_SYSTEM_PROMPT =
  "你是 KittyRed Assistant，只服务沪深 A 股和本地模拟投资。需要行情、个股资料、盘口、K 线、组合、持仓、建议或风险事实时必须调用工具，不要猜测。用简洁中文 Markdown 回答。如果缓存行情不可用，要明确说明并建议用户刷新自选股行情，不要编造实时行情。只有用户明确要求创建模拟委托草稿时，才调用 paper_order_draft。";
export const DEFAULT_RECOMMENDATION_SYSTEM_PROMPT =
  "你是 KittyRed 的沪深 A 股模拟投资助手。只输出 JSON，不要输出 Markdown 或解释性前后缀。必须始终提供 rationale。没有清晰机会时返回 has_trade=false，并在 rationale 里说明最重要的 2 到 3 个未满足条件。如果 has_trade=true，只能给本地模拟买入或已有持仓卖出计划，必须包含 direction、confidence_score、rationale、entry_low、entry_high、stop_loss、take_profit、amount_cny、invalidation、max_loss_cny。卖出只适用于 position_context 存在的股票，代表退出或减仓本地模拟持仓，不代表开空仓；无持仓股票只能返回买入或观望。不要输出杠杆、真实交易、券商账户、其他市场或套利建议。has_trade=false 时不要只写“暂无机会”，要结合输入中的价格、成交额、价差、K 线或风控阈值说明原因。";

const inMemoryFallbackStorage = new Map<string, string>();

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function getBrowserStorage() {
  if (
    typeof globalThis !== "undefined" &&
    "localStorage" in globalThis &&
    typeof globalThis.localStorage?.getItem === "function" &&
    typeof globalThis.localStorage?.setItem === "function"
  ) {
    return globalThis.localStorage;
  }

  return null;
}

function defaultSettings(): SettingsFormData {
  return {
    exchanges: DEFAULT_EXCHANGES.map((exchange) => {
      const enabled = false;
      return {
        exchange,
        enabled,
        apiKey: "",
        apiSecret: "",
        extraPassphrase: "",
        hasStoredApiKey: false,
        hasStoredApiSecret: false,
        hasStoredExtraPassphrase: false,
        connectionStatus: enabled ? "market_data_only" : "disabled",
        permissionRead: false,
        permissionTrade: false,
        permissionWithdraw: false,
      };
    }),
    modelPreset: CUSTOM_MODEL_PROVIDER,
    modelProvider: "OpenAI-compatible",
    modelName: "gpt-5.5",
    modelBaseUrl: "",
    modelApiKey: "",
    modelTemperature: 0,
    modelMaxTokens: 4_096,
    modelMaxContext: 128_000,
    hasStoredModelApiKey: false,
    autoAnalyzeEnabled: true,
    autoAnalyzeFrequency: "10m",
    scanScope: "all_markets",
    watchlistSymbols: [],
    dailyMaxAiCalls: 24,
    useBidAskData: true,
    useFinancialReportData: false,
    aiKlineBarCount: 60,
    aiKlineFrequencies: ["5m", "1h", "1d", "1w"],
    pauseAfterConsecutiveLosses: 3,
    minConfidenceScore: 60,
    allowedMarkets: "perpetual",
    allowedDirection: "long_short",
    maxLeverage: 3,
    maxLossPerTradePercent: 1,
    maxDailyLossPercent: 3,
    minRiskRewardRatio: 1.5,
    min24hVolume: 20_000_000,
    maxSpreadBps: 12,
    allowMemeCoins: true,
    whitelistSymbols: [],
    blacklistSymbols: [],
    promptExtension: "",
    assistantSystemPrompt: DEFAULT_ASSISTANT_SYSTEM_PROMPT,
    recommendationSystemPrompt: DEFAULT_RECOMMENDATION_SYSTEM_PROMPT,
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

export function createDefaultSettingsFormData(): SettingsFormData {
  return defaultSettings();
}

function normalizeModelInterface(value?: string | null): ModelInterfaceSetting {
  return value?.trim().toLowerCase() === "anthropic-compatible"
    ? "Anthropic-compatible"
    : "OpenAI-compatible";
}

function normalizeModelPreset(
  value?: string | null,
  baseUrl?: string | null,
): string {
  if (typeof value === "string" && value.trim().length > 0) {
    return value.trim();
  }

  const normalizedBaseUrl = baseUrl?.trim();
  const preset = MODEL_PROVIDER_PRESETS.find(
    (item) => item.baseUrl === normalizedBaseUrl,
  );
  return preset?.provider ?? CUSTOM_MODEL_PROVIDER;
}

function normalizeSymbolList(
  values: Array<string | null | undefined>,
): string[] {
  const normalized: string[] = [];

  for (const value of values) {
    const cleaned = typeof value === "string" ? value.trim().toUpperCase() : "";
    if (!cleaned || normalized.includes(cleaned)) {
      continue;
    }
    normalized.push(cleaned);
  }

  return normalized;
}

function normalizeAiKlineFrequencies(
  values: Array<string | null | undefined> | undefined,
): AiKlineFrequency[] {
  const allowed: AiKlineFrequency[] = ["1m", "5m", "30m", "1h", "1d", "1w", "1M"];
  const normalized = (values ?? []).filter((value): value is AiKlineFrequency =>
    allowed.includes(value as AiKlineFrequency),
  );
  return normalized.length > 0 ? Array.from(new Set(normalized)) : ["5m", "1h", "1d", "1w"];
}

function normalizePersistedSettings(
  value: Partial<PersistedSettings> | null | undefined,
): SettingsFormData {
  const defaults = defaultSettings();
  const persistedExchanges = new Map(
    (value?.exchanges ?? []).map((item) => [item.exchange, item]),
  );
  const watchlistSymbols = Array.isArray(value?.watchlistSymbols)
    ? normalizeSymbolList(value.watchlistSymbols)
    : defaults.watchlistSymbols;
  const whitelistSymbols = Array.isArray(value?.whitelistSymbols)
    ? normalizeSymbolList(value.whitelistSymbols)
    : defaults.whitelistSymbols;
  const blacklistSymbols = Array.isArray(value?.blacklistSymbols)
    ? normalizeSymbolList(value.blacklistSymbols)
    : defaults.blacklistSymbols;

  return {
    ...defaults,
    ...value,
    exchanges: defaults.exchanges.map((item) => {
      const persisted = persistedExchanges.get(item.exchange);
      return {
        ...item,
        ...persisted,
        apiKey: "",
        apiSecret: "",
        extraPassphrase: "",
        connectionStatus: item.connectionStatus,
        permissionRead: item.permissionRead,
        permissionTrade: item.permissionTrade,
        permissionWithdraw: item.permissionWithdraw,
      };
    }),
    notifications: {
      ...defaults.notifications,
      ...(value?.notifications ?? {}),
    },
    modelPreset: normalizeModelPreset(value?.modelPreset, value?.modelBaseUrl),
    modelProvider: normalizeModelInterface(value?.modelProvider),
    scanScope:
      value?.scanScope === "watchlist_only" ||
      value?.scanScope === "all_markets"
        ? value.scanScope
        : defaults.scanScope,
    allowedMarkets: "perpetual",
    watchlistSymbols,
    whitelistSymbols,
    blacklistSymbols,
    useBidAskData: value?.useBidAskData ?? defaults.useBidAskData,
    useFinancialReportData:
      value?.useFinancialReportData ?? defaults.useFinancialReportData,
    aiKlineBarCount: Math.max(1, Number(value?.aiKlineBarCount ?? defaults.aiKlineBarCount)),
    aiKlineFrequencies: normalizeAiKlineFrequencies(value?.aiKlineFrequencies),
    modelApiKey: "",
    hasStoredModelApiKey:
      value?.hasStoredModelApiKey ?? defaults.hasStoredModelApiKey,
    autoAnalyzeFrequency:
      (value?.autoAnalyzeFrequency as AnalyzeFrequency | undefined) ??
      defaults.autoAnalyzeFrequency,
    signalWatchlistSymbols: Array.isArray(value?.signalWatchlistSymbols)
      ? normalizeSymbolList(value.signalWatchlistSymbols)
      : defaults.signalWatchlistSymbols,
    signalScanFrequency:
      value?.signalScanFrequency === "5m" ||
      value?.signalScanFrequency === "10m" ||
      value?.signalScanFrequency === "15m" ||
      value?.signalScanFrequency === "30m" ||
      value?.signalScanFrequency === "1h"
        ? value.signalScanFrequency
        : defaults.signalScanFrequency,
  };
}

function toPersistedSettings(data: SettingsFormData): PersistedSettings {
  const { modelApiKey: _modelApiKey, exchanges, ...rest } = data;
  return {
    ...rest,
    exchanges: exchanges.map((exchange) => ({
      exchange: exchange.exchange,
      enabled: exchange.enabled,
      hasStoredApiKey: exchange.hasStoredApiKey,
      hasStoredApiSecret: exchange.hasStoredApiSecret,
      hasStoredExtraPassphrase: exchange.hasStoredExtraPassphrase,
    })),
  };
}

async function getMetadataStore() {
  if (!isTauriRuntime()) {
    return null;
  }

  return new LazyStore(SETTINGS_STORAGE_PATH);
}

async function readPersistedMetadata(): Promise<PersistedSettings | null> {
  if (!isTauriRuntime()) {
    const storage = getBrowserStorage();
    const raw =
      storage?.getItem(FALLBACK_METADATA_KEY) ??
      inMemoryFallbackStorage.get(FALLBACK_METADATA_KEY) ??
      null;
    return raw ? (JSON.parse(raw) as PersistedSettings) : null;
  }

  const store = await getMetadataStore();
  return (await store?.get<PersistedSettings>(SETTINGS_STORAGE_KEY)) ?? null;
}

async function writePersistedMetadata(value: PersistedSettings) {
  if (!isTauriRuntime()) {
    const raw = JSON.stringify(value);
    const storage = getBrowserStorage();
    if (storage) {
      storage.setItem(FALLBACK_METADATA_KEY, raw);
    } else {
      inMemoryFallbackStorage.set(FALLBACK_METADATA_KEY, raw);
    }
    return;
  }

  const store = await getMetadataStore();
  await store?.set(SETTINGS_STORAGE_KEY, value);
  await store?.save();
}

function toRuntimeSettingsDto(value: PersistedSettings): RuntimeSettingsDto {
  const { min24hVolume, modelPreset: _modelPreset, ...rest } = value;
  return {
    ...rest,
    minVolume24h: min24hVolume,
  };
}

async function syncRuntimeSettingsSnapshot(value: PersistedSettings) {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("save_runtime_settings", {
    settings: toRuntimeSettingsDto(value),
  });
}

async function syncRuntimeSecrets(payload: RuntimeSecretsSyncDto) {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("sync_runtime_secrets", { payload });
}

async function readRuntimeSnapshot(): Promise<SettingsSnapshotDto | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<SettingsSnapshotDto>("get_settings_snapshot");
}

function mergeSnapshot(
  settings: SettingsFormData,
  snapshot: SettingsSnapshotDto | null,
): SettingsFormData {
  if (!snapshot) {
    return settings;
  }

  const credentialMap = new Map(
    snapshot.exchange_credentials.map((item) => [item.exchange, item]),
  );

  return {
    ...settings,
    exchanges: settings.exchanges.map((exchange) => {
      const credential = credentialMap.get(exchange.exchange);
      if (!credential) {
        return exchange;
      }

      return {
        ...exchange,
        connectionStatus: credential.status,
        permissionRead: credential.permission_read,
        permissionTrade: credential.permission_trade,
        permissionWithdraw: credential.permission_withdraw,
      };
    }),
  };
}

async function getStrongholdStore(password: string) {
  const baseDir = await appLocalDataDir();
  const vaultPath = await join(baseDir, "kittyalpha.vault.hold");
  const stronghold = await Stronghold.load(vaultPath, password);
  let client;

  try {
    client = await stronghold.loadClient("kittyalpha");
  } catch {
    client = await stronghold.createClient("kittyalpha");
  }

  return {
    stronghold,
    store: client.getStore(),
  };
}

async function saveSecret(
  secretKey: string,
  value: string,
  vaultPassword: string,
) {
  if (!value) return;

  if (!isTauriRuntime()) {
    const storage = getBrowserStorage();
    if (storage) {
      storage.setItem(`${FALLBACK_SECRET_PREFIX}${secretKey}`, value);
    } else {
      inMemoryFallbackStorage.set(
        `${FALLBACK_SECRET_PREFIX}${secretKey}`,
        value,
      );
    }
    return;
  }

  const { stronghold, store } = await getStrongholdStore(vaultPassword);
  const bytes = Array.from(new TextEncoder().encode(value));
  await store.insert(secretKey, bytes);
  await stronghold.save();
}

export async function loadSettingsFormData(): Promise<SettingsFormData> {
  const persisted = await readPersistedMetadata();

  if (persisted) {
    await syncRuntimeSettingsSnapshot(persisted).catch(() => undefined);
  }

  const snapshot = await readRuntimeSnapshot().catch(() => null);
  const normalized = mergeSnapshot(
    normalizePersistedSettings(persisted),
    snapshot,
  );

  return normalized;
}

function nextSecretPayload(
  value: string,
  hasStoredValue: boolean,
): string | null {
  if (value.trim().length > 0) {
    return value;
  }

  return hasStoredValue ? null : "";
}

export async function saveSettingsFormData(
  data: SettingsFormData,
  vaultPassword: string,
): Promise<{ secretsPersisted: boolean; message: string }> {
  const strongholdMirrorEnabled =
    isTauriRuntime() && vaultPassword.trim().length > 0;
  const secretsPersisted = true;
  const nextData: SettingsFormData = {
    ...data,
    exchanges: data.exchanges.map((exchange) => ({
      ...exchange,
      hasStoredApiKey:
        exchange.hasStoredApiKey || exchange.apiKey.trim().length > 0,
      hasStoredApiSecret:
        exchange.hasStoredApiSecret || exchange.apiSecret.trim().length > 0,
      hasStoredExtraPassphrase:
        exchange.hasStoredExtraPassphrase ||
        exchange.extraPassphrase.trim().length > 0,
    })),
    hasStoredModelApiKey:
      data.hasStoredModelApiKey || data.modelApiKey.trim().length > 0,
  };
  const persisted = toPersistedSettings(nextData);

  await writePersistedMetadata(persisted);
  await syncRuntimeSettingsSnapshot(persisted);
  await syncRuntimeSecrets({
    persist: secretsPersisted,
    modelApiKey: nextSecretPayload(data.modelApiKey, data.hasStoredModelApiKey),
    exchanges: data.exchanges.map((exchange) => ({
      exchange: exchange.exchange,
      apiKey: nextSecretPayload(exchange.apiKey, exchange.hasStoredApiKey),
      apiSecret: nextSecretPayload(
        exchange.apiSecret,
        exchange.hasStoredApiSecret,
      ),
      extraPassphrase: nextSecretPayload(
        exchange.extraPassphrase,
        exchange.hasStoredExtraPassphrase,
      ),
    })),
  });

  if (strongholdMirrorEnabled) {
    for (const exchange of data.exchanges) {
      await saveSecret(
        `exchange.${exchange.exchange}.apiKey`,
        exchange.apiKey,
        vaultPassword,
      );
      await saveSecret(
        `exchange.${exchange.exchange}.apiSecret`,
        exchange.apiSecret,
        vaultPassword,
      );
      await saveSecret(
        `exchange.${exchange.exchange}.extraPassphrase`,
        exchange.extraPassphrase,
        vaultPassword,
      );
    }

    await saveSecret("model.apiKey", data.modelApiKey, vaultPassword);
  }

  return {
    secretsPersisted,
    message: strongholdMirrorEnabled
      ? "Settings saved. Non-secret fields were synced to the backend, API keys were stored locally, and the Stronghold mirror was updated."
      : "Settings saved. Non-secret fields were synced to the backend, and API keys were stored locally. The optional Stronghold mirror was skipped because the local vault password is empty.",
  };
}

export async function saveAccountModeSetting(
  accountMode: AccountModeSetting,
): Promise<SettingsFormData> {
  const current = normalizePersistedSettings(await readPersistedMetadata());
  const next = {
    ...current,
    accountMode,
  };
  const persisted = toPersistedSettings(next);

  await writePersistedMetadata(persisted);
  await syncRuntimeSettingsSnapshot(persisted);

  return normalizePersistedSettings(persisted);
}

export async function appendWatchlistSymbol(
  symbol: string,
): Promise<SettingsFormData> {
  const current = normalizePersistedSettings(await readPersistedMetadata());
  const next = {
    ...current,
    watchlistSymbols: normalizeSymbolList([
      ...current.watchlistSymbols,
      symbol,
    ]),
  };
  const persisted = toPersistedSettings(next);

  await writePersistedMetadata(persisted);
  await syncRuntimeSettingsSnapshot(persisted);

  return normalizePersistedSettings(persisted);
}

export async function testModelConnection(
  draft: ModelConnectionDraft,
): Promise<ConnectionTestResult> {
  if (!isTauriRuntime()) {
    return {
      ok: true,
      message: "Model connection ok",
    };
  }

  return invoke<ConnectionTestResult>("test_model_connection", {
    payload: {
      modelProvider: draft.modelProvider,
      modelName: draft.modelName,
      modelBaseUrl: draft.modelBaseUrl,
      modelApiKey: draft.modelApiKey,
      modelTemperature: draft.modelTemperature,
      modelMaxTokens: draft.modelMaxTokens,
      modelMaxContext: draft.modelMaxContext,
    },
  });
}

export async function testExchangeConnection(
  exchange: Pick<
    ExchangeSecretDraft,
    "exchange" | "apiKey" | "apiSecret" | "extraPassphrase"
  >,
): Promise<ExchangeConnectionTestResult> {
  if (!isTauriRuntime()) {
    return {
      status:
        exchange.apiKey && exchange.apiSecret
          ? "connected_read_only"
          : "market_data_only",
      permissionRead: Boolean(exchange.apiKey && exchange.apiSecret),
      permissionTrade: false,
      permissionWithdraw: false,
      message: "Exchange connection ok",
    };
  }

  return invoke<ExchangeConnectionTestResult>("test_exchange_connection", {
    payload: {
      exchange: exchange.exchange,
      apiKey: exchange.apiKey,
      apiSecret: exchange.apiSecret,
      extraPassphrase: exchange.extraPassphrase,
    },
  });
}

export async function deleteExchangeCredentials(
  exchange: string,
): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("delete_exchange_credentials", { exchange });
}
