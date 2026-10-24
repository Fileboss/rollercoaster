#!/bin/sh
# Builds Rollercoaster.app and Rollercoaster-<version>-macos-arm64.dmg in
# target/release/ (macOS ARM). Not signed with a Developer ID nor notarized:
# the app is signed ad hoc, macOS asks to open it by hand the first time
# (right click › Open), see README.md.
#   tools/macos/bundle.sh
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release -p rollercoaster

out=target/release
app=$out/Rollercoaster.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$out/rollercoaster" "$app/Contents/MacOS/rollercoaster"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>Rollercoaster</string>
	<key>CFBundleDisplayName</key>
	<string>Rollercoaster</string>
	<key>CFBundleIdentifier</key>
	<string>io.github.fileboss.rollercoaster</string>
	<key>CFBundleExecutable</key>
	<string>rollercoaster</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>$version</string>
	<key>CFBundleVersion</key>
	<string>$version</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
EOF
codesign --force --deep --sign - "$app"

dmg=$out/Rollercoaster-$version-macos-arm64.dmg
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
hdiutil create -volname Rollercoaster -srcfolder "$stage" -ov -format UDZO "$dmg" >/dev/null
echo "$dmg"
