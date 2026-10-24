#!/bin/sh
# Renders crates/rollercoaster/assets/icon.svg into AppIcon.icns beside it,
# used by bundle.sh, and icon.png (512 px), the window and Dock icon the
# application sets itself. Needs Inkscape and iconutil (macOS); run it again after
# changing the SVG, and commit both (the CI does not render them).
#   tools/macos/icon.sh
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
assets=$root/crates/rollercoaster/assets
set_dir=$(mktemp -d)/AppIcon.iconset
trap 'rm -rf "$(dirname "$set_dir")"' EXIT
mkdir -p "$set_dir"
for size in 16 32 128 256 512; do
    inkscape "$assets/icon.svg" -w $size -h $size -o "$set_dir/icon_${size}x${size}.png" 2>/dev/null
    double=$((size * 2))
    inkscape "$assets/icon.svg" -w $double -h $double -o "$set_dir/icon_${size}x${size}@2x.png" 2>/dev/null
done
iconutil -c icns "$set_dir" -o "$assets/AppIcon.icns"
cp "$set_dir/icon_512x512.png" "$assets/icon.png"
echo "$assets/AppIcon.icns"
