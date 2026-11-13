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
    cmake -S addons/fcitx5 -B build/fcitx5 -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr -DTGSKK_INSTALL_GOOGLE_HELPER=OFF
    cmake --build build/fcitx5
    cmake -S addons/ibus -B build/ibus -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr -DTGSKK_INSTALL_GOOGLE_HELPER=OFF
    cmake --build build/ibus
    ;;
  check)
    cargo test --workspace --locked --offline
    cargo test --manifest-path third_party/gskkserv/Cargo.toml --locked --offline --target-dir "$project_root/target"
    python3 -m unittest discover -s scripts/tests
    ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
    ctest --test-dir build/ibus --output-on-failure --no-tests=error
    ;;
  install)
    : "${2:?Usage: build-dist.sh install DESTDIR [common|fcitx5|ibus|all]}"
    destination="$(realpath -m "$2")"
    case "${3:-all}" in
      common)
        DESTDIR="$destination" cmake --install build/zenz
        install -Dm755 target/release/gskkserv "$destination/usr/libexec/tgskk-google-skkserv"
        install -Dm644 third_party/gskkserv/LICENSE-APACHE "$destination/usr/share/licenses/tgskk/google-ime-skkserv/LICENSE-APACHE"
        install -Dm644 third_party/gskkserv/UPSTREAM.md "$destination/usr/share/licenses/tgskk/google-ime-skkserv/UPSTREAM.md"
        install -Dm644 LICENSE "$destination/usr/share/licenses/tgskk/LICENSE"
        install -Dm644 third_party/cskk/LICENSE.txt "$destination/usr/share/licenses/tgskk/cskk-LICENSE"
        install -Dm644 README.md "$destination/usr/share/doc/tgskk/README.md"
        for doc in usage ibus distribution; do
          install -Dm644 "docs/$doc.md" "$destination/usr/share/doc/tgskk/docs/$doc.md"
        done
        ;;
      fcitx5|ibus) DESTDIR="$destination" cmake --install "build/$3" ;;
      all)
        for component in common fcitx5 ibus; do
          bash scripts/build-dist.sh install "$destination" "$component"
        done
        ;;
      *) echo 'Expected common, fcitx5 or ibus' >&2; exit 2 ;;
    esac
    ;;
  *)
    echo 'Usage: build-dist.sh {prepare|build|check|install DESTDIR [common|fcitx5|ibus|all]}' >&2
    exit 2
    ;;
esac
