from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta
import json
import queue
import subprocess
import sys
import threading
import time
from typing import Any, Callable

import pandas as pd
import requests


class AkshareAdapter:
    def __init__(
        self,
        client: Any,
        now: Callable[[], datetime] | None = None,
        quote_attempts: int = 2,
        history_attempts: int = 3,
        history_retry_backoff_seconds: float = 0.25,
        xueqiu_token: str = "",
        intraday_data_source: str = "sina",
        historical_data_source: str = "eastmoney",
    ):
        self.client = client
        self.now = now or datetime.now
        self.quote_attempts = max(1, quote_attempts)
        self.history_attempts = max(1, history_attempts)
        self.history_retry_backoff_seconds = max(0, history_retry_backoff_seconds)
        self.xueqiu_token = xueqiu_token.strip()
        self.intraday_data_source = self._normalize_intraday_data_source(intraday_data_source)
        self.historical_data_source = self._normalize_historical_data_source(
            historical_data_source
        )
    def current_quote(self, symbol: str) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return self._empty_quote(symbol)

        return self._xueqiu_quote(clean_symbol)

    def current_quotes(self, symbols: list[str]) -> list[dict[str, Any]]:
        clean_symbols = [self._normalize_symbol(symbol) for symbol in symbols]
        clean_symbols = [symbol for symbol in clean_symbols if symbol]
        if not clean_symbols:
            return []

        max_workers = min(10, len(clean_symbols))
        last_error = None
        quotes = []
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [
                executor.submit(self._xueqiu_quote, symbol)
                for symbol in clean_symbols
            ]
            for future in futures:
                try:
                    quotes.append(future.result())
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
        start_date: str | None = None,
        end_date: str | None = None,
    ) -> list[dict[str, Any]]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return []

        end = self.now()
        start = end - timedelta(days=max(count, 1) + 60)
        if self._is_intraday_frequency(frequency):
            rows = self._retry_history_request(
                lambda: self._load_intraday_rows(
                    clean_symbol, frequency, start_date, end_date
                )
            )
            bars = self._intraday_rows_to_bars(rows)
            bars = self._filter_bars_by_date(bars, start_date, end_date)
            return bars[-max(count, 1) :]

        bars = self._load_historical_bars(
            clean_symbol,
            frequency,
            (start_date or start.strftime("%Y%m%d")).replace("-", ""),
            (end_date or end.strftime("%Y%m%d")).replace("-", ""),
        )
        bars = self._filter_bars_by_date(bars, start_date, end_date)
        bars = bars[-max(count, 1) :]
        if merge_realtime:
            bars = self._merge_realtime_bars(clean_symbol, frequency, bars)
            return self._filter_bars_by_date(bars, start_date, end_date)
        return bars

    def stock_info(self, symbol: str) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol)
        if not clean_symbol:
            return {"stock_code": symbol, "items": {}, "source": "akshare:xueqiu_basic"}

        rows = self.client.stock_individual_basic_info_xq(
            symbol=self._xueqiu_symbol(clean_symbol),
            token=self.xueqiu_token or None,
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

    def financial_reports(self, symbol: str = "", years: int = 2) -> dict[str, Any]:
        clean_symbol = self._normalize_symbol(symbol) if symbol.strip() else ""
        years = max(1, int(years or 2))
        cutoff = self.now() - timedelta(days=365 * years)
        report_dates = self._financial_report_dates(years)
        sections = []
        for section, label, endpoint in self._financial_report_specs():
            rows: list[dict[str, Any]] = []
            errors: list[str] = []
            for report_date in report_dates:
                try:
                    frame = getattr(self.client, endpoint)(date=report_date)
                except TypeError:
                    try:
                        frame = getattr(self.client, endpoint)(report_date)
                    except Exception as error:
                        errors.append(str(error))
                        continue
                except Exception as error:
                    errors.append(str(error))
                    continue
                rows.extend(self._financial_rows(frame, clean_symbol, cutoff))
            rows = self._dedupe_financial_rows(rows)
            sections.append(
                {
                    "section": section,
                    "label": label,
                    "source": f"akshare:{endpoint}",
                    "rows": rows,
                    "error": f"{label}读取失败：{'; '.join(errors)}" if errors and not rows else "",
                }
            )
        return {
            "stock_code": clean_symbol or "ALL",
            "years": years,
            "sections": sections,
            "source": "akshare",
            "fetched_at": self.now().isoformat(),
        }

    def financial_report_probe(self) -> dict[str, Any]:
        report_dates = self._financial_report_dates(1)
        if not report_dates:
            raise ValueError("未找到可用财报报告期")
        report_date = report_dates[0]
        frame = self.client.stock_yjkb_em(date=report_date)
        return {
            "endpoint": "stock_yjkb_em",
            "report_date": report_date,
            "row_count": int(len(frame.index)),
            "source": "akshare:stock_yjkb_em",
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
            "merge_realtime=sys.argv[4] == '1'; xueqiu_token=sys.argv[5]; intraday_data_source=sys.argv[6]; historical_data_source=sys.argv[7]; "
            "bars=AkshareAdapter(ak, xueqiu_token=xueqiu_token, intraday_data_source=intraday_data_source, historical_data_source=historical_data_source).history_bars(symbol, frequency, count, merge_realtime=merge_realtime); "
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
                    self.xueqiu_token,
                    self.intraday_data_source,
                    self.historical_data_source,
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

    def _retry_history_request(self, load: Callable[[], Any]) -> Any:
        last_error: Exception | None = None
        for attempt in range(self.history_attempts):
            try:
                return load()
            except Exception as error:
                last_error = error
                if attempt + 1 < self.history_attempts and self.history_retry_backoff_seconds:
                    time.sleep(self.history_retry_backoff_seconds * (attempt + 1))
        if last_error is not None:
            raise last_error
        return load()

    def _load_intraday_rows(
        self,
        symbol: str,
        frequency: str,
        start_date: str | None,
        end_date: str | None,
    ) -> pd.DataFrame:
        period = self._akshare_minute_period(frequency)
        if self.intraday_data_source == "eastmoney":
            return self.client.stock_zh_a_hist_min_em(
                symbol=self._akshare_code(symbol),
                start_date=(start_date or "1979-09-01 09:32:00"),
                end_date=(end_date or "2222-01-01 09:32:00"),
                period=period,
                adjust="",
            )
        return self.client.stock_zh_a_minute(
            symbol=self._akshare_daily_symbol(symbol),
            period=period,
            adjust="",
        )

    def _load_historical_bars(
        self,
        symbol: str,
        frequency: str,
        start_date: str,
        end_date: str,
    ) -> list[dict[str, Any]]:
        normalized = frequency.lower()
        if self.historical_data_source == "eastmoney":
            try:
                rows = self.client.stock_zh_a_hist(
                    symbol=self._akshare_code(symbol),
                    period=self._akshare_period(frequency),
                    start_date=start_date,
                    end_date=end_date,
                    adjust="qfq",
                    timeout=3,
                )
                return self._hist_rows_to_bars(rows, "日期")
            except Exception:
                daily_rows = self.client.stock_zh_a_daily(
                    symbol=self._akshare_daily_symbol(symbol),
                    start_date=start_date,
                    end_date=end_date,
                    adjust="qfq",
                )
                bars = self._daily_rows_to_bars(symbol, daily_rows)
                if normalized in {"1w", "weekly"}:
                    return self._daily_bars_to_weekly_bars(bars)
                if normalized in {"1m", "monthly"}:
                    return self._daily_bars_to_monthly_bars(bars)
                return bars

        daily_rows = self._load_daily_rows_from_selected_source(symbol, start_date, end_date)
        bars = self._selected_daily_rows_to_bars(symbol, daily_rows)
        if normalized in {"1w", "weekly"}:
            return self._daily_bars_to_weekly_bars(bars)
        if normalized in {"1m", "monthly"}:
            return self._daily_bars_to_monthly_bars(bars)
        return bars

    def _load_daily_rows_from_selected_source(
        self,
        symbol: str,
        start_date: str,
        end_date: str,
    ) -> pd.DataFrame:
        if self.historical_data_source == "tencent":
            return self.client.stock_zh_a_hist_tx(
                symbol=self._akshare_daily_symbol(symbol),
                start_date=start_date,
                end_date=end_date,
                adjust="qfq",
                timeout=3,
            )
        return self.client.stock_zh_a_daily(
            symbol=self._akshare_daily_symbol(symbol),
            start_date=start_date,
            end_date=end_date,
            adjust="qfq",
        )

    def _selected_daily_rows_to_bars(
        self,
        symbol: str,
        rows: pd.DataFrame,
    ) -> list[dict[str, Any]]:
        if self.historical_data_source == "tencent":
            return self._tencent_daily_rows_to_bars(rows)
        return self._daily_rows_to_bars(symbol, rows)

    def _merge_realtime_bars(
        self,
        symbol: str,
        frequency: str,
        bars: list[dict[str, Any]],
    ) -> list[dict[str, Any]]:
        try:
            if frequency.lower() in {"1d", "daily"}:
                bars = self._merge_realtime_daily_bar(symbol, bars)
            if frequency.lower() in {"1w", "weekly"}:
                bars = self._merge_realtime_weekly_bar(symbol, bars)
        except Exception:
            return bars
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

    def _financial_report_specs(self) -> list[tuple[str, str, str]]:
        return [
            ("performance_report", "业绩报表", "stock_yjbb_em"),
            ("performance_express", "业绩快报", "stock_yjkb_em"),
            ("performance_forecast", "业绩预告", "stock_yjyg_em"),
            ("balance_sheet", "资产负债表", "stock_zcfz_em"),
            ("income_statement", "利润表", "stock_lrb_em"),
            ("cash_flow_statement", "现金流量表", "stock_xjll_em"),
        ]

    def _financial_report_dates(self, years: int) -> list[str]:
        now = self.now()
        cutoff = now - timedelta(days=365 * max(1, int(years or 2)))
        current_year = now.year
        dates = []
        for year in range(current_year, current_year - years - 1, -1):
            for suffix in ("1231", "0930", "0630", "0331"):
                report_date = datetime.strptime(f"{year}{suffix}", "%Y%m%d")
                if cutoff <= report_date <= now:
                    dates.append(f"{year}{suffix}")
        return dates

    def _financial_rows(
        self,
        frame: pd.DataFrame,
        symbol: str,
        cutoff: datetime,
    ) -> list[dict[str, Any]]:
        code = self._akshare_code(symbol)
        rows = []
        for _, row in frame.iterrows():
            raw = {
                str(key): self._json_safe_value(value)
                for key, value in row.to_dict().items()
            }
            row_code = self._financial_row_code(raw)
            if symbol and row_code and row_code != code:
                continue
            report_date = self._financial_report_date(raw)
            if report_date:
                try:
                    parsed_date = datetime.strptime(report_date, "%Y-%m-%d")
                    if parsed_date < cutoff:
                        continue
                except ValueError:
                    pass
            rows.append(
                {
                    "stock_code": self._internal_symbol(row_code) if row_code else symbol,
                    "report_date": report_date,
                    "stock_name": str(raw.get("股票简称", raw.get("名称", raw.get("SECURITY_NAME_ABBR", "")))).strip(),
                    "raw": raw,
                }
            )
        return rows

    def _dedupe_financial_rows(self, rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
        seen = set()
        deduped = []
        for row in rows:
            key = json.dumps(row.get("raw", {}), ensure_ascii=False, sort_keys=True)
            if key in seen:
                continue
            seen.add(key)
            deduped.append(row)
        return sorted(deduped, key=lambda item: item.get("report_date") or "", reverse=True)

    def _financial_row_code(self, raw: dict[str, Any]) -> str:
        for key in ("股票代码", "代码", "SECURITY_CODE", "股票代码".encode().decode()):
            value = str(raw.get(key, "")).strip()
            digits = "".join(ch for ch in value if ch.isdigit())
            if digits:
                return digits.zfill(6)
        return ""

    def _financial_report_date(self, raw: dict[str, Any]) -> str:
        for key in ("报告期", "公告日期", "REPORT_DATE", "报告日期", "日期"):
            value = raw.get(key)
            if value in (None, ""):
                continue
            text = self._date_string(value)
            digits = "".join(ch for ch in text if ch.isdigit())
            if len(digits) >= 8:
                return f"{digits[:4]}-{digits[4:6]}-{digits[6:8]}"
            return text[:10]
        return ""

    def _json_safe_value(self, value: Any) -> Any:
        if pd.isna(value):
            return None
        if hasattr(value, "item"):
            try:
                return value.item()
            except Exception:
                pass
        if hasattr(value, "isoformat"):
            return value.isoformat()
        return value

    def _xueqiu_quote(self, symbol: str) -> dict[str, Any]:
        try:
            rows = self.client.stock_individual_spot_xq(
                symbol=self._xueqiu_symbol(symbol),
                token=self.xueqiu_token or None,
            )
        except KeyError as error:
            raise ConnectionError("雪球实时行情返回缺少 data 字段，可能是 token 无效或响应结构变更") from error
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
        cleaned = frequency.strip()
        return cleaned == "1m" or cleaned.lower() in {"5m", "15m", "30m", "60m", "1h"}

    def _normalize_intraday_data_source(self, source: str) -> str:
        return "eastmoney" if source.strip().lower() == "eastmoney" else "sina"

    def _normalize_historical_data_source(self, source: str) -> str:
        normalized = source.strip().lower()
        if normalized in {"sina", "tencent"}:
            return normalized
        return "eastmoney"

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

    def _eastmoney_minute_rows_to_bars(self, rows: pd.DataFrame) -> list[dict[str, Any]]:
        return [
            {
                "open_time": str(row.get("时间", "")),
                "open": self._number(row.get("开盘", 0)),
                "high": self._number(row.get("最高", 0)),
                "low": self._number(row.get("最低", 0)),
                "close": self._number(row.get("收盘", 0)),
                "volume": self._number(row.get("成交量", 0)),
                "turnover": self._number(row.get("成交额", 0)),
            }
            for _, row in rows.iterrows()
        ]

    def _intraday_rows_to_bars(self, rows: pd.DataFrame) -> list[dict[str, Any]]:
        if self.intraday_data_source == "eastmoney":
            return self._eastmoney_minute_rows_to_bars(rows)
        return self._sina_minute_rows_to_bars(rows)

    def _tencent_daily_rows_to_bars(self, rows: pd.DataFrame) -> list[dict[str, Any]]:
        return [
            {
                "open_time": self._date_string(row.get("date", "")),
                "open": self._number(row.get("open", 0)),
                "high": self._number(row.get("high", 0)),
                "low": self._number(row.get("low", 0)),
                "close": self._number(row.get("close", 0)),
                "volume": self._number(row.get("amount", 0)),
                "turnover": None,
            }
            for _, row in rows.iterrows()
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
            current["turnover"] = self._sum_optional_numbers(
                current.get("turnover"),
                bar.get("turnover"),
            )
        return [weekly[key] for key in sorted(weekly)]

    def _daily_bars_to_monthly_bars(self, bars: list[dict[str, Any]]) -> list[dict[str, Any]]:
        monthly: dict[str, dict[str, Any]] = {}
        for bar in bars:
            try:
                open_date = datetime.strptime(str(bar["open_time"]), "%Y-%m-%d")
            except ValueError:
                continue
            month_start = open_date.replace(day=1).strftime("%Y-%m-%d")
            current = monthly.get(month_start)
            if current is None:
                monthly[month_start] = {
                    "open_time": month_start,
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
            current["turnover"] = self._sum_optional_numbers(
                current.get("turnover"),
                bar.get("turnover"),
            )
        return [monthly[key] for key in sorted(monthly)]

    def _sum_optional_numbers(self, left: Any, right: Any) -> float | None:
        values = [value for value in (left, right) if value is not None]
        if not values:
            return None
        return sum(self._number(value) for value in values)

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

    def _filter_bars_by_date(
        self,
        bars: list[dict[str, Any]],
        start_date: str | None,
        end_date: str | None,
    ) -> list[dict[str, Any]]:
        if not start_date and not end_date:
            return bars

        start = (start_date or "")[:10]
        end = (end_date or "")[:10]
        filtered = []
        for bar in bars:
            open_time = self._date_string(bar.get("open_time", ""))
            bar_date = open_time[:10]
            if start and bar_date < start:
                continue
            if end and bar_date > end:
                continue
            filtered.append(bar)
        return filtered

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
