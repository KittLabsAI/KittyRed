from __future__ import annotations

import argparse
from pathlib import Path
import sys

PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from backend.runtime_bundle import DEFAULT_RUNTIME_ROOT, build_runtime


def main() -> int:
    parser = argparse.ArgumentParser(description="Build bundled Python runtime for KittyRed.")
    parser.add_argument(
        "--output",
        default=str(DEFAULT_RUNTIME_ROOT),
        help="Output directory for the bundled Python runtime.",
    )
    parser.add_argument(
        "--python",
        default=None,
        help="Python executable used to create the venv.",
    )
    args = parser.parse_args()
    runtime_root = build_runtime(Path(args.output).resolve(), python_executable=args.python)
    print(runtime_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
