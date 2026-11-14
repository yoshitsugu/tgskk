#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
export CARGO_TARGET_DIR="$project_root/target"

for manifest in Cargo.toml third_party/cskk/Cargo.toml third_party/gskkserv/Cargo.toml; do
  cargo fmt --manifest-path "$manifest" --all --check
  cargo clippy --manifest-path "$manifest" --workspace --all-targets --all-features \
    --no-deps --locked "$@" -- -D warnings
done
