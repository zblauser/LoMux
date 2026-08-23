#!/usr/bin/env bash
# Capture a README screenshot and an action GIF of the running LoMux window (macOS).
#
# Requires: LoMux running, ffmpeg on PATH, and Screen Recording permission granted to
# the terminal running this script (System Settings → Privacy & Security → Screen Recording).
#
# Usage:
#   packaging/capture-demo.sh shot              # still → assets/screenshot.png
#   packaging/capture-demo.sh gif [seconds]     # recording → assets/demo.gif (default 12s)
set -euo pipefail

MODE="${1:-shot}"
SECONDS_TO_RECORD="${2:-12}"
OUT_DIR="assets"
GIF_WIDTH=900

bounds() {
	osascript -e 'tell application "System Events" to tell process "LoMux" to get {position, size} of window 1' \
		| tr -d ' '
}

if ! pgrep -f "LoMux" >/dev/null 2>&1; then
	echo "LoMux is not running — start it first." >&2
	exit 1
fi

osascript -e 'tell application "LoMux" to activate' 2>/dev/null || true
sleep 1

B=$(bounds)
X=$(echo "$B" | cut -d, -f1)
Y=$(echo "$B" | cut -d, -f2)
W=$(echo "$B" | cut -d, -f3)
H=$(echo "$B" | cut -d, -f4)
RECT="${X},${Y},${W},${H}"

mkdir -p "$OUT_DIR"

case "$MODE" in
	shot)
		screencapture -x -R"$RECT" "$OUT_DIR/screenshot.png"
		echo "wrote $OUT_DIR/screenshot.png (${W}x${H} points)"
		echo "If this shows only wallpaper, grant Screen Recording permission and retry."
		;;
	gif)
		TMP=$(mktemp -d)
		echo "recording ${SECONDS_TO_RECORD}s — drive the app now…"
		screencapture -x -v -V "$SECONDS_TO_RECORD" -R"$RECT" "$TMP/demo.mov"
		echo "encoding gif…"
		ffmpeg -hide_banner -loglevel error -i "$TMP/demo.mov" \
			-filter_complex "fps=12,scale=${GIF_WIDTH}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3" \
			-loop 0 -y "$OUT_DIR/demo.gif"
		rm -rf "$TMP"
		SIZE=$(du -h "$OUT_DIR/demo.gif" | cut -f1)
		echo "wrote $OUT_DIR/demo.gif ($SIZE)"
		;;
	*)
		echo "usage: $0 [shot|gif] [seconds]" >&2
		exit 1
		;;
esac
