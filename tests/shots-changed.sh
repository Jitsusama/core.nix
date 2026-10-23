# shellcheck shell=bash
# What changed on screen between two runs of a VM test.
#
# The desktop test photographs every state it puts the desktop in. Run it on
# main and on a branch, then give this the two results: it names each picture
# that changed, by how many pixels, and draws each one before, after and the
# difference, cropped to where it changed, so a change to one surface that
# moved another is seen rather than missed.
#
#   shots-changed <before> <after> [<where to draw>]
#
# Pictures that change by themselves between two runs of the same code, a
# clock or a camera's moving pattern, are named in shots-that-move.txt
# beside this file and listed apart, so they don't bury a real change. They
# are drawn all the same, since a real change can land in one too.

before=$1
after=$2
drawn=${3:-shots-changed}
moving=@moving@
margin=24

[ -d "$before" ] && [ -d "$after" ] || {
  echo "usage: shots-changed <before> <after> [<where to draw>]" >&2
  exit 2
}
mkdir -p "$drawn"

names() { (cd "$1" && find . -maxdepth 1 -name '*.png' -printf '%f\n' | sort); }
moves() { grep -qxF "${1%.png}" <(sed 's/[[:space:]]*#.*//; /^$/d' "$moving"); }

changed=()
quiet=()
while read -r name; do
  # Every pixel that differs by more than the few levels in 255 a blur's
  # rounding moves from one run to the next, which no eye sees, and the box
  # holding them all.
  read -r pixels w h x y < <(magick "$before/$name" "$after/$name" -alpha off \
    -compose difference -composite -colorspace gray -threshold 3% \
    -format '%[fx:round(mean*w*h)] ' -write info: -trim -format '%w %h %X %Y\n' info: 2>/dev/null)
  [ "$pixels" = 0 ] && continue
  if moves "$name"; then
    quiet+=("$(printf '%8d %s' "$pixels" "$name")")
  else
    changed+=("$(printf '%8d %s' "$pixels" "$name")")
  fi
  crop="$((w + 2 * margin))x$((h + 2 * margin))$(printf '%+d%+d' "$((x - margin))" "$((y - margin))")"
  magick compare "$before/$name" "$after/$name" "$drawn/.difference.png" 2>/dev/null || true
  magick "$before/$name" "$after/$name" "$drawn/.difference.png" -crop "$crop" +repage \
    -background none -gravity west -splice "${margin}x0" +append -chop "${margin}x0" \
    "$drawn/$name"
  rm -f "$drawn/.difference.png"
done < <(comm -12 <(names "$before") <(names "$after"))

if [ ${#changed[@]} -gt 0 ]; then
  echo "Changed, most first (pixels, picture), drawn in $drawn:"
  printf '%s\n' "${changed[@]}" | sort -rn
else
  echo "Nothing changed."
fi
if [ ${#quiet[@]} -gt 0 ]; then
  echo "Changed, as they do by themselves (shots-that-move.txt):"
  printf '%s\n' "${quiet[@]}" | sort -rn
fi
gone=$(comm -23 <(names "$before") <(names "$after") | tr '\n' ' ')
new=$(comm -13 <(names "$before") <(names "$after") | tr '\n' ' ')
if [ -n "$gone" ]; then echo "Only before: $gone"; fi
if [ -n "$new" ]; then echo "Only after: $new"; fi
