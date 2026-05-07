import { create } from "zustand";
import type { OrderRow, PortfolioOverview, PositionRow } from "../lib/types";

type PortfolioStore = {
  overview: PortfolioOverview;
  positions: PositionRow[];
  orders: OrderRow[];
};

export const usePortfolioStore = create<PortfolioStore>(() => ({
  overview: {
    totalEquity: 1_000_000,
    totalMarketValue: 0,
    totalPnl: 0,
    todayPnl: 0,
    todayPnlPct: 0,
    riskSummary: "当前只有人民币现金账户，尚未产生模拟持仓。",
    exchanges: [{ name: "人民币现金", equity: 1_000_000, weight: 100 }],
  },
  positions: [],
  orders: [],
}));
