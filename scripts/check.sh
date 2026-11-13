#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

cargo fmt --all --check
cargo test --workspace --locked "$@"
CARGO_TARGET_DIR="$project_root/target" cargo test \
  --manifest-path third_party/cskk/Cargo.toml --workspace --features cskk/capi --locked "$@"
cargo clippy --workspace --all-targets --no-deps --locked "$@" -- -D warnings
