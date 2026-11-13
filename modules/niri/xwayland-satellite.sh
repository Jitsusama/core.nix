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
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand.
set -euo pipefail

for arg in "$@"; do
  case $arg in
  -h | --help)
    cat << 'EOF'
xwayland-satellite  niri's Xwayland, open only to its cookie

EXAMPLES
  xwayland-satellite :0 -listenfd 3   what niri starts

The cookie is written where XAUTHORITY says, once a
login, and Xwayland is given it with -auth. Every other
option is the satellite's own.

EXIT
  0   Xwayland ended
EOF
    exit 0
    ;;
  *) ;;
  esac
done

# A niri started before the session named XAUTHORITY starts programs that
# couldn't find a cookie, so its Xwayland stays open, as before, until the
# next login.
# shellcheck disable=SC2154 # SATELLITE comes from xwayland-satellite.nix's runtimeEnv.
if [ -z "${XAUTHORITY-}" ]; then
  echo "xwayland-satellite: no XAUTHORITY, so Xwayland takes any client" >&2
  exec "$SATELLITE" "$@"
fi
auth=$XAUTHORITY

if [ ! -s "$auth" ]; then
  hex() { od -An -v -tx1 | tr -d ' \n'; }
  umask 077
  : > "$auth.new"
  protocol=$(printf MIT-MAGIC-COOKIE-1 | hex)
  cookie=$(head -c 16 /dev/urandom | hex)
  printf 'ffff 0000  0000  0012 %s 0010 %s\n' "$protocol" "$cookie" |
    xauth -q -f "$auth.new" nmerge -
  mv "$auth.new" "$auth"
fi

for arg in "$@"; do
  [ "$arg" = --test-listenfd-support ] && exec "$SATELLITE" "$@"
done
exec "$SATELLITE" "$@" -auth "$auth"
