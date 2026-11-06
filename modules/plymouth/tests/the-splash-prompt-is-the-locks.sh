# shellcheck shell=bash
# Checks the boot splash's prompt is built as the lock's field, from the
# theme's own sizes: its panel one prompt's row inside its hairline, with
# room for its shadows, at each scale; the padlock's cell the inset rung in
# from the frame's inside edge; what's typed as far after the cell as Qt
# measures Prompt.qml's glyph and its room, which falls between the glyph's
# advance and its ink, with the room after it, and doubles at twice the
# scale; and every face the script places by its capitals measured at each
# scale.
# usage: the-splash-prompt-is-the-locks.sh <theme folder> <mono font> <row> <border> <inset> <gap>
set -euo pipefail

theme=$1
font=$2
row=$3
border=$4
inset=$5
gap=$6
script=$theme/jitsusama.script
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work
# The lock glyph, a private-use code point the theme's face draws.
lock=$'\uf023'
# The lock's field is as wide as its card.
width=400

fail() {
  echo "$1"
  exit 1
}

# What the script gives a place, by the line that names it.
given() {
  sed -n "s/^ *$1 \\([0-9]*\\)\\( \\*\\|;\\).*/\\1/p" "$script" | head -n 1
}

cell=$(given 'local.cell = global.panel.left +')
want=$((border + inset))
[ "$cell" = "$want" ] ||
  fail "the padlock's cell is ${cell:-unnamed} from the frame's edge, not the inset's $want"
room=$(given 'local.room =')
[ -n "$room" ] || fail "the script names no room round the padlock"
grep -q '^ *global.typed = cell + measure("lead");' "$script" ||
  fail "what's typed doesn't start the measured lead after the padlock's cell"

# What the script was told of the lead at a scale.
lead() {
  sed -n "s/^measured\[\"lead-$1\"\] = \([0-9]*\);$/\1/p" "$script"
}
one=$(lead 1)
[ -n "$one" ] || fail "the script has no lead for the padlock"
[ "$(lead 2)" = $((2 * one)) ] ||
  fail "the padlock's lead at 2 is $(lead 2), not twice its ${one} at 1"

for scale in 1 2; do
  # The panel's picture holds its shadows' reach on every side, which its
  # width says.
  across=$(magick "$theme/panel-$scale.png" -format '%w' info:)
  reach=$(((across / scale - width) / 2))
  want=$(((row + 2 * border + 2 * reach) * scale))
  tall=$(magick "$theme/panel-$scale.png" -format '%h' info:)
  [ "$tall" = "$want" ] ||
    fail "the panel drawn at $scale is $tall tall, not the lock's field's $want with its reach"

  pango-view -q --no-display --dpi=$((96 * scale)) --margin=0 \
    --hinting=none --font="$font" --output=cell.png --text="$lock"
  advance=$(magick cell.png -format '%w' info:)
  # Where the padlock's ink ends, from its cell, a room in from the left of
  # its picture.
  ink=$(magick "$theme/lock-$scale.png" -format '%@' info:)
  right=$((${ink%%x*} + $(echo "$ink" | cut -d+ -f2) - room * scale))
  led=$(lead "$scale")
  # Qt sets the face at whole pixels, so its advance can fall a pixel short
  # of pango's.
  [ "$led" -ge $((advance - scale + gap * scale)) ] && [ "$led" -le $((right + gap * scale)) ] ||
    fail "the padlock leads by $led at $scale, not between its advance $advance and its ink $right, with $((gap * scale)) after"

  for face in name under said voice typed; do
    for place in top baseline; do
      grep -q "^measured\[\"$face-$place-$scale\"\] = [0-9]*;$" "$script" ||
        fail "the script has no $place for the $face face at $scale"
    done
  done
done
