import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import type { StrategyMeta, StrategyConfig } from "../../lib/types";

interface StrategyConfigModalProps {
  meta: StrategyMeta;
  config: StrategyConfig | undefined;
  onSave: (enabled: boolean, params: Record<string, number>) => void;
  onClose: () => void;
}

export function StrategyConfigModal({
  meta,
  config,
  onSave,
  onClose,
}: StrategyConfigModalProps) {
  const [enabled, setEnabled] = useState(config?.enabled ?? true);
  const [params, setParams] = useState<Record<string, number>>(
    config?.params && Object.keys(config.params).length > 0
      ? { ...meta.defaultParams, ...config.params }
      : { ...meta.defaultParams },
  );
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  const handleSave = () => {
    onSave(enabled, params);
    onClose();
  };

  const handleReset = () => {
    setParams({ ...meta.defaultParams });
    setEnabled(true);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="modal-content strategy-config-modal border border-white/10 bg-[color:var(--panel-strong)] shadow-[var(--shadow-workbench)]"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
        role="dialog"
        aria-label={`配置 ${meta.name}`}
        aria-modal="true"
        tabIndex={-1}
      >
        <div className="modal-header">
          <div>
            <h3>{meta.name}</h3>
            <p className="panel__meta">{meta.description}</p>
          </div>
          <Button aria-label="关闭配置" onClick={onClose} size="sm" variant="ghost">
            ✕
          </Button>
        </div>

        <div className="modal-body">
          <div className="strategy-config__toggle">
            <span>启用策略</span>
            <button
              className={`toggle-switch${enabled ? " toggle-switch--on" : ""}`}
              onClick={() => setEnabled((e) => !e)}
              type="button"
              role="switch"
              aria-checked={enabled}
              aria-label="启用策略"
            >
              <span className="toggle-switch__knob" />
            </button>
          </div>

          <div className="strategy-config__params">
            {Object.entries(params).map(([key, value]) => (
              <div key={key} className="strategy-config__param">
                <Label htmlFor={`param-${key}`}>
                  {strategyParamLabel(key)}
                </Label>
                <Input
                  id={`param-${key}`}
                  className="h-11"
                  type="number"
                  step="any"
                  value={value}
                  onChange={(e) =>
                    setParams((prev) => ({
                      ...prev,
                      [key]: parseFloat(e.target.value) || 0,
                    }))
                  }
                />
              </div>
            ))}
          </div>
        </div>

        <div className="modal-footer">
          <Button onClick={handleReset} variant="ghost">
            恢复默认
          </Button>
          <Button onClick={handleSave}>
            保存
          </Button>
        </div>
      </div>
    </div>
  );
}

function strategyParamLabel(key: string) {
  const labels: Record<string, string> = {
    fast_period: "快速周期",
    slow_period: "慢速周期",
    min_confidence: "最低置信度",
    min_strength: "最低强度",
    period: "周期",
    oversold: "超卖阈值",
    overbought: "超买阈值",
    fast: "快速线",
    slow: "慢速线",
    signal: "信号线",
    std_dev: "标准差",
    lookback: "回看周期",
    surge_multiplier: "放量倍数",
  };

  return labels[key] ?? key.replace(/_/g, " ");
}
