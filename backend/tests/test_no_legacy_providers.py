import unittest
from pathlib import Path


class NoLegacyProvidersTest(unittest.TestCase):
    def test_repository_does_not_contain_removed_provider_names(self):
        root = Path(__file__).resolve().parents[2]
        skipped = {".git", "node_modules", "dist", "target", "__pycache__"}
        forbidden = [
            "Coin" + "Gecko",
            "Bin" + "ance",
            "O" + "KX",
            "By" + "bit",
            "Bit" + "get",
            "Ku" + "Coin",
            "Ga" + "te",
            "ME" + "XC",
            "my" + "quant",
            "My" + "Quant",
            "掘" + "金量化",
            "gm" + ".api",
        ]
        hits = []

        for path in root.rglob("*"):
            if any(part in skipped for part in path.parts):
                continue
            if not path.is_file():
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            for item in forbidden:
                if item in text:
                    hits.append(f"{path.relative_to(root)} contains removed provider marker")

        self.assertEqual(hits, [])
