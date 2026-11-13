# shellcheck shell=bash
# niri starts xwayland-satellite as the first X11 program connects, and the
# satellite starts Xwayland, which with no -auth lets any account on the
# machine connect, read keys and type into X11 windows. This hands Xwayland
# a cookie, kept where the session's XAUTHORITY says (home.nix), readable
# by Joel alone, so only a program that can read it connects.
#
# The first client connects before the satellite starts, so its cookie has
# to be there first: it is written when niri, starting, asks whether the
# satellite takes the sockets it opens, and kept for the login, as the
# runtime folder it's in is. One entry with no display or address, which a
# client takes for any display on any host, serves every niri of the
# session, a nested one's too, and every restart of Xwayland.
# usage: xwayland-satellite [:<display>] [option]...
set -euo pipefail

# A niri started before the session named XAUTHORITY starts programs that
# couldn't find a cookie, so its Xwayland stays open, as before, until the
# next login.
if [ -z "${XAUTHORITY-}" ]; then
  echo "xwayland-satellite: no XAUTHORITY, so Xwayland takes any client" >&2
  exec "$SATELLITE" "$@"
fi
auth=$XAUTHORITY

if [ ! -s "$auth" ]; then
  hex() { od -An -v -tx1 | tr -d ' \n'; }
  umask 077
  : > "$auth.new"
  printf 'ffff 0000  0000  0012 %s 0010 %s\n' "$(printf MIT-MAGIC-COOKIE-1 | hex)" \
    "$(head -c 16 /dev/urandom | hex)" |
    xauth -q -f "$auth.new" nmerge -
  mv "$auth.new" "$auth"
fi

for arg in "$@"; do
  [ "$arg" = --test-listenfd-support ] && exec "$SATELLITE" "$@"
done
exec "$SATELLITE" "$@" -auth "$auth"
