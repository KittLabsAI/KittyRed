import { create } from "zustand";
import type { RecommendationHistoryRow, RecommendationRun } from "../lib/types";

type RecommendationStore = {
  latest: RecommendationRun[];
  history: RecommendationHistoryRow[];
  assistantDraft: string;
  setLatest: (value: RecommendationRun[]) => void;
  prependHistory: (value: RecommendationHistoryRow) => void;
  setAssistantDraft: (value: string) => void;
};

export const useRecommendationStore = create<RecommendationStore>((set) => ({
  latest: [{
    id: "rec-20260503-1758",
    status: "completed",
    hasTrade: true,
    symbol: "SHSE.600000",
    marketType: "沪市A股",
    direction: "买入",
    confidence: 78,
    riskStatus: "approved",
    thesis: "浦发银行短期量价改善，但仍需等待回踩确认，模拟账户可小仓位观察。",
    entryLow: 8.68,
    entryHigh: 8.76,
    stopLoss: 8.45,
    takeProfit: "9.10 / 9.35",
    leverage: 1,
    amountCny: 20_000,
    invalidation: "若跌破 8.45 且成交额放大，本次反弹假设失效。",
    maxLossCny: 700,
    riskDetails: {
      status: "approved",
      riskScore: 42,
      maxLossEstimate: "0.07%",
      checks: [],
      modifications: [],
      blockReasons: [],
    },
    generatedAt: "2026-05-03T17:58:00+08:00",
  }],
  history: [
    {
      id: "rec-1",
      createdAt: "2026-05-03 17:20",
      symbol: "SHSE.600000",
      exchange: "模拟账户",
      marketType: "沪市A股",
      direction: "买入",
      risk: "approved",
      result: "Win",
      pnl5m: 0.32,
      pnl10m: 0.48,
      pnl30m: 1.12,
      pnl60m: 1.94,
      pnl24h: 4.82,
      pnl7d: 6.2,
      outcome: "达到第一目标价",
    },
    {
      id: "rec-2",
      createdAt: "2026-05-03 15:40",
      symbol: "SZSE.000001",
      exchange: "模拟账户",
      marketType: "深市A股",
      direction: "买入",
      risk: "blocked",
      result: "Blocked",
      pnl5m: -0.12,
      pnl10m: -0.18,
      pnl30m: -0.27,
      pnl60m: -0.44,
      pnl24h: -1.14,
      pnl7d: -1.8,
      outcome: "因止损不清晰被拦截",
    },
    {
      id: "rec-3",
      createdAt: "2026-05-03 11:05",
      symbol: "SHSE.600519",
      exchange: "模拟账户",
      marketType: "沪市A股",
      direction: "观察",
      risk: "approved",
      result: "Pending",
      pnl5m: 0.18,
      pnl10m: 0.34,
      pnl30m: 0.58,
      pnl60m: 0.76,
      pnl24h: 2.42,
      pnl7d: 3.1,
      outcome: "仍在模拟观察",
    },
  ],
  setLatest: (value) => set({ latest: value }),
  prependHistory: (value) =>
    set((state) => ({
      history: [value, ...state.history].slice(0, 20),
    })),
  assistantDraft: "",
  setAssistantDraft: (value) => set({ assistantDraft: value }),
}));
