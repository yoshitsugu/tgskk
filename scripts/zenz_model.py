#!/usr/bin/env python3
"""Install the pinned Zenz model for tgskk; no compiler is required."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import tempfile
import urllib.request

MODEL_REVISION = "c67e03e07d215c869f591b274c1631170d3e11fe"
MODEL_SHA = "29c223d4c23327b80fd13ebb5ab2555057a46317997d5da391584ffbef0db673"
MODEL_NAME = "zenz-v3.2-small-Q5_K_M.gguf"
MODEL_URL = f"https://huggingface.co/Miwa-Keita/zenz-v3.2-small-gguf/resolve/{MODEL_REVISION}/ggml-model-Q5_K_M.gguf"


def digest(path):
    with path.open("rb") as data:
        return hashlib.file_digest(data, "sha256").hexdigest()


def verified_download(url, destination, expected, offline=False):
    if destination.exists() and digest(destination) == expected:
        return
    if offline:
        raise RuntimeError(f"Verified artifact missing or corrupt: {destination.name}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=".tgskk-download-", delete=False) as output:
            partial = Path(output.name)
            print(f"Downloading {destination.name}")
            with urllib.request.urlopen(url, timeout=60) as response:
                shutil.copyfileobj(response, output)
        if digest(partial) != expected:
            raise RuntimeError(f"SHA256 mismatch: {destination.name}")
        partial.chmod(0o644)
        partial.replace(destination)
    finally:
        if partial is not None:
            partial.unlink(missing_ok=True)


def default_directory():
    if os.environ.get("FCITX_DATA_HOME"):
        return Path(os.environ["FCITX_DATA_HOME"]) / "tgskk"
    data_home = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local/share")
    return Path(data_home) / "fcitx5/tgskk"


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog="Model by Keita Miwa, CC BY-SA 4.0: https://huggingface.co/Miwa-Keita/zenz-v3.2-small-gguf")
    parser.add_argument("--directory", type=Path, default=default_directory())
    parser.add_argument("--offline", action="store_true", help="Only verify an existing download")
    args = parser.parse_args()
    model = args.directory.expanduser().resolve() / MODEL_NAME
    try:
        verified_download(MODEL_URL, model, MODEL_SHA, args.offline)
    except (OSError, RuntimeError) as error:
        parser.exit(1, f"tgskk-setup-model: {error}\n")
    print(f"Verified model: {model}")
    print("Restart fcitx5 to load the model. For a custom directory, set ZenzModel to this path.")


if __name__ == "__main__":
    main()
