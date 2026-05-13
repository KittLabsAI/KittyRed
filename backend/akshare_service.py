from contextlib import redirect_stdout
import io
import json
import sys
from typing import Any

import akshare as ak

from backend.akshare_adapter import AkshareAdapter

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


def handle_request(
    payload: dict[str, Any],
    adapter: AkshareAdapter | None = None,
) -> dict[str, Any]:
    action = payload.get("action")
    adapter = adapter or AkshareAdapter(
        ak,
        xueqiu_token=str(payload.get("xueqiu_token", "")).strip(),
        intraday_data_source=str(payload.get("intraday_data_source", "sina")).strip(),
        historical_data_source=str(
            payload.get("historical_data_source", "eastmoney")
        ).strip(),
    )
    adapter.intraday_data_source = adapter._normalize_intraday_data_source(
        str(payload.get("intraday_data_source", adapter.intraday_data_source)).strip()
    )
    adapter.historical_data_source = adapter._normalize_historical_data_source(
        str(
            payload.get(
                "historical_data_source",
                adapter.historical_data_source,
            )
        ).strip()
    )

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
        raw_start_date = payload.get("start_date")
        raw_end_date = payload.get("end_date")
        data = adapter.history_bars(
            symbol=str(payload.get("symbol", "")).strip(),
            frequency=str(payload.get("frequency", "1d")).strip(),
            count=int(payload.get("count", 120)),
            start_date=(str(raw_start_date).strip() if raw_start_date is not None else "") or None,
            end_date=(str(raw_end_date).strip() if raw_end_date is not None else "") or None,
        )
        return {"ok": True, "data": data}

    if action == "stock_info":
        data = adapter.stock_info(
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

    if action == "financial_report_probe":
        data = adapter.financial_report_probe()
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
