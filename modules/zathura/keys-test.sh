# shellcheck shell=bash
# A throwaway: what zathura shows around a search, never merged.
set -eu

zathura "$1" > said 2>&1 &
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
grim -t png s0-home.png
echo "home page $(page)"
wtype /find -k Return
sleep 3
grim -t png s1-together.png
echo "together page $(page)"
wtype -k Escape
wtype -k Home
sleep 1
wtype /find
sleep 1
wtype -k KP_Enter
sleep 3
grim -t png s2-kp.png
echo "kp page $(page)"
wtype -k Escape
wtype -k Home
sleep 1
wtype /find
sleep 1
wtype -k Return
sleep 1
wtype -k Return
sleep 3
grim -t png s3-twice.png
echo "twice page $(page)"
cat said
touch passed
