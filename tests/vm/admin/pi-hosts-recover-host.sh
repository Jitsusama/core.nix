# shellcheck shell=bash
# Stands in for the node a pi host runs on, in pi-hosts-recover's VM. It
# takes the host's command line, writes its pid to its kind's starts as it
# begins, says it's ready as the host does once it listens, and pings the
# watchdog each second from its own loop. A file named hang in its folder
# makes one start stop pinging while it stays alive, as a host whose loop
# is wedged does; one named crash makes every start fail once it's ready.
kind="${*: -1}"
dir="$HOME/stand-in"
mkdir -p "$dir"
echo "$$" >>"$dir/$kind.starts"
systemd-notify --ready "STATUS=ready, serving no conversations"
while true; do
  if [ -e "$dir/crash" ]; then
    exit 1
  fi
  # Only one start hangs, so the next comes back to a loop that turns.
  if mv "$dir/hang" "$dir/hung" 2>/dev/null; then
    exec sleep infinity
  fi
  systemd-notify WATCHDOG=1
  sleep 1
done
