#!/usr/bin/env python3
"""Unit tests for stage-windows.py."""

import hashlib
from pathlib import Path
import struct
import sys
import tempfile
import tomllib
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import importlib.util
spec = importlib.util.spec_from_file_location('stage_windows', Path(__file__).resolve().parents[1] / 'stage-windows.py')
stage_windows = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage_windows)
import zenz_model


def make_pe(path: Path, machine: int = 0x8664) -> None:
    """Write a minimal synthetic PE file with the given machine type."""
    e_lfanew = 0x40
    buf = bytearray(e_lfanew + 6)
    buf[0:2] = b"MZ"
    struct.pack_into("<I", buf, 0x3C, e_lfanew)
    buf[e_lfanew : e_lfanew + 4] = b"PE\x00\x00"
    struct.pack_into("<H", buf, e_lfanew + 4, machine)
    path.write_bytes(bytes(buf))


class StageWindowsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

        self.binaries = self.root / "binaries"
        self.binaries.mkdir()
        for name in stage_windows.REQUIRED_BINARIES:
            make_pe(self.binaries / name)

        self.runtime = self.root / "runtime"
        self.runtime.mkdir()
        for name in stage_windows.REQUIRED_RUNTIME_DLLS:
            make_pe(self.runtime / name)
        make_pe(self.runtime / "msvcp140_2.dll")

        self.dictionary = self.root / "dict"
        self.dictionary.write_text("dictionary content")

        self.model = self.root / "model.gguf"
        self.model.write_bytes(b"synthetic model")

        self.llama_source = self.root / "llama"
        self.llama_source.mkdir()
        (self.llama_source / "LICENSE").write_text("llama license")

        self.vendor = self.root / "vendor"
        crate = self.vendor / "aws-lc-sys-0.47.0"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text('[package]\nname="aws-lc-sys"\nversion="0.47.0"\nlicense="ISC AND MIT AND BSD-3-Clause"\n')
        (crate / "LICENSE").write_text("AWS-LC incorporated notices sentinel")
        self.output = self.root / "output"

        # Patch MODEL_SHA to match our synthetic model.
        self._orig_model_sha = zenz_model.MODEL_SHA
        zenz_model.MODEL_SHA = hashlib.sha256(b"synthetic model").hexdigest()
        self.addCleanup(setattr, zenz_model, "MODEL_SHA", self._orig_model_sha)

    def _argv(self):
        return [
            "--binaries",
            str(self.binaries),
            "--dictionary",
            str(self.dictionary),
            "--model",
            str(self.model),
            "--runtime",
            str(self.runtime),
            "--llama-source",
            str(self.llama_source),
            "--vendor", str(self.vendor),
            "--output",
            str(self.output),
        ]

    def test_successful_layout_and_hash_validation(self):
        stage_windows.main(self._argv())

        payload = self.output / "payload"
        self.assertTrue(payload.exists())

        for name in stage_windows.REQUIRED_BINARIES:
            self.assertTrue((payload / name).exists())

        for name in stage_windows.REQUIRED_RUNTIME_DLLS:
            self.assertTrue((payload / name).exists())
        self.assertTrue((payload / "msvcp140_2.dll").exists())

        self.assertEqual((payload / "SKK-JISYO.L").read_text(), "dictionary content")

        model_path = payload / "models" / zenz_model.MODEL_NAME
        self.assertTrue(model_path.exists())
        self.assertEqual(model_path.read_bytes(), b"synthetic model")

        licenses = payload / "licenses"
        self.assertTrue((licenses / "LICENSE").exists())
        self.assertTrue((licenses / "cskk-LICENSE.txt").exists())
        self.assertTrue((licenses / "gskkserv-LICENSE-APACHE").exists())
        self.assertTrue((licenses / "gskkserv-UPSTREAM.md").exists())
        self.assertTrue((licenses / "NOTICE.md").exists())
        self.assertTrue((licenses / "llama-LICENSE").exists())

        self.assertTrue((payload / "addons" / "windows" / "README.md").exists())

        sums_file = payload / "SHA256SUMS"
        self.assertTrue(sums_file.exists())
        self.assertEqual((payload / "licenses/rust/aws-lc-sys-0.47.0/LICENSE").read_text(), "AWS-LC incorporated notices sentinel")
        sums = {}
        for line in sums_file.read_text().splitlines():
            digest, rel = line.split("  ", 1)
            sums[rel] = digest

        for path in payload.rglob("*"):
            if path.is_file() and path.name != "SHA256SUMS":
                rel = path.relative_to(payload).as_posix()
                self.assertIn(rel, sums)
                self.assertEqual(sums[rel], hashlib.sha256(path.read_bytes()).hexdigest())

        # Verify relative paths are POSIX (no backslashes).
        for rel in sums:
            self.assertNotIn("\\", rel)

        version = tomllib.loads(
            (Path(__file__).resolve().parents[1].parent / "Cargo.toml").read_text()
        )["workspace"]["package"]["version"]
        archive = self.output / f"tgskk-{version}-windows-x64.zip"
        self.assertTrue(archive.exists())
        with zipfile.ZipFile(archive) as zf:
            names = zf.namelist()
            self.assertTrue(all(name.startswith("tgskk/") for name in names))
            self.assertIn("tgskk/SHA256SUMS", names)
            for rel in sums:
                self.assertIn(f"tgskk/{rel}", names)

    def test_runtime_cannot_overwrite_an_application_binary(self):
        make_pe(self.runtime / "tgskk-tip.dll")
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertFalse((self.output / "payload").exists())

    def test_existing_archive_is_not_overwritten(self):
        self.output.mkdir()
        version = tomllib.loads((Path(__file__).parents[2] / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        archive = self.output / f"tgskk-{version}-windows-x64.zip"
        archive.write_bytes(b"existing archive")
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertEqual(archive.read_bytes(), b"existing archive")

    def test_missing_binary(self):
        (self.binaries / "tgskk-tip.dll").unlink()
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())

    def test_missing_runtime(self):
        (self.runtime / "msvcp140.dll").unlink()
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertFalse((self.output / "payload").exists())

    def test_non_x64_pe(self):
        make_pe(self.binaries / "tgskk-tip.dll", machine=0x014C)
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertFalse((self.output / "payload").exists())

    def test_corrupt_model(self):
        zenz_model.MODEL_SHA = "0" * 64
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertFalse((self.output / "payload").exists())

    def test_cleanup_on_failure(self):
        zenz_model.MODEL_SHA = "deadbeef" * 8
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())
        self.assertFalse((self.output / "payload").exists())

    def test_no_overwrite_of_existing_output(self):
        self.output.mkdir()
        (self.output / "payload").mkdir()
        with self.assertRaises(SystemExit):
            stage_windows.main(self._argv())


if __name__ == "__main__":
    unittest.main()
