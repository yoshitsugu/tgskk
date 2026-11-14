#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

for manifest in Cargo.toml third_party/cskk/Cargo.toml third_party/gskkserv/Cargo.toml; do
  # One policy covers three graphs; allowances may be unused in an individual graph.
  cargo deny --manifest-path "$manifest" --workspace --locked check \
    --config "$project_root/deny.toml" -D warnings \
    -A license-not-encountered -A unmatched-skip -A advisory-not-detected "$@"
done
