#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
target="${1:-}"
case "$target" in
  arch) base=archlinux:base; kind=arch ;;
  debian-13) base=debian:13; kind=deb ;;
  ubuntu-24.04) base=ubuntu:24.04; kind=deb ;;
  ubuntu-26.04) base=ubuntu:26.04; kind=deb ;;
  *) echo 'Usage: build-package.sh {arch|debian-13|ubuntu-24.04|ubuntu-26.04}' >&2; exit 2 ;;
esac
command -v docker >/dev/null || { echo 'Docker is required to build distribution packages.' >&2; exit 2; }
output="$project_root/build/packages/$target"
mkdir -p "$output"
archive="$(mktemp "$project_root/build/source-XXXXXX.tar.gz")"
trap 'rm -f "$archive"' EXIT
bash scripts/export-source.sh "$archive"
docker build -f packaging/Containerfile --build-arg "BASE=$base" -t "tgskk-build:$target" .
mounts=(--mount "type=bind,src=$archive,dst=/source.tar.gz,readonly" --mount "type=bind,src=$output,dst=/out")
if [[ -n "${TGSKK_REGISTRY_CACHE:-}" ]]; then
  mounts+=(--mount "type=bind,src=$TGSKK_REGISTRY_CACHE,dst=/cached-registry,readonly")
fi
if [[ -n "${TGSKK_DOWNLOAD_CACHE:-}" ]]; then
  mounts+=(--mount "type=bind,src=$TGSKK_DOWNLOAD_CACHE,dst=/cached-downloads,readonly")
fi
if [[ -n "${TGSKK_TEST_MODEL:-}" ]]; then
  mounts+=(--mount "type=bind,src=$TGSKK_TEST_MODEL,dst=/test-model.gguf,readonly")
fi
docker run --rm "${mounts[@]}" -e "OUTPUT_UID=$(id -u)" -e "OUTPUT_GID=$(id -g)" \
  "tgskk-build:$target" bash -ceu \
  'mkdir -p /work/entry; tar -xzf /source.tar.gz -C /work/entry packaging/container-build.sh; exec bash /work/entry/packaging/container-build.sh "$1"' -- "$kind"
