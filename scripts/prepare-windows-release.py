#!/usr/bin/env python3
"""Verify a tested Windows bundle before publishing its files separately."""
import argparse
import hashlib
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def verify(directory, tag, revision):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('revision must be a full Git commit ID')
    if tag != f'windows-preview-v{version}-{revision[:12]}':
        raise ValueError('preview tag must match the workspace version and commit')
    manifest = directory / 'SHA256SUMS'
    if manifest.is_symlink() or not manifest.is_file():
        raise ValueError('missing regular SHA256SUMS file')
    entries = {}
    for line in manifest.read_text(encoding='utf-8').splitlines():
        match = re.fullmatch(r'([0-9a-f]{64}) [ *](?:\./)?([A-Za-z0-9_.-]+)', line)
        if not match or match[2] in entries:
            raise ValueError('malformed or duplicate checksum entry')
        entries[match[2]] = match[1]
    required = {
        f'tgskk-{version}-windows-x64-setup.exe',
        f'tgskk-{version}-windows-x64.zip',
        'tgskk-source.tar.gz',
    }
    sources = {name for name in entries if re.fullmatch(r'llama-[0-9a-f]{40}\.tar\.gz', name)}
    if len(sources) != 1 or set(entries) != required | sources:
        raise ValueError('expected installer, ZIP, complete tgskk source and llama.cpp source')
    if {p.name for p in directory.iterdir()} != set(entries) | {'SHA256SUMS'}:
        raise ValueError('unexpected or missing release files')
    for name, expected in entries.items():
        path = directory / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'{name}: expected a regular file')
        with path.open('rb') as source:
            if hashlib.file_digest(source, 'sha256').hexdigest() != expected:
                raise ValueError(f'{name}: checksum mismatch')
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, default=Path('build/windows-release'))
    parser.add_argument('--tag', required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--signing', choices=('signed', 'unsigned'), required=True)
    parser.add_argument('--notes', type=Path, default=Path('build/windows-release-notes.md'))
    args = parser.parse_args()
    version = verify(args.directory, args.tag, args.revision)
    signing_notice = (
        'セットアップEXEはAuthenticode署名付きです。'
        if args.signing == 'signed' else
        'セットアップEXEは未署名です。Windowsが実行時に警告を表示したり、実行をブロックしたりする場合があります。'
    )
    args.notes.parent.mkdir(parents=True, exist_ok=True)
    args.notes.write_text(
        f'Windows x64向けの試用版です（v{version}、コミット `{args.revision}`）。'
        f'{signing_notice}\n\n'
        f'通常の導入には **`tgskk-{version}-windows-x64-setup.exe`だけ**をダウンロードしてください。'
        '辞書・AIモデル・設定GUIを同梱しています。GitHubへのログインは不要です。\n\n'
        '1. `SHA256SUMS`とPowerShellの`Get-FileHash`でインストーラーのSHA256を照合します。\n'
        '2. インストーラーを実行し、管理者権限を許可します。\n'
        '3. Windowsの日本語の入力方式にtgskkを追加し、Win＋Spaceで切り替えます。\n\n'
        '設定はスタートメニューの「tgskk settings」から変更できます。'
        '保存した設定は、次に入力を再開したときに反映されます。\n\n'
        '手動登録用ZIPと、依存ライブラリを含むソースは別の添付ファイルです。'
        '通常のインストール時にソースをダウンロードする必要はありません。'
        '32bit・ARM64・管理者権限のアプリは対象外です。\n\n'
        f'[導入・更新・削除の詳細](https://github.com/yoshitsugu/tgskk/blob/{args.revision}/addons/windows/README.md)\n\n'
        f'[Code signing policy](https://github.com/yoshitsugu/tgskk/blob/{args.revision}/addons/windows/README.md#code-signing-policy)\n',
        encoding='utf-8',
    )


if __name__ == '__main__':
    main()
