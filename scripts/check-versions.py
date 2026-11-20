#!/usr/bin/env python3
"""Check that every packaged release version matches the Cargo workspace version."""
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent

VERSION_FILES = (
    ('addons/fcitx5/CMakeLists.txt', r'^project\(fcitx5-tgskk VERSION (\S+)'),
    ('addons/ibus/CMakeLists.txt', r'^project\(ibus-tgskk VERSION (\S+)'),
    ('addons/windows/CMakeLists.txt', r'^project\(tgskk-windows VERSION (\S+)'),
    ('addons/fcitx5/tgskk.conf', r'^Version=(\S+)$'),
    ('packaging/arch/PKGBUILD', r'^pkgver=(\S+)'),
    ('packaging/arch/.SRCINFO', r'^\tpkgver = (\S+)$'),
)
# A changelog holds one entry per release, so only the leading entry is checked.
CHANGELOG = 'packaging/debian/changelog'
CHANGELOG_HEADER = r'tgskk \(([^\s)-]+)-[^\s)]+\)'
PKGREL_FILES = (
    ('packaging/arch/PKGBUILD', r'^pkgrel=(\S+)'),
    ('packaging/arch/.SRCINFO', r'^\tpkgrel = (\S+)$'),
)
SRCINFO = 'packaging/arch/.SRCINFO'
SRCINFO_EXPANDED = (
    (r'^\tprovides = tgskk-common=(\S+)$', 'tgskk-common', False),
    (r'^\tdepends = tgskk-common-git=(\S+)$', 'tgskk-common-git', True),
)


def read(path, errors):
    try:
        return (ROOT / path).read_text(encoding='utf-8')
    except UnicodeDecodeError:
        errors.append(f'{path}: invalid UTF-8')
    except OSError as error:
        errors.append(f'{path}: {error.strerror}')
    return None


def values(path, pattern, errors):
    text = read(path, errors)
    return None if text is None else re.findall(pattern, text, re.MULTILINE)


def single(path, label, pattern, errors):
    found = values(path, pattern, errors)
    if found is None:
        return None
    if not found:
        errors.append(f'{path}: {label} not found')
    elif len(found) > 1:
        errors.append(f'{path}: {len(found)} {label} definitions found')
    else:
        return found[0]
    return None


def workspace_version(errors):
    text = read('Cargo.toml', errors)
    if text is None:
        return None
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        errors.append(f'Cargo.toml: invalid TOML: {error}')
        return None
    try:
        return data['workspace']['package']['version']
    except (KeyError, TypeError):
        errors.append('Cargo.toml: [workspace.package] version not found')
        return None


def changelog_version(errors):
    text = read(CHANGELOG, errors)
    if text is None:
        return None
    line = text.lstrip('\n').partition('\n')[0]
    match = re.match(CHANGELOG_HEADER, line)
    if match is None:
        errors.append(f'{CHANGELOG}: version not found in the leading entry')
        return None
    return match.group(1)


def check_versions(expected, errors):
    for path, pattern in VERSION_FILES:
        value = single(path, 'version', pattern, errors)
        if value is not None and value != expected:
            errors.append(f'{path}: version {value} does not match {expected}')
    changelog = changelog_version(errors)
    if changelog is not None and changelog != expected:
        errors.append(f'{CHANGELOG}: version {changelog} does not match {expected}')
    pkgrels = {path: single(path, 'pkgrel', pattern, errors) for path, pattern in PKGREL_FILES}
    pkgbuild_rel = pkgrels['packaging/arch/PKGBUILD']
    srcinfo_rel = pkgrels[SRCINFO]
    if pkgbuild_rel is not None and srcinfo_rel is not None and pkgbuild_rel != srcinfo_rel:
        errors.append(f'{SRCINFO}: pkgrel {srcinfo_rel} does not match packaging/arch/PKGBUILD pkgrel {pkgbuild_rel}')
    if srcinfo_rel is None:
        return
    for pattern, name, uses_pkgrel in SRCINFO_EXPANDED:
        wanted = f'{expected}-{srcinfo_rel}' if uses_pkgrel else expected
        found = values(SRCINFO, pattern, errors)
        if found is not None and not found:
            errors.append(f'{SRCINFO}: {name} not found')
        for value in found or ():
            if value != wanted:
                errors.append(f'{SRCINFO}: {name} {value} does not match {wanted}')


def main():
    errors = []
    expected = workspace_version(errors)
    if expected is not None:
        check_versions(expected, errors)
    if errors:
        for error in dict.fromkeys(errors):
            print(error, file=sys.stderr)
        return 1
    print(f'All release versions match {expected}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
