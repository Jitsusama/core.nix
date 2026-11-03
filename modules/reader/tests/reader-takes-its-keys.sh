# shellcheck shell=bash
# Runs inside cage: opens a long markdown file in the reader, photographs
# the first screen once the window has the keyboard, then presses Neovim's
# keys through wtype and reads where each took the view from the reader's
# key log (READER_KEYS_LOG), and closes it with q. cage doesn't pass on how
# this ended, so it leaves ./passed once every key did what it should.
#
# Each wtype adds a keyboard of its own and takes it away as it ends, so
# each one waits before typing for the reader to take the new keyboard.
#
# usage: reader-takes-its-keys.sh <file> <photo>
set -euo pipefail

file=$1
photo=$2
: > said

READER_KEYS_LOG=1 reader "$file" > said 2>&1 &
reader=$!

fail() {
  echo "$1"
  cat said
  kill "$reader" 2> /dev/null || true
  exit 1
}

# Waits until the log holds `count` lines matching `pattern`, or 50 seconds
# pass, however quiet the log is meanwhile.
logged() {
  local pattern=$1 count=$2 deadline=$((SECONDS + 50))
  until [ "$(grep -c -a -E "$pattern" said || true)" -ge "$count" ]; do
    ((SECONDS < deadline)) || fail "the reader never logged $count of: $pattern"
    inotifywait -q -q -t 1 -e modify said || true
  done
}

# Where the view's top was after the last key, in points.
heading() { grep -a -E '^key ' said | tail -n 1 | sed -E 's/.*heading ([0-9.]+) pt/\1/'; }

# Presses keys after the reader has taken wtype's keyboard. wtype puts a
# space between separate words, so a sequence is one word: gg, not g g.
press() { wtype -s 800 "$@"; }

logged '^keys: focused' 1
grim "$photo"

press G
logged '^key Last None' 1
end=$(heading)
awk "BEGIN { exit !($end > 1000) }" || fail "G went to $end pt, not the end"

press gg
logged '^key First None: heading 0.0 pt' 1

press 5j
logged '^key Down Some\(5\)' 1

press -M ctrl f -m ctrl
logged '^key ScreenDown None' 1
awk "BEGIN { exit !($(heading) > 100) }" || fail "Ctrl+F stayed at $(heading) pt"

press -M ctrl o -m ctrl
logged '^key JumpBack None' 1

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

press q
wait "$reader" || true
touch passed
