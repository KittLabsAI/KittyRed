import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { StrategyCard } from "./StrategyCard";
import type { StrategyMeta } from "../../lib/types";

const mockMeta: StrategyMeta = {
  strategyId: "ma_cross",
  name: "均线交叉",
  category: "Trend",
  applicableMarkets: ["a_share"],
  description: "识别均线金叉和死叉",
  defaultParams: { fast_period: 5, slow_period: 20 },
};

describe("StrategyCard", () => {
  it("renders strategy name and category", () => {
    render(<StrategyCard meta={mockMeta} config={undefined} stats={undefined} onClick={() => {}} />);
    expect(screen.getByText("均线交叉")).toBeInTheDocument();
    expect(screen.getByText("趋势")).toHaveClass("badge--strategy-trend");
    expect(screen.getByText("趋势")).not.toHaveAttribute("style");
  });

  it("shows disabled style when enabled=false", () => {
    render(
      <StrategyCard
        meta={mockMeta}
        config={{ strategyId: "ma_cross", enabled: false, params: {} }}
        stats={undefined}
        onClick={() => {}}
      />,
    );
    const card = screen.getByRole("button");
    expect(card.className).toContain("strategy-card--disabled");
  });

  it("shows buy/sell counts from stats", () => {
    render(
      <StrategyCard
        meta={mockMeta}
        config={undefined}
        stats={{
          strategyId: "ma_cross",
          totalSignals: 20,
          buyCount: 12,
          sellCount: 8,
          neutralCount: 0,
          avgScore: 72.3,
          lastGeneratedAt: "2026-05-05T10:00:00Z",
        }}
        onClick={() => {}}
      />,
    );
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
  });

  it("opens with keyboard activation", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();

    render(<StrategyCard meta={mockMeta} config={undefined} stats={undefined} onClick={onClick} />);

    screen.getByRole("button", { name: /均线交叉/i }).focus();
    await user.keyboard("[Space]");

    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
