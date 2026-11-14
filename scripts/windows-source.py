#!/usr/bin/env python3
"""Bundle the exact tracked sources and all locked Rust registry dependencies."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'build/windows-package')
    parser.add_argument('--directory', type=Path, default=ROOT / 'build/windows-source')
    args = parser.parse_args()
    args.directory = args.directory.resolve()
    args.output = args.output.resolve()
    if (args.output / 'tgskk-source.tar.gz').exists():
        raise FileExistsError('Source archive already exists')
    args.directory.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryFile() as archive:
        subprocess.run(['git', 'archive', '--format=tar', 'HEAD'], cwd=ROOT, stdout=archive, check=True)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as package:
            package.extractall(args.directory, filter='data')
    subprocess.run(['cargo', '+1.88.0', 'vendor', '--locked', '--versioned-dirs',
                    '--sync', str(ROOT / 'third_party/gskkserv/Cargo.toml'),
                    '--sync', str(ROOT / 'third_party/cskk/Cargo.toml'),
                    str(args.directory / 'vendor')], cwd=ROOT, check=True, stdout=subprocess.PIPE)
    configuration = args.directory / '.cargo/config.toml'
    configuration.parent.mkdir(exist_ok=True)
    if configuration.exists():
        raise RuntimeError('Refusing to replace tracked Cargo configuration')
    configuration.write_text('[source.crates-io]\nreplace-with = "vendored-sources"\n'
                             '[source.vendored-sources]\ndirectory = "vendor"\n', encoding='utf-8')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    (args.directory / 'SOURCE_REVISION').write_text(revision + '\n', encoding='utf-8')
    args.output.mkdir(parents=True, exist_ok=True)
    shutil.make_archive(str(args.output / 'tgskk-source'), 'gztar', root_dir=args.directory)


if __name__ == '__main__':
    main()
