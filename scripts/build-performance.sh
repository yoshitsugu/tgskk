#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
reference_root="$(cd "${1:?Usage: build-performance.sh REFERENCE_ROOT}" && pwd)"
cd "$project_root"

for variant in baseline candidate; do
  source_root="$project_root"
  if [[ "$variant" == baseline ]]; then source_root="$reference_root"; fi
  output="$project_root/build/performance/$variant"
  cargo build --manifest-path "$source_root/Cargo.toml" --release -p tgskk-ffi \
    --locked --target-dir "$output/target"
  mkdir -p "$output"
  g++ -O3 -std=c++17 -Wall -Wextra -Werror -I "$source_root/crates/tgskk-ffi/include" \
    benchmarks/input.cpp "$output/target/release/libtgskk_ffi.a" \
    -lxkbcommon -ldl -lpthread -lm -o "$output/input"
  python3 "$source_root/scripts/setup-zenz.py" --jobs 2
done
