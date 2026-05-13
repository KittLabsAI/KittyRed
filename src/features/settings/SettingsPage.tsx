import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  CUSTOM_MODEL_PROVIDER,
  createDefaultSettingsFormData,
  loadSettingsFormData,
  MODEL_PROVIDER_PRESETS,
  saveSettingsFormData,
  testAkshareConnectionItem,
  testModelConnection,
  type AkshareConnectionTestDraft,
  type AkshareConnectionTestItemId,
  type AiKlineFrequency,
  type EffortLevel,
  type HistoricalDataSourceSetting,
  type IntradayDataSourceSetting,
  type ModelInterfaceSetting,
  type SentimentSamplingOrder,
  type SettingsFormData,
} from "../../lib/settings";
import { useAppStore } from "../../store/appStore";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Select } from "../../components/ui/select";
import { Textarea } from "../../components/ui/textarea";
import {
  captureSentimentPlatformLoginState,
  getSentimentPlatformAuthStatuses,
  testSentimentPlatformConnections,
} from "../../lib/tauri";
import type { SentimentPlatformAuthStatus } from "../../lib/types";

type SettingsTab = {
  id: string;
  label: string;
  blurb: string;
};

const tabs: SettingsTab[] = [
  { id: "akshare", label: "数据源", blurb: "检查 AKShare 本地数据接口状态。" },
  { id: "sentiment", label: "舆情分析", blurb: "管理社媒平台登录态并测试平台连接。" },
  { id: "models", label: "模型", blurb: "设置 AI 分析使用的模型服务。" },
  { id: "aiTrade", label: "AI交易", blurb: "集中设置 AI 分析、风险阈值和策略信号。" },
  { id: "prompt", label: "提示词", blurb: "编辑 Assistant 和 AI 推荐的完整系统提示词。" },
];

const modelInterfaceOptions: Array<{ id: ModelInterfaceSetting; label: string }> = [
  { id: "OpenAI-compatible", label: "OpenAI 兼容" },
  { id: "Anthropic-compatible", label: "Anthropic 兼容" },
];

const aiKlineFrequencyOptions: Array<{ id: AiKlineFrequency; label: string }> = [
  { id: "1m", label: "1分钟" },
  { id: "5m", label: "5分钟" },
  { id: "30m", label: "30分钟" },
  { id: "1h", label: "1小时" },
  { id: "1d", label: "1天" },
  { id: "1w", label: "1周" },
  { id: "1M", label: "1月" },
];

const intradayDataSourceOptions: Array<{
  id: IntradayDataSourceSetting;
  label: string;
}> = [
  { id: "sina", label: "新浪财经" },
  { id: "eastmoney", label: "东方财富" },
];

const historicalDataSourceOptions: Array<{
  id: HistoricalDataSourceSetting;
  label: string;
}> = [
  { id: "sina", label: "新浪财经" },
  { id: "eastmoney", label: "东方财富" },
  { id: "tencent", label: "腾讯证券" },
];

const effortLevelOptions: Array<{ id: EffortLevel; label: string }> = [
  { id: "off", label: "关闭" },
  { id: "low", label: "低" },
  { id: "medium", label: "中" },
  { id: "high", label: "高" },
];

type AkshareConnectionRowStatus = "testing" | "success" | "error";
type SentimentLoginPlatform = "zhihu" | "xiaohongshu" | "douyin" | "xueqiu";

type AkshareConnectionRow = {
  id: AkshareConnectionTestItemId;
  label: string;
  sourceLabel: string;
  status: AkshareConnectionRowStatus;
  message: string;
};

type SentimentPlatformDragState = {
  platform: string;
  x: number;
  y: number;
  width: number;
  height: number;
};

const intradayDataSourceLabels: Record<IntradayDataSourceSetting, string> = {
  sina: "新浪财经",
  eastmoney: "东方财富",
};

const historicalDataSourceLabels: Record<HistoricalDataSourceSetting, string> = {
  sina: "新浪财经",
  eastmoney: "东方财富",
  tencent: "腾讯证券",
};

const akshareConnectionItems: Array<{
  id: AkshareConnectionTestItemId;
  label: string;
  sourceLabel: (form: SettingsFormData) => string;
}> = [
  { id: "quote", label: "个股实时行情", sourceLabel: () => "雪球接口" },
  {
    id: "intraday",
    label: "分时数据",
    sourceLabel: (form) => intradayDataSourceLabels[form.intradayDataSource],
  },
  {
    id: "historical",
    label: "历史行情数据",
    sourceLabel: (form) => historicalDataSourceLabels[form.historicalDataSource],
  },
  { id: "financial", label: "财报数据", sourceLabel: () => "东方财富" },
  { id: "companyInfo", label: "公司基础资料", sourceLabel: () => "雪球接口" },
  { id: "tradeCalendar", label: "交易日历", sourceLabel: () => "新浪交易日历" },
];

const sentimentLoginPlatforms: Array<{ id: SentimentLoginPlatform; label: string; url: string }> = [
  { id: "zhihu", label: "知乎", url: "https://www.zhihu.com" },
  { id: "xiaohongshu", label: "小红书", url: "https://www.xiaohongshu.com" },
  { id: "douyin", label: "抖音", url: "https://www.douyin.com" },
  { id: "xueqiu", label: "雪球", url: "https://xueqiu.com" },
];

const sentimentPlatformLabels: Record<string, string> = {
  weibo: "微博",
  xiaohongshu: "小红书",
  bilibili: "B站",
  zhihu: "知乎",
  douyin: "抖音",
  wechat: "微信公众号",
  baidu: "百度",
  toutiao: "今日头条",
  xueqiu: "雪球",
};

const sentimentSamplingOrderOptions: Array<{ id: SentimentSamplingOrder; label: string }> = [
  { id: "time_first", label: "时间优先" },
  { id: "platform_first", label: "平台优先" },
];

function sentimentPlatformLabel(platform: string) {
  return sentimentPlatformLabels[platform] ?? platform;
}

function statusIcon(status: AkshareConnectionRowStatus) {
  if (status === "success") {
    return <span aria-label="成功" className="text-base font-semibold text-emerald-500">✓</span>;
  }
  if (status === "error") {
    return <span aria-label="失败" className="text-base font-semibold text-red-500">✕</span>;
  }
  return <span aria-label="测试中" className="text-base font-semibold text-muted-foreground">○</span>;
}

function statusLabel(status: AkshareConnectionRowStatus) {
  if (status === "success") {
    return "成功";
  }
  if (status === "error") {
    return "失败";
  }
  return "测试中";
}

function buildAkshareConnectionRows(form: SettingsFormData): AkshareConnectionRow[] {
  return akshareConnectionItems.map((item) => ({
    id: item.id,
    label: item.label,
    sourceLabel: item.sourceLabel(form),
    status: "testing",
    message: "测试中",
  }));
}

function modelProviderOptions(currentPreset: string) {
  const options = [
    {
      provider: CUSTOM_MODEL_PROVIDER,
      baseUrl: "",
      interface: "OpenAI-compatible" as ModelInterfaceSetting,
    },
    ...MODEL_PROVIDER_PRESETS,
  ];

  if (
    currentPreset.trim().length > 0 &&
    currentPreset !== CUSTOM_MODEL_PROVIDER &&
    !MODEL_PROVIDER_PRESETS.some((preset) => preset.provider === currentPreset)
  ) {
    options.push({
      provider: currentPreset,
      baseUrl: "",
      interface: "OpenAI-compatible" as ModelInterfaceSetting,
    });
  }

  return options;
}

function Field({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
  );
}

export function SettingsPage() {
  const setAccountMode = useAppStore((state) => state.setAccountMode);
  const [activeTab, setActiveTab] = useState("akshare");
  const [form, setForm] = useState<SettingsFormData>(
    createDefaultSettingsFormData(),
  );
  const [statusMessage, setStatusMessage] = useState("正在加载设置...");
  const [isSaving, setIsSaving] = useState(false);
  const [isTestingAkshare, setIsTestingAkshare] = useState(false);
  const [isTestingModel, setIsTestingModel] = useState(false);
  const [akshareConnectionRows, setAkshareConnectionRows] = useState<AkshareConnectionRow[]>([]);
  const [sentimentAuthStatuses, setSentimentAuthStatuses] = useState<SentimentPlatformAuthStatus[]>([]);
  const [sentimentConnectionRows, setSentimentConnectionRows] = useState<AkshareConnectionRow[]>([]);
  const [isTestingSentiment, setIsTestingSentiment] = useState(false);
  const [isCapturingSentimentLogin, setIsCapturingSentimentLogin] = useState(false);
  const [loginPlatform, setLoginPlatform] = useState<(typeof sentimentLoginPlatforms)[number] | null>(null);
  const draggedSentimentPlatformRef = useRef<string | null>(null);
  const [sentimentPlatformDrag, setSentimentPlatformDrag] = useState<SentimentPlatformDragState | null>(null);
  const currentTab = tabs.find((tab) => tab.id === activeTab) ?? tabs[0];

  useEffect(() => {
    let cancelled = false;

    loadSettingsFormData()
      .then((data) => {
        if (!cancelled) {
          setForm({ ...data, accountMode: "paper" });
          setAccountMode("paper");
          setStatusMessage("设置已加载。当前仅启用模拟账号模式。");
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setStatusMessage(`设置加载失败：${String(error)}`);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [setAccountMode]);

  useEffect(() => {
    let cancelled = false;
    getSentimentPlatformAuthStatuses()
      .then((items) => {
        if (!cancelled) {
          setSentimentAuthStatuses(items);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  async function handleSave() {
    setIsSaving(true);
    try {
      await saveSettingsFormData({ ...form, accountMode: "paper" }, "");
      setStatusMessage("设置已保存。AKShare 将作为 A 股行情和模拟交易价格来源。");
    } catch (error) {
      setStatusMessage(`保存失败：${String(error)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function handleTestAkshareConnection() {
    setIsTestingAkshare(true);
    const testDraft: AkshareConnectionTestDraft = {
      intradayDataSource: form.intradayDataSource,
      historicalDataSource: form.historicalDataSource,
      xueqiuToken: form.xueqiuToken,
    };
    const nextRows = buildAkshareConnectionRows(form);
    setAkshareConnectionRows(nextRows);
    setStatusMessage("正在测试 AKShare 连接...");
    try {
      const results = await Promise.all(
        akshareConnectionItems.map(async (item) => {
          const result = await testAkshareConnectionItem(item.id, testDraft).catch((error) => ({
            itemId: item.id,
            ok: false,
            message: String(error),
          }));

          setAkshareConnectionRows((current) =>
            current.map((row) =>
              row.id === item.id
                ? {
                    ...row,
                    status: result.ok ? "success" : "error",
                    message: result.message,
                  }
                : row,
            ),
          );

          return result;
        }),
      );
      const successCount = results.filter((item) => item.ok).length;
      const failureCount = results.length - successCount;
      setStatusMessage(
        failureCount > 0
          ? `AKShare 连接测试完成：${successCount} 项成功，${failureCount} 项失败。`
          : `AKShare 连接测试完成：${successCount} 项成功。`,
      );
    } catch (error) {
      setStatusMessage(`AKShare 连接测试失败：${String(error)}`);
    } finally {
      setIsTestingAkshare(false);
    }
  }

  async function handleTestModelConnection() {
    setIsTestingModel(true);
    try {
      const result = await testModelConnection({
        modelProvider: form.modelProvider,
        modelName: form.modelName,
        modelBaseUrl: form.modelBaseUrl,
        modelApiKey: form.modelApiKey,
        recommendationModel: form.recommendationModel,
      });
      setStatusMessage(result.message);
    } catch (error) {
      setStatusMessage(`模型连接失败：${String(error)}`);
    } finally {
      setIsTestingModel(false);
    }
  }

  async function handleConfirmSentimentLogin() {
    if (!loginPlatform) return;
    setIsCapturingSentimentLogin(true);
    try {
      await captureSentimentPlatformLoginState(loginPlatform.id);
      const statuses = await getSentimentPlatformAuthStatuses();
      setSentimentAuthStatuses(statuses);
      const isCaptured = statuses.some((item) => item.platform === loginPlatform.id && item.hasLoginState);
      if (!isCaptured) {
        setStatusMessage(`${loginPlatform.label} 登录态获取失败：后端未确认有效登录态。`);
        return;
      }
      setStatusMessage(`${loginPlatform.label} 登录态已成功获取。`);
      setLoginPlatform(null);
    } catch (error) {
      setStatusMessage(`${loginPlatform.label} 登录态获取失败：${String(error)}`);
    } finally {
      setIsCapturingSentimentLogin(false);
    }
  }

  async function openSentimentLoginPlatform(platform: (typeof sentimentLoginPlatforms)[number]) {
    setLoginPlatform(platform);
    try {
      await openUrl(platform.url);
    } catch {
      window.open(platform.url, "_blank", "noopener,noreferrer");
    }
  }

  async function handleTestSentimentConnections() {
    setIsTestingSentiment(true);
    const initialRows = Object.entries(sentimentPlatformLabels).map(([id, label]) => ({
      id: id as AkshareConnectionTestItemId,
      label,
      sourceLabel: "社媒平台",
      status: "testing" as AkshareConnectionRowStatus,
      message: "测试中",
    }));
    setSentimentConnectionRows(initialRows);
    setStatusMessage("正在测试社媒平台连接...");
    try {
      const results = await testSentimentPlatformConnections();
      setSentimentConnectionRows(
        initialRows.map((row) => {
          const result = results.find((item) => item.platform === row.id);
          return result
            ? {
                ...row,
                status: result.ok ? "success" : "error",
                message: result.message,
              }
            : row;
        }),
      );
      const successCount = results.filter((item) => item.ok).length;
      const failureCount = results.length - successCount;
      setStatusMessage(
        failureCount > 0
          ? `社媒平台连接测试完成：${successCount} 项成功，${failureCount} 项失败。`
          : `社媒平台连接测试完成：${successCount} 项成功。`,
      );
    } catch (error) {
      setStatusMessage(`社媒平台连接测试失败：${String(error)}`);
    } finally {
      setIsTestingSentiment(false);
    }
  }

  function toggleAiKlineFrequency(frequency: AiKlineFrequency) {
    setForm((current) => {
      const selected = current.aiKlineFrequencies.includes(frequency)
        ? current.aiKlineFrequencies.filter((item) => item !== frequency)
        : [...current.aiKlineFrequencies, frequency];
      return {
        ...current,
        aiKlineFrequencies: selected.length > 0 ? selected : [frequency],
      };
    });
  }

  function reorderSentimentPlatformPriority(source: string, target: string) {
    if (source === target) return;
    setForm((current) => {
      const next = current.sentimentPlatformPriority.filter((platform) => platform !== source);
      const targetIndex = next.indexOf(target);
      if (targetIndex < 0) return current;
      next.splice(targetIndex, 0, source);
      return { ...current, sentimentPlatformPriority: next };
    });
  }

  function moveDraggedSentimentPlatform(target: string) {
    const source = draggedSentimentPlatformRef.current;
    if (!source) return;
    reorderSentimentPlatformPriority(source, target);
  }

  function startSentimentPlatformDrag(event: React.PointerEvent<HTMLButtonElement>, platform: string) {
    const rect = event.currentTarget.getBoundingClientRect();
    draggedSentimentPlatformRef.current = platform;
    setSentimentPlatformDrag({
      platform,
      x: event.clientX,
      y: event.clientY,
      width: rect.width,
      height: rect.height,
    });
    event.currentTarget.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  }

  function updateSentimentPlatformDrag(event: React.PointerEvent<HTMLButtonElement>) {
    if (!draggedSentimentPlatformRef.current) return;
    setSentimentPlatformDrag((current) => current ? { ...current, x: event.clientX, y: event.clientY } : current);
    const target = document
      .elementsFromPoint(event.clientX, event.clientY)
      .find((element) => element instanceof HTMLElement && element.dataset.sentimentPlatform)
      ?.getAttribute("data-sentiment-platform");
    if (target) {
      moveDraggedSentimentPlatform(target);
    }
  }

  function finishSentimentPlatformDrag(event?: React.PointerEvent<HTMLButtonElement>, target?: string) {
    if (target) {
      moveDraggedSentimentPlatform(target);
    }
    if (event) {
      event.currentTarget.releasePointerCapture?.(event.pointerId);
    }
    draggedSentimentPlatformRef.current = null;
    setSentimentPlatformDrag(null);
  }

  function setSentimentNumberSetting(
    key: "sentimentFetchRecentDays" | "sentimentItemMaxChars",
    value: string,
    max: number,
  ) {
    const parsed = Math.trunc(Number(value) || 1);
    setForm((current) => ({
      ...current,
      [key]: Math.min(max, Math.max(1, parsed)),
    }));
  }

  return (
    <section className="page">
      <div className="settings-layout">
        <aside className="settings-tabs" role="tablist" aria-label="设置分类">
          {tabs.map((tab) => (
            <Button
              aria-selected={activeTab === tab.id}
              className={activeTab === tab.id ? "settings-tab settings-tab--active justify-start" : "settings-tab justify-start"}
              variant={activeTab === tab.id ? "default" : "outline"}
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              role="tab"
              type="button"
            >
              {tab.label}
            </Button>
          ))}
        </aside>

        <Card className="panel settings-panel" role="tabpanel">
          <CardHeader className="items-start px-6 pb-4 pt-6 text-left">
            <div className="w-full text-left">
              <CardTitle className="text-[1.7rem]">{currentTab.label}</CardTitle>
              <p className="settings-copy text-left text-sm text-muted-foreground">{currentTab.blurb}</p>
            </div>
          </CardHeader>
          <CardContent className="grid gap-6 px-6 pb-6 pt-0">
          {activeTab === "akshare" ? (
            <div className="form-grid">
              <Field label="行情市场">
                <Input readOnly value="沪深 A 股（SHSE / SZSE）" />
              </Field>
              <Field label="数据接口">
                <Input readOnly value="AKShare Python SDK" />
              </Field>
              <Field label="分时数据">
                <Select
                  aria-label="分时数据"
                  value={form.intradayDataSource}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      intradayDataSource: event.target.value as IntradayDataSourceSetting,
                    }))
                  }
                >
                  {intradayDataSourceOptions.map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.label}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="历史行情数据">
                <Select
                  aria-label="历史行情数据"
                  value={form.historicalDataSource}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      historicalDataSource: event.target.value as HistoricalDataSourceSetting,
                    }))
                  }
                >
                  {historicalDataSourceOptions.map((option) => (
                    <option key={option.id} value={option.id}>
                      {option.label}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="雪球 Token">
                <Input
                  aria-label="雪球 Token"
                  placeholder={
                    form.hasStoredXueqiuToken
                      ? "已保存雪球 Token，可直接覆盖"
                      : "请输入已登录雪球后的 xq_a_token"
                  }
                  type="password"
                  value={form.xueqiuToken}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      xueqiuToken: event.target.value,
                    }))
                  }
                />
              </Field>
              <p className="settings-copy text-sm text-muted-foreground">
                新浪财经和腾讯证券的周线/月线由本地日线聚合生成。
              </p>
              <Field label="连接测试">
                <div className="space-y-3">
                  <Button
                    disabled={isTestingAkshare}
                    onClick={() => void handleTestAkshareConnection()}
                    type="button"
                  >
                    {isTestingAkshare ? "测试中..." : "测试 AKShare 连接"}
                  </Button>
                  {akshareConnectionRows.length > 0 ? (
                    <div className="grid gap-2">
                      {akshareConnectionRows.map((row) => (
                        <div
                          key={row.id}
                          className="flex items-center justify-between rounded-lg border border-border bg-muted/20 px-3 py-2 text-sm"
                          title={row.message}
                        >
                          <span>{`${row.label} · ${row.sourceLabel}`}</span>
                          <span className="inline-flex items-center gap-2">
                            {statusIcon(row.status)}
                            <span>{statusLabel(row.status)}</span>
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : null}
                </div>
              </Field>
            </div>
          ) : null}

          {activeTab === "sentiment" ? (
            <div className="form-stack">
              <section className="settings-section-block">
                <span className="section-label">舆情抽样设置</span>
                <div className="field">
                  <label>AI分析舆情时的平台优先级</label>
                  <div className="sentiment-platform-priority" aria-label="AI分析舆情时的平台优先级">
                    {form.sentimentPlatformPriority.map((platform) => (
                      <button
                        className={sentimentPlatformDrag?.platform === platform ? "sentiment-platform-chip sentiment-platform-chip--dragging-source" : "sentiment-platform-chip"}
                        data-sentiment-platform={platform}
                        key={platform}
                        onPointerDown={(event) => startSentimentPlatformDrag(event, platform)}
                        onPointerMove={(event) => updateSentimentPlatformDrag(event)}
                        onPointerUp={(event) => finishSentimentPlatformDrag(event, platform)}
                        onPointerCancel={(event) => finishSentimentPlatformDrag(event)}
                        type="button"
                      >
                        {sentimentPlatformLabel(platform)}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="form-grid">
                  <Field label="拉取舆情的最近天数">
                    <Input
                      aria-label="拉取舆情的最近天数"
                      max={30}
                      min={1}
                      step="1"
                      type="number"
                      value={form.sentimentFetchRecentDays}
                      onChange={(event) => setSentimentNumberSetting("sentimentFetchRecentDays", event.target.value, 30)}
                    />
                  </Field>
                  <Field label="每条舆情的限制字数">
                    <Input
                      aria-label="每条舆情的限制字数"
                      max={1000}
                      min={1}
                      step="1"
                      type="number"
                      value={form.sentimentItemMaxChars}
                      onChange={(event) => setSentimentNumberSetting("sentimentItemMaxChars", event.target.value, 1000)}
                    />
                  </Field>
                  <Field label="抽样优先规则">
                    <Select
                      aria-label="抽样优先规则"
                      value={form.sentimentSamplingOrder}
                      onChange={(event) => setForm((current) => ({ ...current, sentimentSamplingOrder: event.target.value as SentimentSamplingOrder }))}
                    >
                      {sentimentSamplingOrderOptions.map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.label}
                        </option>
                      ))}
                    </Select>
                  </Field>
                </div>
              </section>

              <section className="settings-section-block">
                <span className="section-label">平台登录态</span>
                <div className="grid gap-3">
                  {sentimentLoginPlatforms.map((platform) => {
                    const hasLoginState = sentimentAuthStatuses.some(
                      (item) => item.platform === platform.id && item.hasLoginState,
                    );
                    return (
                      <div className="flex items-center justify-between rounded-lg border border-border bg-muted/20 px-3 py-2 text-sm" key={platform.id}>
                        <div className="grid gap-1">
                          <span>{platform.label}</span>
                          {hasLoginState ? <span className="text-emerald-500">已成功获取登录态</span> : null}
                        </div>
                        <Button
                          onClick={() => void openSentimentLoginPlatform(platform)}
                          type="button"
                          variant="outline"
                        >
                          {hasLoginState ? "刷新登录态" : "获取登录态"}
                        </Button>
                      </div>
                    );
                  })}
                </div>
              </section>

              <section className="settings-section-block">
                <span className="section-label">连接测试</span>
                <div className="space-y-3">
                  <Button
                    disabled={isTestingSentiment}
                    onClick={() => void handleTestSentimentConnections()}
                    type="button"
                  >
                    {isTestingSentiment ? "测试中..." : "测试社媒平台连接"}
                  </Button>
                  {sentimentConnectionRows.length > 0 ? (
                    <div className="grid gap-2">
                      {sentimentConnectionRows.map((row) => (
                        <div
                          key={row.id}
                          className="flex items-center justify-between rounded-lg border border-border bg-muted/20 px-3 py-2 text-sm"
                          title={row.message}
                        >
                          <span>{row.label}</span>
                          <span className="inline-flex items-center gap-2">
                            {statusIcon(row.status)}
                            <span>{statusLabel(row.status)}</span>
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : null}
                </div>
              </section>
            </div>
          ) : null}

          {activeTab === "models" ? (
            <>
              <div className="form-grid settings-model-grid">
                <Field label="模型服务商">
                  <Select
                    aria-label="模型服务商"
                    className="settings-select"
                    onChange={(event) => {
                      const preset = MODEL_PROVIDER_PRESETS.find(
                        (item) => item.provider === event.target.value,
                      );
                      setForm((current) =>
                        preset
                          ? {
                              ...current,
                              modelPreset: preset.provider,
                              modelBaseUrl: preset.baseUrl,
                              modelProvider: preset.interface,
                            }
                          : { ...current, modelPreset: event.target.value },
                      );
                    }}
                    value={form.modelPreset}
                  >
                    {modelProviderOptions(form.modelPreset).map((option) => (
                      <option key={option.provider} value={option.provider}>
                        {option.provider}
                      </option>
                    ))}
                  </Select>
                </Field>
                <Field label="接口类型">
                  <Select
                    aria-label="接口类型"
                    className="settings-select"
                    onChange={(event) =>
                      setForm((current) => ({
                        ...current,
                        modelProvider: event.target.value as ModelInterfaceSetting,
                      }))
                    }
                    value={form.modelProvider}
                  >
                    {modelInterfaceOptions.map((option) => (
                      <option key={option.id} value={option.id}>
                        {option.label}
                      </option>
                    ))}
                  </Select>
                </Field>
                <Field label="模型名称">
                  <Input aria-label="模型名称" value={form.modelName} onChange={(event) => setForm((current) => ({ ...current, modelName: event.target.value }))} />
                </Field>
                <Field label="接口地址">
                  <Input aria-label="接口地址" value={form.modelBaseUrl} onChange={(event) => setForm((current) => ({ ...current, modelBaseUrl: event.target.value }))} />
                </Field>
                <Field label="模型 API Key">
                  <Input
                    aria-label="模型 API Key"
                    placeholder={
                      form.hasStoredModelApiKey
                        ? "已保存 API Key，可直接覆盖"
                        : undefined
                    }
                    type="password"
                    value={form.modelApiKey}
                    onChange={(event) =>
                      setForm((current) => ({
                        ...current,
                        modelApiKey: event.target.value,
                      }))
                    }
                  />
                </Field>
              </div>
              <section className="settings-section-block">
                <span className="section-label">AI推荐/回测</span>
                <p className="settings-copy text-sm text-muted-foreground">控制推荐生成和回测分析的模型参数。</p>
                <div className="form-grid form-grid--four">
                  <Field label="温度">
                    <Input aria-label="推荐模型温度" max={2} min={0} step="0.1" type="number" value={form.recommendationModel.temperature} onChange={(event) => setForm((current) => ({ ...current, recommendationModel: { ...current.recommendationModel, temperature: Number(event.target.value || 0) } }))} />
                  </Field>
                  <Field label="最大输出 Token">
                    <Input aria-label="推荐模型最大输出 Token" min={1} step="1" type="number" value={form.recommendationModel.maxTokens} onChange={(event) => setForm((current) => ({ ...current, recommendationModel: { ...current.recommendationModel, maxTokens: Number(event.target.value || 1) } }))} />
                  </Field>
                  <Field label="最大上下文 Token">
                    <Input aria-label="推荐模型最大上下文 Token" min={1024} step="1024" type="number" value={form.recommendationModel.maxContext} onChange={(event) => setForm((current) => ({ ...current, recommendationModel: { ...current.recommendationModel, maxContext: Number(event.target.value || 1024) } }))} />
                  </Field>
                  <Field label="思考深度">
                    <Select aria-label="推荐模型思考深度" value={form.recommendationModel.effortLevel} onChange={(event) => setForm((current) => ({ ...current, recommendationModel: { ...current.recommendationModel, effortLevel: event.target.value as EffortLevel } }))}>
                      {effortLevelOptions.map((option) => (
                        <option key={option.id} value={option.id}>{option.label}</option>
                      ))}
                    </Select>
                  </Field>
                </div>
              </section>
              <section className="settings-section-block">
                <span className="section-label">AI助手</span>
                <p className="settings-copy text-sm text-muted-foreground">控制 AI 对话助手的模型参数。</p>
                <div className="form-grid form-grid--four">
                  <Field label="温度">
                    <Input aria-label="助手模型温度" max={2} min={0} step="0.1" type="number" value={form.assistantModel.temperature} onChange={(event) => setForm((current) => ({ ...current, assistantModel: { ...current.assistantModel, temperature: Number(event.target.value || 0) } }))} />
                  </Field>
                  <Field label="最大输出 Token">
                    <Input aria-label="助手模型最大输出 Token" min={1} step="1" type="number" value={form.assistantModel.maxTokens} onChange={(event) => setForm((current) => ({ ...current, assistantModel: { ...current.assistantModel, maxTokens: Number(event.target.value || 1) } }))} />
                  </Field>
                  <Field label="最大上下文 Token">
                    <Input aria-label="助手模型最大上下文 Token" min={1024} step="1024" type="number" value={form.assistantModel.maxContext} onChange={(event) => setForm((current) => ({ ...current, assistantModel: { ...current.assistantModel, maxContext: Number(event.target.value || 1024) } }))} />
                  </Field>
                  <Field label="思考深度">
                    <Select aria-label="助手模型思考深度" value={form.assistantModel.effortLevel} onChange={(event) => setForm((current) => ({ ...current, assistantModel: { ...current.assistantModel, effortLevel: event.target.value as EffortLevel } }))}>
                      {effortLevelOptions.map((option) => (
                        <option key={option.id} value={option.id}>{option.label}</option>
                      ))}
                    </Select>
                  </Field>
                </div>
              </section>
              <section className="settings-section-block">
                <span className="section-label">AI财报分析</span>
                <p className="settings-copy text-sm text-muted-foreground">控制财报 AI 分析的模型参数。</p>
                <div className="form-grid form-grid--four">
                  <Field label="温度">
                    <Input aria-label="财报模型温度" max={2} min={0} step="0.1" type="number" value={form.financialReportModel.temperature} onChange={(event) => setForm((current) => ({ ...current, financialReportModel: { ...current.financialReportModel, temperature: Number(event.target.value || 0) } }))} />
                  </Field>
                  <Field label="最大输出 Token">
                    <Input aria-label="财报模型最大输出 Token" min={1} step="1" type="number" value={form.financialReportModel.maxTokens} onChange={(event) => setForm((current) => ({ ...current, financialReportModel: { ...current.financialReportModel, maxTokens: Number(event.target.value || 1) } }))} />
                  </Field>
                  <Field label="最大上下文 Token">
                    <Input aria-label="财报模型最大上下文 Token" min={1024} step="1024" type="number" value={form.financialReportModel.maxContext} onChange={(event) => setForm((current) => ({ ...current, financialReportModel: { ...current.financialReportModel, maxContext: Number(event.target.value || 1024) } }))} />
                  </Field>
                  <Field label="思考深度">
                    <Select aria-label="财报模型思考深度" value={form.financialReportModel.effortLevel} onChange={(event) => setForm((current) => ({ ...current, financialReportModel: { ...current.financialReportModel, effortLevel: event.target.value as EffortLevel } }))}>
                      {effortLevelOptions.map((option) => (
                        <option key={option.id} value={option.id}>{option.label}</option>
                      ))}
                    </Select>
                  </Field>
                </div>
              </section>
              <section className="settings-section-block">
                <span className="section-label">AI舆情分析</span>
                <p className="settings-copy text-sm text-muted-foreground">控制舆情 AI 分析的模型参数。</p>
                <div className="form-grid form-grid--four">
                  <Field label="温度">
                    <Input aria-label="舆情模型温度" max={2} min={0} step="0.1" type="number" value={form.sentimentAnalysisModel.temperature} onChange={(event) => setForm((current) => ({ ...current, sentimentAnalysisModel: { ...current.sentimentAnalysisModel, temperature: Number(event.target.value || 0) } }))} />
                  </Field>
                  <Field label="最大输出 Token">
                    <Input aria-label="舆情模型最大输出 Token" min={1} step="1" type="number" value={form.sentimentAnalysisModel.maxTokens} onChange={(event) => setForm((current) => ({ ...current, sentimentAnalysisModel: { ...current.sentimentAnalysisModel, maxTokens: Number(event.target.value || 1) } }))} />
                  </Field>
                  <Field label="最大上下文 Token">
                    <Input aria-label="舆情模型最大上下文 Token" min={1024} step="1024" type="number" value={form.sentimentAnalysisModel.maxContext} onChange={(event) => setForm((current) => ({ ...current, sentimentAnalysisModel: { ...current.sentimentAnalysisModel, maxContext: Number(event.target.value || 1024) } }))} />
                  </Field>
                  <Field label="思考深度">
                    <Select aria-label="舆情模型思考深度" value={form.sentimentAnalysisModel.effortLevel} onChange={(event) => setForm((current) => ({ ...current, sentimentAnalysisModel: { ...current.sentimentAnalysisModel, effortLevel: event.target.value as EffortLevel } }))}>
                      {effortLevelOptions.map((option) => (
                        <option key={option.id} value={option.id}>{option.label}</option>
                      ))}
                    </Select>
                  </Field>
                </div>
              </section>
              <div className="hero-panel__actions">
                <Button
                  disabled={isTestingModel}
                  onClick={() => void handleTestModelConnection()}
                  type="button"
                >
                  {isTestingModel ? "测试中..." : "测试模型连接"}
                </Button>
              </div>
            </>
          ) : null}

          {activeTab === "aiTrade" ? (
            <div className="settings-trade-grid">
              <section className="settings-section-block settings-ai-analysis-card">
                <div className="settings-section-block__header">
                  <div>
                    <span className="section-label">AI分析</span>
                    <h3>推荐与回测共用的数据输入</h3>
                  </div>
                </div>
                <div className="settings-ai-analysis-card__grid">
                  <div className="settings-ai-row settings-ai-row--schedule">
                    <Field label="自动分析频率">
                      <Select value={form.autoAnalyzeFrequency} onChange={(event) => setForm((current) => ({ ...current, autoAnalyzeFrequency: event.target.value as SettingsFormData["autoAnalyzeFrequency"] }))}>
                        <option value="5m">5 分钟</option>
                        <option value="10m">10 分钟</option>
                        <option value="30m">30 分钟</option>
                        <option value="1h">1 小时</option>
                      </Select>
                    </Field>
                    <Field label="扫描范围">
                      <Select value={form.scanScope} onChange={(event) => setForm((current) => ({ ...current, scanScope: event.target.value as SettingsFormData["scanScope"] }))}>
                        <option value="watchlist_only">仅自选股</option>
                        <option value="all_markets">全部缓存股票</option>
                      </Select>
                    </Field>
                    <Field label="每日 AI 调用上限">
                      <Input inputMode="numeric" value={form.dailyMaxAiCalls} onChange={(event) => setForm((current) => ({ ...current, dailyMaxAiCalls: Number(event.target.value) || 0 }))} />
                    </Field>
                  </div>
                  <div className="settings-ai-row settings-ai-row--inputs">
                    <Field label="使用财报数据">
                      <div className="settings-ai-toggle-box">
                        <span>使用财报数据</span>
                        <input
                          aria-label="使用财报数据"
                          checked={form.useFinancialReportData}
                          onChange={(event) => setForm((current) => ({ ...current, useFinancialReportData: event.target.checked }))}
                          type="checkbox"
                        />
                      </div>
                    </Field>
                    <Field label="K线根数">
                      <input
                        aria-label="K线根数"
                        className="settings-inline-number"
                        inputMode="numeric"
                        min={1}
                        type="number"
                        value={form.aiKlineBarCount}
                        onChange={(event) => setForm((current) => ({ ...current, aiKlineBarCount: Math.max(1, Number(event.target.value) || 1) }))}
                      />
                    </Field>
                  </div>
                  <div className="settings-ai-row settings-ai-row--levels">
                    <div className="settings-ai-kline-field">
                      <span>K线级别</span>
                      <div className="settings-kline-levels__options" aria-label="K线级别">
                        {aiKlineFrequencyOptions.map((option) => (
                          <label className="settings-kline-levels__option" key={option.id}>
                            <input
                              checked={form.aiKlineFrequencies.includes(option.id)}
                              onChange={() => toggleAiKlineFrequency(option.id)}
                              type="checkbox"
                            />
                            <span>{option.label}</span>
                          </label>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </section>

              <section className="settings-section-block">
                <span className="section-label">风险</span>
                <div className="form-grid form-grid--three">
                  <Field label="单笔最大亏损">
                    <Select value={form.maxLossPerTradePercent} onChange={(event) => setForm((current) => ({ ...current, maxLossPerTradePercent: Number(event.target.value) }))}>
                      <option value={0.5}>0.5%</option>
                      <option value={1}>1%</option>
                      <option value={1.5}>1.5%</option>
                      <option value={2}>2%</option>
                    </Select>
                  </Field>
                  <Field label="日内最大亏损">
                    <Select value={form.maxDailyLossPercent} onChange={(event) => setForm((current) => ({ ...current, maxDailyLossPercent: Number(event.target.value) }))}>
                      <option value={1}>1%</option>
                      <option value={2}>2%</option>
                      <option value={3}>3%</option>
                      <option value={5}>5%</option>
                    </Select>
                  </Field>
                  <Field label="最低置信度">
                    <Select value={form.minConfidenceScore} onChange={(event) => setForm((current) => ({ ...current, minConfidenceScore: Number(event.target.value) }))}>
                      <option value={50}>50</option>
                      <option value={60}>60</option>
                      <option value={70}>70</option>
                      <option value={80}>80</option>
                    </Select>
                  </Field>
                </div>
              </section>

              <section className="settings-section-block">
                <span className="section-label">策略信号</span>
                <div className="form-grid form-grid--two">
                  <Field label="策略扫描">
                    <Select value={form.signalsEnabled ? "on" : "off"} onChange={(event) => setForm((current) => ({ ...current, signalsEnabled: event.target.value === "on" }))}>
                      <option value="on">开启</option>
                      <option value="off">关闭</option>
                    </Select>
                  </Field>
                  <Field label="扫描频率">
                    <Select value={form.signalScanFrequency} onChange={(event) => setForm((current) => ({ ...current, signalScanFrequency: event.target.value as SettingsFormData["signalScanFrequency"] }))}>
                      <option value="5m">5 分钟</option>
                      <option value="10m">10 分钟</option>
                      <option value="15m">15 分钟</option>
                      <option value="30m">30 分钟</option>
                      <option value="1h">1 小时</option>
                    </Select>
                  </Field>
                </div>
              </section>
            </div>
          ) : null}

          {activeTab === "prompt" ? (
            <div className="form-stack">
              <Field label="Assistant 系统提示词">
                <Textarea
                  aria-label="Assistant 系统提示词"
                  value={form.assistantSystemPrompt}
                  onChange={(event) => setForm((current) => ({ ...current, assistantSystemPrompt: event.target.value }))}
                />
              </Field>
              <Field label="AI推荐/回测系统提示词">
                <Textarea
                  aria-label="AI推荐/回测系统提示词"
                  value={form.recommendationSystemPrompt}
                  onChange={(event) => setForm((current) => ({ ...current, recommendationSystemPrompt: event.target.value }))}
                />
              </Field>
              <Field label="AI财报分析系统提示词">
                <Textarea
                  aria-label="AI财报分析系统提示词"
                  value={form.financialReportSystemPrompt}
                  onChange={(event) => setForm((current) => ({ ...current, financialReportSystemPrompt: event.target.value }))}
                />
              </Field>
              <Field label="AI舆情分析系统提示词">
                <Textarea
                  aria-label="AI舆情分析系统提示词"
                  value={form.sentimentAnalysisSystemPrompt}
                  onChange={(event) => setForm((current) => ({ ...current, sentimentAnalysisSystemPrompt: event.target.value }))}
                />
              </Field>
            </div>
          ) : null}
          </CardContent>
        </Card>
      </div>

      <div className="settings-actions">
        <Button
          className="settings-save-button"
          disabled={isSaving}
          onClick={handleSave}
          type="button"
        >
          {isSaving ? "保存中..." : "保存设置"}
        </Button>
        <p className="settings-status" role="status">
          {statusMessage}
        </p>
      </div>
      {sentimentPlatformDrag ? (
        <div
          className="sentiment-platform-drag-preview"
          style={{
            height: sentimentPlatformDrag.height,
            left: sentimentPlatformDrag.x,
            top: sentimentPlatformDrag.y,
            width: sentimentPlatformDrag.width,
          }}
        >
          {sentimentPlatformLabel(sentimentPlatformDrag.platform)}
        </div>
      ) : null}
      {loginPlatform ? (
        <div aria-label={`${loginPlatform.label}登录态获取`} aria-modal="true" className="modal-overlay" role="dialog">
          <div className="modal-content">
            <div className="modal-header">
              <h2>{`获取${loginPlatform.label}登录态`}</h2>
            </div>
            <div className="modal-body">
              <p>{`请在打开的浏览器中登录${loginPlatform.label}，登录成功后回到这里点击确定。`}</p>
            </div>
            <div className="modal-actions">
              <Button onClick={() => setLoginPlatform(null)} type="button" variant="outline">
                取消
              </Button>
              <Button disabled={isCapturingSentimentLogin} onClick={() => void handleConfirmSentimentLogin()} type="button">
                {isCapturingSentimentLogin ? "获取中..." : "确定"}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
