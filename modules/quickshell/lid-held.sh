# shellcheck shell=bash
# Runs under Keep Awake's hold of the lid (home.nix's keep-awake-lid), for as
# long as it holds, and locks the screen each time the lid shuts away from a
# desk, as the sleep the lid would have brought did first. At a desk,
# lid-docked holds the lid too (../lid/nixos.nix) and the monitor's in use,
# so nothing locks. A machine without the lid module has no lid-docked, and
# every shut lid locks.
#
# It hears logind's word on the lid rather than the lid's switch, since
# logind's is the one the hold changes, and asks again at each thing logind
# says, since a change of one of its properties may come without its value.

lid_shut() {
  local said
  said=$(busctl get-property org.freedesktop.login1 /org/freedesktop/login1 \
    org.freedesktop.login1.Manager LidClosed 2> /dev/null) || return 1
  [ "$said" = 'b true' ]
}

docked() {
  systemctl is-active --quiet lid-docked.service
}

# A line for each thing logind says, which needs no privilege to hear.
said() {
  gdbus monitor --system --dest org.freedesktop.login1 --object-path /org/freedesktop/login1
}

shut=false
if lid_shut; then shut=true; fi

# The watch's own end is read by what follows rather than its status, so a
# watch that fails ends the same way as one that stops.
said | while read -r _; do
  if lid_shut; then now=true; else now=false; fi
  if [ "$now" = true ] && [ "$shut" = false ] && ! docked; then
    echo "locking: the lid shut, kept awake" >&2
    qs ipc call lock lock || echo "the lock didn't answer" >&2
  fi
  shut=$now
done || true

# The hold lasts as long as this runs, so a watch that ends, as when logind
# restarts, keeps the lid held and only stops locking.
echo "logind's word on the lid ended, so a shut lid no longer locks" >&2
exec sleep infinity
