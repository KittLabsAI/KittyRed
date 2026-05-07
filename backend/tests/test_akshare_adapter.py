import unittest
from datetime import datetime
import time

import pandas as pd

from backend.akshare_adapter import AkshareAdapter
from backend.akshare_service import handle_request


class FakeAkshareClient:
    def __init__(self, failures_before_success=0):
        self.bid_ask_calls = []
        self.xq_calls = []
        self.xq_failures = set()
        self.failures_before_success = failures_before_success
        self.hist_failures_before_success = 0
        self.hist_min_calls = []
        self.sina_minute_calls = []

    def stock_individual_spot_xq(self, symbol):
        self.xq_calls.append(symbol)
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

    def stock_individual_basic_info_xq(self, symbol):
        self.basic_info_call = symbol
        return pd.DataFrame(
            [
                {"item": "org_name_cn", "value": "上海浦东发展银行股份有限公司"},
                {"item": "main_operation_business", "value": "商业银行业务"},
                {"item": "industry", "value": "银行"},
                {"item": "listed_date", "value": "1999-11-10"},
            ]
        )

    def stock_bid_ask_em(self, symbol):
        self.bid_ask_calls.append(symbol)
        if self.failures_before_success:
            self.failures_before_success -= 1
            raise ConnectionError("remote disconnected")
        return pd.DataFrame(
            [
                {"item": "最新", "value": 8.72},
                {"item": "今开", "value": 8.70},
                {"item": "最高", "value": 8.80},
                {"item": "最低", "value": 8.60},
                {"item": "总手", "value": 1000},
                {"item": "金额", "value": 872000},
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

    def stock_zh_a_minute(self, symbol, period, adjust):
        self.sina_minute_calls.append(
            {
                "symbol": symbol,
                "period": period,
                "adjust": adjust,
            }
        )
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

    def stock_zh_a_daily(self, symbol, adjust):
        self.daily_call = {"symbol": symbol, "adjust": adjust}
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
    def test_current_quote_reads_xueqiu_realtime_snapshot(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client)

        quote = adapter.current_quote("SHSE.600000")

        self.assertEqual(client.xq_calls, ["SH600000"])
        self.assertEqual(client.bid_ask_calls, [])
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

    def test_bid_ask_uses_eastmoney_quote_depth(self):
        client = FakeAkshareClient()
        adapter = AkshareAdapter(client)

        quote = adapter.bid_ask("SHSE.600000")

        self.assertEqual(client.bid_ask_calls, ["600000"])
        self.assertEqual(quote["stock_code"], "SHSE.600000")
        self.assertEqual(quote["last_price"], 8.72)
        self.assertEqual(quote["ask_levels"], [])

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

    def test_is_trade_date_uses_akshare_calendar(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        self.assertTrue(adapter.is_trade_date("2026-05-07"))
        self.assertFalse(adapter.is_trade_date("2026-05-08"))

    def test_current_quote_retries_once_when_akshare_disconnects(self):
        client = FakeAkshareClient(failures_before_success=1)
        client.xq_failures.add("SH600000")
        adapter = AkshareAdapter(client)

        quote = adapter.current_quote("SHSE.600000")

        self.assertEqual(client.bid_ask_calls, ["600000", "600000"])
        self.assertEqual(quote["last"], 8.72)

    def test_current_quote_does_not_use_daily_history_when_realtime_disconnects(self):
        client = FakeAkshareClient(failures_before_success=2)
        client.xq_failures.add("SH600000")
        adapter = AkshareAdapter(client)

        with self.assertRaises(ConnectionError):
            adapter.current_quote("SHSE.600000")

        self.assertFalse(hasattr(client, "daily_call"))

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

    def test_history_bars_fallback_daily_merges_realtime_bar_when_hist_disconnects(self):
        client = FakeAkshareClient()
        client.hist_failures_before_success = 1
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "1d", 120)

        self.assertEqual(client.daily_call, {"symbol": "sh600000", "adjust": ""})
        self.assertEqual(bars[-1]["open_time"], "2026-05-07")
        self.assertEqual(bars[-1]["close"], 9.16)

    def test_history_bars_fallback_weekly_merges_current_week_when_hist_disconnects(self):
        client = FakeAkshareClient()
        client.hist_failures_before_success = 1
        adapter = AkshareAdapter(client, now=lambda: datetime(2026, 5, 7, 10, 10))

        bars = adapter.history_bars("SHSE.600000", "1w", 80)

        self.assertEqual(client.daily_call, {"symbol": "sh600000", "adjust": ""})
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
        response = handle_request(
            {"action": "current_quote", "symbol": "SZSE.000001"},
            adapter=AkshareAdapter(FakeAkshareClient()),
        )

        self.assertEqual(response["ok"], True)
        self.assertEqual(response["data"]["symbol"], "SZSE.000001")
        self.assertEqual(response["data"]["source"], "akshare:xueqiu")

    def test_submit_order_is_not_a_data_source_capability(self):
        adapter = AkshareAdapter(FakeAkshareClient())

        with self.assertRaises(NotImplementedError):
            adapter.submit_order("SHSE.600000", "buy", 100)


if __name__ == "__main__":
    unittest.main()
