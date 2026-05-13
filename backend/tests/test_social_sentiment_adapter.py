import unittest

from backend.social_sentiment_adapter import (
    SUPPORTED_PLATFORMS,
    SocialSentimentAdapter,
    _xueqiu_query,
    normalize_discussion_item,
    normalize_discussion_items,
    validate_login_state,
)
from backend.social_sentiment_service import handle_request


class FakeSocialClient:
    def fetch_platform(self, platform, stock_code, stock_name, limit):
        if platform == "xueqiu":
            raise RuntimeError("需要登录态")
        return [
            {
                "title": f"{stock_name} 舆情",
                "text": f"{stock_name} 在 {platform} 的真实搜索结果",
                "author": "测试用户",
                "url": f"https://example.com/{platform}",
            }
        ]


class RecentDaysClient:
    def __init__(self):
        self.calls = []

    def fetch_platform(self, platform, stock_code, stock_name, limit):
        self.calls.append((platform, stock_code, stock_name, limit))
        return [{"title": "讨论", "text": "讨论正文"}]


class SocialSentimentAdapterTest(unittest.TestCase):
    def test_supported_platforms_match_last30days_cn_coverage(self):
        self.assertEqual(
            SUPPORTED_PLATFORMS,
            (
                "weibo",
                "xiaohongshu",
                "bilibili",
                "zhihu",
                "douyin",
                "wechat",
                "baidu",
                "toutiao",
                "xueqiu",
            ),
        )

    def test_normalize_discussion_item_preserves_source_metadata(self):
        item = normalize_discussion_item(
            "zhihu",
            {
                "title": "浦发银行怎么看？",
                "text": "讨论正文",
                "author_name": "用户A",
                "date": "2026-05-12",
                "url": "https://example.com/zhihu",
                "engagement": {"likes": 12},
            },
            fetched_at="2026-05-12T10:00:00+00:00",
        )

        self.assertEqual(item["platform"], "zhihu")
        self.assertEqual(item["title"], "浦发银行怎么看？")
        self.assertEqual(item["text"], "讨论正文")
        self.assertEqual(item["author"], "用户A")
        self.assertEqual(item["publishedAt"], "2026-05-12")
        self.assertEqual(item["url"], "https://example.com/zhihu")
        self.assertEqual(item["engagement"], {"likes": 12})
        self.assertEqual(item["raw"]["title"], "浦发银行怎么看？")

    def test_probe_platforms_returns_mixed_platform_rows(self):
        adapter = SocialSentimentAdapter(client=FakeSocialClient())

        result = adapter.probe_platforms(["zhihu", "xueqiu"])

        self.assertEqual([row["platform"] for row in result], ["zhihu", "xueqiu"])
        self.assertTrue(result[0]["ok"])
        self.assertFalse(result[1]["ok"])
        self.assertIn("真实搜索成功", result[0]["message"])
        self.assertIn("真实搜索失败", result[1]["message"])

    def test_login_state_validation_requires_real_platform_cookie(self):
        self.assertIsNone(
            validate_login_state(
                "xueqiu",
                {
                    "cookies": [
                        {
                            "domain": ".xueqiu.com",
                            "name": "xq_a_token",
                            "value": "secret-token",
                        }
                    ]
                },
            )
        )
        self.assertIn(
            "必需 Cookie",
            validate_login_state(
                "xueqiu",
                {"cookies": [{"domain": ".xueqiu.com", "name": "u", "value": "1"}]},
            ),
        )

    def test_xueqiu_query_uses_a_share_market_symbol_for_cdp_search(self):
        self.assertEqual(_xueqiu_query("SHSE.600000", "浦发银行"), "SH600000")
        self.assertEqual(_xueqiu_query("SZSE.000001", "平安银行"), "SZ000001")
        self.assertEqual(_xueqiu_query("", "浦发银行"), "浦发银行")

    def test_normalize_discussion_items_skips_empty_rows_without_failing_stock_fetch(self):
        rows = normalize_discussion_items(
            "bilibili",
            [
                {"title": "", "description": ""},
                {"title": "浦发银行视频", "description": ""},
            ],
            "2026-05-12T10:00:00+00:00",
        )

        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["text"], "浦发银行视频")

    def test_service_fetch_discussions_returns_normalized_json_shape(self):
        adapter = SocialSentimentAdapter(client=FakeSocialClient())
        response = handle_request(
            {
                "action": "fetch_discussions",
                "stock_code": "SHSE.600000",
                "stock_name": "浦发银行",
                "platforms": ["zhihu"],
            },
            adapter=adapter,
        )

        self.assertTrue(response["ok"])
        data = response["data"]
        self.assertEqual(data["stockCode"], "SHSE.600000")
        self.assertEqual(data["stockName"], "浦发银行")
        self.assertEqual(data["platformStatuses"][0]["platform"], "zhihu")
        self.assertEqual(data["items"][0]["platform"], "zhihu")
        self.assertIn("浦发银行", data["items"][0]["text"])

    def test_service_fetch_discussions_accepts_recent_days(self):
        adapter = SocialSentimentAdapter(client=RecentDaysClient())
        response = handle_request(
            {
                "action": "fetch_discussions",
                "stock_code": "SHSE.600000",
                "stock_name": "浦发银行",
                "platforms": ["zhihu"],
                "recent_days": 7,
            },
            adapter=adapter,
        )

        self.assertTrue(response["ok"])
        self.assertEqual(adapter.client.calls[0][0], "zhihu")


if __name__ == "__main__":
    unittest.main()
