from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys

PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from backend.runtime_bundle import DEFAULT_RUNTIME_ROOT, resolve_bundled_python


def run_module(python_executable: Path, module: str, payload: dict, cwd: Path) -> dict:
    result = subprocess.run(
        [str(python_executable), "-m", module],
        input=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(cwd),
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return json.loads(result.stdout.decode("utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke test bundled Python runtime.")
    parser.add_argument(
        "--runtime-root",
        default=str(DEFAULT_RUNTIME_ROOT),
        help="Bundled Python runtime root.",
    )
    args = parser.parse_args()

    runtime_root = Path(args.runtime_root).resolve()
    python_executable = resolve_bundled_python(runtime_root)
    app_root = runtime_root / "app"

    akshare_response = run_module(
        python_executable,
        "backend.akshare_service",
        {"action": "current_quote", "symbol": ""},
        app_root,
    )
    if not akshare_response.get("ok"):
        raise RuntimeError(f"AKShare smoke check failed: {akshare_response}")

    sentiment_response = run_module(
        python_executable,
        "backend.social_sentiment_service",
        {"action": "supported_platforms"},
        app_root,
    )
    if not sentiment_response.get("ok"):
        raise RuntimeError(f"sentiment smoke check failed: {sentiment_response}")

    print(json.dumps({"ok": True}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
