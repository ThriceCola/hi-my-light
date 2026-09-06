#!/usr/bin/env bash
set -euo pipefail

home="${HOME:-.}"
data="${XDG_DATA_HOME:-$home/.local/share}"
config="${XDG_CONFIG_HOME:-$home/.config}"
cache="${XDG_CACHE_HOME:-$home/.cache}"

menu="$data/applications/hi-my-light.desktop"
auto="$config/autostart/hi-my-light.desktop"

exec_path_from_desktop() {
  local file="$1" line path
  [[ -f "$file" ]] || return 1
  line="$(grep -E '^Exec=' "$file" | head -n1 || true)"
  [[ -n "$line" ]] || return 1
  line="${line#Exec=}"
  if [[ "$line" == \"* ]]; then
    path="${line#\"}"
    path="${path%%\"*}"
  else
    path="${line%%[[:space:]]*}"
  fi
  [[ -n "$path" ]] || return 1
  printf '%s\n' "$path"
}

bins=()
for desktop in "$menu" "$auto"; do
  if path="$(exec_path_from_desktop "$desktop")"; then
    bins+=("$path")
  fi
done
bins+=("$home/.local/bin/hi-my-light")

seen=""
unique=()
for bin in "${bins[@]}"; do
  case " $seen " in
    *" $bin "*) continue ;;
  esac
  seen+=" $bin"
  unique+=("$bin")
done

for bin in "${unique[@]}"; do
  name="$(basename -- "$bin")"
  [[ "$name" == hi-my-light ]] || continue
  pkill -x "$name" 2>/dev/null || true
done

for bin in "${unique[@]}"; do
  name="$(basename -- "$bin")"
  [[ "$name" == hi-my-light ]] || continue
  rm -f -- "$bin"
done

rm -f -- "$menu" "$auto"
rm -f -- "$data/icons/hicolor/scalable/apps/hi-my-light.svg"
for size in 32 64 128 256; do
  rm -f -- "$data/icons/hicolor/${size}x${size}/apps/hi-my-light.png"
done
rm -rf -- "$cache/hi-my-light"

update-desktop-database "$data/applications" >/dev/null 2>&1 || true
kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || true

echo "已卸载 Hi My Light"
