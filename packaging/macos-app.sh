#!/usr/bin/env bash
# Build LoMux.app from an already-compiled binary.
# Usage: packaging/macos-app.sh <path-to-binary> <output-dir> <version>
set -euo pipefail

BINARY="${1:?path to compiled lomux binary required}"
OUTDIR="${2:-.}"
VERSION="${3:-0.0.0}"

APP="$OUTDIR/LoMux.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/LoMux"
chmod +x "$APP/Contents/MacOS/LoMux"

if [ -f assets/LoMux.icns ]; then
	cp assets/LoMux.icns "$APP/Contents/Resources/LoMux.icns"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>LoMux</string>
	<key>CFBundleDisplayName</key>
	<string>LoMux</string>
	<key>CFBundleExecutable</key>
	<string>LoMux</string>
	<key>CFBundleIdentifier</key>
	<string>com.zblauser.lomux</string>
	<key>CFBundleIconFile</key>
	<string>LoMux.icns</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>$VERSION</string>
	<key>CFBundleVersion</key>
	<string>$VERSION</string>
	<key>NSHumanReadableCopyright</key>
	<string>Copyright (c) 2025-2026 zblauser. MIT licensed.</string>
	<key>LSMinimumSystemVersion</key>
	<string>10.15</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.video</string>
</dict>
</plist>
PLIST

echo "built $APP"
