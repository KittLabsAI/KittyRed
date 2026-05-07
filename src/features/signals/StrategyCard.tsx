import type { StrategyMeta, StrategyConfig, StrategyStats } from "../../lib/types";

interface StrategyCardProps {
  meta: StrategyMeta;
  config: StrategyConfig | undefined;
  stats: StrategyStats | undefined;
  onClick: () => void;
}

const categoryClasses: Record<string, string> = {
  Trend: "badge--strategy-trend",
  Momentum: "badge--strategy-momentum",
};

const categoryLabels: Record<string, string> = {
  Trend: "趋势",
  Momentum: "动量",
};

export function StrategyCard({ meta, config, stats, onClick }: StrategyCardProps) {
  const enabled = config?.enabled ?? true;
  const buyCount = stats?.buyCount ?? 0;
  const sellCount = stats?.sellCount ?? 0;
  const avgScore = stats?.avgScore ?? 0;
  const lastGen = stats?.lastGeneratedAt;
  const categoryClass = categoryClasses[meta.category] ?? "badge--strategy-neutral";

  return (
    <button
      className={`strategy-card${!enabled ? " strategy-card--disabled" : ""}`}
      onClick={onClick}
      type="button"
    >
      <div className="strategy-card__header">
        <div>
          <div className="strategy-card__name">{meta.name}</div>
          <span className={`badge ${categoryClass}`}>
            {categoryLabels[meta.category] ?? meta.category}
          </span>
        </div>
        <span
          className={`strategy-card__dot${enabled ? " strategy-card__dot--on" : ""}`}
        />
      </div>
      <div className="strategy-card__stats">
        <div>
          <span className="strategy-card__stat-label">买入</span>
          <span className="positive-text">{buyCount}</span>
        </div>
        <div>
          <span className="strategy-card__stat-label">卖出</span>
          <span className="negative-text">{sellCount}</span>
        </div>
        <div>
          <span className="strategy-card__stat-label">均分</span>
          <span>{avgScore.toFixed(1)}</span>
        </div>
      </div>
      <div className="strategy-card__footer">
        <span>{marketLabels(meta.applicableMarkets).join(" · ")}</span>
        {lastGen ? (
          <span>最近：{new Date(lastGen).toLocaleTimeString()}</span>
        ) : (
          <span className="strategy-card__no-data">暂无数据</span>
        )}
      </div>
    </button>
  );
}

function marketLabels(markets: string[]) {
  const labels = markets.map((market) => (market === "a_share" || market === "ashare" ? "A 股" : market));
  return [...new Set(labels)];
}
