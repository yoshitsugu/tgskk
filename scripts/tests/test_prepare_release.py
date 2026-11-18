import hashlib
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'prepare-release.py'
VERSION = tomllib.loads((SCRIPT.parent.parent / 'Cargo.toml').read_text())['workspace']['package']['version']
TAG = f'v{VERSION}'
WRONG_VERSION = '999.999.999'


class ReleaseBundles(unittest.TestCase):
    def test_each_distribution_keeps_all_three_packages_and_checksum(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for target in ('arch', 'debian-13', 'ubuntu-24.04', 'ubuntu-26.04'):
                directory = root / target
                directory.mkdir()
                arch = target == 'arch'
                for stem in ('tgskk-common', 'tgskk', 'tgskk-ibus'):
                    name = stem + (f'-git-{VERSION}-1-x86_64.pkg.tar.zst' if arch else f'_{VERSION}-1_amd64.deb')
                    (directory / name).write_bytes(target.encode())
                subprocess.run([sys.executable, str(SCRIPT), '--tag', TAG, '--target', target, '--package-root', str(root), '--output-dir', str(root / 'out')], check=True, capture_output=True)
                archive = root / 'out' / f'tgskk-{TAG}-{target}-x86_64.tar.gz'
                with tarfile.open(archive) as tar:
                    self.assertEqual(set(tar.getnames()), {p.name for p in directory.iterdir()})
                    self.assertTrue(all(tar.extractfile(member).read() == target.encode() for member in tar))
                self.assertEqual(archive.with_name(archive.name + '.sha256').read_text(), f'{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n')

    def test_rejects_wrong_tag_missing_package_and_mixed_version(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / 'debian-13'
            directory.mkdir()
            for tag in (f'v{WRONG_VERSION}', f'{TAG}-rc.1', f'../{TAG}'):
                self.assert_rejected(root, tag)
            self.assert_rejected(root, TAG)
            for stem in ('tgskk-common', 'tgskk', 'tgskk-ibus'):
                (directory / f'{stem}_{WRONG_VERSION}-2_amd64.deb').write_bytes(b'package')
            self.assert_rejected(root, TAG)
            for package in directory.iterdir():
                package.unlink()
            for stem, revision in (('tgskk-common', 1), ('tgskk', 2), ('tgskk-ibus', 2)):
                (directory / f'{stem}_{VERSION}-{revision}_amd64.deb').write_bytes(b'package')
            self.assert_rejected(root, TAG)
            self.assertFalse((root / 'out').exists())

    def assert_rejected(self, root, tag):
        result = subprocess.run([sys.executable, str(SCRIPT), '--tag', tag, '--target', 'debian-13', '--package-root', str(root), '--output-dir', str(root / 'out')], capture_output=True)
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
