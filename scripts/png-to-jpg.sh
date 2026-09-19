#!/usr/bin/env bash
# Convert PNG screenshots to visually-lossless JPEGs.
#
# Settings chosen for UI screenshots (sharp text, flat colors):
#   - quality 92 with 4:4:4 chroma subsampling: no color blur on text edges
#   - -strip: removes metadata bloat
#   - PNG is only deleted when the JPEG exists and is actually smaller
#
# Usage: scripts/png-to-jpg.sh [directory]   (default: docs/screens)

set -euo pipefail

dir="${1:-docs/screens}"
quality=92

if ! command -v magick >/dev/null 2>&1; then
  echo "error: ImageMagick 'magick' not found (brew install imagemagick)" >&2
  exit 1
fi
if [ ! -d "$dir" ]; then
  echo "error: directory not found: $dir" >&2
  exit 1
fi

shopt -s nullglob
pngs=("$dir"/*.png)
if [ ${#pngs[@]} -eq 0 ]; then
  echo "no .png files in $dir"
  exit 0
fi

bytes() { stat -f%z "$1" 2>/dev/null || stat -c%s "$1"; }

before_total=0
after_total=0
converted=0
kept=0

for png in "${pngs[@]}"; do
  jpg="${png%.png}.jpg"
  tmp="${jpg}.tmp.jpg"

  # -alpha remove flattens onto white if a future screenshot has transparency;
  # harmless for opaque images.
  magick "$png" -background white -alpha remove -alpha off \
    -sampling-factor 4:4:4 -quality "$quality" -strip "$tmp"

  png_size=$(bytes "$png")
  jpg_size=$(bytes "$tmp")

  if [ "$jpg_size" -lt "$png_size" ]; then
    mv "$tmp" "$jpg"
    rm "$png"
    before_total=$((before_total + png_size))
    after_total=$((after_total + jpg_size))
    converted=$((converted + 1))
    printf 'converted  %-40s %8s -> %8s\n' \
      "$(basename "$jpg")" "$(numfmt --to=iec "$png_size" 2>/dev/null || echo "${png_size}B")" \
      "$(numfmt --to=iec "$jpg_size" 2>/dev/null || echo "${jpg_size}B")"
  else
    rm "$tmp"
    kept=$((kept + 1))
    printf 'kept png   %-40s (%dB, jpeg would be %dB)\n' \
      "$(basename "$png")" "$png_size" "$jpg_size"
  fi
done

echo
echo "converted: $converted, kept as png: $kept"
if [ "$before_total" -gt 0 ]; then
  pct=$((100 - after_total * 100 / before_total))
  echo "total: $(numfmt --to=iec "$before_total" 2>/dev/null || echo "${before_total}B") -> $(numfmt --to=iec "$after_total" 2>/dev/null || echo "${after_total}B") ($pct% smaller)"
fi
