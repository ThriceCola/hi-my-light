#!/usr/bin/env bash
set -euo pipefail

home="${HOME:-.}"
data="${XDG_DATA_HOME:-$home/.local/share}"
config="${XDG_CONFIG_HOME:-$home/.config}"
cache="${XDG_CACHE_HOME:-$home/.cache}"

pkill -x hi-my-light 2>/dev/null || true

rm -f "$home/.local/bin/hi-my-light"
rm -f "$data/applications/hi-my-light.desktop"
rm -f "$config/autostart/hi-my-light.desktop"
rm -f "$data/icons/hicolor/scalable/apps/hi-my-light.svg"
for size in 32 64 128 256; do
  rm -f "$data/icons/hicolor/${size}x${size}/apps/hi-my-light.png"
done
rm -rf "$cache/hi-my-light"

update-desktop-database "$data/applications" >/dev/null 2>&1 || true
kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || true

echo "已卸载 Hi My Light"
