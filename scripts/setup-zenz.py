#!/usr/bin/env python3
"""Fetch verified Zenz/llama.cpp artifacts and build the local CPU worker."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
REVISION = "52e53a76df0298d2c316ae7e704eeec03b0bf478"
MODEL_REVISION = "c67e03e07d215c869f591b274c1631170d3e11fe"
SOURCE_SHA = "68c7379ed21b7d896a0bf6e3414b15655ef6f58eb50e75a41311ad50673f92e4"
MODEL_SHA = "29c223d4c23327b80fd13ebb5ab2555057a46317997d5da391584ffbef0db673"


def verified_download(url, destination, expected, offline):
    if destination.exists():
        with destination.open("rb") as data:
            if hashlib.file_digest(data, "sha256").hexdigest() == expected:
                return
    if offline:
        raise RuntimeError(f"Verified artifact missing: {destination}")
    partial = destination.with_suffix(destination.suffix + ".part")
    try:
        print(f"Downloading {destination.name}", flush=True)
        with urllib.request.urlopen(url, timeout=60) as response, partial.open("wb") as output:
            shutil.copyfileobj(response, output)
        with partial.open("rb") as data:
            if hashlib.file_digest(data, "sha256").hexdigest() != expected:
                raise RuntimeError(f"SHA256 mismatch: {destination.name}")
        partial.replace(destination)
    finally:
        partial.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true", help="Use previously downloaded, verified artifacts")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--prefix", default="/usr")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    downloads = ROOT / "build/downloads"
    models = ROOT / "build/models"
    downloads.mkdir(parents=True, exist_ok=True)
    models.mkdir(parents=True, exist_ok=True)
    archive = downloads / f"llama-{REVISION}.tar.gz"
    model = models / "zenz-v3.2-small-Q5_K_M.gguf"
    verified_download(f"https://codeload.github.com/ggml-org/llama.cpp/tar.gz/{REVISION}", archive, SOURCE_SHA, args.offline)
    verified_download(
        f"https://huggingface.co/Miwa-Keita/zenz-v3.2-small-gguf/resolve/{MODEL_REVISION}/ggml-model-Q5_K_M.gguf",
        model, MODEL_SHA, args.offline,
    )
    source = ROOT / "build" / f"llama.cpp-{REVISION}"
    if not source.exists():
        with tarfile.open(archive) as package:
            package.extractall(ROOT / "build", filter="data")
    vocab = source / "src/llama-vocab.cpp"
    text = vocab.read_text()
    alias = 'tokenizer_pre == "gpt2-small-japanese-char"'
    if alias not in text:
        anchor = 'tokenizer_pre == "jina-v2-de" ||'
        if text.count(anchor) != 1:
            raise RuntimeError("llama.cpp tokenizer patch no longer applies")
        vocab.write_text(text.replace(anchor, anchor + '\n                    ' + alias + ' ||'))
    subprocess.run([
        "cmake", "-S", str(ROOT / "native/zenz"), "-B", str(ROOT / "build/zenz"),
        "-DCMAKE_BUILD_TYPE=Release", f"-DCMAKE_INSTALL_PREFIX={args.prefix}",
        f"-DTGSKK_LLAMA_SOURCE={source}", f"-DTGSKK_ZENZ_MODEL_FILE={model}",
    ], check=True)
    subprocess.run(["cmake", "--build", str(ROOT / "build/zenz"), "-j", str(args.jobs)], check=True)
    print("Built build/zenz/tgskk-zenz-worker; no running input method was changed.")


if __name__ == "__main__":
    main()
