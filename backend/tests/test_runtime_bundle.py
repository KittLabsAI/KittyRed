import unittest
from pathlib import Path

from backend.runtime_bundle import (
    bundled_python_candidates,
    should_copy_backend_path,
)


class RuntimeBundleTest(unittest.TestCase):
    def test_posix_runtime_prefers_python3_then_python(self):
        runtime_root = Path("/tmp/runtime")

        self.assertEqual(
            bundled_python_candidates(runtime_root, platform="darwin"),
            [
                runtime_root / "venv" / "bin" / "python3",
                runtime_root / "venv" / "bin" / "python",
            ],
        )

    def test_windows_runtime_uses_python_exe(self):
        runtime_root = Path("C:/runtime")

        self.assertEqual(
            bundled_python_candidates(runtime_root, platform="win32"),
            [runtime_root / "venv" / "Scripts" / "python.exe"],
        )

    def test_backend_copy_skips_generated_files(self):
        self.assertTrue(should_copy_backend_path(Path("backend/akshare_service.py")))
        self.assertFalse(should_copy_backend_path(Path("backend/__pycache__/x.pyc")))
        self.assertFalse(should_copy_backend_path(Path("backend/tests/test_runtime_bundle.py")))


if __name__ == "__main__":
    unittest.main()
