#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bin="${root}/target/release/hi-my-light"
log="${XDG_DATA_HOME:-$HOME/.local/share}/hi-my-light/shutdown.log"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=${XDG_RUNTIME_DIR}/bus}"
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

if [[ ! -x "$bin" ]]; then
  echo "missing $bin" >&2
  exit 2
fi

pkill -x hi-my-light 2>/dev/null || true
sleep 0.4
: >"$log"

"$bin" --background >/tmp/hml-sigterm-test.log 2>&1 &
pid=$!
trap 'kill -9 "$pid" 2>/dev/null || true' EXIT

for _ in $(seq 1 20); do
  if systemd-inhibit --list 2>/dev/null | grep -q hi-my-light; then
    break
  fi
  sleep 0.25
done

kill -TERM "$pid"
for _ in $(seq 1 40); do
  if ! kill -0 "$pid" 2>/dev/null; then
    break
  fi
  sleep 0.1
done

if ! grep -q '独立关灯' "$log"; then
  echo "FAIL: shutdown.log 没有独立关灯" >&2
  cat "$log" >&2 || true
  exit 1
fi
echo "PASS"
cat "$log"
