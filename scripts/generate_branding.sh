#!/usr/bin/env bash
# Requires Node.js/npm and ImageMagick. Tauri renders the SVG with resvg.
set -euo pipefail
cd "$(dirname "$0")/.."
assets_dir="$(mktemp -d)"
trap 'rm -rf "$assets_dir"' EXIT
source_svg="branding/fabspeed-logo.svg"
# Optional path to an already installed Tauri 2.12.3 CLI (tauri.js).
if [[ -n "${TAURI_CLI:-}" ]]; then
  tauri=(node "$TAURI_CLI")
else
  tauri=(npm exec --yes --package=@tauri-apps/cli@2.12.3 -- tauri)
fi

"${tauri[@]}" icon "$source_svg" --output "$assets_dir/desktop"
"${tauri[@]}" icon "$source_svg" --output "$assets_dir/web" --png 16 --png 32 --png 48 --png 180

cp "$source_svg" ui/public/fabspeed-logo.svg
cp "$source_svg" ui/public/favicon.svg
cp "$assets_dir/web/32x32.png" ui/public/favicon-32.png
cp "$assets_dir/web/180x180.png" ui/public/apple-touch-icon.png
magick "$assets_dir/web/16x16.png" "$assets_dir/web/32x32.png" "$assets_dir/web/48x48.png" ui/public/favicon.ico
# Desktop targets only; do not copy the generated Android/iOS directories.
for asset in "$assets_dir/desktop/"*.png "$assets_dir/desktop/"*.ico "$assets_dir/desktop/"*.icns; do
  cp "$asset" crates/fabspeed-desktop/icons/
done
