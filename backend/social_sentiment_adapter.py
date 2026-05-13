from __future__ import annotations

from datetime import datetime, timedelta, timezone
import importlib
import json
import os
from pathlib import Path
import re
import sys
from typing import Any
import urllib.parse
import urllib.request


SUPPORTED_PLATFORMS: tuple[str, ...] = (
    "weibo",
    "xiaohongshu",
    "bilibili",
    "zhihu",
    "douyin",
    "wechat",
    "baidu",
    "toutiao",
    "xueqiu",
)

PLATFORM_LABELS: dict[str, str] = {
    "weibo": "微博",
    "xiaohongshu": "小红书",
    "bilibili": "B站",
    "zhihu": "知乎",
    "douyin": "抖音",
    "wechat": "微信公众号",
    "baidu": "百度",
    "toutiao": "今日头条",
    "xueqiu": "雪球",
}

LOGIN_COOKIE_DOMAINS: dict[str, tuple[str, ...]] = {
    "zhihu": ("zhihu.com",),
    "xiaohongshu": ("xiaohongshu.com",),
    "douyin": ("douyin.com",),
    "xueqiu": ("xueqiu.com",),
}

REQUIRED_LOGIN_COOKIES: dict[str, tuple[str, ...]] = {
    "xueqiu": ("xq_a_token",),
}

COOKIE_DIR = Path.home() / ".config" / "last30days-cn" / "browser_cookies"
LAST30DAYS_CN_SCRIPT_DIRS: tuple[Path, ...] = (
    Path(os.environ["KITTYRED_LAST30DAYS_CN_SCRIPT_DIR"])
    if os.environ.get("KITTYRED_LAST30DAYS_CN_SCRIPT_DIR")
    else Path("/Users/yejiming/Desktop/Kitt智能体/.claude/skills/last30days-cn/scripts"),
    Path("/Users/yejiming/Desktop/Kitt智能体/.codex/skills/last30days-cn/scripts"),
)
_LAST30DAYS_MODULES: dict[str, Any] | None = None


class SocialSentimentAdapter:
    def __init__(self, client: Any | None = None):
        self.client = client

    def supported_platforms(self) -> list[dict[str, str]]:
        return [
            {"id": platform, "label": PLATFORM_LABELS[platform]}
            for platform in SUPPORTED_PLATFORMS
        ]

    def probe_platforms(self, platforms: list[str] | None = None) -> list[dict[str, Any]]:
        selected = self._selected_platforms(platforms)
        rows: list[dict[str, Any]] = []
        for platform in selected:
            try:
                fetched = self._fetch_platform(
                    platform=platform,
                    stock_code="SHSE.600000",
                    stock_name="浦发银行",
                    fetched_at=self._now(),
                    limit=3,
                    allow_empty=True,
                )
                rows.append(
                    {
                        "platform": platform,
                        "ok": True,
                        "message": f"{PLATFORM_LABELS[platform]} 真实搜索成功，返回 {len(fetched)} 条结果",
                    }
                )
            except Exception as error:
                rows.append(
                    {
                        "platform": platform,
                        "ok": False,
                        "message": f"{PLATFORM_LABELS[platform]} 真实搜索失败：{error}",
                    }
                )
        return rows

    def capture_login_state(self, platform: str) -> dict[str, Any]:
        if platform not in LOGIN_COOKIE_DOMAINS:
            raise ValueError(f"该平台暂不支持登录态获取：{platform}")
        state = self._load_cached_browser_state(platform)
        if not state and platform in {"zhihu", "douyin", "xueqiu"}:
            state = self._capture_state_from_cdp(platform)
        if not state:
            raise ValueError(f"未找到 {PLATFORM_LABELS[platform]} 的浏览器登录态，请先在打开的浏览器中完成登录")
        validation_error = validate_login_state(platform, state)
        if validation_error:
            raise ValueError(validation_error)
        return {
            "platform": platform,
            "source": state.get("source", "browser_state"),
            "storageState": {
                "cookies": state.get("cookies", []),
                "origins": state.get("origins", []),
            },
            "capturedAt": self._now(),
        }

    def fetch_discussions(
        self,
        stock_code: str,
        stock_name: str = "",
        platforms: list[str] | None = None,
        recent_days: int = 30,
    ) -> dict[str, Any]:
        selected = self._selected_platforms(platforms)
        recent_days = max(1, min(30, int(recent_days or 30)))
        now = self._now()
        items: list[dict[str, Any]] = []
        statuses: list[dict[str, Any]] = []
        for platform in selected:
            try:
                fetched = self._fetch_platform(
                    platform,
                    stock_code,
                    stock_name,
                    now,
                    recent_days=recent_days,
                    allow_empty=True,
                )
                items.extend(fetched)
                statuses.append(
                    {
                        "platform": platform,
                        "status": "succeeded",
                        "itemCount": len(fetched),
                        "errorMessage": None,
                    }
                )
            except Exception as error:
                statuses.append(
                    {
                        "platform": platform,
                        "status": "failed",
                        "itemCount": 0,
                        "errorMessage": f"{PLATFORM_LABELS[platform]} 拉取失败：{error}",
                    }
                )
        return {
            "stockCode": stock_code,
            "stockName": stock_name or None,
            "items": items,
            "platformStatuses": statuses,
            "fetchedAt": now,
        }

    def _fetch_platform(
        self,
        platform: str,
        stock_code: str,
        stock_name: str,
        fetched_at: str,
        limit: int = 20,
        recent_days: int = 30,
        allow_empty: bool = False,
    ) -> list[dict[str, Any]]:
        if self.client and hasattr(self.client, "fetch_platform"):
            raw_items = self.client.fetch_platform(platform, stock_code, stock_name, limit)
            return normalize_discussion_items(platform, raw_items, fetched_at)
        query_name = stock_name or stock_code
        raw_items = fetch_real_platform_items(
            platform,
            query_name,
            limit,
            stock_code=stock_code,
            recent_days=recent_days,
        )
        if not raw_items and not allow_empty:
            raise ValueError("真实搜索未返回可用结果")
        return normalize_discussion_items(platform, raw_items, fetched_at)

    def _load_cached_browser_state(self, platform: str) -> dict[str, Any] | None:
        path = COOKIE_DIR / f"{platform}_cookies.json"
        if not path.exists():
            return None
        try:
            return normalize_browser_state(json.loads(path.read_text(encoding="utf-8")), source="last30days-cn")
        except Exception:
            return None

    def _capture_state_from_cdp(self, platform: str) -> dict[str, Any] | None:
        try:
            from playwright.sync_api import sync_playwright
        except Exception:
            return None
        try:
            with sync_playwright() as playwright:
                browser = playwright.chromium.connect_over_cdp("http://127.0.0.1:9222")
                contexts = browser.contexts
                if not contexts:
                    browser.close()
                    return None
                state = normalize_browser_state(contexts[0].storage_state(), source="chrome_cdp")
                browser.close()
                if state:
                    path = COOKIE_DIR / f"{platform}_cookies.json"
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")
                return state
        except Exception:
            return None

    def _selected_platforms(self, platforms: list[str] | None) -> list[str]:
        if not platforms:
            return list(SUPPORTED_PLATFORMS)
        normalized: list[str] = []
        for platform in platforms:
            key = str(platform).strip()
            if key not in SUPPORTED_PLATFORMS:
                raise ValueError(f"不支持的社媒平台：{key}")
            if key not in normalized:
                normalized.append(key)
        return normalized

    def _now(self) -> str:
        return datetime.now(timezone.utc).isoformat()


def normalize_discussion_item(
    platform: str,
    raw: dict[str, Any],
    fetched_at: str | None = None,
) -> dict[str, Any]:
    if platform not in SUPPORTED_PLATFORMS:
        raise ValueError(f"不支持的社媒平台：{platform}")
    text = str(
        raw.get("text")
        or raw.get("desc")
        or raw.get("description")
        or raw.get("excerpt")
        or raw.get("abstract")
        or raw.get("content")
        or raw.get("snippet")
        or ""
    ).strip()
    title = str(raw.get("title") or "").strip()
    if not text and title:
        text = title
    if not text:
        raise ValueError("社媒讨论缺少正文")
    engagement = raw.get("engagement")
    if not isinstance(engagement, dict):
        engagement = {}
    return {
        "platform": platform,
        "title": title or None,
        "text": text,
        "author": (
            raw.get("author")
            or raw.get("author_name")
            or raw.get("source_name")
            or raw.get("author_handle")
            or raw.get("channel_name")
        ),
        "publishedAt": raw.get("publishedAt") or raw.get("published_at") or raw.get("date"),
        "url": raw.get("url") or None,
        "engagement": engagement,
        "fetchedAt": fetched_at or datetime.now(timezone.utc).isoformat(),
        "raw": raw,
    }


def normalize_discussion_items(
    platform: str,
    raw_items: list[dict[str, Any]],
    fetched_at: str,
) -> list[dict[str, Any]]:
    normalized: list[dict[str, Any]] = []
    for item in raw_items:
        try:
            normalized.append(normalize_discussion_item(platform=platform, raw=item, fetched_at=fetched_at))
        except ValueError:
            continue
    return normalized


def normalize_browser_state(value: Any, source: str) -> dict[str, Any] | None:
    if isinstance(value, list):
        return {"cookies": value, "origins": [], "source": source}
    if isinstance(value, dict):
        cookies = value.get("cookies") if isinstance(value.get("cookies"), list) else []
        origins = value.get("origins") if isinstance(value.get("origins"), list) else []
        return {"cookies": cookies, "origins": origins, "source": source}
    return None


def validate_login_state(platform: str, state: dict[str, Any]) -> str | None:
    cookies = state.get("cookies") if isinstance(state.get("cookies"), list) else []
    domains = LOGIN_COOKIE_DOMAINS.get(platform, ())
    platform_cookies = [
        cookie for cookie in cookies
        if any(domain in str(cookie.get("domain", "")) for domain in domains)
    ]
    if not platform_cookies:
        return f"未捕获到 {PLATFORM_LABELS[platform]} 域名下的 Cookie"
    required = REQUIRED_LOGIN_COOKIES.get(platform, ())
    missing = [
        name for name in required
        if not any(cookie.get("name") == name and cookie.get("value") for cookie in platform_cookies)
    ]
    if missing:
        return f"未捕获到 {PLATFORM_LABELS[platform]} 必需 Cookie：{', '.join(missing)}"
    return None


def fetch_real_platform_items(
    platform: str,
    query: str,
    limit: int,
    stock_code: str | None = None,
    recent_days: int = 30,
) -> list[dict[str, Any]]:
    reference_items = _fetch_via_last30days_provider(
        platform,
        query,
        limit,
        stock_code=stock_code,
        recent_days=recent_days,
    )
    if reference_items:
        return reference_items
    if platform == "weibo":
        return _fetch_json_items(
            f"https://m.weibo.cn/api/container/getIndex?containerid=100103type%3D1%26q%3D{_quote(query)}&page_type=searchall",
            parser=_parse_weibo,
            headers={"User-Agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15"},
            limit=limit,
        )
    if platform == "bilibili":
        return _fetch_json_items(
            f"https://api.bilibili.com/x/web-interface/search/type?search_type=video&keyword={_quote(query)}&page=1&page_size={limit}&order=totalrank",
            parser=_parse_bilibili,
            headers={"User-Agent": _desktop_ua(), "Referer": "https://search.bilibili.com/"},
            limit=limit,
        )
    if platform == "zhihu":
        return _fetch_json_items(
            f"https://www.zhihu.com/api/v4/search_v3?t=general&q={_quote(query)}&offset=0&limit={min(limit, 20)}",
            parser=_parse_zhihu,
            headers={"User-Agent": _desktop_ua(), "Referer": "https://www.zhihu.com/"},
            limit=limit,
        )
    if platform == "douyin":
        return _fetch_json_items(
            f"https://www.douyin.com/aweme/v1/web/general/search/single/?keyword={_quote(query)}&count={min(limit, 20)}&search_channel=aweme_general&sort_type=0&publish_time=0",
            parser=_parse_douyin,
            headers={"User-Agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15", "Referer": "https://www.douyin.com/"},
            limit=limit,
        )
    if platform == "baidu":
        return _fetch_html_items(
            f"https://cn.bing.com/search?q={_quote(query)}&setmkt=zh-CN&ensearch=0",
            parser=_parse_bing_html,
            headers={"User-Agent": _desktop_ua(), "Referer": "https://cn.bing.com/"},
            limit=limit,
        )
    if platform == "wechat":
        return _fetch_html_items(
            f"https://weixin.sogou.com/weixin?type=2&query={_quote(query)}&ie=utf8",
            parser=_parse_sogou_wechat_html,
            headers={"User-Agent": _desktop_ua()},
            limit=limit,
        )
    if platform == "toutiao":
        return _fetch_json_items(
            f"https://www.toutiao.com/api/search/content/?keyword={_quote(query)}&count={min(limit, 20)}&offset=0",
            parser=_parse_toutiao,
            headers={"User-Agent": _desktop_ua(), "Referer": "https://www.toutiao.com/", "Cookie": "tt_webid=1"},
            limit=limit,
        )
    if platform == "xiaohongshu":
        return _fetch_json_items(
            f"https://www.xiaohongshu.com/fe_api/burdock/weixin/v2/search/notes?keyword={_quote(query)}&page=1&page_size={limit}",
            parser=_parse_xiaohongshu,
            headers={"User-Agent": "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15"},
            limit=limit,
        )
    if platform == "xueqiu":
        return _fetch_html_items(
            f"https://xueqiu.com/k?q={_quote(query)}",
            parser=_parse_xueqiu_public_html,
            headers={"User-Agent": _desktop_ua(), "Referer": "https://xueqiu.com/"},
            limit=limit,
        )
    raise ValueError(f"不支持的社媒平台：{platform}")


def _fetch_via_last30days_provider(
    platform: str,
    query: str,
    limit: int,
    stock_code: str | None = None,
    recent_days: int = 30,
) -> list[dict[str, Any]]:
    modules = _load_last30days_modules()
    if not modules:
        return []
    recent_days = max(1, min(30, int(recent_days or 30)))
    from_date = (datetime.now() - timedelta(days=recent_days)).strftime("%Y-%m-%d")
    to_date = datetime.now().strftime("%Y-%m-%d")
    config = modules["env"].get_config()
    depth = "quick"
    if platform == "weibo":
        return modules["weibo"].search_weibo(
            query, from_date, to_date, depth=depth, token=config.get("WEIBO_ACCESS_TOKEN")
        )[:limit]
    if platform == "xiaohongshu":
        return modules["xiaohongshu"].search_xiaohongshu(
            query,
            from_date,
            to_date,
            depth=depth,
            token=config.get("SCRAPECREATORS_API_KEY"),
            api_base=modules["env"].get_xiaohongshu_api_base(config),
        )[:limit]
    if platform == "bilibili":
        return modules["bilibili"].search_bilibili(query, from_date, to_date, depth=depth)[:limit]
    if platform == "zhihu":
        return modules["zhihu"].search_zhihu(
            query, from_date, to_date, depth=depth, cookie=config.get("ZHIHU_COOKIE")
        )[:limit]
    if platform == "douyin":
        return modules["douyin"].search_douyin(
            query,
            from_date,
            to_date,
            depth=depth,
            token=config.get("TIKHUB_API_KEY") or config.get("DOUYIN_API_KEY"),
        )[:limit]
    if platform == "wechat":
        return modules["wechat"].search_wechat(
            query, from_date, to_date, depth=depth, api_key=config.get("WECHAT_API_KEY")
        )[:limit]
    if platform == "baidu":
        return modules["baidu"].search_baidu(
            query,
            from_date,
            to_date,
            depth=depth,
            api_key=config.get("BAIDU_API_KEY"),
            secret_key=config.get("BAIDU_SECRET_KEY"),
        )[:limit]
    if platform == "toutiao":
        return modules["toutiao"].search_toutiao(query, from_date, to_date, depth=depth)[:limit]
    if platform == "xueqiu":
        return modules["xueqiu"].search_xueqiu(
            _xueqiu_query(stock_code, query), from_date, to_date, depth=depth
        )[:limit]
    return []


def _load_last30days_modules() -> dict[str, Any] | None:
    global _LAST30DAYS_MODULES
    if _LAST30DAYS_MODULES is not None:
        return _LAST30DAYS_MODULES
    script_dir = next((path for path in LAST30DAYS_CN_SCRIPT_DIRS if path.exists()), None)
    if not script_dir:
        _LAST30DAYS_MODULES = {}
        return None
    script_dir_text = str(script_dir)
    if script_dir_text not in sys.path:
        sys.path.insert(0, script_dir_text)
    try:
        _LAST30DAYS_MODULES = {
            name: importlib.import_module(f"lib.{name}")
            for name in (
                "env",
                "weibo",
                "xiaohongshu",
                "bilibili",
                "zhihu",
                "douyin",
                "wechat",
                "baidu",
                "toutiao",
                "xueqiu",
            )
        }
    except Exception:
        _LAST30DAYS_MODULES = {}
    return _LAST30DAYS_MODULES or None


def _xueqiu_query(stock_code: str | None, fallback: str) -> str:
    code = (stock_code or "").strip().upper()
    if code.startswith("SHSE.") and len(code) >= 11:
        return f"SH{code.split('.', 1)[1]}"
    if code.startswith("SZSE.") and len(code) >= 11:
        return f"SZ{code.split('.', 1)[1]}"
    return fallback


def _fetch_json_items(url: str, parser: Any, headers: dict[str, str], limit: int) -> list[dict[str, Any]]:
    data = _request_json(url, headers)
    return parser(data, limit)


def _fetch_html_items(url: str, parser: Any, headers: dict[str, str], limit: int) -> list[dict[str, Any]]:
    html = _request_text(url, headers)
    return parser(html, limit)


def _request_json(url: str, headers: dict[str, str]) -> Any:
    text = _request_text(url, headers)
    return json.loads(text)


def _request_text(url: str, headers: dict[str, str]) -> str:
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=15) as response:
        return response.read().decode("utf-8", errors="replace")


def _quote(value: str) -> str:
    return urllib.parse.quote(value)


def _desktop_ua() -> str:
    return "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"


def _clean_html(value: Any) -> str:
    return re.sub(r"\s+", " ", re.sub(r"<[^>]+>", "", str(value or ""))).strip()


def _parse_weibo(data: Any, limit: int) -> list[dict[str, Any]]:
    cards = data.get("data", {}).get("cards", []) if isinstance(data, dict) else []
    rows: list[dict[str, Any]] = []
    for card in cards:
        candidates = [card.get("mblog")] if card.get("mblog") else [
            group.get("mblog") for group in card.get("card_group", []) if isinstance(group, dict)
        ]
        for mblog in candidates:
            if not isinstance(mblog, dict):
                continue
            user = mblog.get("user") or {}
            rows.append({
                "title": _clean_html(mblog.get("text", ""))[:80],
                "text": _clean_html(mblog.get("text", "")),
                "author": user.get("screen_name", ""),
                "url": f"https://weibo.com/{user.get('id', '')}/{mblog.get('mid') or mblog.get('id', '')}",
                "engagement": {"comments": mblog.get("comments_count", 0), "likes": mblog.get("attitudes_count", 0)},
            })
            if len(rows) >= limit:
                return rows
    return rows


def _parse_bilibili(data: Any, limit: int) -> list[dict[str, Any]]:
    results = data.get("data", {}).get("result", []) if isinstance(data, dict) else []
    return [
        {
            "title": _clean_html(item.get("title", "")),
            "text": _clean_html(item.get("description") or item.get("title", "")),
            "author": item.get("author", ""),
            "url": f"https://www.bilibili.com/video/{item.get('bvid', '')}",
            "engagement": {"views": item.get("play", 0), "comments": item.get("review", 0)},
        }
        for item in results[:limit]
        if item.get("title")
    ]


def _parse_zhihu(data: Any, limit: int) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for entry in (data.get("data", []) if isinstance(data, dict) else []):
        obj = entry.get("object", entry) if isinstance(entry, dict) else {}
        question = obj.get("question") if isinstance(obj.get("question"), dict) else {}
        title = _clean_html(obj.get("title") or question.get("title", ""))
        text = _clean_html(obj.get("excerpt") or obj.get("content") or title)
        if not title and not text:
            continue
        author = obj.get("author") if isinstance(obj.get("author"), dict) else {}
        rows.append({"title": title or text[:80], "text": text or title, "author": author.get("name", ""), "url": obj.get("url", ""), "engagement": {}})
        if len(rows) >= limit:
            break
    return rows


def _parse_douyin(data: Any, limit: int) -> list[dict[str, Any]]:
    if isinstance(data, dict) and data.get("search_nil_info", {}).get("search_nil_item") == "invalid_app":
        raise ValueError("公开接口返回 invalid_app，当前会话未通过站点校验")
    rows: list[dict[str, Any]] = []
    for entry in (data.get("data", []) if isinstance(data, dict) else []):
        aweme = entry.get("aweme_info", entry) if isinstance(entry, dict) else {}
        author = aweme.get("author") or {}
        stats = aweme.get("statistics") or {}
        text = aweme.get("desc", "")
        if not text:
            continue
        rows.append({"title": text[:80], "text": text, "author": author.get("nickname", ""), "url": f"https://www.douyin.com/video/{aweme.get('aweme_id', '')}", "engagement": {"likes": stats.get("digg_count", 0), "comments": stats.get("comment_count", 0)}})
        if len(rows) >= limit:
            break
    return rows


def _parse_toutiao(data: Any, limit: int) -> list[dict[str, Any]]:
    rows = []
    entries = data.get("data") if isinstance(data, dict) else []
    if not isinstance(entries, list):
        return rows
    for item in entries:
        title = _clean_html(item.get("title", ""))
        if title:
            rows.append({"title": title, "text": _clean_html(item.get("abstract", title)), "author": item.get("source") or item.get("media_name"), "url": item.get("article_url") or item.get("display_url"), "engagement": {"comments": item.get("comment_count", 0)}})
        if len(rows) >= limit:
            break
    return rows


def _parse_xiaohongshu(data: Any, limit: int) -> list[dict[str, Any]]:
    notes = data.get("data", {}).get("notes", []) if isinstance(data, dict) else []
    return [{"title": item.get("title") or item.get("display_title", ""), "text": item.get("desc") or item.get("title") or "", "author": (item.get("user") or {}).get("nickname", "") if isinstance(item.get("user"), dict) else "", "url": f"https://www.xiaohongshu.com/explore/{item.get('note_id') or item.get('id', '')}", "engagement": {}} for item in notes[:limit] if item.get("title") or item.get("desc")]


def _parse_bing_html(html: str, limit: int) -> list[dict[str, Any]]:
    rows = []
    for block in re.findall(r'<li class="b_algo"[^>]*>([\s\S]*?)</li>', html)[:limit]:
        match = re.search(r'<h2[^>]*>\s*<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>', block, re.S)
        if match:
            rows.append({"title": _clean_html(match.group(2)), "text": _clean_html(block), "url": match.group(1), "author": "Bing", "engagement": {}})
    return rows


def _parse_sogou_wechat_html(html: str, limit: int) -> list[dict[str, Any]]:
    rows = []
    for href, title_html in re.findall(r'<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>', html, re.S)[:limit]:
        title = _clean_html(title_html)
        if title and ("weixin.qq.com" in href or "sogou.com" in href):
            rows.append({"title": title, "text": title, "url": href, "author": "微信公众号", "engagement": {}})
    return rows


def _parse_xueqiu_public_html(html: str, limit: int) -> list[dict[str, Any]]:
    if "xq_a_token" not in html and "window.SNB.data" not in html:
        raise ValueError("公开搜索未返回结构化数据，可能需要 Chrome CDP 登录态")
    return []
