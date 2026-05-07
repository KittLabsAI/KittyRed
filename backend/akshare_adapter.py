from datetime import datetime, timedelta
import json
import queue
import subprocess
import sys
import threading
import time
from typing import Any, Callable

import pandas as pd


class AkshareAdapter:
    def __init__(
        self,
        client: Any,
        now: Callable[[], datetime] | None = None,
        quote_attempts: int = 2,
    ):
        self.client = client
        self.now = now or datetime.now
        self.quote_attempts = max(1, quote_attempts)

    def current_quote(self, symbol: str) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return self._empty_quote(symbol)

        try:
            return self._xueqiu_quote(clean_symbol)
        except Exception:
            pass

        code = self._akshare_code(clean_symbol)
        spot_row = self._spot_row_for_code(code)
        try:
            rows = self._stock_bid_ask(code)
        except Exception:
            raise

        values = {
            str(row["item"]): row["value"]
            for _, row in rows.iterrows()
        }
        return {
            "symbol": clean_symbol,
            "name": self._spot_value(spot_row, "名称", ""),
            "last": self._number(values.get("最新", 0)),
            "open": self._number(values.get("今开", 0)),
            "high": self._number(values.get("最高", 0)),
            "low": self._number(values.get("最低", 0)),
            "change_pct": self._number(self._spot_value(spot_row, "涨跌幅", 0)),
            "volume": self._number(values.get("总手", 0)) * 100,
            "amount": self._number(values.get("金额", 0)),
            "updated_at": str(self._spot_value(spot_row, "更新时间", "")),
            "source": "akshare",
        }

    def current_quotes(self, symbols: list[str]) -> list[dict[str, Any]]:
        clean_symbols = [self._normalize_symbol(symbol) for symbol in symbols]
        clean_symbols = [symbol for symbol in clean_symbols if symbol]
        if not clean_symbols:
            return []

        last_error = None
        quotes = []
        for symbol in clean_symbols:
            try:
                quotes.append(self._xueqiu_quote(symbol))
            except Exception as error:
                last_error = error
        if not quotes and last_error is not None:
            raise last_error
        return quotes

    def history_bars(
        self,
        symbol: str,
        frequency: str,
        count: int,
        merge_realtime: bool = True,
    ) -> list[dict[str, Any]]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return []

        end = self.now()
        start = end - timedelta(days=max(count, 1) + 60)
        if self._is_intraday_frequency(frequency):
            rows = self.client.stock_zh_a_minute(
                symbol=self._akshare_daily_symbol(clean_symbol),
                period=self._akshare_minute_period(frequency),
                adjust="",
            )
            return self._sina_minute_rows_to_bars(rows.tail(max(count, 1)))

        try:
            rows = self.client.stock_zh_a_hist(
                symbol=self._akshare_code(clean_symbol),
                period=self._akshare_period(frequency),
                start_date=start.strftime("%Y%m%d"),
                end_date=end.strftime("%Y%m%d"),
                adjust="qfq",
                timeout=3,
            )
        except Exception:
            daily_rows = self.client.stock_zh_a_daily(
                symbol=self._akshare_daily_symbol(clean_symbol),
                adjust="",
            )
            bars = self._daily_rows_to_bars(clean_symbol, daily_rows.tail(max(count, 1)))
            if frequency.lower() in {"1w", "weekly"}:
                bars = self._daily_bars_to_weekly_bars(bars)
            if merge_realtime:
                return self._merge_realtime_bars(clean_symbol, frequency, bars)
            return bars

        tail = rows.tail(max(count, 1))
        bars = self._hist_rows_to_bars(tail, "日期")
        if merge_realtime:
            return self._merge_realtime_bars(clean_symbol, frequency, bars)
        return bars

    def stock_info(self, symbol: str) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return {"stock_code": symbol, "items": {}, "source": "akshare:xueqiu_basic"}

        rows = self.client.stock_individual_basic_info_xq(
            symbol=self._xueqiu_symbol(clean_symbol),
        )
        items = {
            str(row["item"]): row["value"]
            for _, row in rows.iterrows()
        }
        return {
            "stock_code": clean_symbol,
            "items": items,
            "source": "akshare:xueqiu_basic",
        }

    def bid_ask(self, symbol: str) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return {
                "stock_code": symbol,
                "bid_levels": [],
                "ask_levels": [],
                "source": "akshare:eastmoney_bid_ask",
            }

        rows = self._stock_bid_ask(self._akshare_code(clean_symbol))
        values = {
            str(row["item"]): row["value"]
            for _, row in rows.iterrows()
        }
        return {
            "stock_code": clean_symbol,
            "last_price": self._number(values.get("最新", 0)),
            "average_price": self._number(values.get("均价", 0)),
            "change_percent": self._number(values.get("涨幅", 0)),
            "change_amount": self._number(values.get("涨跌", 0)),
            "volume": self._number(values.get("总手", 0)) * 100,
            "turnover": self._number(values.get("金额", 0)),
            "turnover_rate": self._number(values.get("换手", 0)),
            "volume_ratio": self._number(values.get("量比", 0)),
            "high": self._number(values.get("最高", 0)),
            "low": self._number(values.get("最低", 0)),
            "open": self._number(values.get("今开", 0)),
            "previous_close": self._number(values.get("昨收", 0)),
            "limit_up": self._number(values.get("涨停", 0)),
            "limit_down": self._number(values.get("跌停", 0)),
            "outer_volume": self._number(values.get("外盘", 0)) * 100,
            "inner_volume": self._number(values.get("内盘", 0)) * 100,
            "bid_levels": self._price_levels(values, "buy"),
            "ask_levels": self._price_levels(values, "sell"),
            "source": "akshare:eastmoney_bid_ask",
        }

    def multi_frequency_bars(
        self,
        symbol: str,
        count: int = 60,
        timeout_seconds: float = 15,
    ) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return {"stock_code": symbol, "bars": {}, "source": "akshare"}

        requests = {
            "5m": lambda: self.history_bars(clean_symbol, "5m", count),
            "1h": lambda: self.history_bars(clean_symbol, "1h", count),
            "1d": lambda: self.history_bars(clean_symbol, "1d", count, merge_realtime=False),
            "1w": lambda: self.history_bars(clean_symbol, "1w", count, merge_realtime=False),
        }
        if self._uses_default_akshare_client():
            bars, messages = self._parallel_kline_subprocesses(
                clean_symbol,
                count,
                timeout_seconds,
            )
        else:
            bars, messages = self._parallel_kline_requests(requests, timeout_seconds)
        return {
            "stock_code": clean_symbol,
            "bars": bars,
            "messages": messages,
            "source": "akshare",
        }

    def _parallel_kline_requests(
        self,
        requests: dict[str, Callable[[], list[dict[str, Any]]]],
        timeout_seconds: float,
    ) -> tuple[dict[str, list[dict[str, Any]]], dict[str, str]]:
        result_queue: queue.Queue[tuple[str, bool, list[dict[str, Any]], str]] = queue.Queue()
        threads = []

        def run(label: str, load: Callable[[], list[dict[str, Any]]]) -> None:
            try:
                result_queue.put((label, True, load(), ""))
            except Exception as error:
                result_queue.put((label, False, [], str(error)))

        for label, load in requests.items():
            thread = threading.Thread(target=run, args=(label, load), daemon=True)
            thread.start()
            threads.append(thread)

        deadline = time.monotonic() + max(timeout_seconds, 0)
        results: dict[str, list[dict[str, Any]]] = {label: [] for label in requests}
        messages: dict[str, str] = {}
        pending = set(requests)

        while pending:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                break
            try:
                label, ok, bars, error = result_queue.get(timeout=remaining)
            except queue.Empty:
                break
            if label not in pending:
                continue
            pending.remove(label)
            if ok:
                results[label] = bars
            else:
                messages[label] = f"网络原因导致 {label} K 线读取失败：{error}"

        for label in pending:
            messages[label] = f"网络原因导致 {label} K 线读取超时，已返回空值。"

        for thread in threads:
            thread.join(timeout=0)

        return results, messages

    def _parallel_kline_subprocesses(
        self,
        symbol: str,
        count: int,
        timeout_seconds: float,
    ) -> tuple[dict[str, list[dict[str, Any]]], dict[str, str]]:
        specs = {
            "5m": ("5m", True),
            "1h": ("1h", True),
            "1d": ("1d", False),
            "1w": ("1w", False),
        }
        code = (
            "import json, sys, akshare as ak; "
            "from backend.akshare_adapter import AkshareAdapter; "
            "symbol=sys.argv[1]; frequency=sys.argv[2]; count=int(sys.argv[3]); "
            "merge_realtime=sys.argv[4] == '1'; "
            "bars=AkshareAdapter(ak).history_bars(symbol, frequency, count, merge_realtime=merge_realtime); "
            "print(json.dumps(bars, ensure_ascii=False))"
        )
        processes = {
            label: subprocess.Popen(
                [
                    sys.executable,
                    "-c",
                    code,
                    symbol,
                    frequency,
                    str(count),
                    "1" if merge_realtime else "0",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            for label, (frequency, merge_realtime) in specs.items()
        }
        deadline = time.monotonic() + max(timeout_seconds, 0)
        results: dict[str, list[dict[str, Any]]] = {label: [] for label in specs}
        messages: dict[str, str] = {}

        for label, process in processes.items():
            remaining = max(0, deadline - time.monotonic())
            try:
                stdout, stderr = process.communicate(timeout=remaining)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate()
                messages[label] = f"网络原因导致 {label} K 线读取超时，已返回空值。"
                continue

            if process.returncode != 0:
                error = stderr.strip().splitlines()[-1] if stderr.strip() else "unknown error"
                messages[label] = f"网络原因导致 {label} K 线读取失败：{error}"
                continue

            try:
                results[label] = json.loads(stdout)
            except json.JSONDecodeError as error:
                messages[label] = f"网络原因导致 {label} K 线响应解析失败：{error}"

        return results, messages

    def _uses_default_akshare_client(self) -> bool:
        return getattr(self.client, "__name__", "") == "akshare"

    def _merge_realtime_bars(
        self,
        symbol: str,
        frequency: str,
        bars: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        if frequency.lower() in {"1d", "daily"}:
            bars = self._merge_realtime_daily_bar(symbol, bars)
        if frequency.lower() in {"1w", "weekly"}:
            bars = self._merge_realtime_weekly_bar(symbol, bars)
        return bars

    def search_stocks(self, query: str) -> list[dict[str, Any]]:
        clean_query = query.strip().upper()
        if not clean_query:
            return []

        items = self.stock_universe()
        results = []
        for item in items:
            symbol = item["symbol"].upper()
            code = symbol.split(".")[-1]
            name = item["name"].upper()
            if clean_query in symbol or clean_query in code or clean_query in name:
                results.append(item)
            if len(results) >= 20:
                break
        return results

    def stock_universe(self) -> list[dict[str, Any]]:
        rows = self.client.stock_info_a_code_name()
        code_key = "code" if "code" in rows.columns else "代码"
        name_key = "name" if "name" in rows.columns else "名称"
        items = []
        for _, row in rows.iterrows():
            code = str(row.get(code_key, "")).zfill(6)
            if not code.strip():
                continue
            symbol = self._internal_symbol(code)
            items.append(
                {
                    "symbol": symbol,
                    "name": str(row.get(name_key, "")).strip(),
                    "market": self._market_label(symbol),
                }
            )
        return items

    def is_trade_date(self, date: str) -> bool:
        rows = self.client.tool_trade_date_hist_sina()
        dates = {
            self._date_string(row.get("trade_date", row.get("交易日", "")))
            for _, row in rows.iterrows()
        }
        return date in dates

    def submit_order(self, symbol: str, side: str, quantity: int) -> dict[str, Any]:
        raise NotImplementedError("AKShare is a market data source, not a trading engine")

    def _stock_bid_ask(self, code: str) -> pd.DataFrame:
        last_error = None
        for attempt in range(self.quote_attempts):
            try:
                return self.client.stock_bid_ask_em(symbol=code)
            except Exception as error:
                last_error = error
                if attempt + 1 < self.quote_attempts:
                    time.sleep(0.2)
        raise last_error

    def _xueqiu_quote(self, symbol: str) -> dict[str, Any]:
        rows = self.client.stock_individual_spot_xq(symbol=self._xueqiu_symbol(symbol))
        return self._quote_from_xueqiu_rows(symbol, rows)

    def _quote_from_xueqiu_rows(self, fallback_symbol: str, rows: pd.DataFrame) -> dict[str, Any]:
        values = {
            str(row["item"]): row["value"]
            for _, row in rows.iterrows()
        }
        symbol = self._internal_symbol_from_xueqiu(str(values.get("代码", ""))) or fallback_symbol
        return {
            "symbol": symbol,
            "name": str(values.get("名称", "")).strip(),
            "last": self._number(values.get("现价", 0)),
            "open": self._number(values.get("今开", 0)),
            "high": self._number(values.get("最高", 0)),
            "low": self._number(values.get("最低", 0)),
            "change_pct": self._number(values.get("涨幅", 0)),
            "volume": self._number(values.get("成交量", 0)),
            "amount": self._number(values.get("成交额", 0)),
            "updated_at": str(values.get("时间", "")),
            "source": "akshare:xueqiu",
        }

    def _quote_from_row(self, symbol: str, row: pd.Series) -> dict[str, Any]:
        volume = self._number(row.get("成交量", 0))
        return {
            "symbol": symbol,
            "last": self._number(row.get("最新价", 0)),
            "open": self._number(row.get("今开", 0)),
            "high": self._number(row.get("最高", 0)),
            "low": self._number(row.get("最低", 0)),
            "volume": volume * 100,
            "amount": self._number(row.get("成交额", 0)),
            "updated_at": str(row.get("更新时间", row.get("时间戳", ""))),
            "source": "akshare",
        }

    def _quote_from_daily_row(
        self,
        symbol: str,
        row: pd.Series,
        spot_row: pd.Series | None = None,
    ) -> dict[str, Any]:
        return {
            "symbol": symbol,
            "name": self._spot_value(spot_row, "名称", ""),
            "last": self._number(row.get("close", 0)),
            "open": self._number(row.get("open", 0)),
            "high": self._number(row.get("high", 0)),
            "low": self._number(row.get("low", 0)),
            "change_pct": self._number(self._spot_value(spot_row, "涨跌幅", 0)),
            "volume": self._number(row.get("volume", 0)),
            "amount": self._number(row.get("amount", 0)),
            "updated_at": str(row.get("date", "")),
            "source": "akshare",
        }

    def _daily_rows_to_bars(self, symbol: str, rows: pd.DataFrame) -> list[dict[str, Any]]:
        return [
            {
                "symbol": symbol,
                "open_time": self._date_string(row.get("date", "")),
                "open": self._number(row.get("open", 0)),
                "high": self._number(row.get("high", 0)),
                "low": self._number(row.get("low", 0)),
                "close": self._number(row.get("close", 0)),
                "volume": self._number(row.get("volume", 0)),
                "turnover": self._number(row.get("amount", 0)),
            }
            for _, row in rows.iterrows()
        ]

    def _empty_quote(self, symbol: str) -> dict[str, Any]:
        return {
            "symbol": symbol,
            "name": "",
            "last": 0,
            "open": 0,
            "high": 0,
            "low": 0,
            "change_pct": 0,
            "volume": 0,
            "amount": 0,
            "updated_at": "",
            "source": "akshare",
        }

    def _akshare_period(self, frequency: str) -> str:
        if frequency.lower() in {"1w", "weekly"}:
            return "weekly"
        if frequency in {"1M", "monthly"}:
            return "monthly"
        return "daily"

    def _akshare_minute_period(self, frequency: str) -> str:
        cleaned = frequency.strip().lower()
        if cleaned in {"1h", "60m", "60"}:
            return "60"
        return cleaned.removesuffix("m")

    def _is_intraday_frequency(self, frequency: str) -> bool:
        return frequency.strip().lower() in {"1m", "5m", "15m", "30m", "60m", "1h"}

    def _normalize_symbol(self, symbol: str) -> str:
        raw = symbol.strip().upper()
        if raw.startswith("SHSE.") or raw.startswith("SZSE."):
            return raw
        code = "".join(ch for ch in raw if ch.isdigit()).zfill(6)
        return self._internal_symbol(code) if code.strip("0") else ""

    def _akshare_code(self, symbol: str) -> str:
        return symbol.split(".")[-1].zfill(6)

    def _akshare_daily_symbol(self, symbol: str) -> str:
        prefix = "sh" if symbol.startswith("SHSE.") else "sz"
        return f"{prefix}{self._akshare_code(symbol)}"

    def _xueqiu_symbol(self, symbol: str) -> str:
        prefix = "SH" if symbol.startswith("SHSE.") else "SZ"
        return f"{prefix}{self._akshare_code(symbol)}"

    def _internal_symbol_from_xueqiu(self, symbol: str) -> str:
        value = symbol.strip().upper()
        if value.startswith("SH"):
            return f"SHSE.{value[2:].zfill(6)}"
        if value.startswith("SZ"):
            return f"SZSE.{value[2:].zfill(6)}"
        return ""

    def _internal_symbol(self, code: str) -> str:
        return f"SHSE.{code}" if code.startswith(("5", "6", "9")) else f"SZSE.{code}"

    def _market_label(self, symbol: str) -> str:
        if symbol.startswith("SHSE."):
            return "沪市A股"
        if symbol.startswith("SZSE."):
            return "深市A股"
        return "A股"

    def _spot_rows_by_code(self) -> dict[str, pd.Series]:
        try:
            rows = self.client.stock_zh_a_spot_em()
        except Exception:
            return {}
        return {
            str(row.get("代码", "")).zfill(6): row
            for _, row in rows.iterrows()
        }

    def _spot_row_for_code(self, code: str) -> pd.Series | None:
        return self._spot_rows_by_code().get(code)

    def _spot_value(self, row: pd.Series | None, key: str, default: Any) -> Any:
        if row is None:
            return default
        return row.get(key, default)

    def _quote_from_spot_row(self, symbol: str, row: pd.Series) -> dict[str, Any]:
        volume = self._number(row.get("成交量", 0))
        return {
            "symbol": symbol,
            "name": str(row.get("名称", "")).strip(),
            "last": self._number(row.get("最新价", 0)),
            "open": self._number(row.get("今开", 0)),
            "high": self._number(row.get("最高", 0)),
            "low": self._number(row.get("最低", 0)),
            "change_pct": self._number(row.get("涨跌幅", 0)),
            "volume": volume * 100,
            "amount": self._number(row.get("成交额", 0)),
            "updated_at": str(row.get("更新时间", row.get("时间戳", ""))),
            "source": "akshare",
        }

    def _hist_rows_to_bars(self, rows: pd.DataFrame, time_key: str) -> list[dict[str, Any]]:
        return [
            {
                "open_time": self._date_string(row.get(time_key, "")),
                "open": self._number(row.get("开盘", 0)),
                "high": self._number(row.get("最高", 0)),
                "low": self._number(row.get("最低", 0)),
                "close": self._number(row.get("收盘", 0)),
                "volume": self._number(row.get("成交量", 0)) * 100,
                "turnover": self._number(row.get("成交额", 0)),
            }
            for _, row in rows.iterrows()
        ]

    def _sina_minute_rows_to_bars(self, rows: pd.DataFrame) -> list[dict[str, Any]]:
        return [
            {
                "open_time": str(row.get("day", "")),
                "open": self._number(row.get("open", 0)),
                "high": self._number(row.get("high", 0)),
                "low": self._number(row.get("low", 0)),
                "close": self._number(row.get("close", 0)),
                "volume": self._number(row.get("volume", 0)),
                "turnover": self._number(row.get("amount", 0)),
            }
            for _, row in rows.iterrows()
        ]

    def _price_levels(self, values: dict[str, Any], side: str) -> list[dict[str, Any]]:
        return [
            {
                "level": level,
                "price": self._number(values.get(f"{side}_{level}", 0)),
                "volume": self._number(values.get(f"{side}_{level}_vol", 0)),
            }
            for level in range(1, 6)
            if self._number(values.get(f"{side}_{level}", 0)) > 0
        ]

    def _daily_bars_to_weekly_bars(self, bars: list[dict[str, Any]]) -> list[dict[str, Any]]:
        weekly: dict[str, dict[str, Any]] = {}
        for bar in bars:
            try:
                open_date = datetime.strptime(str(bar["open_time"]), "%Y-%m-%d")
            except ValueError:
                continue
            week_start = (open_date - timedelta(days=open_date.weekday())).strftime("%Y-%m-%d")
            current = weekly.get(week_start)
            if current is None:
                weekly[week_start] = {
                    "open_time": week_start,
                    "open": bar["open"],
                    "high": bar["high"],
                    "low": bar["low"],
                    "close": bar["close"],
                    "volume": bar["volume"],
                    "turnover": bar["turnover"],
                }
                continue
            current["high"] = max(current["high"], bar["high"])
            current["low"] = min(current["low"], bar["low"])
            current["close"] = bar["close"]
            current["volume"] += bar["volume"]
            current["turnover"] += bar["turnover"]
        return [weekly[key] for key in sorted(weekly)]

    def _merge_realtime_daily_bar(
        self,
        symbol: str,
        bars: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        quote = self._realtime_quote_for_bar(symbol)
        today = self.now().strftime("%Y-%m-%d")
        if quote["last"] <= 0:
            return bars
        current = {
            "open_time": today,
            "open": quote["open"] or quote["last"],
            "high": quote["high"] or quote["last"],
            "low": quote["low"] or quote["last"],
            "close": quote["last"],
            "volume": quote["volume"],
            "turnover": quote["amount"],
        }
        if bars and bars[-1]["open_time"] == today:
            bars[-1] = current
        else:
            bars.append(current)
        return bars

    def _merge_realtime_weekly_bar(
        self,
        symbol: str,
        bars: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        quote = self._realtime_quote_for_bar(symbol)
        if quote["last"] <= 0:
            return bars
        week_start = (self.now() - timedelta(days=self.now().weekday())).strftime("%Y-%m-%d")
        current = {
            "open_time": week_start,
            "open": quote["open"] or quote["last"],
            "high": quote["high"] or quote["last"],
            "low": quote["low"] or quote["last"],
            "close": quote["last"],
            "volume": quote["volume"],
            "turnover": quote["amount"],
        }
        if bars and bars[-1]["open_time"] >= week_start:
            previous = bars[-1]
            current["open"] = previous["open"]
            current["high"] = max(previous["high"], current["high"])
            current["low"] = min(previous["low"], current["low"])
            current["volume"] = previous["volume"]
            current["turnover"] = previous["turnover"]
            bars[-1] = current
        else:
            bars.append(current)
        return bars

    def _realtime_quote_for_bar(self, symbol: str) -> dict[str, Any]:
        return self.current_quote(symbol)

    def _number(self, value: Any) -> float:
        if pd.isna(value):
            return 0
        try:
            return float(value)
        except (TypeError, ValueError):
            return 0

    def _date_string(self, value: Any) -> str:
        if hasattr(value, "strftime"):
            return value.strftime("%Y-%m-%d")
        return str(value)
