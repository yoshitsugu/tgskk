#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
cargo build --release --locked -p tgskk-ffi "$@"
cargo build --release --locked --manifest-path third_party/gskkserv/Cargo.toml --target-dir "$project_root/target" "$@"
cmake -S addons/ibus -B build/ibus -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr
cmake --build build/ibus -j "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"
ctest --test-dir build/ibus --output-on-failure --no-tests=error
