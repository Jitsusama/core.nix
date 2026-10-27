# shellcheck shell=bash
# Whether google-opens opens each of a meeting's links the way it's meant
# to: a Tuple call through Tuple when Tuple is there and in the browser
# when it isn't, Google's own pages in a window of their own, any other
# link in the browser, and what isn't a link nowhere. Each program is a
# stand-in that writes down what it was asked.
# usage: google-opens-its-links.sh OPENS
set -euo pipefail

opens=$1
here=$PWD
mkdir -p bin without

stand_in() {
  printf '#!%s\necho "%s $*" >> %s/asked\n' "$BASH" "$1" "$here" > "$2"
  chmod +x "$2"
}
stand_in window window
stand_in open open
stand_in tuple bin/tuple

failures=0
# Opens $2 with PATH $1 and says whether what was asked is $3.
expect() {
  : > asked
  PATH=$1 "$BASH" "$opens" "$here/window" "$here/open" "$2" 2> /dev/null || true
  said=$(cat asked)
  if [ "$said" != "$3" ]; then
    echo "$2: asked '$said', meant '$3'"
    failures=$((failures + 1))
  fi
}

with=$here/bin:$PATH
without=$here/without:$(dirname "$(command -v cat)")

expect "$with" https://tuple.app/c/abc123 "tuple call join https://tuple.app/c/abc123"
expect "$with" https://production.tuple.app/c/abc123 \
  "tuple call join https://production.tuple.app/c/abc123"
expect "$without" https://tuple.app/c/abc123 "open https://tuple.app/c/abc123"
expect "$with" https://meet.google.com/abc-defg-hij "window https://meet.google.com/abc-defg-hij"
expect "$with" https://calendar.google.com/ "window https://calendar.google.com/"
expect "$with" https://mail.google.com/ "window https://mail.google.com/"
expect "$with" https://zoom.us/j/123 "open https://zoom.us/j/123"
expect "$with" https://meet.google.com.example.com/x "open https://meet.google.com.example.com/x"
expect "$with" "file:///etc/passwd" ""
expect "$with" "--help" ""

if [ "$failures" -gt 0 ]; then
  exit 1
fi
