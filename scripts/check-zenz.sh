#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
zenz_worker="$project_root/build/zenz/tgskk-zenz-worker"
zenz_model="$project_root/build/models/zenz-v3.2-small-Q5_K_M.gguf"
if [[ ! -x "$zenz_worker" || ! -f "$zenz_model" ]]; then
  echo 'Run python3 scripts/setup-zenz.py first.' >&2
  exit 1
fi
cmake --build build/zenz
ctest --test-dir build/zenz --output-on-failure --no-tests=error
cargo build --release --locked -p tgskk-ffi -p tgskk-zenz "$@"
cargo build --release --locked --manifest-path third_party/gskkserv/Cargo.toml --target-dir "$project_root/target" "$@"
cargo build --release --locked -p tgskk-zenz --examples "$@"
cmake -S addons/fcitx5 -B build/fcitx5-zenz-test -DCMAKE_BUILD_TYPE=RelWithDebInfo \
  -DTGSKK_ZENZ_WORKER="$zenz_worker" -DTGSKK_ZENZ_MODEL="$zenz_model"
cmake --build build/fcitx5-zenz-test
TGSKK_TEST_ZENZ=1 ctest --test-dir build/fcitx5-zenz-test --output-on-failure
./target/release/examples/evaluate "$zenz_worker" "$zenz_model" examples/zenz-cases.tsv 60
./target/release/examples/evaluate "$zenz_worker" "$zenz_model" examples/zenz-cases.tsv 0

if [[ -f /usr/share/skk/SKK-JISYO.L ]]; then
  ./target/release/examples/evaluate "$zenz_worker" "$zenz_model" examples/zenz-cases.tsv 30 /usr/share/skk/SKK-JISYO.L euc-jp
  ./target/release/examples/evaluate "$zenz_worker" "$zenz_model" examples/zenz-cases.tsv 60 /usr/share/skk/SKK-JISYO.L euc-jp
fi
