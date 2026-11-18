#!/bin/sh
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
fixture_dir=/tmp/termlevel-file-preview-test
mkdir -p "$fixture_dir"
cp "$project_dir/assets/brand/termlevel-icon-v1.png" "$fixture_dir/image.png"
ffmpeg -loglevel error -y -f lavfi -i 'color=c=blue:s=160x90:d=1' -an -c:v libx264 -pix_fmt yuv420p "$fixture_dir/video.mp4"
ffmpeg -loglevel error -y -f lavfi -i 'sine=frequency=440:duration=1' "$fixture_dir/audio.wav"
printf '# Preview test\n\nA local document.\n' > "$fixture_dir/notes.md"
printf '%%PDF-1.4\000unsupported' > "$fixture_dir/report.pdf"
printf 'not a playable movie' > "$fixture_dir/invalid.mp4"
printf '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="red"/></svg>' > "$fixture_dir/drawing.svg"
