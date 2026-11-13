import hashlib
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from zenz_model import default_directory, verified_download


class ModelDownloadTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "source"
        self.source.write_bytes(b"verified model")
        self.expected = hashlib.sha256(self.source.read_bytes()).hexdigest()
        self.destination = self.root / "models/model.gguf"

    def test_installs_verified_model_and_offline_reuses_it(self):
        verified_download(self.source.as_uri(), self.destination, self.expected)
        self.assertEqual(self.destination.read_bytes(), b"verified model")
        self.source.unlink()
        verified_download(self.source.as_uri(), self.destination, self.expected, offline=True)
        self.assertEqual(self.destination.stat().st_mode & 0o777, 0o644)

    def test_bad_download_preserves_existing_file_and_cleans_partial(self):
        self.destination.parent.mkdir()
        self.destination.write_bytes(b"original")
        with self.assertRaisesRegex(RuntimeError, "SHA256 mismatch"):
            verified_download(self.source.as_uri(), self.destination, "0" * 64)
        self.assertEqual(self.destination.read_bytes(), b"original")
        self.assertEqual(list(self.destination.parent.iterdir()), [self.destination])

    def test_offline_missing_or_corrupt_file_fails_without_fetch(self):
        for existing in [False, True]:
            if existing:
                self.destination.parent.mkdir()
                self.destination.write_bytes(b"corrupt")
            with patch("urllib.request.urlopen", side_effect=AssertionError("network")):
                with self.assertRaisesRegex(RuntimeError, "Verified artifact missing"):
                    verified_download(self.source.as_uri(), self.destination, self.expected, offline=True)

    def test_network_error_cleans_partial(self):
        with patch("urllib.request.urlopen", side_effect=OSError("offline")):
            with self.assertRaises(OSError):
                verified_download(self.source.as_uri(), self.destination, self.expected)
        self.assertEqual(list(self.destination.parent.iterdir()), [])

    def test_directory_respects_fcitx_and_xdg_data_overrides(self):
        with patch.dict(os.environ, {"FCITX_DATA_HOME": "/tmp/fcitx", "XDG_DATA_HOME": "/tmp/xdg"}):
            self.assertEqual(default_directory(), Path("/tmp/fcitx/tgskk"))
        with patch.dict(os.environ, {"FCITX_DATA_HOME": "", "XDG_DATA_HOME": "/tmp/xdg"}):
            self.assertEqual(default_directory(), Path("/tmp/xdg/fcitx5/tgskk"))

    def test_unset_or_empty_xdg_data_home_uses_home_directory(self):
        for environment in [{}, {"XDG_DATA_HOME": ""}]:
            with self.subTest(environment=environment):
                with patch.dict(os.environ, {"HOME": str(self.root), **environment}, clear=True):
                    self.assertEqual(default_directory(), self.root / ".local/share/fcitx5/tgskk")


if __name__ == "__main__":
    unittest.main()
