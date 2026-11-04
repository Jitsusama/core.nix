# shellcheck shell=bash
# A throwaway: what zathura hears around a search, never merged.
set -eu

zathura -l debug "$1" > said 2>&1 &
zathura=$!
trap 'kill $zathura 2>/dev/null || true' EXIT

waits() {
  tries=$1
  shift
  for _ in $(seq "$tries"); do
    if "$@"; then return 0; fi
    sleep 0.1
  done
  return 1
}
never() {
  echo "never: $1" >&2
  cat said >&2
  exit 1
}
until_true() {
  what=$1
  shift
  waits 1200 "$@" || never "$what"
}

bus=""
named() {
  bus=$(gdbus call --session --dest org.freedesktop.DBus \
    --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.ListNames |
    grep -o "org\.pwmt\.zathura\.PID-$zathura" || true)
  [ -n "$bus" ]
}
until_true "zathura on the bus" named

page() {
  gdbus call --session --dest "$bus" --object-path /org/pwmt/zathura \
    --method org.freedesktop.DBus.Properties.Get org.pwmt.zathura pagenumber |
    grep -o -E '[0-9]+' | tail -1
}
on() { [ "$(page)" = "$1" ]; }
until_true "the first page" on 0

ends=0
until wtype -k End && waits 300 on 5; do
  ends=$((ends + 1))
  if [ "$ends" -ge 6 ]; then never "End to the last page"; fi
done

wtype -k Home
until_true "Home" on 0
sleep 2
mark() { echo "=== $1 from line $(wc -l < said)"; }
mark slash
wtype /
sleep 1
mark find
wtype find
sleep 2
mark return
wtype -k Return
sleep 3
grim -t png s1-return.png
echo "return page $(page)"
mark escape
wtype -k Escape
sleep 2
grim -t png s2-escape.png
echo "escape page $(page)"
mark colon
wtype ':close'
sleep 1
wtype -k Return
sleep 2
grim -t png s3-colon.png
mark end
cat -n said | tail -200
touch passed
