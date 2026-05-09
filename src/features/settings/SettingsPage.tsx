import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  CUSTOM_MODEL_PROVIDER,
  createDefaultSettingsFormData,
  loadSettingsFormData,
  MODEL_PROVIDER_PRESETS,
  saveSettingsFormData,
  testModelConnection,
  type ModelInterfaceSetting,
  type AiKlineFrequency,
  type SettingsFormData,
} from "../../lib/settings";
import { getAkshareCurrentQuote } from "../../lib/akshare";
import { useAppStore } from "../../store/appStore";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Select } from "../../components/ui/select";
import { Textarea } from "../../components/ui/textarea";

type SettingsTab = {
  id: string;
  label: string;
  blurb: string;
};

const tabs: SettingsTab[] = [
  { id: "akshare", label: "数据源", blurb: "检查 AKShare 本地数据接口状态。" },
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
    try {
      const result = await getAkshareCurrentQuote("SHSE.600000");
      if (!result.ok) {
        setStatusMessage(`AKShare 连接失败：${result.error ?? "未知错误"}`);
        return;
      }
      const quote = result.data;
      setStatusMessage(
        quote
          ? `AKShare 连接正常：${quote.symbol} 最新价 ${quote.last}。`
          : "AKShare 连接正常。",
      );
    } catch (error) {
      setStatusMessage(`AKShare 连接失败：${String(error)}`);
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
        modelTemperature: form.modelTemperature,
        modelMaxTokens: form.modelMaxTokens,
        modelMaxContext: form.modelMaxContext,
      });
      setStatusMessage(result.message);
    } catch (error) {
      setStatusMessage(`模型连接失败：${String(error)}`);
    } finally {
      setIsTestingModel(false);
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
              <Field label="连接测试">
                <Button
                  disabled={isTestingAkshare}
                  onClick={() => void handleTestAkshareConnection()}
                  type="button"
                >
                  {isTestingAkshare ? "测试中..." : "测试 AKShare 连接"}
                </Button>
              </Field>
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
                  <Input aria-label="模型 API Key" type="password" value={form.modelApiKey} onChange={(event) => setForm((current) => ({ ...current, modelApiKey: event.target.value }))} />
                </Field>
                <Field label="温度">
                  <Input aria-label="温度" max={2} min={0} step="0.1" type="number" value={form.modelTemperature} onChange={(event) => setForm((current) => ({ ...current, modelTemperature: Number(event.target.value || 0) }))} />
                </Field>
                <Field label="最大输出 Token">
                  <Input aria-label="最大输出 Token" min={1} step="1" type="number" value={form.modelMaxTokens} onChange={(event) => setForm((current) => ({ ...current, modelMaxTokens: Number(event.target.value || 1) }))} />
                </Field>
                <Field label="最大上下文 Token">
                  <Input aria-label="最大上下文 Token" min={1024} step="1024" type="number" value={form.modelMaxContext} onChange={(event) => setForm((current) => ({ ...current, modelMaxContext: Number(event.target.value || 1024) }))} />
                </Field>
              </div>
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
                    <Field label="使用买卖盘数据">
                      <div className="settings-ai-toggle-box">
                        <span>使用买卖盘数据</span>
                        <input
                          aria-label="使用买卖盘数据"
                          checked={form.useBidAskData}
                          onChange={(event) => setForm((current) => ({ ...current, useBidAskData: event.target.checked }))}
                          type="checkbox"
                        />
                      </div>
                    </Field>
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
              <Field label="AI 推荐系统提示词">
                <Textarea
                  aria-label="AI 推荐系统提示词"
                  value={form.recommendationSystemPrompt}
                  onChange={(event) => setForm((current) => ({ ...current, recommendationSystemPrompt: event.target.value }))}
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
    </section>
  );
}
