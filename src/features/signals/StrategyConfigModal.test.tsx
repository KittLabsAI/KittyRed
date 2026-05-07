import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StrategyConfigModal } from "./StrategyConfigModal";
import type { StrategyMeta } from "../../lib/types";

const mockMeta: StrategyMeta = {
  strategyId: "ma_cross",
  name: "均线交叉",
  category: "Trend",
  applicableMarkets: ["a_share"],
  description: "识别均线金叉和死叉",
  defaultParams: { fast_period: 5, slow_period: 20 },
};

describe("StrategyConfigModal", () => {
  it("renders strategy name and params", () => {
    render(
      <StrategyConfigModal
        meta={mockMeta}
        config={undefined}
        onSave={() => {}}
        onClose={() => {}}
      />,
    );
    expect(screen.getByRole("dialog", { name: "配置 均线交叉" })).toHaveAttribute("aria-modal", "true");
    expect(screen.getByText("均线交叉")).toBeInTheDocument();
    expect(screen.getByLabelText("快速周期")).toBeInTheDocument();
    expect(screen.getByLabelText("慢速周期")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "启用策略" })).toHaveAttribute("aria-checked", "true");
  });

  it("calls onSave with enabled and params", () => {
    const onSave = vi.fn();
    render(
      <StrategyConfigModal
        meta={mockMeta}
        config={{ strategyId: "ma_cross", enabled: true, params: {} }}
        onSave={onSave}
        onClose={() => {}}
      />,
    );
    fireEvent.click(screen.getByText("保存"));
    expect(onSave).toHaveBeenCalledWith(true, { fast_period: 5, slow_period: 20 });
  });

  it("resets to defaults on reset click", () => {
    render(
      <StrategyConfigModal
        meta={mockMeta}
        config={{ strategyId: "ma_cross", enabled: true, params: { fast_period: 10, slow_period: 30 } }}
        onSave={() => {}}
        onClose={() => {}}
      />,
    );
    fireEvent.click(screen.getByText("恢复默认"));
    expect(screen.getByLabelText("快速周期")).toHaveValue(5);
    expect(screen.getByLabelText("慢速周期")).toHaveValue(20);
  });

  it("closes when Escape is pressed", () => {
    const onClose = vi.fn();
    render(
      <StrategyConfigModal
        meta={mockMeta}
        config={{ strategyId: "ma_cross", enabled: true, params: {} }}
        onSave={() => {}}
        onClose={onClose}
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog", { name: "配置 均线交叉" }), { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
