import { create } from "zustand";
import type {
  AnalyzeJob,
  ArbitrageOpportunity,
  MarketRow,
  PairDetailSnapshot,
  SpreadOpportunity,
} from "../lib/types";

const aShareRows: MarketRow[] = [
  {
    symbol: "SHSE.600000",
    baseAsset: "浦发银行",
    marketType: "沪市A股",
    marketSizeTier: "large",
    last: 8.72,
    change24h: 0.81,
    volume24h: 1_260_000_000,
    spreadBps: 0,
    venues: ["akshare"],
    updatedAt: "10:00",
  },
  {
    symbol: "SZSE.000001",
    baseAsset: "平安银行",
    marketType: "深市A股",
    marketSizeTier: "large",
    last: 11.34,
    change24h: -0.35,
    volume24h: 1_850_000_000,
    spreadBps: 0,
    venues: ["akshare"],
    updatedAt: "10:00",
  },
];

type MarketStore = {
  jobs: AnalyzeJob[];
  watchlist: MarketRow[];
  markets: MarketRow[];
  spreads: SpreadOpportunity[];
  arbitrage: ArbitrageOpportunity[];
  pairDetail: PairDetailSnapshot;
};

export const useMarketStore = create<MarketStore>(() => ({
  jobs: [],
  watchlist: aShareRows,
  markets: aShareRows,
  spreads: [],
  arbitrage: [],
  pairDetail: {
    symbol: "SHSE.600000",
    marketType: "沪市A股",
    thesis: "浦发银行短期量价温和改善，适合作为模拟账户观察标的。",
    sourceNote: "浏览器预览使用本地 A 股样例，桌面运行时通过 AKShare 读取。",
    coinInfo: {
      name: "浦发银行",
      symbol: "SHSE.600000",
      summary: "沪市银行股样例，用于本地预览 A 股行情和模拟交易流程。",
      ecosystem: "沪市A股",
      listedExchanges: ["上海证券交易所"],
      riskTags: ["银行板块", "大盘股"],
    },
    venues: [
      {
        exchange: "akshare",
        last: 8.72,
        bid: 8.71,
        ask: 8.72,
        volume24h: 1_260_000_000,
        updatedAt: "10:00",
      },
    ],
    orderbooks: [],
    recentTrades: [],
    spreads: [],
  },
}));
