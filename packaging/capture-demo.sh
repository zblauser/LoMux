#!/usr/bin/env bash
# Capture a README screenshot and an action GIF of the running LoMux window (macOS).
#
# Requires: LoMux running, ffmpeg on PATH, and Screen Recording permission granted to
# the terminal running this script (System Settings → Privacy & Security → Screen Recording).
#
# Usage:
#   packaging/capture-demo.sh shot              # still → assets/screenshot.png
#   packaging/capture-demo.sh gif [seconds]     # recording → assets/demo.gif (default 12s)
#   packaging/capture-demo.sh mat               # re-mat an existing screenshot.png on a backdrop
#
# For a shot that reads well on the README rather than like a dark terminal crop:
#   - pick a colourful theme first (Emerald or Warm Dark beat Studio Dark on a white page)
#   - fill the queue with a few files and set a per-file preset override, so the rows
#     have colour and the "mixed presets" hint is visible
#   - open one collapsible panel — an all-collapsed window is a grey slab
#   - widen the window; the left column maxes at 480pt, so drag the console side out
set -euo pipefail

MODE="${1:-shot}"
SECONDS_TO_RECORD="${2:-12}"
OUT_DIR="assets"
GIF_WIDTH=900
GIF_FPS=15
MAT_PAD=48
# Backdrop for `mat`. Defaults contrast with LoMux's dark themes so the window lifts
# off the page; override for a light theme, e.g. MAT_C0=0x2b3440 MAT_C1=0x1b2028
MAT_C0="${MAT_C0:-0x6d7f9c}"
MAT_C1="${MAT_C1:-0x35415c}"

bounds() {
	osascript -e 'tell application "System Events" to tell process "LoMux" to get {position, size} of window 1' \
		| tr -d ' '
}

mkdir -p "$OUT_DIR"

# Only shot/gif need the app on screen. mat is post-processing on a file already captured.
if [ "$MODE" != "mat" ]; then
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
fi

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
		# 256 colours + sierra2_4a instead of 128 + bayer. Bayer's ordered checkerboard is
		# exactly what made the old GIF look like a dithered terminal capture; sierra2_4a
		# diffuses error instead, so gradients and accent colours stay smooth.
		# stats_mode=diff builds the palette from what actually changes between frames,
		# which spends the colour budget on the UI rather than the static background.
		ffmpeg -hide_banner -loglevel error -i "$TMP/demo.mov" \
			-filter_complex "fps=${GIF_FPS},scale=${GIF_WIDTH}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=256:stats_mode=diff[p];[s1][p]paletteuse=dither=sierra2_4a:diff_mode=rectangle" \
			-loop 0 -y "$OUT_DIR/demo.gif"
		rm -rf "$TMP"
		SIZE=$(du -h "$OUT_DIR/demo.gif" | cut -f1)
		echo "wrote $OUT_DIR/demo.gif ($SIZE)"
		;;
	mat)
		# Mats a flat rectangular crop onto a soft backdrop with a drop shadow, so the
		# screenshot reads as an application window instead of a dark rectangle.
		SRC="$OUT_DIR/screenshot.png"
		[ -f "$SRC" ] || { echo "no $SRC — run '$0 shot' first" >&2; exit 1; }
		SW=$(ffprobe -v error -select_streams v:0 -show_entries stream=width -of csv=p=0 "$SRC")
		SH=$(ffprobe -v error -select_streams v:0 -show_entries stream=height -of csv=p=0 "$SRC")
		CW=$((SW + MAT_PAD * 2))
		CH=$((SH + MAT_PAD * 2))
		ffmpeg -hide_banner -loglevel error \
			-f lavfi -i "gradients=s=${CW}x${CH}:c0=${MAT_C0}:c1=${MAT_C1}:x0=0:y0=0:x1=${CW}:y1=${CH}:nb_colors=2:d=1" \
			-i "$SRC" \
			-filter_complex "[0:v]trim=end_frame=1,setpts=PTS-STARTPTS[bg];[1:v]format=rgba,pad=iw+24:ih+24:12:12:color=0x00000000[fg];[fg]boxblur=8:1[sh];[bg][sh]overlay=${MAT_PAD}-12:${MAT_PAD}-6[shad];[shad][1:v]overlay=${MAT_PAD}:${MAT_PAD}" \
			-frames:v 1 -y "$OUT_DIR/screenshot-matted.png"
		echo "wrote $OUT_DIR/screenshot-matted.png (${CW}x${CH})"
		echo "If you like it, replace screenshot.png with it."
		;;
	*)
		echo "usage: $0 [shot|gif [seconds]|mat]" >&2
		exit 1
		;;
esac
