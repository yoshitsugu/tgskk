#!/usr/bin/env bash
set -euo pipefail
kind="${1:?Expected arch or deb}"
export CARGO_HOME=/work/cargo
mkdir -p /work/tgskk /work/packages "$CARGO_HOME"
tar -xzf /source.tar.gz -C /work/tgskk
cd /work/tgskk
if [[ -d /cached-registry ]]; then
  mkdir -p "$CARGO_HOME"
  cat > "$CARGO_HOME/config.toml" <<'EOF'
[source.crates-io]
replace-with = "local-sources"
[source.local-sources]
directory = "/cached-registry"
[net]
offline = true
EOF
fi
if [[ -d /cached-downloads ]]; then
  mkdir -p build/downloads
  cp /cached-downloads/llama-*.tar.gz build/downloads/
fi
case "$kind" in
  arch)
    mkdir -p /work/recipe/src
    cp packaging/arch/PKGBUILD /work/recipe/
    ln -s /work/tgskk /work/recipe/src/tgskk
    chown -R builder:builder /work
    runuser -u builder -- bash -ceu 'cd /work/tgskk; bash scripts/build-dist.sh prepare'
    runuser -u builder -- bash -ceu 'cd /work/recipe; PKGDEST=/work/packages makepkg --noextract --nodeps'
    test ! -e /work/recipe/src/tgskk/.git
    package=(/work/packages/tgskk-git-*.pkg.tar.zst)
    pacman -U --noconfirm "${package[@]}"
    ;;
  deb)
    cp -a packaging/debian debian
    chmod +x debian/rules
    bash scripts/build-dist.sh prepare
    # Rust is supplied by the container toolchain, outside dpkg's database.
    dpkg-buildpackage --build=binary --no-sign -d
    package=(/work/tgskk_*.deb)
    dpkg -i "${package[@]}"
    ;;
  *) echo 'Expected arch or deb' >&2; exit 2 ;;
esac
user_data="$HOME/.local/share/fcitx5/tgskk"
mkdir -p "$user_data"
printf '%s\n' ';; user dictionary retained across package changes' > "$user_data/SKK-JISYO.user"
printf '%s\n' 'user model retained across package changes' > "$user_data/zenz-v3.2-small-Q5_K_M.gguf"
cp "$user_data/SKK-JISYO.user" /work/dictionary-before
cp "$user_data/zenz-v3.2-small-Q5_K_M.gguf" /work/model-before
case "$kind" in
  arch) pacman -U --noconfirm "${package[@]}" ;;
  deb) dpkg -i "${package[@]}" ;;
esac
cmp /work/dictionary-before "$user_data/SKK-JISYO.user"
cmp /work/model-before "$user_data/zenz-v3.2-small-Q5_K_M.gguf"
if [[ -f /test-model.gguf ]]; then
  cc -shared -fPIC native/zenz/tests/backend_probe.c -o /work/libggml-cpu-probe.so
  python3 native/zenz/tests/check_worker.py /usr/libexec/tgskk-zenz-worker /test-model.gguf /work/libggml-cpu-probe.so
  TGSKK_TEST_INSTALLED=1 TGSKK_TEST_USER_MODEL=/test-model.gguf TGSKK_TEST_ZENZ=1 \
    ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
else
  TGSKK_TEST_INSTALLED=1 ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
fi
/usr/bin/tgskk-setup-model --help >/dev/null
response="$(printf 0 | /usr/libexec/tgskk-google-skkserv --stdio --encoding=utf-8)"
[[ -z "$response" ]]
test -x /usr/libexec/tgskk-zenz-worker
case "$kind" in
  arch) pacman -R --noconfirm tgskk-git ;;
  deb) dpkg --remove tgskk ;;
esac
test ! -e /usr/bin/tgskk-setup-model
test ! -e /usr/libexec/tgskk-google-skkserv
test ! -e /usr/libexec/tgskk-zenz-worker
cmp /work/dictionary-before "$user_data/SKK-JISYO.user"
cmp /work/model-before "$user_data/zenz-v3.2-small-Q5_K_M.gguf"
for artifact in "${package[@]}"; do
  install -m644 -o "${OUTPUT_UID:-0}" -g "${OUTPUT_GID:-0}" "$artifact" /out/
done
printf '%s\n' 'Package build, installed-addon integration and removal passed.'
