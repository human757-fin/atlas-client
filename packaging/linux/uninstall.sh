#!/usr/bin/env sh
set -eu

rm -f "${HOME}/.local/share/applications/atlas-client.desktop"
rm -rf "${HOME}/.local/opt/atlas-client"
echo "Atlas was removed. Your game files and settings in ~/.local/share/Atlas and ~/.config/Atlas were kept."
