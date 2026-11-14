#!/usr/bin/env python3
"""Stage tgskk Windows x64 binaries into a distribution ZIP."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import re
import struct
import sys
import tomllib
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import zenz_model

REQUIRED_BINARIES = (
    "tgskk-tip.dll",
    "tgskk-broker.exe",
    "tgskk-settings.exe",
    "tgskk-zenz-worker.exe",
    "tgskk-google-skkserv.exe",
)

REQUIRED_RUNTIME_DLLS = {
    "msvcp140.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
}


def validate_pe(path: Path) -> None:
    """Raise ValueError if *path* is not a valid x64 PE file."""
    data = path.read_bytes()
    if len(data) < 0x40:
        raise ValueError(f"{path}: file too small for PE")
    if data[:2] != b"MZ":
        raise ValueError(f"{path}: missing DOS signature")
    e_lfanew = struct.unpack("<I", data[0x3C:0x40])[0]
    if len(data) < e_lfanew + 6:
        raise ValueError(f"{path}: truncated PE header")
    if data[e_lfanew : e_lfanew + 4] != b"PE\x00\x00":
        raise ValueError(f"{path}: missing PE signature")
    machine = struct.unpack("<H", data[e_lfanew + 4 : e_lfanew + 6])[0]
    if machine != 0x8664:
        raise ValueError(f"{path}: machine 0x{machine:04x} is not x64")


def shasum(path: Path) -> str:
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def copy_file(src: Path, dst: Path) -> None:
    if src.is_symlink():
        raise ValueError(f"{src}: symlinks are not allowed")
    shutil.copyfile(src, dst)


def stage(args: argparse.Namespace) -> None:
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]

    payload = args.output / "payload"
    archive = args.output / f"tgskk-{version}-windows-x64.zip"
    if archive.exists():
        raise FileExistsError(f"Archive already exists: {archive}")
    if payload.exists():
        raise FileExistsError(f"{payload}: already exists")

    payload.mkdir(parents=True)
    created_archive = False
    try:

        for name in REQUIRED_BINARIES:
            src = args.binaries / name
            if not src.exists():
                raise FileNotFoundError(f"{src}: required binary missing")
            if src.is_symlink():
                raise ValueError(f"{src}: symlinks are not allowed")
            validate_pe(src)
            copy_file(src, payload / name)

        runtime_dlls = {
            p.name: p
            for p in args.runtime.iterdir()
            if p.is_file() and p.suffix.lower() == ".dll"
        }
        missing = REQUIRED_RUNTIME_DLLS - set(runtime_dlls)
        if missing:
            raise FileNotFoundError(
                f"missing required runtime DLLs: {', '.join(sorted(missing))}"
            )
        for name, src in sorted(runtime_dlls.items()):
            if not re.fullmatch(r"(?:msvcp140\w*|vcruntime140\w*|concrt140|vccorlib140)\.dll", name, re.IGNORECASE):
                raise ValueError(f"Unexpected CRT file: {name}")
            if src.is_symlink():
                raise ValueError(f"{src}: symlinks are not allowed")
            validate_pe(src)
            copy_file(src, payload / name)

        if args.dictionary.is_symlink():
            raise ValueError(f"{args.dictionary}: symlinks are not allowed")
        copy_file(args.dictionary, payload / "SKK-JISYO.L")

        if args.model.is_symlink():
            raise ValueError(f"{args.model}: symlinks are not allowed")
        model_dst = payload / "models"
        model_dst.mkdir()
        copy_file(args.model, model_dst / zenz_model.MODEL_NAME)
        model_hash = shasum(model_dst / zenz_model.MODEL_NAME)
        if model_hash != zenz_model.MODEL_SHA:
            raise ValueError(f"Staged model SHA256 mismatch (expected {zenz_model.MODEL_SHA}, got {model_hash})")

        licenses = payload / "licenses"
        licenses.mkdir()
        copy_file(root / "LICENSE", licenses / "LICENSE")
        copy_file(root / "third_party" / "cskk" / "LICENSE.txt", licenses / "cskk-LICENSE.txt")
        copy_file(
            root / "third_party" / "gskkserv" / "LICENSE-APACHE",
            licenses / "gskkserv-LICENSE-APACHE",
        )
        copy_file(
            root / "third_party" / "gskkserv" / "UPSTREAM.md",
            licenses / "gskkserv-UPSTREAM.md",
        )
        copy_file(root / "native" / "zenz" / "NOTICE.md", licenses / "NOTICE.md")
        copy_file(root / "packaging/windows/dictionary-Unicode-LICENSE.txt", licenses / "dictionary-Unicode-LICENSE.txt")
        copy_file(root / "packaging/windows/NOTICE.md", licenses / "dictionary-NOTICE.md")

        rust = licenses / "rust"
        rust.mkdir()
        index = []
        for crate in sorted(args.vendor.iterdir()):
            manifest = crate / "Cargo.toml"
            if not crate.is_dir() or crate.is_symlink() or not manifest.is_file():
                raise ValueError(f"Invalid vendored crate: {crate}")
            package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
            target = rust / crate.name
            target.mkdir()
            notices = [path for path in crate.iterdir()
                       if path.name.lower().startswith(("license", "licence", "copying", "copyright", "notice"))]
            explicit = package.get("license-file", package.get("license_file"))
            if explicit:
                notice = (crate / explicit).resolve()
                notice.relative_to(crate.resolve())
                notices.append(notice)
            for notice in set(notices):
                if notice.is_symlink():
                    raise ValueError(f"Symlink in dependency notice: {notice}")
                if notice.is_dir():
                    for item in notice.rglob("*"):
                        if item.is_file():
                            destination = target / item.relative_to(crate)
                            destination.parent.mkdir(parents=True, exist_ok=True)
                            copy_file(item, destination)
                else:
                    copy_file(notice, target / notice.name)
            index.append({"name": package["name"], "version": package["version"],
                          "license": package.get("license"), "repository": package.get("repository"),
                          "authors": package.get("authors", []), "source": f"vendor/{crate.name}"})
        if not index:
            raise ValueError("The vendored dependency directory is empty")
        (rust / "index.json").write_text(json.dumps(index, indent=2) + "\n", encoding="utf-8")

        llama_license = args.llama_source / "LICENSE"
        if not llama_license.exists():
            raise FileNotFoundError(f"{llama_license}: llama LICENSE missing")
        if llama_license.is_symlink():
            raise ValueError(f"{llama_license}: symlinks are not allowed")
        copy_file(llama_license, licenses / "llama-LICENSE")

        # Add-ons README
        addon_dir = payload / "addons" / "windows"
        addon_dir.mkdir(parents=True)
        copy_file(root / "addons" / "windows" / "README.md", addon_dir / "README.md")

        # SHA256SUMS (exclude the checksum file itself)
        lines = []
        for path in sorted(payload.rglob("*")):
            if path.is_file() and path.name != "SHA256SUMS":
                rel = path.relative_to(payload).as_posix()
                lines.append(f"{shasum(path)}  {rel}\n")
        (payload / "SHA256SUMS").write_text("".join(lines))

        # ZIP with tgskk/ prefix
        archive = args.output / f"tgskk-{version}-windows-x64.zip"
        with archive.open("xb") as destination:
            created_archive = True
            with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as zf:
                for path in sorted(payload.rglob("*")):
                    if path.is_file():
                        arcname = "tgskk/" + path.relative_to(payload).as_posix()
                        zf.write(path, arcname)
    except Exception:
        if created_archive:
            archive.unlink(missing_ok=True)
        if payload.exists():
            shutil.rmtree(payload, ignore_errors=True)
        raise


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", type=Path, required=True, metavar="DIR")
    parser.add_argument("--dictionary", type=Path, required=True, metavar="FILE")
    parser.add_argument("--model", type=Path, required=True, metavar="FILE")
    parser.add_argument("--runtime", type=Path, required=True, metavar="DIR")
    parser.add_argument("--llama-source", type=Path, required=True, metavar="DIR")
    parser.add_argument("--vendor", type=Path, required=True, metavar="DIR")
    parser.add_argument("--output", type=Path, required=True, metavar="DIR")
    args = parser.parse_args(argv)

    checks = (
        ("binaries", args.binaries, True),
        ("vendor", args.vendor, True),
        ("dictionary", args.dictionary, False),
        ("model", args.model, False),
        ("runtime", args.runtime, True),
        ("llama-source", args.llama_source, True),
    )
    for name, path, need_dir in checks:
        if not path.exists():
            parser.error(f"{name}: {path} does not exist")
        if path.is_symlink():
            parser.error(f"{name}: {path} is a symlink")
        if need_dir:
            if not path.is_dir():
                parser.error(f"{name}: {path} is not a directory")
        else:
            if not path.is_file():
                parser.error(f"{name}: {path} is not a regular file")

    if args.output.exists():
        if args.output.is_symlink():
            parser.error(f"output: {args.output} is a symlink")
        if not args.output.is_dir():
            parser.error(f"output: {args.output} is not a directory")

    try:
        stage(args)
    except (FileExistsError, FileNotFoundError, ValueError) as exc:
        parser.error(str(exc))


if __name__ == "__main__":
    main()
