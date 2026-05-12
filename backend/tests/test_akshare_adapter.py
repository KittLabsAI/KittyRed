import unittest
from datetime import datetime
import time
import threading
from unittest.mock import Mock

import pandas as pd

from backend.akshare_adapter import AkshareAdapter
from backend.akshare_service import handle_request


class FakeAkshareClient:
    def __init__(self, failures_before_success=0):
        self.xq_calls = []
        self.xq_tokens = []
        self.basic_info_tokens = []
        self.xq_failures = set()
        self.failures_before_success = failures_before_success
        self.hist_failures_before_success = 0
        self.minute_failures_before_success = 0
        self.hist_min_calls = []
        self.sina_minute_calls = []
        self.financial_calls = []
        self.financial_failures = set()

    def stock_individual_spot_xq(self, symbol, token=None):
        self.xq_calls.append(symbol)
        self.xq_tokens.append(token)
        if symbol in self.xq_failures:
            raise ConnectionError("xueqiu disconnected")
        fixtures = {
            "SH600000": {
                "代码": "SH600000",
                "名称": "浦发银行",
                "现价": 9.16,
                "今开": 9.18,
                "最高": 9.25,
                "最低": 9.12,
                "涨幅": -0.22,
                "成交量": 26808998,
                "成交额": 245736322,
                "时间": "2026-05-07 11:19:19",
            },
            "SZ000001": {
                "代码": "SZ000001",
                "名称": "平安银行",
                "现价": 11.32,
                "今开": 11.35,
                "最高": 11.39,
                "最低": 11.28,
                "涨幅": -0.35,
                "成交量": 47518798,
                "成交额": 539031999,
                "时间": "2026-05-07 11:19:21",
            },
        }
        values = fixtures[symbol]
        return pd.DataFrame(
            [{"item": item, "value": value} for item, value in values.items()]
        )


class SlowXueqiuClient(FakeAkshareClient):
    def __init__(self, sleep_seconds=0.05):
        super().__init__()
        self.sleep_seconds = sleep_seconds
        self.active_calls = 0
        self.max_concurrent_calls = 0
        self.lock = threading.Lock()

    def stock_individual_spot_xq(self, symbol, token=None):
        with self.lock:
            self.active_calls += 1
            self.max_concurrent_calls = max(self.max_concurrent_calls, self.active_calls)
        try:
            time.sleep(self.sleep_seconds)
            return super().stock_individual_spot_xq(symbol, token=token)
        finally:
            with self.lock:
                self.active_calls -= 1

    def stock_individual_basic_info_xq(self, symbol, token=None):
        self.basic_info_call = symbol
        self.basic_info_tokens.append(token)
        return pd.DataFrame(
            [
                {"item": "org_name_cn", "value": "上海浦东发展银行股份有限公司"},
                {"item": "main_operation_business", "value": "商业银行业务"},
                {"item": "industry", "value": "银行"},
                {"item": "listed_date", "value": "1999-11-10"},
            ]
        )

    def stock_zh_a_hist_min_em(self, symbol, start_date, end_date, period, adjust):
        self.hist_min_calls.append(
            {
                "symbol": symbol,
                "start_date": start_date,
                "end_date": end_date,
                "period": period,
                "adjust": adjust,
            }
        )
        self.hist_min_call = {
            "symbol": symbol,
            "start_date": start_date,
            "end_date": end_date,
            "period": period,
            "adjust": adjust,
        }
        return pd.DataFrame(
            [
                {
                    "时间": "2026-05-06 10:00:00",
                    "开盘": 8.60,
                    "收盘": 8.72,
                    "最高": 8.75,
                    "最低": 8.58,
                    "成交量": 12000,
                    "成交额": 10464000,
                }
            ]
        )

    def stock_zh_a_hist_tx(self, symbol, start_date, end_date, adjust, timeout=None):
        self.hist_tx_call = {
            "symbol": symbol,
            "start_date": start_date,
            "end_date": end_date,
            "adjust": adjust,
            "timeout": timeout,
        }
        return pd.DataFrame(
            [
                {
                    "date": "2026-04-29",
                    "open": 8.4,
                    "close": 8.6,
                    "high": 8.7,
                    "low": 8.3,
                    "amount": 500000.0,
                },
                {
                    "date": "2026-04-30",
                    "open": 8.6,
                    "close": 8.8,
                    "high": 8.9,
                    "low": 8.5,
                    "amount": 600000.0,
                },
                {
                    "date": "2026-05-06",
                    "open": 8.9,
                    "close": 9.1,
                    "high": 9.2,
                    "low": 8.8,
                    "amount": 700000.0,
                },
                {
                    "date": "2026-05-07",
                    "open": 9.1,
                    "close": 9.2,
                    "high": 9.3,
                    "low": 9.0,
                    "amount": 800000.0,
                },
            ]
        )

    def stock_zh_a_minute(self, symbol, period, adjust):
        self.sina_minute_calls.append(
            {
                "symbol": symbol,
                "period": period,
                "adjust": adjust,
            }
        )
        if self.minute_failures_before_success:
            self.minute_failures_before_success -= 1
            raise ConnectionError("sina ssl disconnected")
        self.sina_minute_call = {
            "symbol": symbol,
            "period": period,
            "adjust": adjust,
        }
        return pd.DataFrame(
            [
                {
                    "day": "2026-05-07 11:30:00",
                    "open": "9.150",
                    "high": "9.160",
                    "low": "9.150",
                    "close": "9.150",
                    "volume": "167100",
                    "amount": "1530125.9910",
                }
            ]
        )

    def stock_zh_a_spot_em(self):
        return pd.DataFrame(
            [
                {
                    "代码": "600000",
                    "名称": "浦发银行",
                    "最新价": 8.72,
                    "今开": 8.70,
                    "最高": 8.80,
                    "最低": 8.60,
                    "涨跌幅": 1.25,
                    "成交量": 1000,
                    "成交额": 872000,
                    "更新时间": "2026-05-06 10:00:00",
                },
                {
                    "代码": "000001",
                    "名称": "平安银行",
                    "最新价": 12.34,
                    "今开": 12.10,
                    "最高": 12.50,
                    "最低": 12.00,
                    "涨跌幅": -0.64,
                    "成交量": 2000,
                    "成交额": 2468000,
                    "更新时间": "2026-05-06 10:01:00",
                },
            ]
        )

    def stock_zh_a_daily(self, symbol, adjust, start_date=None, end_date=None):
        self.daily_call = {
            "symbol": symbol,
            "adjust": adjust,
            "start_date": start_date,
            "end_date": end_date,
        }
        return pd.DataFrame(
            [
                {
                    "date": "2026-05-06",
                    "open": 9.27,
                    "high": 9.29,
                    "low": 9.16,
                    "close": 9.18,
                    "volume": 87935791,
                    "amount": 808632515,
                }
            ]
        )

    def stock_zh_a_hist(self, symbol, period, start_date, end_date, adjust, timeout=None):
        self.hist_call = {
            "symbol": symbol,
            "period": period,
            "start_date": start_date,
            "end_date": end_date,
            "adjust": adjust,
            "timeout": timeout,
        }
        if self.hist_failures_before_success:
            self.hist_failures_before_success -= 1
            raise ConnectionError("history disconnected")
        return pd.DataFrame(
            [
                {
                    "日期": "2026-05-05",
                    "开盘": 8.50,
                    "最高": 8.80,
                    "最低": 8.40,
                    "收盘": 8.72,
                    "成交量": 12000,
                    "成交额": 10464000,
                }
            ]
        )

    def stock_info_a_code_name(self):
        return pd.DataFrame(
            [
                {"code": "600000", "name": "浦发银行"},
                {"code": "000001", "name": "平安银行"},
            ]
        )

    def tool_trade_date_hist_sina(self):
        return pd.DataFrame(
            [
                {"trade_date": "2026-05-06"},
                {"trade_date": "2026-05-07"},
            ]
        )

    def _financial_rows(self, endpoint, date):
        self.financial_calls.append((endpoint, date))
        if endpoint in self.financial_failures:
            raise ConnectionError(f"{endpoint} disconnected")
        return pd.DataFrame(
            [
                {
                    "股票代码": "600000",
                    "股票简称": "浦发银行",
                    "报告期": date,
                    "营业收入": 100.5,
                    "净利润": 20.25,
                },
                {
                    "股票代码": "000001",
                    "股票简称": "平安银行",
                    "报告期": date,
                    "营业收入": 88.0,
                    "净利润": 16.0,
                },
            ]
        )

    def stock_yjbb_em(self, date):
        return self._financial_rows("stock_yjbb_em", date)

    def stock_yjkb_em(self, date):
        return self._financial_rows("stock_yjkb_em", date)

    def stock_yjyg_em(self, date):
        return self._financial_rows("stock_yjyg_em", date)

    def stock_zcfz_em(self, date):
        return self._financial_rows("stock_zcfz_em", date)

    def stock_lrb_em(self, date):
        return self._financial_rows("stock_lrb_em", date)

    def stock_xjll_em(self, date):
        return self._financial_rows("stock_xjll_em", date)


class SlowKlineClient(FakeAkshareClient):
    def __init__(self, sleep_seconds=0.08, timeout_periods=None):
        super().__init__()
        self.sleep_seconds = sleep_seconds
        self.timeout_periods = set(timeout_periods or [])

    def stock_zh_a_minute(self, symbol, period, adjust):
        if period in self.timeout_periods:
            time.sleep(self.sleep_seconds)
        return super().stock_zh_a_minute(symbol, period, adjust)

    def stock_zh_a_hist(self, symbol, period, start_date, end_date, adjust, timeout=None):
        if period in self.timeout_periods:
            time.sleep(self.sleep_seconds)
        return super().stock_zh_a_hist(symbol, period, start_date, end_date, adjust, timeout)


class AkshareAdapterTest(unittest.TestCase):
    def test_xueqiu_token_is_forwarded_to_all_xueqiu_endpoints(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, xueqiu_token="xq-token-test")

        adapter.current_quote("SHSE.600000")
        adapter.current_quotes(["SHSE.600000", "SZSE.000001"])
        adapter.stock_info("SHSE.600000")

        self.assertEqual(client.xq_tokens, ["xq-token-test", "xq-token-test", "xq-token-test"])
        self.assertEqual(client.basic_info_tokens, ["xq-token-test"])

    def test_current_quote_reads_xueqiu_realtime_snapshot(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client)

        quote = adapter.current_quote("SHSE.600000")

        self.assertEqual(client.xq_calls, ["SH600000"])
        self.assertEqual(
            quote,
            {
                "symbol": "SHSE.600000",
                "name": "浦发银行",
                "last": 9.16,
                "open": 9.18,
                "high": 9.25,
                "low": 9.12,
                "change_pct": -0.22,
                "volume": 26808998,
                "amount": 245736322,
                "updated_at": "2026-05-07 11:19:19",
                "source": "akshare:xueqiu",
            },
        )

    def test_current_quotes_uses_xueqiu_names_and_change_percent(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        quotes = adapter.current_quotes(["SHSE.600000", "SZSE.000001"])

        self.assertEqual(quotes[0]["name"], "浦发银行")
        self.assertEqual(quotes[0]["change_pct"], -0.22)
        self.assertEqual(quotes[1]["name"], "平安银行")
        self.assertEqual(quotes[1]["change_pct"], -0.35)
        self.assertEqual(quotes[0]["source"], "akshare:xueqiu")

    def test_stock_info_uses_xueqiu_basic_info(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client)

        info = adapter.stock_info("SHSE.600000")

        self.assertEqual(client.basic_info_call, "SH600000")
        self.assertEqual(info["stock_code"], "SHSE.600000")
        self.assertEqual(info["source"], "akshare:xueqiu_basic")
        self.assertEqual(
            info["items"]["org_name_cn"],
            "上海浦东发展银行股份有限公司",
        )

    def test_multi_frequency_bars_returns_assistant_kline_set(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        result = adapter.multi_frequency_bars("SHSE.600000", count=1)

        self.assertEqual(result["stock_code"], "SHSE.600000")
        self.assertEqual(set(result["bars"].keys()), {"5m", "1h", "1d", "1w"})
        self.assertEqual([call["period"] for call in client.sina_minute_calls], ["5", "60"])
        self.assertEqual(client.hist_min_calls, [])
        self.assertEqual(client.xq_calls, [])

    def test_multi_frequency_bars_requests_levels_in_parallel(self):
        client = SlowKlineClient(sleep_seconds=0.08, timeout_periods={"5", "60", "daily", "weekly"})
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        start = time.perf_counter()
        result = adapter.multi_frequency_bars("SHSE.600000", count=1, timeout_seconds=1)
        elapsed = time.perf_counter() - start

        self.assertLess(elapsed, 0.2)
        self.assertEqual(set(result["bars"].keys()), {"5m", "1h", "1d", "1w"})
        self.assertEqual(result["messages"], {})

    def test_multi_frequency_bars_returns_empty_level_on_timeout(self):
        client = SlowKlineClient(sleep_seconds=0.2, timeout_periods={"5"})
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        result = adapter.multi_frequency_bars("SHSE.600000", count=1, timeout_seconds=0.05)

        self.assertEqual(result["bars"]["5m"], [])
        self.assertIn("网络原因", result["messages"]["5m"])
        self.assertEqual(len(result["bars"]["1h"]), 1)

    def test_current_quotes_keeps_successful_xueqiu_rows_when_one_symbol_fails(self):
        client = FakeAkshareClient()
        client.xq_failures.add("SZ000001")
        adapter = AkshareAdapter(client)

        quotes = adapter.current_quotes(["SHSE.600000", "SZSE.000001"])

        self.assertEqual([quote["symbol"] for quote in quotes], ["SHSE.600000"])
        self.assertEqual(client.xq_calls, ["SH600000", "SZ000001"])

    def test_current_quotes_fetches_xueqiu_quotes_in_parallel_with_limit_ten(self):
        client = SlowXueqiuClient(sleep_seconds=0.05)
        adapter = AkshareAdapter(client)
        symbols = ["SHSE.600000"] * 6 + ["SZSE.000001"] * 6

        start = time.perf_counter()
        quotes = adapter.current_quotes(symbols)
        elapsed = time.perf_counter() - start

        self.assertEqual(len(quotes), 12)
        self.assertLess(elapsed, 0.25)
        self.assertGreaterEqual(client.max_concurrent_calls, 2)
        self.assertLessEqual(client.max_concurrent_calls, 10)

    def test_is_trade_date_uses_akshare_calendar(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        self.assertTrue(adapter.is_trade_date("2026-05-07"))
        self.assertFalse(adapter.is_trade_date("2026-05-08"))

    def test_current_quote_raises_when_xueqiu_realtime_snapshot_fails(self):
        client = FakeAkshareClient(failures_before_success=1)
        adapter = AkshareAdapter(client)
        client.xq_failures.add("SH600000")

        with self.assertRaises(ConnectionError):
            adapter.current_quote("SHSE.600000")

    def test_current_quote_does_not_use_daily_history_when_realtime_disconnects(self):
        client = FakeAkshareClient(failures_before_success=2)
        adapter = AkshareAdapter(client)
        client.xq_failures.add("SH600000")

        with self.assertRaises(ConnectionError):
            adapter.current_quote("SHSE.600000")

        self.assertFalse(hasattr(client, "daily_call"))

    def test_current_quote_reports_missing_xueqiu_data_field_as_connection_error(self):
        client = Mock()
        client.stock_individual_spot_xq.side_effect = KeyError("data")
        adapter = AkshareAdapter(client)

        with self.assertRaisesRegex(ConnectionError, "缺少 data 字段"):
            adapter.current_quote("SHSE.600000")

    def test_history_bars_uses_akshare_daily_history(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 6))

        bars = adapter.history_bars("SHSE.600000", "1d", 120)

        self.assertEqual(
            client.hist_call,
            {
                "symbol": "600000",
                "period": "daily",
                "start_date": "20251107",
                "end_date": "20260506",
                "adjust": "qfq",
                "timeout": 3,
            },
        )
        self.assertEqual(bars[0]["open_time"], "2026-05-05")
        self.assertEqual(bars[0]["volume"], 1200000)
        self.assertEqual(bars[0]["turnover"], 10464000)

    def test_history_bars_merges_realtime_bar_for_weekly_history(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "1w", 80)

        self.assertEqual(client.hist_call["period"], "weekly")
        self.assertEqual(bars[-1]["open_time"], "2026-05-04")
        self.assertEqual(bars[-1]["close"], 9.16)
        self.assertEqual(bars[-1]["volume"], 1200000)
        self.assertEqual(bars[-1]["turnover"], 10464000)

    def test_history_bars_uses_sina_minute_history_for_intraday_frequency(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 6, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "60m", 40)

        self.assertEqual(
            client.sina_minute_call,
            {"symbol": "sh600000", "period": "60", "adjust": ""},
        )
        self.assertFalse(hasattr(client, "hist_min_call"))
        self.assertEqual(bars[0]["open_time"], "2026-05-07 11:30:00")
        self.assertEqual(bars[0]["close"], 9.15)

    def test_handle_request_routes_intraday_history_to_eastmoney_when_configured(self):
        client = FakeAkshareClient()

        response = handle_request(
            {
                "action": "history_bars",
                "symbol": "SHSE.600000",
                "frequency": "60m",
                "count": 40,
                "intraday_data_source": "eastmoney",
            },
            adapter=AkshareAdapter(client, now=lambda: datetime(2026, 5, 6, 10, 10)),
        )

        self.assertTrue(response["ok"])
        self.assertEqual(client.hist_min_call["symbol"], "600000")
        self.assertEqual(client.hist_min_call["period"], "60")
        self.assertEqual(client.sina_minute_calls, [])

    def test_handle_request_routes_historical_daily_to_tencent_when_configured(self):
        client = FakeAkshareClient()

        response = handle_request(
            {
                "action": "history_bars",
                "symbol": "SHSE.600000",
                "frequency": "1d",
                "count": 120,
                "historical_data_source": "tencent",
            },
            adapter=AkshareAdapter(client, now=lambda: datetime(2026, 5, 6)),
        )

        self.assertTrue(response["ok"])
        self.assertEqual(
            client.hist_tx_call,
            {
                "symbol": "sh600000",
                "start_date": "20251107",
                "end_date": "20260506",
                "adjust": "qfq",
                "timeout": 3,
            },
        )
        self.assertFalse(hasattr(client, "hist_call"))

    def test_handle_request_aggregates_weekly_history_from_tencent_daily_bars(self):
        client = FakeAkshareClient()

        response = handle_request(
            {
                "action": "history_bars",
                "symbol": "SHSE.600000",
                "frequency": "1w",
                "count": 80,
                "historical_data_source": "tencent",
            },
            adapter=AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10)),
        )

        self.assertTrue(response["ok"])
        bars = response["data"]
        self.assertEqual([bar["open_time"] for bar in bars], ["2026-04-27", "2026-05-04"])
        self.assertEqual(bars[0]["open"], 8.4)
        self.assertEqual(bars[0]["close"], 8.8)
        self.assertEqual(bars[1]["high"], 9.3)

    def test_handle_request_aggregates_monthly_history_from_tencent_daily_bars(self):
        client = FakeAkshareClient()

        response = handle_request(
            {
                "action": "history_bars",
                "symbol": "SHSE.600000",
                "frequency": "1M",
                "count": 24,
                "historical_data_source": "tencent",
            },
            adapter=AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10)),
        )

        self.assertTrue(response["ok"])
        bars = response["data"]
        self.assertEqual([bar["open_time"] for bar in bars], ["2026-04-01", "2026-05-01"])
        self.assertEqual(bars[0]["open"], 8.4)
        self.assertEqual(bars[0]["close"], 8.8)
        self.assertEqual(bars[1]["close"], 9.2)

    def test_history_bars_filters_intraday_rows_by_requested_date_range(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 11, 10, 10))
        client.stock_zh_a_minute = lambda symbol, period, adjust: pd.DataFrame(
            [
                {
                    "day": "2026-04-01 09:30:00",
                    "open": "8.100",
                    "high": "8.200",
                    "low": "8.000",
                    "close": "8.150",
                    "volume": "100",
                    "amount": "815",
                },
                {
                    "day": "2026-04-30 15:00:00",
                    "open": "8.500",
                    "high": "8.600",
                    "low": "8.400",
                    "close": "8.550",
                    "volume": "100",
                    "amount": "855",
                },
                {
                    "day": "2026-05-06 09:30:00",
                    "open": "8.900",
                    "high": "9.000",
                    "low": "8.800",
                    "close": "8.950",
                    "volume": "100",
                    "amount": "895",
                },
            ]
        )

        bars = adapter.history_bars(
            "SHSE.600000",
            "5m",
            900,
            start_date="2026-04-01",
            end_date="2026-04-30",
        )

        self.assertEqual(
            [bar["open_time"] for bar in bars],
            ["2026-04-01 09:30:00", "2026-04-30 15:00:00"],
        )

    def test_history_bars_retries_sina_minute_history_once(self):
        client = FakeAkshareClient()
        client.minute_failures_before_success = 1
        adapter = AkshareAdapter(
            client,
            now=lambda: datetime(2026, 5, 6, 10, 10),
            history_attempts=2,
            history_retry_backoff_seconds=0,
        )

        bars = adapter.history_bars("SHSE.600000", "5m", 40)

        self.assertEqual(len(client.sina_minute_calls), 2)
        self.assertEqual(bars[0]["close"], 9.15)

    def test_history_bars_reports_sina_minute_failure_after_retry_limit(self):
        client = FakeAkshareClient()
        client.minute_failures_before_success = 2
        adapter = AkshareAdapter(
            client,
            now=lambda: datetime(2026, 5, 6, 10, 10),
            history_attempts=2,
            history_retry_backoff_seconds=0,
        )

        with self.assertRaises(ConnectionError):
            adapter.history_bars("SHSE.600000", "5m", 40)

        self.assertEqual(len(client.sina_minute_calls), 2)

    def test_history_bars_fallback_daily_merges_realtime_bar_when_hist_disconnects(self):
        client = FakeAkshareClient()
        client.hist_failures_before_success = 1
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "1d", 120)

        self.assertEqual(client.daily_call["symbol"], "sh600000")
        self.assertEqual(client.daily_call["adjust"], "qfq")
        self.assertEqual(bars[-1]["open_time"], "2026-05-07")
        self.assertEqual(bars[-1]["close"], 9.16)

    def test_history_bars_fallback_weekly_merges_current_week_when_hist_disconnects(self):
        client = FakeAkshareClient()
        client.hist_failures_before_success = 1
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "1w", 80)

        self.assertEqual(client.daily_call["symbol"], "sh600000")
        self.assertEqual(client.daily_call["adjust"], "qfq")
        self.assertEqual(bars[-1]["open_time"], "2026-05-04")
        self.assertEqual(bars[-1]["close"], 9.16)

    def test_search_stocks_uses_akshare_stock_list(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        results = adapter.search_stocks("浦发")

        self.assertEqual(
            results,
            [{"symbol": "SHSE.600000", "name": "浦发银行", "market": "沪市A股"}],
        )

    def test_handle_request_returns_akshare_quote(self):
        adapter = AkshareAdapter(FakeAkshareClient())
        response = handle_request(
            {"action": "current_quote", "symbol": "SZSE.000001"},
            adapter=adapter,
        )

        self.assertEqual(response["ok"], True)
        self.assertEqual(response["data"]["symbol"], "SZSE.000001")
        self.assertEqual(response["data"]["source"], "akshare:xueqiu")

    def test_submit_order_is_not_a_data_source_capability(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        with self.assertRaises(NotImplementedError):
            adapter.submit_order("SHSE.600000", "buy", 100)

    def test_financial_reports_fetches_all_required_akshare_sections_for_all_stocks(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 8))

        result = adapter.financial_reports(years=2)

        self.assertEqual(result["stock_code"], "ALL")
        self.assertEqual(result["years"], 2)
        self.assertEqual(
            {section["section"] for section in result["sections"]},
            {
                "performance_report",
                "performance_express",
                "performance_forecast",
                "balance_sheet",
                "income_statement",
                "cash_flow_statement",
            },
        )
        self.assertTrue(
            all(section["source"].startswith("akshare:") for section in result["sections"])
        )
        self.assertTrue(all(section["rows"] for section in result["sections"]))
        self.assertNotIn(("stock_yjbb_em", "20260630"), client.financial_calls)
        self.assertIn(("stock_yjbb_em", "20260331"), client.financial_calls)
        self.assertIn(("stock_xjll_em", "20240630"), client.financial_calls)
        self.assertNotIn(("stock_yjbb_em", "20240331"), client.financial_calls)
        stock_codes = {row["stock_code"] for row in result["sections"][0]["rows"]}
        self.assertIn("SHSE.600000", stock_codes)
        self.assertIn("SZSE.000001", stock_codes)

    def test_financial_reports_filters_to_latest_two_year_report_window(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 8))

        result = adapter.financial_reports(years=2)
        rows = result["sections"][0]["rows"]

        self.assertNotIn("2024-03-31", {row["report_date"] for row in rows})
        self.assertIn("2024-06-30", {row["report_date"] for row in rows})

    def test_financial_reports_keeps_section_error_without_dropping_successes(self):
        client = FakeAkshareClient()
        client.financial_failures.add("stock_lrb_em")
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 8))

        result = adapter.financial_reports(years=2)
        income = next(
            section for section in result["sections"] if section["section"] == "income_statement"
        )
        balance = next(
            section for section in result["sections"] if section["section"] == "balance_sheet"
        )

        self.assertEqual(income["rows"], [])
        self.assertIn("读取失败", income["error"])
        self.assertTrue(balance["rows"])

    def test_handle_request_returns_financial_reports(self):
        response = handle_request(
            {"action": "financial_reports", "years": 2},
            adapter=AkshareAdapter(FakeAkshareClient(), now=lambda: datetime(2026, 5, 8)),
        )

        self.assertEqual(response["ok"], True)
        self.assertEqual(response["data"]["stock_code"], "ALL")
        self.assertEqual(len(response["data"]["sections"]), 6)

    def test_handle_request_returns_financial_report_probe_from_stock_yjkb(self):
        client = SlowXueqiuClient()

        response = handle_request(
            {"action": "financial_report_probe"},
            adapter=AkshareAdapter(client, now=lambda: datetime(2026, 5, 8)),
        )

        self.assertEqual(response["ok"], True)
        self.assertEqual(response["data"]["endpoint"], "stock_yjkb_em")
        self.assertEqual(response["data"]["source"], "akshare:stock_yjkb_em")
        self.assertTrue(response["data"]["row_count"] > 0)
        self.assertEqual(client.financial_calls, [("stock_yjkb_em", "20260331")])


if __name__ == "__main__":
    unittest.main()
