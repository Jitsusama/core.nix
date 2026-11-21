# shellcheck shell=bash
# How reader-takes-its-keys.sh presses keys and reads what the reader did
# with them, sourced by it with the reader's process, so a failure can end
# it, and the colour of the page's heading, by which a photo finds the
# title. The reader logs each key, copy and selection to ./said.
set -euo pipefail
reader=$1
strong=$2

fail() {
  echo "$1"
  cat said
  kill "$reader" 2> /dev/null || true
  exit 1
}

# How many lines of the log match `pattern`.
how_many() { grep -c -a -E "$1" said || true; }

# Whether the log holds `count` lines matching `pattern`.
holds() {
  local count
  count=$(how_many "$1")
  ((count >= $2))
}

# Waits until the log holds `count` lines matching `pattern`, or 50 seconds
# pass, however quiet the log is meanwhile.
logged() {
  local deadline=$((SECONDS + 50)) count
  count=$(how_many "$1")
  while ((count < $2)); do
    ((SECONDS < deadline)) || fail "the reader never logged $2 of: $1"
    inotifywait -q -q -t 1 -e modify said || true
    count=$(how_many "$1")
  done
}

# Where the view's top was after the last key, in points.
heading() { grep -a -E '^key ' said | tail -n 1 | sed -E 's/.*heading ([0-9.]+) pt/\1/'; }

# Where the cursor was after the last key, in points.
cursor() { grep -a -E '^cursor: ' said | tail -n 1 | sed -E 's/cursor: ([0-9.]+) pt/\1/'; }

# Whether two places are a point apart at most.
near() { awk "BEGIN { exit !($1 - $2 < 1 && $2 - $1 < 1) }"; }

# Presses keys after the reader has taken wtype's keyboard. wtype puts a
# space between separate words, so a sequence is one word: gg, not g g.
press() { wtype -s 800 "$@"; }

# Presses keys that make `acts` acts, and waits until the reader has done
# them all.
keys=0
act() {
  local acts=$1
  shift
  press "$@"
  keys=$((keys + acts))
  logged '^key ' "$keys"
}

# Whether the selection shows `words` as the page sets them, said as Rust
# quotes them, by the log's last word on it, which comes before its key's,
# whichever kind of selection it is.
shows() {
  local said
  said=$(grep -a -E '^selected: ' said | tail -n 1)
  said=${said#selected: }
  said=${said#Chars }
  said=${said#Pieces }
  [ "$said" = "$1" ] || fail "the selection is $said, not $1"
}

# Whether the cursor stands on `character` at rest, said as Rust quotes
# it, or on nothing but its line, by the log's last word on it.
stands() {
  local said
  said=$(grep -a -E '^character: ' said | tail -n 1)
  said=${said#character: }
  [ "$said" = "$1" ] || fail "the cursor stands on $said, not $1"
}

# Waits for the next copy, and whether it took `markdown`, said as Rust
# quotes it.
copies=0
copied() {
  copies=$((copies + 1))
  logged '^copied: ' "$copies"
  local said
  said=$(grep -a -E '^copied: ' said | sed -n "${copies}p")
  [ "$said" = "copied: $1" ] || fail "copied ${said#copied: }, not $1"
}

# How many pixels of a photo, left of the title's words, are `colour`.
left_of_words() {
  local box left
  box=$(magick "$1" -fill black +opaque "$strong" -fill white -opaque "$strong" \
    -format '%@' info:)
  left=${box#*+}
  left=${left%%+*}
  magick "$1" -crop "${left}x100000+0+0" +repage -fill black +opaque "$2" \
    -fill white -opaque "$2" -format '%[fx:round(mean*w*h)]' info:
}
