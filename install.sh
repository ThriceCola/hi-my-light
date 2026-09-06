#!/usr/bin/env bash
set -euo pipefail

url="https://github.com/ThriceCola/hi-my-light/releases/latest/download/hi-my-light-linux-amd64"
dest="${XDG_CACHE_HOME:-$HOME/.cache}/hi-my-light/hi-my-light"

mkdir -p "$(dirname "$dest")"
curl -fL --retry 3 --retry-delay 1 "$url" -o "$dest"
chmod +x "$dest"
exec "$dest" --install
