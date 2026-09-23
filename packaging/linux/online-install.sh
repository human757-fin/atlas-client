#!/usr/bin/env sh
set -eu
repo='human757-fin/atlas-client'
api="https://api.github.com/repos/${repo}/releases?per_page=20"
command -v curl >/dev/null 2>&1 || { echo 'Atlas setup needs curl. Install curl and try again.' >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo 'Atlas setup needs Python 3. Install Python 3 and try again.' >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo 'Atlas setup needs sha256sum.' >&2; exit 1; }
echo '  ATLAS  /  Preparing your game space'
echo '  Finding the latest Atlas release…'
release=$(curl -fsSL "$api" | python3 -c 'import json,sys; rs=json.load(sys.stdin); print(next((r["tag_name"] for r in rs if not r["draft"] and any(a["name"] == "atlas-client-linux-x86_64.tar.gz" for a in r["assets"])), ""))')
[ -n "$release" ] || { echo 'No Linux release is available yet.' >&2; exit 1; }
base="https://github.com/${repo}/releases/download/${release}"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
echo '  Downloading Atlas…'
curl -fL --progress-bar "$base/atlas-client-linux-x86_64.tar.gz" -o "$tmp/atlas.tar.gz"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"
(cd "$tmp" && grep 'atlas-client-linux-x86_64.tar.gz' SHA256SUMS | sha256sum -c -) || { echo 'Download verification failed.' >&2; exit 1; }
echo '  Installing Atlas and creating your app menu shortcut…'
tar -xzf "$tmp/atlas.tar.gz" -C "$tmp"
"$tmp/install.sh"
echo '  Atlas is ready. Find it in your applications menu.'
