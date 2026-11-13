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
    common=(/work/packages/tgskk-common-git-*.pkg.tar.zst)
    fcitx=(/work/packages/tgskk-git-*.pkg.tar.zst)
    ibus=(/work/packages/tgskk-ibus-git-*.pkg.tar.zst)
    frontend_name=tgskk-git
    ibus_name=tgskk-ibus-git
    common_name=tgskk-common-git
    ;;
  deb)
    cp -a packaging/debian debian
    chmod +x debian/rules
    bash scripts/build-dist.sh prepare
    # Rust is supplied by the container toolchain, outside dpkg's database.
    dpkg-buildpackage --build=binary --no-sign -d
    common=(/work/tgskk-common_*.deb)
    fcitx=(/work/tgskk_*.deb)
    ibus=(/work/tgskk-ibus_*.deb)
    frontend_name=tgskk
    ibus_name=tgskk-ibus
    common_name=tgskk-common
    ;;
  *) echo 'Expected arch or deb' >&2; exit 2 ;;
esac
install_packages() {
  case "$kind" in
    arch) pacman -U --noconfirm "$@" ;;
    deb) apt-get install --no-install-recommends -y "$@" ;;
  esac
}
remove_packages() {
  case "$kind" in
    arch) pacman -R --noconfirm "$@" ;;
    deb) dpkg --remove "$@" ;;
  esac
}
for artifact in "${common[@]}" "${fcitx[@]}" "${ibus[@]}"; do test -f "$artifact"; done
package=("${common[@]}" "${fcitx[@]}" "${ibus[@]}")
mkdir -p /work/sentinels
for path in .local/share/fcitx5/tgskk/SKK-JISYO.user .local/share/ibus/tgskk/SKK-JISYO.user .local/share/fcitx5/tgskk/zenz-v3.2-small-Q5_K_M.gguf .config/tgskk/ibus.conf .config/fcitx5/conf/tgskk.conf; do
  mkdir -p "$HOME/$(dirname "$path")" "/work/sentinels/$(dirname "$path")"
  printf '%s\n' ';; retained across package changes' > "$HOME/$path"
  cp "$HOME/$path" "/work/sentinels/$path"
done
check_user_data() {
  while IFS= read -r -d '' sentinel; do
    cmp "$sentinel" "$HOME/${sentinel#/work/sentinels/}"
  done < <(find /work/sentinels -type f -print0)
}
if [[ -f /legacy-package ]]; then
  if [[ "$kind" == deb ]]; then
    cp /legacy-package /work/legacy.deb
    install_packages /work/legacy.deb
  else
    install_packages /legacy-package
  fi
  install_packages "${package[@]}"
  check_user_data
  remove_packages "$frontend_name" "$ibus_name" "$common_name"
  check_user_data
  printf '%s\n' 'Upgrade from the monolithic package passed.'
fi
install_packages "${common[@]}" "${ibus[@]}"
test ! -e /usr/lib/fcitx5/fcitx5-tgskk.so
test ! -e /usr/lib/x86_64-linux-gnu/fcitx5/fcitx5-tgskk.so
case "$kind" in
  arch) dependencies="$(pacman -Qip "${ibus[0]}")" ;;
  deb) dependencies="$(dpkg-deb --field "${ibus[0]}" Depends)" ;;
esac
if [[ "$dependencies" == *fcitx* ]]; then echo 'IBus package depends on fcitx' >&2; exit 1; fi
ldd /usr/libexec/ibus-engine-tgskk > /work/ibus-libraries
if grep -q libfcitx /work/ibus-libraries; then echo 'IBus engine links to fcitx' >&2; exit 1; fi
if [[ -f /test-model.gguf ]]; then
  TGSKK_TEST_INSTALLED=1 TGSKK_TEST_MODEL=/test-model.gguf ctest --test-dir build/ibus --output-on-failure --no-tests=error
else
  TGSKK_TEST_INSTALLED=1 ctest --test-dir build/ibus --output-on-failure --no-tests=error
fi
install_packages "${fcitx[@]}"
install_packages "${package[@]}"
check_user_data
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
remove_packages "$ibus_name"
test ! -e /usr/libexec/ibus-engine-tgskk
TGSKK_TEST_INSTALLED=1 ctest --test-dir build/fcitx5 --output-on-failure --no-tests=error
install_packages "${ibus[@]}"
remove_packages "$frontend_name"
TGSKK_TEST_INSTALLED=1 ctest --test-dir build/ibus --output-on-failure --no-tests=error
remove_packages "$ibus_name"
test -x /usr/libexec/tgskk-zenz-worker
remove_packages "$common_name"
test ! -e /usr/bin/tgskk-setup-model
test ! -e /usr/libexec/tgskk-google-skkserv
test ! -e /usr/libexec/tgskk-zenz-worker
check_user_data
# Publish only the verified package set, replacing artifacts from earlier builds.
rm -f /out/tgskk*.deb /out/tgskk*.pkg.tar.zst
for artifact in "${package[@]}"; do
  install -m644 -o "${OUTPUT_UID:-0}" -g "${OUTPUT_GID:-0}" "$artifact" /out/
done
printf '%s\n' 'Split-package install, coexistence, reinstall, removal and user-data preservation passed.'
