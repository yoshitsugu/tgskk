#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

cargo build --release --locked -p tgskk-ffi "$@"
cargo build --release --locked --manifest-path third_party/gskkserv/Cargo.toml --target-dir "$project_root/target" "$@"
cmake -S addons/fcitx5 -B build/fcitx5 -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=/usr
cmake --build build/fcitx5
ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
