#!/usr/bin/env python3
"""Check first-party C/C++ formatting or analyze configured CMake builds."""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SOURCE_ROOTS = (ROOT / "addons", ROOT / "native", ROOT / "crates/tgskk-ffi/include")
EXTENSIONS = {".c", ".cpp", ".h", ".hpp"}


def owned(path):
    return path.suffix in EXTENSIONS and any(path.is_relative_to(root) for root in SOURCE_ROOTS)


def format_sources(executable, fix):
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    sources = [str(ROOT / name) for name in tracked if name and owned(ROOT / name)]
    subprocess.run([executable, *(["-i"] if fix else ["--dry-run", "--Werror"]), *sources],
                   cwd=ROOT, check=True)


def tidy_build(executable, build):
    build = build.resolve()
    entries = json.loads((build / "compile_commands.json").read_text(encoding="utf-8"))
    sources = set()
    for entry in entries:
        path = Path(entry["file"])
        if not path.is_absolute():
            path = Path(entry["directory"]) / path
        path = path.resolve()
        if owned(path):
            sources.add(path)
    if not sources:
        raise RuntimeError(f"No first-party C/C++ sources in {build}/compile_commands.json")
    for source in sorted(sources):
        subprocess.run([executable, "-p", str(build), str(source)], cwd=ROOT, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    formatting = commands.add_parser("format")
    formatting.add_argument("--executable", default="clang-format")
    formatting.add_argument("--fix", action="store_true")
    tidy = commands.add_parser("tidy")
    tidy.add_argument("--executable", default="clang-tidy")
    tidy.add_argument("build", type=Path, nargs="+")
    args = parser.parse_args()
    if args.command == "format":
        format_sources(args.executable, args.fix)
    else:
        for build in args.build:
            tidy_build(args.executable, build)


if __name__ == "__main__":
    main()
