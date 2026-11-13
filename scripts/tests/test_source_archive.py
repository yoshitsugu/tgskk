import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest


class SourceArchiveTest(unittest.TestCase):
    def test_exports_current_tracked_content_without_untracked_local_settings(self):
        script = Path(__file__).resolve().parents[1] / "export-source.sh"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            shutil.copyfile(script, root / "scripts/export-source.sh")
            env = {**os.environ, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull}
            subprocess.run(["git", "init", "-q", str(root)], check=True, env=env)
            (root / "public.rs").write_text("staged version")
            subprocess.run(["git", "-C", str(root), "add", "public.rs"], check=True, env=env)
            (root / "public.rs").write_text("working tree version")
            for name in [".mcp.json", ".gitconfig", ".bashrc", "untracked.rs"]:
                (root / name).write_text("local-only test data")
            archive = root / "source.tar.gz"
            subprocess.run(["bash", str(root / "scripts/export-source.sh"), str(archive)], check=True, env=env)
            with tarfile.open(archive) as package:
                self.assertEqual(package.getnames(), ["public.rs"])
                self.assertEqual(package.extractfile("public.rs").read(), b"working tree version")


if __name__ == "__main__":
    unittest.main()
