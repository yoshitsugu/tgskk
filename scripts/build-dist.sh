#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CMAKE_BUILD_PARALLEL_LEVEL="${CMAKE_BUILD_PARALLEL_LEVEL:-2}"

case "${1:-}" in
  prepare)
    cargo fetch --locked
    cargo fetch --locked --manifest-path third_party/gskkserv/Cargo.toml
    python3 scripts/setup-zenz.py --prepare-only --no-model
    ;;
  build)
    cargo build --release --locked --offline -p tgskk-ffi
    cargo build --release --locked --offline --manifest-path third_party/gskkserv/Cargo.toml --target-dir "$project_root/target"
    python3 scripts/setup-zenz.py --offline --no-model --prefix /usr --jobs "$CMAKE_BUILD_PARALLEL_LEVEL"
    cmake -S addons/fcitx5 -B build/fcitx5 -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr
    cmake --build build/fcitx5
    ;;
  check)
    cargo test --workspace --locked --offline
    cargo test --manifest-path third_party/gskkserv/Cargo.toml --locked --offline --target-dir "$project_root/target"
    python3 -m unittest discover -s scripts/tests
    ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
    ;;
  install)
    : "${2:?Usage: build-dist.sh install DESTDIR}"
    destination="$(realpath -m "$2")"
    DESTDIR="$destination" cmake --install build/fcitx5
    DESTDIR="$destination" cmake --install build/zenz
    install -Dm644 LICENSE "$destination/usr/share/licenses/tgskk/LICENSE"
    install -Dm644 third_party/cskk/LICENSE.txt "$destination/usr/share/licenses/tgskk/cskk-LICENSE"
    install -Dm644 README.md "$destination/usr/share/doc/tgskk/README.md"
    install -Dm644 docs/usage.md "$destination/usr/share/doc/tgskk/usage.md"
    ;;
  *)
    echo 'Usage: build-dist.sh {prepare|build|check|install DESTDIR}' >&2
    exit 2
    ;;
esac
