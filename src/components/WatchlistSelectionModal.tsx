import { useEffect, useMemo, useState } from "react";
import { Button } from "./ui/button";
import type { MarketRow } from "../lib/types";

interface WatchlistSelectionModalProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  watchlist: MarketRow[];
  onConfirm: (symbols: string[]) => void;
  onClose: () => void;
}

export function WatchlistSelectionModal({
  open,
  title,
  description,
  confirmLabel,
  watchlist,
  onConfirm,
  onClose,
}: WatchlistSelectionModalProps) {
  const [selectedSymbols, setSelectedSymbols] = useState<string[]>([]);

  useEffect(() => {
    if (open) {
      setSelectedSymbols(watchlist.map((row) => row.symbol));
    }
  }, [open, watchlist]);

  const selectedSet = useMemo(() => new Set(selectedSymbols), [selectedSymbols]);
  const selectedCount = selectedSymbols.length;
  const allChecked = watchlist.length > 0 && selectedCount === watchlist.length;

  const toggleSymbol = (symbol: string) => {
    setSelectedSymbols((current) =>
      current.includes(symbol) ? current.filter((item) => item !== symbol) : [...current, symbol],
    );
  };

  const selectAll = () => setSelectedSymbols(watchlist.map((row) => row.symbol));
  const clearAll = () => setSelectedSymbols([]);

  if (!open) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        aria-label={title}
        aria-modal="true"
        className="modal-content watchlist-selection-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="modal-header">
          <div>
            <h3>{title}</h3>
            <p className="panel__meta">{description}</p>
          </div>
          <Button aria-label="关闭选择" onClick={onClose} size="sm" variant="ghost">
            ✕
          </Button>
        </div>

        <div className="modal-body grid gap-3">
          <div className="watchlist-selection-modal__toolbar">
            <span>已选 {selectedCount} 只</span>
            <div className="watchlist-selection-modal__toolbar-actions">
              <Button onClick={selectAll} size="sm" variant="ghost">
                全选
              </Button>
              <Button onClick={clearAll} size="sm" variant="ghost">
                清空
              </Button>
            </div>
          </div>

          {watchlist.length > 0 ? (
            <div className="backtest-watchlist-select">
              {watchlist.map((row) => (
                <label className="backtest-watchlist-option" key={row.symbol}>
                  <input
                    checked={selectedSet.has(row.symbol)}
                    onChange={() => toggleSymbol(row.symbol)}
                    type="checkbox"
                  />
                  <span>
                    <strong>{row.symbol}</strong>
                    <small>{row.baseAsset}</small>
                  </span>
                </label>
              ))}
            </div>
          ) : (
            <div className="backtest-watchlist-empty">
              <strong>自选股票池为空</strong>
              <span>请先在行情页维护自选股，再继续分析。</span>
            </div>
          )}

          {selectedCount === 0 ? <p className="watchlist-selection-modal__hint">至少选择 1 只股票才能继续。</p> : null}
        </div>

        <div className="modal-footer">
          <Button onClick={onClose} variant="ghost">
            取消
          </Button>
          <Button disabled={selectedCount === 0} onClick={() => onConfirm(selectedSymbols)}>
            {allChecked ? confirmLabel : `${confirmLabel}（${selectedCount}）`}
          </Button>
        </div>
      </div>
    </div>
  );
}
