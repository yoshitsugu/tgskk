#!/usr/bin/env bash
set -euo pipefail
client="${1:?client binary}"
engine="${2:?engine binary}"
component="${3:?component XML}"
dictionary="${4:?fixture dictionary}"
if [[ -n "${TGSKK_TEST_INSTALLED:-}" ]]; then
  engine=/usr/libexec/ibus-engine-tgskk
  component=/usr/share/ibus/component/tgskk.xml
fi
root=$(mktemp -d /tmp/tgskk-ibus-session-XXXXXX)
daemon_pid=
: > "$root/daemon.log"
cleanup() {
  if [[ -n "$daemon_pid" ]]; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  cat "$root/daemon.log"
  rm -rf "$root"
}
trap cleanup EXIT
export HOME="$root" XDG_CONFIG_HOME="$root/config" XDG_DATA_HOME="$root/data" XDG_CACHE_HOME="$root/cache"
export XDG_RUNTIME_DIR="$root/run" GSETTINGS_BACKEND=memory GIO_USE_VFS=local
export IBUS_ADDRESS="unix:path=$root/ibus.sock" IBUS_COMPONENT_PATH="$root/components" IBUS_USE_PORTAL=0
unset DISPLAY WAYLAND_DISPLAY
mkdir -p "$XDG_CONFIG_HOME/tgskk" "$IBUS_COMPONENT_PATH" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
if [[ -n "${TGSKK_TEST_INSTALLED:-}" ]]; then
  cp "$component" "$IBUS_COMPONENT_PATH/tgskk.xml"
else
python3 - "$component" "$engine" "$IBUS_COMPONENT_PATH/tgskk.xml" <<'PY'
import sys
import shlex
import xml.etree.ElementTree as ET
component = ET.parse(sys.argv[1])
component.getroot().find('exec').text = shlex.quote(sys.argv[2]) + ' --ibus'
component.write(sys.argv[3])
PY
fi
cp "$dictionary" "$root/dictionary"
printf 'いちらん /一/二/三/四/五/六/七/八/九/十/十一/十二/\n' >> "$root/dictionary"
printf '[General]\nDictionary=%s/dictionary\nDictionaryEncoding=utf-8\nZenzEnabled=%s\nZenzModel=%s\nSkkservMode=Disabled\nModeIndicator=false\n' "$root" "$([[ -n "${TGSKK_TEST_MODEL:-}" ]] && printf true || printf false)" "${TGSKK_TEST_MODEL:-/usr/share/tgskk/zenz-v3.2-small-Q5_K_M.gguf}" > "$XDG_CONFIG_HOME/tgskk/ibus.conf"
ibus-daemon --single --address="$IBUS_ADDRESS" --panel=disable --config=disable --emoji-extension=disable --cache=none > "$root/daemon.log" 2>&1 &
daemon_pid=$!
for ((i=0;i<200;i++)); do
  [[ -S "$root/ibus.sock" ]] && break
  kill -0 "$daemon_pid"
  sleep .01
done
"$client"
