# shellcheck shell=bash
# Reads the console's font and fails, saying why, unless it's a PSF 2 font
# of the kernel's 16 by 32 cell, naming what each glyph stands for, of at
# most 256 glyphs so the console keeps its bright colours, with slot 32
# blank, drawn from the face named at Medium, and with a glyph for each of
# a sample of letters, lines, blocks and marks. The header's own size says
# where the glyphs start.
# usage: console-wears-the-face.sh <font> <face>
set -euo pipefail

font=$1
face=$2
# PSF 2's magic number, read as a little-endian word.
psf2=2253043058
# The slot the console clears the screen with, which kbd refuses a font
# without.
blank=32

# The header's nth 32-bit word.
word() {
  od -An -t u4 -j $(($1 * 4)) -N 4 "$font" | tr -d ' '
}
fail() {
  echo "The console's font $1."
  exit 1
}

header=$(word 2)
glyphs=$(word 4)
size=$(word 5)
[ "$(word 0)" = "$psf2" ] || fail "isn't a PSF 2 font"
[ "$(word 3)" = 1 ] || fail "doesn't say what its glyphs stand for"
[ "$glyphs" -le 256 ] || fail "holds $glyphs glyphs, past which the console loses its bright colours"
[ "$(word 6)x$(word 7)" = 32x16 ] || fail "draws $(word 7) by $(word 6), not the kernel's 16 by 32"
ink=$(od -An -t x1 -v -j $((header + blank * size)) -N "$size" "$font" | tr -d ' \n0')
[ -z "$ink" ] || fail "has ink in slot $blank, which the console clears the screen with"
printf '%s\nMedium\n' "$face" | cmp -s - "$(dirname "$font")/face" \
  || fail "isn't drawn from $face at Medium"
table=$(mktemp)
trap 'rm -f "$table"' EXIT
tail -c +$((header + glyphs * size + 1)) "$font" | tr '\377' '\n' > "$table"
for c in A '~' ─ █ ● é; do
  grep -qxF "$c" "$table" || fail "has no glyph for $c"
done
