# shellcheck shell=bash
# Runs the console font's build (modules/console/font.sh) with stand-ins for
# fc-match and the font converter, and checks what it makes of fontconfig's
# answer: the face asked for at Medium is drawn and named beside the font,
# even among the face's other names; another face, or the face without a
# Medium, is refused, saying what fontconfig offered instead.
# usage: console-font.sh <font.sh>
set -euo pipefail

work=$(mktemp -d)
export WORK=$work
mkdir "$work/bin"

# fc-match keeps what it was asked for in $WORK/asked and answers with
# $WORK/answer: a family, a style and a file, a line each.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
printf '%s\n' "$3" > "$WORK/asked"
cat "$WORK/answer"
EOF
} > "$work/bin/fc-match"

# The converter writes which file and glyphs it drew from in place of a font.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
printf 'drawn from %s with %s\n' "$1" "$2" > "$3"
EOF
} > "$work/bin/convert"
chmod +x "$work/bin/fc-match" "$work/bin/convert"

fails() {
  echo "the console font's build: $1" >&2
  exit 1
}

# Builds the font for Monaspace with fontconfig answering as given, into a
# fresh $out, keeping what it said in $WORK/said and its status in $status.
build() {
  printf '%s\n' "$@" > "$work/answer"
  rm -rf "$work/out"
  status=0
  env PATH="$work/bin:$PATH" TMPDIR="$work" out="$work/out" face=Monaspace \
    glyphs=/glyphs.txt convert="$work/bin/convert" \
    "$BASH" "$font" 2> "$work/said" || status=$?
}
font=$1

build Monaspace Medium /fonts/monaspace-medium.otf
((status == 0)) || fails "refused the face it asked for: $(cat "$work/said")"
[[ $(cat "$work/asked") == "Monaspace:style=Medium" ]] \
  || fails "asked fontconfig for $(cat "$work/asked"), not Monaspace at Medium"
[[ $(cat "$work/out/console.psf") == "drawn from /fonts/monaspace-medium.otf with /glyphs.txt" ]] \
  || fails "didn't draw the file fontconfig found with the glyphs given"
printf 'Monaspace\nMedium\n' | cmp -s - "$work/out/face" \
  || fails "didn't name the face it drew beside the font"

# fontconfig lists every name a face goes by, and the face asked for counts
# wherever it falls among them.
build "Monaspace Xe,Monaspace" "Regular,Medium" /fonts/monaspace-medium.otf
((status == 0)) || fails "refused the face among its other names: $(cat "$work/said")"

build "DejaVu Sans Mono" Book /fonts/dejavu.ttf
((status == 1)) || fails "drew fontconfig's nearest face rather than refusing it"
grep -qF "Monaspace isn't among the machine's fonts (fonts.packages); fontconfig offered DejaVu Sans Mono" \
  "$work/said" || fails "refused another face without saying which: $(cat "$work/said")"

build Monaspace Regular /fonts/monaspace-regular.otf
((status == 1)) || fails "drew the face without its Medium rather than refusing it"
grep -qF "Monaspace has no Medium; fontconfig offered Regular" "$work/said" \
  || fails "refused a missing Medium without saying what was offered: $(cat "$work/said")"
[[ ! -e $work/out ]] || fails "left a font behind after refusing"
