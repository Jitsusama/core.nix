# shellcheck shell=bash
# Checks the boot splash's prompt is built as the lock's field, from the
# theme's own sizes: its panel one prompt's row inside its hairline, with
# room for its shadows, at each scale; the padlock's cell the inset rung in
# from the frame's inside edge; what's typed Prompt.qml's room after the
# glyph's advance, which the padlock's picture keeps by its room; and every
# face the script places by its capitals measured at each scale.
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
after=$(sed -n 's/.*- 2 \* room + \([0-9]*\) \* global.scale;.*/\1/p' "$script")
[ "$after" = "$gap" ] ||
  fail "what's typed starts ${after:-nothing} after the padlock's advance, not Prompt.qml's $gap"

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
  drawn=$(magick "$theme/lock-$scale.png" -format '%w' info:)
  [ $((drawn - 2 * room * scale)) = "$advance" ] ||
    fail "the padlock drawn at $scale is $drawn wide, not its advance $advance and its room"

  for face in name under said voice typed; do
    for place in top baseline; do
      grep -q "^measured\[\"$face-$place-$scale\"\] = [0-9]*;$" "$script" ||
        fail "the script has no $place for the $face face at $scale"
    done
  done
done
