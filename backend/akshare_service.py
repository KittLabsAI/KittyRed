from contextlib import redirect_stdout
import io
import json
import sys
from typing import Any

import akshare as ak

from backend.akshare_adapter import AkshareAdapter


def handle_request(
    payload: dict[str, Any],
    adapter: AkshareAdapter | None = None,
) -> dict[str, Any]:
    action = payload.get("action")
    adapter = adapter or AkshareAdapter(ak)

    if action == "current_quote":
        data = adapter.current_quote(
            symbol=str(payload.get("symbol", "")).strip(),
        )
        return {"ok": True, "data": data}

    if action == "current_quotes":
        raw_symbols = payload.get("symbols", [])
        symbols = raw_symbols if isinstance(raw_symbols, list) else []
        data = adapter.current_quotes(symbols=[str(symbol) for symbol in symbols])
        return {"ok": True, "data": data}

    if action == "history_bars":
        data = adapter.history_bars(
            symbol=str(payload.get("symbol", "")).strip(),
            frequency=str(payload.get("frequency", "1d")).strip(),
            count=int(payload.get("count", 120)),
        )
        return {"ok": True, "data": data}

    if action == "stock_info":
        data = adapter.stock_info(
            symbol=str(payload.get("symbol", "")).strip(),
        )
        return {"ok": True, "data": data}

    if action == "bid_ask":
        data = adapter.bid_ask(
            symbol=str(payload.get("symbol", "")).strip(),
        )
        return {"ok": True, "data": data}

    if action == "multi_frequency_bars":
        data = adapter.multi_frequency_bars(
            symbol=str(payload.get("symbol", "")).strip(),
            count=int(payload.get("count", 60)),
            timeout_seconds=float(payload.get("timeout_seconds", 15)),
        )
        return {"ok": True, "data": data}

    if action == "financial_reports":
        data = adapter.financial_reports(
            symbol=str(payload.get("symbol", "")).strip(),
            years=int(payload.get("years", 2)),
        )
        return {"ok": True, "data": data}

    if action == "search_stocks":
        data = adapter.search_stocks(
            query=str(payload.get("query", "")).strip(),
        )
        return {"ok": True, "data": data}

    if action == "stock_universe":
        data = adapter.stock_universe()
        return {"ok": True, "data": data}

    if action == "is_trade_date":
        data = adapter.is_trade_date(
            date=str(payload.get("date", "")).strip(),
        )
        return {"ok": True, "data": data}

    if action == "submit_order":
        return {
            "ok": False,
            "error": "AKShare is a market data source, not a trading engine",
        }

    return {"ok": False, "error": f"unsupported action: {action}"}


def main() -> int:
    try:
        payload = json.loads(sys.stdin.read())
        with redirect_stdout(io.StringIO()):
            response = handle_request(payload)
    except Exception as error:
        response = {"ok": False, "error": str(error)}

    sys.stdout.write(json.dumps(response, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
