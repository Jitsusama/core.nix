# shellcheck shell=bash
# Checks the boot splash's lock, drawn at each scale, holds all the ink the
# lock glyph has in the font given, and is as tall as a line of that font,
# by drawing the glyph and a line with pango-view and measuring both.
# usage: boot-lock-is-whole.sh <theme folder> <font>
set -euo pipefail

theme=$1
font=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work
# The lock glyph, a private-use code point the theme's face draws.
lock=$'\uf023'

for scale in 1 2; do
  pango-view -q --no-display --dpi=$((96 * scale)) --margin=$((64 * scale)) \
    --antialias=gray --hinting=none --background=transparent --foreground=white \
    --font="$font" --output=roomy.png --text="$lock"
  pango-view -q --no-display --dpi=$((96 * scale)) --margin=0 \
    --hinting=none --font="$font" --output=line.png --text=x
  want=$(magick roomy.png -trim -format '%wx%h' info:)
  got=$(magick "$theme/lock-$scale.png" -trim -format '%wx%h' info:)
  if [ "$got" != "$want" ]; then
    echo "the lock drawn at $scale holds $got of its $want ink"
    exit 1
  fi
  line=$(magick line.png -format '%h' info:)
  height=$(magick "$theme/lock-$scale.png" -format '%h' info:)
  if [ "$height" != "$line" ]; then
    echo "the lock drawn at $scale is $height tall, not the line's $line"
    exit 1
  fi
done
