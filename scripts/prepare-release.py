#!/usr/bin/env python3
"""Bundle tested packages without mixing distribution-specific filenames."""
import argparse
import hashlib
from pathlib import Path
import re
import tarfile
import tomllib

TARGETS = ('arch', 'debian-13', 'ubuntu-24.04', 'ubuntu-26.04')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--target', choices=TARGETS, required=True)
    parser.add_argument('--package-root', type=Path, default=Path('build/packages'))
    parser.add_argument('--output-dir', type=Path, default=Path('build/release'))
    args = parser.parse_args()
    version = tomllib.loads((Path(__file__).resolve().parent.parent / 'Cargo.toml').read_text())['workspace']['package']['version']
    if not re.fullmatch(r'v\d+\.\d+\.\d+', args.tag) or args.tag != f'v{version}':
        parser.error('tag must match the workspace version: v' + version)
    if args.target == 'arch':
        names = ('tgskk-common-git', 'tgskk-git', 'tgskk-ibus-git')
        ending = r'-' + re.escape(version) + r'-(\d+)-x86_64\.pkg\.tar\.zst'
    else:
        names = ('tgskk-common', 'tgskk', 'tgskk-ibus')
        ending = '_' + re.escape(version) + r'-(\d+)_amd64\.deb'
    directory = args.package_root / args.target
    packages = sorted(directory.glob('*'))
    if len(packages) != 3 or any(sum(bool(re.fullmatch(re.escape(name) + ending, p.name)) for p in packages) != 1 for name in names):
        parser.error(f'{directory}: expected exactly three packages for {version}')
    revisions = {re.fullmatch(re.escape(name) + ending, p.name).group(1)
                 for name in names for p in packages if re.fullmatch(re.escape(name) + ending, p.name)}
    if len(revisions) != 1:
        parser.error('all three packages must have the same package revision')
    if any(not p.is_file() or p.is_symlink() for p in packages):
        parser.error('packages must be regular files')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    archive = args.output_dir / f'tgskk-{args.tag}-{args.target}-x86_64.tar.gz'
    with tarfile.open(archive, 'w:gz') as tar:
        for package in packages:
            tar.add(package, arcname=package.name)
    digest = hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest()
    archive.with_name(archive.name + '.sha256').write_text(f'{digest}  {archive.name}\n')
    print(archive)


if __name__ == '__main__':
    main()
