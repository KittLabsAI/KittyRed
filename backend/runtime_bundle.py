from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Iterable


PROJECT_ROOT = Path(__file__).resolve().parent.parent
BACKEND_DIR = PROJECT_ROOT / "backend"
DEFAULT_RUNTIME_ROOT = PROJECT_ROOT / "src-tauri" / "resources" / "python"
REQUIREMENTS_FILE = BACKEND_DIR / "requirements.txt"


def bundled_python_candidates(runtime_root: Path, platform: str | None = None) -> list[Path]:
    current = platform or sys.platform
    if current.startswith("win"):
        return [runtime_root / "venv" / "Scripts" / "python.exe"]
    return [
        runtime_root / "venv" / "bin" / "python3",
        runtime_root / "venv" / "bin" / "python",
    ]


def resolve_bundled_python(runtime_root: Path, platform: str | None = None) -> Path:
    for candidate in bundled_python_candidates(runtime_root, platform=platform):
        if candidate.exists():
            return candidate
    raise FileNotFoundError(f"未找到内置 Python 可执行文件: {runtime_root}")


def should_copy_backend_path(path: Path) -> bool:
    parts = path.parts
    if "__pycache__" in parts:
        return False
    if parts[:2] == ("backend", "tests"):
        return False
    if path.suffix in {".pyc", ".pyo"}:
        return False
    return True


def iter_backend_source_paths() -> Iterable[Path]:
    for path in BACKEND_DIR.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(PROJECT_ROOT)
        if should_copy_backend_path(relative):
            yield path


def copy_backend_sources(runtime_root: Path) -> None:
    target_root = runtime_root / "app" / "backend"
    if target_root.exists():
        shutil.rmtree(target_root)
    target_root.mkdir(parents=True, exist_ok=True)
    for source in iter_backend_source_paths():
        relative = source.relative_to(BACKEND_DIR)
        destination = target_root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    package_init = target_root / "__init__.py"
    if not package_init.exists():
        package_init.write_text("", encoding="utf-8")


def build_runtime(
    runtime_root: Path = DEFAULT_RUNTIME_ROOT,
    python_executable: str | None = None,
) -> Path:
    runtime_root.mkdir(parents=True, exist_ok=True)
    venv_dir = runtime_root / "venv"
    python_cmd = python_executable or os.environ.get("PYTHON") or sys.executable
    subprocess.run([python_cmd, "-m", "venv", str(venv_dir)], check=True)
    bundled_python = resolve_bundled_python(runtime_root)
    subprocess.run(
        [str(bundled_python), "-m", "pip", "install", "-r", str(REQUIREMENTS_FILE)],
        check=True,
    )
    copy_backend_sources(runtime_root)
    return runtime_root
