import { useEffect, useMemo, useRef, useState } from "react";
import { CandlestickSeries, ColorType, HistogramSeries, createChart, type UTCTimestamp } from "lightweight-charts";
import { formatDateTime, formatPercent } from "../../lib/format";
import type { CandleBar } from "../../lib/types";

type CandlestickChartProps = {
  bars: CandleBar[];
};

export function CandlestickChart({ bars }: CandlestickChartProps) {
  const chartContainerRef = useRef<HTMLDivElement | null>(null);
  const [tooltip, setTooltip] = useState<{ bar: CandleBar; x: number; y: number } | null>(null);
  const barsByTime = useMemo(
    () => new Map(bars.map((bar) => [toChartTimestamp(bar.openTime), bar])),
    [bars],
  );

  useEffect(() => {
    if (!chartContainerRef.current || bars.length === 0) {
      return undefined;
    }

    const chart = createChart(chartContainerRef.current, {
      autoSize: true,
      crosshair: {
        mode: 0,
      },
      grid: {
        horzLines: {
          color: "rgba(148, 163, 184, 0.12)",
        },
        vertLines: {
          color: "rgba(148, 163, 184, 0.08)",
        },
      },
      layout: {
        background: {
          color: "transparent",
          type: ColorType.Solid,
        },
        textColor: "#cbd5f5",
      },
      rightPriceScale: {
        borderColor: "rgba(148, 163, 184, 0.12)",
      },
      timeScale: {
        borderColor: "rgba(148, 163, 184, 0.12)",
        timeVisible: true,
      },
    });
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      downColor: "#f97316",
      borderDownColor: "#f97316",
      wickDownColor: "#f97316",
      upColor: "#10b981",
      borderUpColor: "#10b981",
      wickUpColor: "#10b981",
    });
    const volumeSeries = chart.addSeries(HistogramSeries, {
      priceFormat: {
        type: "volume",
      },
      priceScaleId: "",
    });

    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        bottom: 0,
        top: 0.74,
      },
    });

    candlestickSeries.setData(
      bars.map((bar) => ({
        close: bar.close,
        high: bar.high,
        low: bar.low,
        open: bar.open,
        time: toChartTimestamp(bar.openTime),
      })),
    );
    volumeSeries.setData(
      bars.map((bar) => ({
        color: bar.close >= bar.open ? "rgba(16, 185, 129, 0.28)" : "rgba(249, 115, 22, 0.28)",
        time: toChartTimestamp(bar.openTime),
        value: bar.volume,
      })),
    );
    chart.subscribeCrosshairMove((param) => {
      if (!param.point || param.time === undefined) {
        setTooltip(null);
        return;
      }

      const bar = barsByTime.get(param.time as UTCTimestamp);
      setTooltip(bar ? { bar, x: param.point.x, y: param.point.y } : null);
    });
    if (bars.length > 80) {
      chart.timeScale().setVisibleLogicalRange({ from: bars.length - 80, to: bars.length + 2 });
    } else {
      chart.timeScale().fitContent();
    }

    return () => {
      chart.remove();
    };
  }, [bars, barsByTime]);

  if (bars.length === 0) {
    return <p className="panel__meta">暂无 K 线数据。</p>;
  }

  return (
    <div className="kline-chart-shell">
      <div className="kline-chart" ref={chartContainerRef} />
      {tooltip ? <CandleTooltip bar={tooltip.bar} x={tooltip.x} y={tooltip.y} /> : null}
    </div>
  );
}

function CandleTooltip({ bar, x, y }: { bar: CandleBar; x: number; y: number }) {
  const change = bar.close - bar.open;
  const changePercent = bar.open === 0 ? 0 : (change / bar.open) * 100;
  const rangePercent = bar.open === 0 ? 0 : ((bar.high - bar.low) / bar.open) * 100;

  return (
    <div className="kline-tooltip" style={{ left: x + 12, top: y + 12 }}>
      <strong>{formatDateTime(bar.openTime)}</strong>
      <dl>
        <div>
          <dt>开盘</dt>
          <dd>{formatNumber(bar.open)}</dd>
        </div>
        <div>
          <dt>最高</dt>
          <dd>{formatNumber(bar.high)}</dd>
        </div>
        <div>
          <dt>最低</dt>
          <dd>{formatNumber(bar.low)}</dd>
        </div>
        <div>
          <dt>收盘</dt>
          <dd>{formatNumber(bar.close)}</dd>
        </div>
        <div>
          <dt>涨跌</dt>
          <dd>{formatSignedNumber(change)}</dd>
        </div>
        <div>
          <dt>涨跌幅</dt>
          <dd>{formatPercent(changePercent)}</dd>
        </div>
        <div>
          <dt>振幅</dt>
          <dd>{formatPercent(rangePercent).replace("+", "")}</dd>
        </div>
        <div>
          <dt>成交量</dt>
          <dd>{formatNumber(bar.volume)}</dd>
        </div>
        <div>
          <dt>成交额</dt>
          <dd>{bar.turnover === undefined ? "无" : formatNumber(bar.turnover)}</dd>
        </div>
      </dl>
    </div>
  );
}

function formatNumber(value: number) {
  return Number.isInteger(value) ? value.toString() : value.toFixed(4).replace(/0+$/, "").replace(/\.$/, "");
}

function formatSignedNumber(value: number) {
  const sign = value > 0 ? "+" : "";
  return `${sign}${formatNumber(value)}`;
}

function toChartTimestamp(openTime: string): UTCTimestamp {
  const numeric = Number(openTime);
  const timestamp = Number.isFinite(numeric) && numeric > 0 ? normalizeTimestamp(numeric) : Date.parse(openTime);
  return Math.floor(timestamp / 1_000) as UTCTimestamp;
}

function normalizeTimestamp(raw: number): number {
  return raw < 1_000_000_000_000 ? raw * 1_000 : raw;
}
