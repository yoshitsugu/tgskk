#!/usr/bin/env python3
"""Fetch verified Zenz/llama.cpp artifacts and build the local CPU worker."""
import argparse
from pathlib import Path
import subprocess
import tarfile
from zenz_model import MODEL_NAME, MODEL_SHA, MODEL_URL, verified_download

ROOT = Path(__file__).resolve().parent.parent
REVISION = "52e53a76df0298d2c316ae7e704eeec03b0bf478"
SOURCE_SHA = "68c7379ed21b7d896a0bf6e3414b15655ef6f58eb50e75a41311ad50673f92e4"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true", help="Use previously downloaded, verified artifacts")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--prefix", default="/usr")
    parser.add_argument("--no-model", action="store_true", help="Build without downloading or installing model weights")
    parser.add_argument("--prepare-only", action="store_true", help="Fetch and patch sources without building")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    downloads = ROOT / "build/downloads"
    models = ROOT / "build/models"
    downloads.mkdir(parents=True, exist_ok=True)
    models.mkdir(parents=True, exist_ok=True)
    archive = downloads / f"llama-{REVISION}.tar.gz"
    model = models / MODEL_NAME
    verified_download(f"https://codeload.github.com/ggml-org/llama.cpp/tar.gz/{REVISION}", archive, SOURCE_SHA, args.offline)
    if not args.no_model:
        verified_download(MODEL_URL, model, MODEL_SHA, args.offline)
    source = ROOT / "build" / f"llama.cpp-{REVISION}"
    if not source.exists():
        with tarfile.open(archive) as package:
            package.extractall(ROOT / "build", filter="data")
    vocab = source / "src/llama-vocab.cpp"
    text = vocab.read_text(encoding="utf-8")
    alias = 'tokenizer_pre == "gpt2-small-japanese-char"'
    if alias not in text:
        anchor = 'tokenizer_pre == "jina-v2-de" ||'
        if text.count(anchor) != 1:
            raise RuntimeError("llama.cpp tokenizer patch no longer applies")
        vocab.write_text(text.replace(anchor, anchor + '\n                    ' + alias + ' ||'), encoding="utf-8")
    if args.prepare_only:
        return
    subprocess.run([
        "cmake", "-S", str(ROOT / "native/zenz"), "-B", str(ROOT / "build/zenz"),
        "-DCMAKE_BUILD_TYPE=Release", f"-DCMAKE_INSTALL_PREFIX={args.prefix}",
        f"-DTGSKK_LLAMA_SOURCE={source}", f"-DTGSKK_ZENZ_MODEL_FILE={'' if args.no_model else model}",
    ], check=True)
    subprocess.run(["cmake", "--build", str(ROOT / "build/zenz"), "-j", str(args.jobs)], check=True)
    print("Built build/zenz/tgskk-zenz-worker; no running input method was changed.")


if __name__ == "__main__":
    main()
