import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CandlestickChart } from "./CandlestickChart";

const { addSeries, candlestickSetData, createChart, fitContent, histogramSetData, subscribeCrosshairMove } = vi.hoisted(() => {
  const candlestickSetData = vi.fn();
  const histogramSetData = vi.fn();
  const fitContent = vi.fn();
  const subscribeCrosshairMove = vi.fn();
  const remove = vi.fn();
  const addSeries = vi.fn((seriesType: string) => {
    if (seriesType === "CandlestickSeries") {
      return {
        setData: candlestickSetData,
      };
    }

    return {
      priceScale: () => ({
        applyOptions: vi.fn(),
      }),
      setData: histogramSetData,
    };
  });
  const createChart = vi.fn(() => ({
    addSeries,
    applyOptions: vi.fn(),
    remove,
    subscribeCrosshairMove,
    timeScale: () => ({
      fitContent,
    }),
  }));

  return {
    addSeries,
    candlestickSetData,
    createChart,
    fitContent,
    histogramSetData,
    subscribeCrosshairMove,
  };
});

vi.mock("lightweight-charts", () => ({
  CandlestickSeries: "CandlestickSeries",
  ColorType: {
    Solid: "solid",
  },
  HistogramSeries: "HistogramSeries",
  createChart,
}));

describe("CandlestickChart", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders candles with lightweight-charts instead of inline SVG", async () => {
    render(
      <CandlestickChart
        bars={[
          { openTime: "1714734000000", open: 68420, high: 68480, low: 68390, close: 68450, volume: 820 },
          { openTime: "1714734060000", open: 68450, high: 68490, low: 68410, close: 68430, volume: 740 },
        ]}
      />,
    );

    expect(screen.queryByLabelText("Candlestick chart")).not.toBeInTheDocument();

    await waitFor(() => {
      expect(createChart).toHaveBeenCalledTimes(1);
      expect(addSeries).toHaveBeenCalledTimes(2);
      expect(candlestickSetData).toHaveBeenCalledWith([
        { close: 68450, high: 68480, low: 68390, open: 68420, time: 1714734000 },
        { close: 68430, high: 68490, low: 68410, open: 68450, time: 1714734060 },
      ]);
      expect(histogramSetData).toHaveBeenCalledWith([
        { color: "rgba(16, 185, 129, 0.28)", time: 1714734000, value: 820 },
        { color: "rgba(249, 115, 22, 0.28)", time: 1714734060, value: 740 },
      ]);
      expect(fitContent).toHaveBeenCalledTimes(1);
    });
  });

  it("subscribes a tooltip with OHLC change, range, volume, and turnover context", async () => {
    render(
      <CandlestickChart
        bars={[
          { openTime: "1777864020000", open: 100, high: 110, low: 95, close: 108, volume: 12, turnover: 1296 },
        ]}
      />,
    );

    await waitFor(() => {
      expect(subscribeCrosshairMove).toHaveBeenCalledTimes(1);
    });

    const handler = subscribeCrosshairMove.mock.calls[0][0];
    handler({
      point: { x: 40, y: 60 },
      time: 1777864020,
    });

    expect(await screen.findByText("2026-05-04 11:07:00")).toBeInTheDocument();
    expect(screen.getByText("开盘")).toBeInTheDocument();
    expect(screen.getByText("100")).toBeInTheDocument();
    expect(screen.getByText("最高")).toBeInTheDocument();
    expect(screen.getByText("110")).toBeInTheDocument();
    expect(screen.getByText("最低")).toBeInTheDocument();
    expect(screen.getByText("95")).toBeInTheDocument();
    expect(screen.getByText("收盘")).toBeInTheDocument();
    expect(screen.getByText("108")).toBeInTheDocument();
    expect(screen.getByText("涨跌")).toBeInTheDocument();
    expect(screen.getByText("+8")).toBeInTheDocument();
    expect(screen.getByText("涨跌幅")).toBeInTheDocument();
    expect(screen.getByText("+8.00%")).toBeInTheDocument();
    expect(screen.getByText("振幅")).toBeInTheDocument();
    expect(screen.getByText("15.00%")).toBeInTheDocument();
    expect(screen.getByText("成交量")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("成交额")).toBeInTheDocument();
    expect(screen.getByText("1296")).toBeInTheDocument();
  });

  it("shows a fallback when no candle data is returned", () => {
    render(<CandlestickChart bars={[]} />);

    expect(screen.getByText("暂无 K 线数据。")).toBeInTheDocument();
    expect(createChart).not.toHaveBeenCalled();
  });
});
