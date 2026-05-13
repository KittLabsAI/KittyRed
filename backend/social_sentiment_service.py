from __future__ import annotations

from contextlib import redirect_stdout
import io
import json
import sys
from typing import Any

from backend.social_sentiment_adapter import SocialSentimentAdapter

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


def handle_request(
    payload: dict[str, Any],
    adapter: SocialSentimentAdapter | None = None,
) -> dict[str, Any]:
    action = payload.get("action")
    adapter = adapter or SocialSentimentAdapter()

    if action == "supported_platforms":
        return {"ok": True, "data": adapter.supported_platforms()}

    if action == "probe_platforms":
        raw_platforms = payload.get("platforms")
        platforms = raw_platforms if isinstance(raw_platforms, list) else None
        return {"ok": True, "data": adapter.probe_platforms(platforms)}

    if action == "capture_login_state":
        return {
            "ok": True,
            "data": adapter.capture_login_state(str(payload.get("platform", "")).strip()),
        }

    if action == "fetch_discussions":
        raw_platforms = payload.get("platforms")
        platforms = raw_platforms if isinstance(raw_platforms, list) else None
        return {
            "ok": True,
            "data": adapter.fetch_discussions(
                stock_code=str(payload.get("stock_code", "")).strip(),
                stock_name=str(payload.get("stock_name", "")).strip(),
                platforms=platforms,
                recent_days=int(payload.get("recent_days") or 30),
            ),
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
