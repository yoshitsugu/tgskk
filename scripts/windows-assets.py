#!/usr/bin/env python3
"""Fetch the pinned SKK large dictionary used by the Windows package."""
import argparse
from pathlib import Path
from zenz_model import verified_download

REVISION = '14a1df7ec8f84410f5fb006978cf00878768150a'
DICTIONARY_SHA = '6cef65fa11ef0819a6bf5a0c7dae06c76526805325096ab016985489f82760a4'
URL = f'https://raw.githubusercontent.com/skk-dev/dict/{REVISION}/SKK-JISYO.L'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, default=Path('build/windows-assets'))
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    verified_download(URL, args.directory / 'SKK-JISYO.L', DICTIONARY_SHA, args.offline)


if __name__ == '__main__':
    main()
