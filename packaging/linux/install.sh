#!/usr/bin/env sh
set -eu

source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install_dir="${HOME}/.local/opt/atlas-client"
applications_dir="${HOME}/.local/share/applications"
desktop_file="${applications_dir}/atlas-client.desktop"

if [ ! -f "${source_dir}/atlas-client" ]; then
    echo "atlas-client must be beside this installer. Extract the release archive first." >&2
    exit 1
fi

mkdir -p "$install_dir" "$applications_dir"
install -m 755 "${source_dir}/atlas-client" "${install_dir}/atlas-client"
install -m 755 "${source_dir}/uninstall.sh" "${install_dir}/uninstall.sh"

desktop_escape() {
    printf '%s' "$1" | sed 's/[\\"]/\\&/g'
}

escaped_executable=$(desktop_escape "${install_dir}/atlas-client")
cat > "$desktop_file" <<EOF
[Desktop Entry]
Name=Atlas
Comment=Performance-focused Minecraft launcher
Exec="${escaped_executable}"
Icon=applications-games
Terminal=false
Type=Application
Categories=Game;
StartupNotify=true
EOF

chmod 644 "$desktop_file"
echo "Atlas installed. Find it in your applications menu, or run ${install_dir}/atlas-client."
