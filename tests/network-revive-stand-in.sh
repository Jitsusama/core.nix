# shellcheck shell=bash
# Stands in for a service that needs the network: it starts only while the
# uplink reaches 10.0.0.53, then runs until stopped. QEMU's own network is
# on another link networkd keeps up throughout, with a default route, so the
# route has to go by the uplink.
#
# --check says whether it could start, and --never that it never could.
# --once starts and ends, as a one-shot job does. --reexec checks as --check
# does, but the first time it passes it has the account's systemd execute
# itself again, holding it there a while, and returns once systemd stops
# answering, noting that it saw it go. systemd keeps its old bus connection
# open until it's back and only then closes it, so the start that follows
# waits on that connection and is told it went, as a start asked in a switch
# is. Sent without waiting, the start could overtake the request; unheld,
# systemd is back in a blink, and the start finds it there and is never
# asked again.
# usage: network-revive-stand-in.sh [--check | --never | --once | --reexec]
set -euo pipefail

# How many times it asks whether systemd still answers before giving up.
LOOKS=1000

reaches() {
  [[ $(ip -o route get 10.0.0.53 2> /dev/null) == *" dev uplink "* ]]
}

# Has systemd execute itself again, and returns once it stops answering.
reexecute() {
  touch "$HOME/reexecuted" "$HOME/hold"
  busctl --user call --expect-reply=no org.freedesktop.systemd1 \
    /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager Reexecute
  # Asked with call, since get-property does not keep to --timeout.
  local looks=0
  while busctl --user --timeout=0.5 call org.freedesktop.systemd1 \
    /org/freedesktop/systemd1 org.freedesktop.DBus.Properties Get \
    ss org.freedesktop.systemd1.Manager Version > /dev/null 2>&1; do
    looks=$((looks + 1))
    if [ "$looks" -ge "$LOOKS" ]; then
      echo "systemd kept answering" >&2
      exit 1
    fi
  done
  touch "$HOME/away"
}

out_of_reach() {
  echo "10.0.0.53 is out of reach" >&2
  exit 1
}

case "${1-}" in
  --check) reaches ;;
  --never)
    echo "never ready" >&2
    exit 1
    ;;
  --reexec)
    reaches || exit 1
    [ -e "$HOME/reexecuted" ] || reexecute
    ;;
  --once) reaches || out_of_reach ;;
  *)
    reaches || out_of_reach
    exec sleep infinity
    ;;
esac
