"""Ensure C++ analysis checks owned sources and propagates failures."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "lint_cpp", Path(__file__).resolve().parents[1] / "lint-cpp.py"
)
lint_cpp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lint_cpp)


class TidyBuildTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.build = Path(self.temporary.name)

    def database(self, entries):
        (self.build / "compile_commands.json").write_text(json.dumps(entries), encoding="utf-8")

    def entry(self, source, directory=None):
        return {"directory": str(directory or lint_cpp.ROOT), "file": str(source), "command": "c++"}

    def test_excludes_dependencies_and_generated_sources_and_deduplicates(self):
        source = lint_cpp.ROOT / "native/zenz/worker.cpp"
        self.database([
            self.entry(source),
            self.entry("worker.cpp", source.parent),
            self.entry(lint_cpp.ROOT / "build/llama.cpp/src/llama.cpp"),
            self.entry(lint_cpp.ROOT / "third_party/cskk/generated.c"),
        ])
        with patch.object(lint_cpp.subprocess, "run") as run:
            lint_cpp.tidy_build("clang-tidy", self.build)
        run.assert_called_once_with(
            ["clang-tidy", "-p", str(self.build.resolve()), str(source)],
            cwd=lint_cpp.ROOT, check=True,
        )

    def test_empty_analysis_is_an_error(self):
        self.database([self.entry(lint_cpp.ROOT / "build/generated.cpp")])
        with self.assertRaisesRegex(RuntimeError, "No first-party"):
            lint_cpp.tidy_build("clang-tidy", self.build)

    def test_analysis_failure_is_not_ignored(self):
        self.database([self.entry(lint_cpp.ROOT / "addons/windows/tsf.cpp")])
        with patch.object(lint_cpp.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "clang-tidy")):
            with self.assertRaises(subprocess.CalledProcessError):
                lint_cpp.tidy_build("clang-tidy", self.build)


if __name__ == "__main__":
    unittest.main()
