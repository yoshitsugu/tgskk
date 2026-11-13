#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
: "${1:?Usage: export-source.sh OUTPUT.tar.gz}"
# The Git index is the boundary for source intended for distribution.
git ls-files --cached -z | tar --null --files-from=- --create --gzip --file="$1"
