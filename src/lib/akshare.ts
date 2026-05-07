import { invoke } from "@tauri-apps/api/core";

export type AkshareQuote = {
  symbol: string;
  last: number;
  open: number;
  high: number;
  low: number;
  volume: number;
  amount: number;
  updated_at: string;
  source: "akshare";
};

export async function getAkshareCurrentQuote(
  symbol: string,
): Promise<{ ok: boolean; data?: AkshareQuote; error?: string }> {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window === false) {
    return {
      ok: true,
      data: {
        symbol,
        last: 12.34,
        open: 12,
        high: 12.5,
        low: 11.9,
        volume: 100000,
        amount: 1234000,
        updated_at: new Date().toISOString(),
        source: "akshare",
      },
    };
  }

  return invoke("akshare_current_quote", {
    symbol,
  });
}
