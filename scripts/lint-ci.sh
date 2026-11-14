#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

command -v actionlint >/dev/null
command -v shellcheck >/dev/null
actionlint -shellcheck "$(command -v shellcheck)"

mapfile -d '' -t scripts < <(git ls-files -z -- 'scripts/*.sh' 'packaging/*.sh' 'addons/*.sh')
shellcheck "${scripts[@]}"
