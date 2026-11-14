#!/usr/bin/env python3
"""Fetch the pinned SKK large dictionary used by the Windows package."""
import argparse
from pathlib import Path
from zenz_model import verified_download

REVISION = '14a1df7ec8f84410f5fb006978cf00878768150a'
DICTIONARY_SHA = 'c791f578d1b4040fce282db29bc22b2cc7ea46f83e269fab2e0fa779e2967e40'
URL = f'https://raw.githubusercontent.com/skk-dev/dict/{REVISION}/SKK-JISYO.L'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, default=Path('build/windows-assets'))
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    verified_download(URL, args.directory / 'SKK-JISYO.L', DICTIONARY_SHA, args.offline)


if __name__ == '__main__':
    main()
