# shellcheck shell=bash
# Opens a tuple: link, or starts Tuple's daemon when given none. A room's
# name is letters, digits, - and _, as Tuple's links write it, and anything
# else is said rather than run.
# usage: opens.sh <tuple> <notify-send> [link]
set -euo pipefail

tuple=$1
notify=$2
link=${3:-}
if [ -z "$link" ]; then
  exec "$tuple" start
fi

# The room a join link names, by its call_url_slug, or nothing. The query
# is taken apart at each & by hand, so nothing in it is globbed and a
# newline stays in the pair it's in.
room=""
case "$link" in
  tuple://join\?*)
    rest=${link#*\?}
    while :; do
      pair=${rest%%&*}
      case "$pair" in
        call_url_slug=*) room=${pair#call_url_slug=} ;;
      esac
      [ "$pair" != "$rest" ] || break
      rest=${rest#*&}
    done
    ;;
esac
case "$room" in
  "" | *[!A-Za-z0-9_-]*) ;;
  *) exec "$tuple" call join "https://tuple.app/c/$room" ;;
esac
exec "$notify" --app-name=Tuple "Tuple can't open that link" "$link"
