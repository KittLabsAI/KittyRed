import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { formatCurrency } from "../../lib/format";
import { dismissSignal, executeSignal } from "../../lib/tauri";
import type { UnifiedSignal } from "../../lib/types";

interface SignalDetailPanelProps {
  signal: UnifiedSignal;
}

const categoryLabels: Record<string, string> = {
  Trend: "趋势",
  Momentum: "动量",
  Volatility: "波动",
  Volume: "成交量",
  Risk: "风险",
};

const strategyLabels: Record<string, string> = {
  ma_cross: "均线交叉",
  rsi_extreme: "RSI 超买超卖",
  macd_divergence: "MACD 背离",
  bollinger_break: "布林带突破",
  volume_surge: "成交量放大",
  volume_breakout: "成交量突破",
};

const riskCheckLabels: Record<string, string> = {
  market_type_allowed: "市场类型",
  direction_allowed: "方向限制",
  symbol_blacklist: "黑名单",
  symbol_whitelist: "白名单",
  min_score: "最低评分",
  cooldown: "冷却时间",
  daily_max: "当日上限",
  risk_gate: "风控检查",
  stop_loss_required: "止损要求",
  max_single_trade_loss: "单笔亏损上限",
  signal_contributor_minimum: "信号来源数量",
};

function localizedStrategy(value: string) {
  return strategyLabels[value] ?? value.replace(/_/g, " ");
}

function localizedRiskCheck(value: string) {
  return riskCheckLabels[value] ?? value;
}

function localizedRiskDetail(value: string) {
  const contributorMatch = value.match(/^(\d+) contributors, min (\d+)$/);
  if (contributorMatch) {
    return `当前 ${contributorMatch[1]} 个，最低 ${contributorMatch[2]} 个`;
  }
  return value
    .replace(/approved/g, "通过")
    .replace(/blocked/g, "拦截")
    .replace(/Buy/g, "买入")
    .replace(/Sell/g, "卖出")
    .replace(/Neutral/g, "中性")
    .replace(/requested/g, "请求")
    .replace(/allowed/g, "允许")
    .replace(/observed/g, "观察值")
    .replace(/min/g, "最低")
    .replace(/max/g, "最高");
}

function localizedReason(value: string) {
  return value
    .replace("Golden cross detected", "检测到均线金叉")
    .replace("Death cross detected", "检测到均线死叉")
    .replace("RSI oversold", "RSI 超卖")
    .replace("RSI overbought", "RSI 超买")
    .replace("Volume surge detected", "检测到成交量放大")
    .replace("Volume breakout confirmed", "成交量突破确认")
    .replace("single contributor only", "只有单一策略触发");
}

function scoreToneClass(score: number) {
  if (score >= 0.5) {
    return "signal-detail__category-bar--strong";
  }
  if (score >= 0.3) {
    return "signal-detail__category-bar--watch";
  }
  return "signal-detail__category-bar--muted";
}

export function SignalDetailPanel({ signal }: SignalDetailPanelProps) {
  const queryClient = useQueryClient();

  const executeMutation = useMutation({
    mutationFn: () => executeSignal(signal.signalId, "paper-cny"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["signal-history"] });
    },
  });

  const dismissMutation = useMutation({
    mutationFn: () => dismissSignal(signal.signalId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["signal-history"] });
    },
  });

  const getRiskChecks = () => {
    const checks = [];
    if (signal.riskResult) {
      for (const check of signal.riskResult.checks) {
        checks.push({ name: check.name, passed: check.status === "passed", detail: check.detail });
      }
    } else {
      checks.push({ name: "风控检查", passed: signal.riskStatus === "approved", detail: signal.riskStatus });
    }
    return checks;
  };

  return (
    <div className="signal-detail-panel rounded-xl border border-white/8 bg-[color:var(--panel-strong)] p-5 shadow-[var(--shadow-workbench)]">
      <div className="signal-detail__grid">
        <div>
          <span className="section-label">理由</span>
          <p className="signal-detail__reason">{localizedReason(signal.reasonSummary)}</p>
        </div>
        <div>
          <span className="section-label">策略贡献</span>
          <div className="signal-detail__categories">
            {Object.entries(signal.categoryBreakdown).map(([cat, score]) => (
              <div key={cat} className="signal-detail__category-row">
                <span>{categoryLabels[cat] ?? cat}</span>
                <div className="signal-detail__category-bar-bg">
                  <div
                    className={`signal-detail__category-bar ${scoreToneClass(score)}`}
                    style={{
                      width: `${(score * 100).toFixed(0)}%`,
                    }}
                  />
                </div>
                <span className="signal-detail__category-score">{score.toFixed(2)}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="signal-detail__grid">
        <div>
          <span className="section-label">风险评估</span>
          <div className="signal-detail__risk-checks">
            {getRiskChecks().map((check) => (
              <div key={check.name} className="signal-detail__risk-check">
                <span className={check.passed ? "positive-text" : "negative-text"}>
                  {check.passed ? "通过" : "拦截"}
                </span>
                <span>{localizedRiskCheck(check.name)}</span>
                {check.detail && (
                  <span className="signal-detail__risk-detail">{localizedRiskDetail(check.detail)}</span>
                )}
              </div>
            ))}
          </div>
        </div>
        <div>
          <span className="section-label">入场与退出</span>
          <div className="signal-detail__levels">
            <div><span>入场区间</span><span>{formatCurrency(signal.entryZoneLow)} – {formatCurrency(signal.entryZoneHigh)}</span></div>
            <div><span>止损</span><span>{formatCurrency(signal.stopLoss)}</span></div>
            <div><span>止盈</span><span>{formatCurrency(signal.takeProfit)}</span></div>
          </div>

          <span className="section-label signal-detail__contributors-label">触发策略</span>
          <div className="signal-detail__contributors">
            {signal.contributors.map((c) => (
              <Badge key={c} className="badge">{localizedStrategy(c)}</Badge>
            ))}
          </div>

          <div className="signal-detail__actions">
            <Button
              disabled={signal.executed || executeMutation.isPending || executeMutation.isSuccess}
              onClick={() => executeMutation.mutate()}
            >
              {signal.executed || executeMutation.isSuccess ? "已执行" : executeMutation.isPending ? "执行中..." : "执行模拟交易"}
            </Button>
            <Button
              disabled={dismissMutation.isPending}
              onClick={() => dismissMutation.mutate()}
              variant="ghost"
            >
              {dismissMutation.isPending ? "忽略中..." : "忽略"}
            </Button>
          </div>
          {executeMutation.data ? (
            <p className="panel__meta">
              已在 {executeMutation.data.exchange} 生成模拟成交，估算价格 {formatCurrency(executeMutation.data.estimatedFillPrice)}
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
