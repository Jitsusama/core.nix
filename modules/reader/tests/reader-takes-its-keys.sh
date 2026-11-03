# shellcheck shell=bash
# Runs inside cage: opens a long markdown file in the reader, presses
# Neovim's keys through wtype and reads where each took the view from the
# reader's key log (READER_KEYS_LOG), photographs the top of the page once
# its heading is drawn, and closes it with q. cage doesn't pass on how this
# ended, so it leaves ./passed once every key did what it should.
#
# cage has no keyboard until wtype makes one, so the reader has the
# keyboard only once the first key is typed: G is pressed until it's taken.
# Each wtype adds a keyboard of its own and takes it away as it ends, so
# each one waits before typing for the reader to take the new keyboard.
#
# usage: reader-takes-its-keys.sh <file> <photo> <heading colour>
set -euo pipefail

file=$1
photo=$2
strong=$3
: > said

READER_KEYS_LOG=1 reader "$file" > said 2>&1 &
reader=$!

fail() {
  echo "$1"
  cat said
  kill "$reader" 2> /dev/null || true
  exit 1
}

# Whether the log holds `count` lines matching `pattern`.
holds() { [ "$(grep -c -a -E "$1" said || true)" -ge "$2" ]; }

# Waits until the log holds `count` lines matching `pattern`, or 50 seconds
# pass, however quiet the log is meanwhile.
logged() {
  local deadline=$((SECONDS + 50))
  until holds "$1" "$2"; do
    ((SECONDS < deadline)) || fail "the reader never logged $2 of: $1"
    inotifywait -q -q -t 1 -e modify said || true
  done
}

# Where the view's top was after the last key, in points.
heading() { grep -a -E '^key ' said | tail -n 1 | sed -E 's/.*heading ([0-9.]+) pt/\1/'; }

# Presses keys after the reader has taken wtype's keyboard. wtype puts a
# space between separate words, so a sequence is one word: gg, not g g.
press() { wtype -s 800 "$@"; }

deadline=$((SECONDS + 60))
until holds '^key Last None' 1; do
  ((SECONDS < deadline)) || fail "the reader never took G"
  press G
done
awk "BEGIN { exit !($(heading) > 1000) }" || fail "G went to $(heading) pt, not the end"

press gg
logged '^key First None: heading 0.0 pt' 1
# The top of the page, once its heading is drawn in the strong text.
deadline=$((SECONDS + 30))
until grim "$photo" \
  && [ "$(magick "$photo" -fill black +opaque "$strong" -fill white -opaque "$strong" \
    -format '%[fx:round(mean*w*h)]' info:)" -gt 100 ]; do
  ((SECONDS < deadline)) || fail "the page's heading never showed in the strong text"
  inotifywait -q -q -t 1 -e modify said || true
done

press 5j
logged '^key Down Some\(5\)' 1

press -M ctrl f -m ctrl
logged '^key ScreenDown None' 1
awk "BEGIN { exit !($(heading) > 100) }" || fail "Ctrl+F stayed at $(heading) pt"

press -M ctrl o -m ctrl
logged '^key JumpBack None' 1

press -M ctrl = -m ctrl
logged '^key ZoomIn None' 1
press -M ctrl 0 -m ctrl
logged '^key ZoomBack None' 1

press /Every
press -k Return
logged '^search ' 1
grep -a -q -E '^search .*[1-9][0-9]* found' said || fail "/ found nothing"

press n
logged '^key Next None' 1

press ' ?'
logged '^key ShowKeys None' 1
press -k Escape
logged '^key Rest None' 1

# The link at the page's end, back to its heading, by its label.
ends=$(grep -c -a '^key Last None' said || true)
press G
logged '^key Last None' $((ends + 1))
press gx
logged '^links: 1 in view' 1
press a
logged '^link: #every-key-neovim-has' 1
press j
logged '^key Down None' 1
awk "BEGIN { exit !($(heading) < 100) }" || fail "the link went to $(heading) pt, not the top"

press q
wait "$reader" || true
touch passed
