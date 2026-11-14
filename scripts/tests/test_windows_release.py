import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'prepare-windows-release.py'
spec = importlib.util.spec_from_file_location('windows_release', SCRIPT)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
REVISION = 'a' * 40
TAG = 'windows-preview-v0.1.0-' + REVISION[:12]


class WindowsRelease(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.names = ['tgskk-0.1.0-windows-x64-setup.exe',
                      'tgskk-0.1.0-windows-x64.zip', 'tgskk-source.tar.gz',
                      'llama-' + 'b' * 40 + '.tar.gz']
        for name in self.names:
            (self.root / name).write_bytes(name.encode())
        self.manifest()

    def manifest(self):
        (self.root / 'SHA256SUMS').write_text(''.join(
            f'{hashlib.sha256((self.root / name).read_bytes()).hexdigest()}  ./{name}\n'
            for name in self.names))

    def verify(self):
        return release.verify(self.root, TAG, REVISION)

    def test_accepts_complete_bundle_and_shell_checksum_paths(self):
        manifest = self.root / 'SHA256SUMS'
        original = manifest.read_text()
        for separator in ('  ./', '  ', ' *./', ' *'):
            with self.subTest(separator=separator):
                manifest.write_text(original.replace('  ./', separator))
                self.assertEqual(self.verify(), '0.1.0')

    def test_rejects_mismatched_tag_revision_and_version(self):
        for tag, revision in [('v0.1.0', REVISION), (TAG, 'b' * 40),
                              (TAG, 'a' * 12), (TAG.replace('0.1.0', '0.1.1'), REVISION)]:
            with self.subTest(tag=tag, revision=revision), self.assertRaises(ValueError):
                release.verify(self.root, tag, revision)

    def test_rejects_corrupt_files_and_missing_sources(self):
        target = self.root / self.names[0]
        target.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.verify()
        self.manifest()
        target.unlink()
        with self.assertRaises(ValueError):
            self.verify()
        target.write_bytes(b'changed')
        self.names.remove('tgskk-source.tar.gz')
        (self.root / 'tgskk-source.tar.gz').unlink()
        self.manifest()
        with self.assertRaisesRegex(ValueError, 'complete tgskk source'):
            self.verify()

    def test_rejects_extra_duplicate_and_unsafe_entries(self):
        manifest = self.root / 'SHA256SUMS'
        original = manifest.read_text()
        for extra in [original.splitlines()[0] + '\n', '0' * 64 + '  ../outside\n']:
            manifest.write_text(original + extra)
            with self.assertRaises(ValueError):
                self.verify()
        manifest.write_text(original)
        (self.root / 'unexpected.exe').write_bytes(b'extra')
        with self.assertRaisesRegex(ValueError, 'unexpected'):
            self.verify()

    def test_rejects_symlinked_files_and_manifest(self):
        target = self.root / self.names[0]
        target.unlink()
        target.symlink_to(self.names[1])
        with self.assertRaisesRegex(ValueError, 'regular file'):
            self.verify()
        target.unlink()
        target.write_bytes(self.names[0].encode())
        manifest = self.root / 'SHA256SUMS'
        manifest.unlink()
        manifest.symlink_to(self.names[1])
        with self.assertRaisesRegex(ValueError, 'regular SHA256SUMS'):
            self.verify()
