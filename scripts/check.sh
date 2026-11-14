#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

bash scripts/lint.sh "$@"
python3 -m unittest discover -s scripts/tests
cargo test --workspace --locked "$@"
cargo test --manifest-path third_party/gskkserv/Cargo.toml --target-dir "$project_root/target" --locked "$@"
CARGO_TARGET_DIR="$project_root/target" cargo test \
  --manifest-path third_party/cskk/Cargo.toml --workspace --features cskk/capi --locked "$@"
