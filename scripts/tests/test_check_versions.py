from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check-versions.py'
REPO = SCRIPT.parent.parent
VERSION = tomllib.loads((REPO / 'Cargo.toml').read_text())['workspace']['package']['version']
WRONG_VERSION = '999.999.999'
PKGREL = re.search(r'^pkgrel=(\S+)$', (REPO / 'packaging/arch/PKGBUILD').read_text(), re.MULTILINE).group(1)
OTHER_PKGREL = '9999'
FILES = (
    'Cargo.toml',
    'addons/fcitx5/CMakeLists.txt',
    'addons/ibus/CMakeLists.txt',
    'addons/windows/CMakeLists.txt',
    'addons/fcitx5/tgskk.conf',
    'packaging/arch/PKGBUILD',
    'packaging/arch/.SRCINFO',
    'packaging/debian/changelog',
)


class VersionCheck(unittest.TestCase):
    def setUp(self):
        self.assertNotEqual(OTHER_PKGREL, PKGREL)
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in FILES:
            (self.root / name).parent.mkdir(parents=True, exist_ok=True)
        self.reset()

    def reset(self):
        (self.root / 'scripts').mkdir(exist_ok=True)
        shutil.copy(SCRIPT, self.root / 'scripts' / SCRIPT.name)
        for name in FILES:
            shutil.copy(REPO / name, self.root / name)

    def run_check(self):
        return subprocess.run(
            [sys.executable, str(self.root / 'scripts' / SCRIPT.name)],
            cwd=self.root / 'addons', capture_output=True, text=True,
        )

    def edit(self, name, old, new, count=1):
        path = self.root / name
        text = path.read_text(encoding='utf-8')
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, count), encoding='utf-8')

    def assert_rejected(self, name):
        result = self.run_check()
        self.assertEqual(result.returncode, 1)
        self.assertIn(f'{name}:', result.stderr)
        self.assertNotIn('Traceback', result.stderr)

    def test_synchronized_versions_pass(self):
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, f'All release versions match {VERSION}\n')

    def test_each_version_source_mismatch_fails(self):
        for name in FILES[1:]:
            with self.subTest(name=name):
                self.reset()
                self.edit(name, VERSION, WRONG_VERSION)
                self.assert_rejected(name)

    def test_workspace_version_is_the_reference(self):
        self.edit('Cargo.toml', f'version = "{VERSION}"', f'version = "{WRONG_VERSION}"')
        result = self.run_check()
        self.assertEqual(result.returncode, 1)
        reported = {line.split(':', 1)[0] for line in result.stderr.splitlines()}
        self.assertEqual(reported, set(FILES) - {'Cargo.toml'})

    def test_missing_version_field_fails(self):
        cases = (
            ('Cargo.toml', f'version = "{VERSION}"\n', ''),
            ('addons/fcitx5/CMakeLists.txt', f' VERSION {VERSION}', ''),
            ('addons/ibus/CMakeLists.txt', f' VERSION {VERSION}', ''),
            ('addons/windows/CMakeLists.txt', f' VERSION {VERSION}', ''),
            ('addons/fcitx5/tgskk.conf', f'Version={VERSION}\n', ''),
            ('packaging/arch/PKGBUILD', f'pkgver={VERSION}\n', ''),
            ('packaging/arch/.SRCINFO', f'\tpkgver = {VERSION}\n', ''),
            ('packaging/debian/changelog', 'tgskk (', 'tgskk-', -1),
        )
        for name, old, new, *count in cases:
            with self.subTest(name=name):
                self.reset()
                self.edit(name, old, new, *count)
                self.assert_rejected(name)

    def test_missing_pkgrel_fails(self):
        for name, old in (('packaging/arch/PKGBUILD', f'pkgrel={PKGREL}\n'), ('packaging/arch/.SRCINFO', f'\tpkgrel = {PKGREL}\n')):
            with self.subTest(name=name):
                self.reset()
                self.edit(name, old, '')
                self.assert_rejected(name)

    def test_missing_file_fails(self):
        for name in FILES:
            with self.subTest(name=name):
                self.reset()
                (self.root / name).unlink()
                self.assert_rejected(name)

    def test_invalid_cargo_toml_fails(self):
        self.edit('Cargo.toml', f'version = "{VERSION}"', 'version = [')
        self.assert_rejected('Cargo.toml')

    def test_non_utf8_input_fails_without_traceback(self):
        for name in ('Cargo.toml', 'packaging/arch/PKGBUILD', 'packaging/debian/changelog'):
            with self.subTest(name=name):
                self.reset()
                (self.root / name).write_bytes(b'\xff\xfe invalid')
                result = self.run_check()
                self.assertEqual(result.returncode, 1)
                self.assertIn(f'{name}: invalid UTF-8', result.stderr)
                self.assertNotIn('Traceback', result.stderr)

    def test_duplicate_definition_fails(self):
        cases = (
            ('packaging/arch/PKGBUILD', f'pkgver={VERSION}\n', f'pkgver={VERSION}\npkgver={VERSION}\n'),
            ('addons/fcitx5/CMakeLists.txt', f' VERSION {VERSION} LANGUAGES CXX)\n', f' VERSION {VERSION} LANGUAGES CXX)\nproject(fcitx5-tgskk VERSION {VERSION} LANGUAGES CXX)\n'),
            ('addons/fcitx5/tgskk.conf', f'Version={VERSION}\n', f'Version={VERSION}\nVersion={VERSION}\n'),
            ('packaging/arch/.SRCINFO', f'\tpkgver = {VERSION}\n', f'\tpkgver = {VERSION}\n\tpkgver = {VERSION}\n'),
        )
        for name, old, new in cases:
            with self.subTest(name=name):
                self.reset()
                self.edit(name, old, new)
                self.assert_rejected(name)
                self.assertIn('definitions found', self.run_check().stderr)

    def test_srcinfo_pkgrel_must_match_pkgbuild(self):
        for name, old, new in (
            ('packaging/arch/.SRCINFO', f'\tpkgrel = {PKGREL}\n', f'\tpkgrel = {OTHER_PKGREL}\n'),
            ('packaging/arch/PKGBUILD', f'pkgrel={PKGREL}\n', f'pkgrel={OTHER_PKGREL}\n'),
        ):
            with self.subTest(name=name):
                self.reset()
                self.edit(name, old, new)
                result = self.run_check()
                self.assertEqual(result.returncode, 1)
                self.assertIn('packaging/arch/.SRCINFO: pkgrel', result.stderr)
                self.assertIn('packaging/arch/PKGBUILD pkgrel', result.stderr)

    def test_shared_pkgrel_change_is_accepted(self):
        self.edit('packaging/arch/PKGBUILD', f'pkgrel={PKGREL}\n', f'pkgrel={OTHER_PKGREL}\n')
        self.edit('packaging/arch/.SRCINFO', f'\tpkgrel = {PKGREL}\n', f'\tpkgrel = {OTHER_PKGREL}\n')
        self.edit('packaging/arch/.SRCINFO', f'tgskk-common-git={VERSION}-{PKGREL}', f'tgskk-common-git={VERSION}-{OTHER_PKGREL}', -1)
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_srcinfo_expanded_versions_must_match(self):
        cases = (
            f'\tprovides = tgskk-common={VERSION}\n',
            f'\tdepends = tgskk-common-git={VERSION}-{PKGREL}\n',
        )
        for line in cases:
            with self.subTest(line=line):
                self.reset()
                self.edit('packaging/arch/.SRCINFO', line, line.replace(VERSION, WRONG_VERSION))
                self.assert_rejected('packaging/arch/.SRCINFO')

    def test_depends_missing_in_one_package_section_fails(self):
        self.edit(
            'packaging/arch/.SRCINFO',
            f'\tdepends = tgskk-common-git={VERSION}-{PKGREL}\n\tprovides = tgskk-ibus\n',
            '\tprovides = tgskk-ibus\n',
        )
        result = self.run_check()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, '')
        self.assertIn('packaging/arch/.SRCINFO: tgskk-ibus-git:', result.stderr)
        self.assertNotIn('tgskk-git:', result.stderr)
        self.assertNotIn('Traceback', result.stderr)

    def test_changelog_newest_entry_is_the_one_checked(self):
        self.edit('packaging/debian/changelog', 'tgskk (', f'tgskk ({WRONG_VERSION}-1)\n\ntgskk (')
        self.assert_rejected('packaging/debian/changelog')

    def test_broken_leading_changelog_entry_does_not_fall_through(self):
        entries = f'tgskk {VERSION}-2 unstable; urgency=medium\n\ntgskk ({VERSION}-1) unstable; urgency=medium\n\n'
        self.edit('packaging/debian/changelog', 'tgskk (', entries + 'tgskk (')
        self.assert_rejected('packaging/debian/changelog')
        self.assertIn('leading entry', self.run_check().stderr)


if __name__ == '__main__':
    unittest.main()
