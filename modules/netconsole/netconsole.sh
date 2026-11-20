# shellcheck shell=bash
# Starts or stops sending the kernel's log out through one interface to the
# listener, as netconsole's target of the interface's name. nixos.nix runs it
# once the interface has an address, again to stop when it goes, and every
# few minutes between to follow the listener.
#
# netconsole doesn't route, so a listener reached only through a gateway is
# refused, since nothing sent to it would arrive. Lines go to the hardware
# address that answers for the listener's address on the interface's own
# link just now, not to the link's broadcast, so while the listener is
# awake the other machines on a switched link aren't sent them. The link
# itself is asked, not the kernel's table of neighbours, which keeps a
# machine that answered once for as long as nothing needs the room. The
# kernel keeps that hardware address, so following is asking again: when
# the listener's address or hardware address has changed, as when it comes
# back from a sleep another machine answered for, the target is made again.
# When nothing answers from the same address on the same link, the target
# is kept, as for a listener asleep, whose switch may then flood the lines
# to the link's other machines; when the listener can't be found there at
# all, as when the card is on another network, the target goes, so no other
# network is sent them. Whatever answers for the address is sent to, so a
# listener named by address is sent to on any network numbered the same
# where something holds it; one named .local is found only where nixos.nix
# says. The lines aren't signed, so the listener can't tell them from a
# forged sender's; darwin.nix says how they're read. Its help takes the
# shape every command of ours does (docs/commands.md), by hand.

# Whether a refusal is said and its code kept, as when starting, or the
# target is let go and the following carries on, as when following.
following=0
# What the target there sends from and to, which a follow reads first.
sent_from='' sent_to='' sent_mac='' enabled=0
# Whether the listener's address is the one sent to, for a name unanswered.
remembered=0

# A target is changed only while it's off. The kernel turns one off itself
# when its interface goes, and refuses to be told twice, so it's told only
# when it's on.
stop() {
  local enabled=0
  if [ -d "$target" ]; then
    read -r enabled <"$target/enabled" || enabled=0
    if [ "$enabled" = 1 ]; then
      echo 0 >"$target/enabled"
    fi
    rmdir "$target"
  fi
}

# Says why the kernel's log isn't sent, and leaves with the code that says
# what kind of why it is. A follow lets a target go, says why once, and
# leaves as done, since the next follow tries again.
unsent() {
  if [ "$following" = 1 ]; then
    if [ -d "$target" ]; then
      echo "netconsole: $2, so the kernel's log is no longer sent" >&2
      stop
    fi
    exit 0
  fi
  echo "netconsole: $2, so the kernel's log isn't sent" >&2
  exit "$1"
}

# Finds the listener's address, the interface's own, and the hardware
# address answering for the listener on the interface's link.
find_listener() {
  local hosts addresses route answers
  hosts=$(getent ahostsv4 "$listener") || hosts=
  read -r address _ <<<"$hosts"
  addresses=$(ip -4 -o address show dev "$interface") || addresses=
  source=$(awk 'NR == 1 { split($4, a, "/"); print a[1] }' <<<"$addresses")
  # A .local listener asleep stops answering for its name too, so a follow
  # from the address it sent from asks for the address it sent to.
  if [ -z "$address" ] && [ "$following" = 1 ] && [ -n "$source" ] &&
    [ "$source" = "$sent_from" ]; then
    address=$sent_to
    remembered=1
  fi
  [ -n "$address" ] || unsent 69 "no address for $listener"
  [ -n "$source" ] || unsent 75 "$interface has no address"
  route=$(ip -4 route get "$address" oif "$interface") || route=
  if [ -z "$route" ] || [[ $route == *" via "* ]]; then
    unsent 78 "$listener at $address isn't on $interface's network"
  fi
  # The link is asked who has the address, and the first answer is taken,
  # as the kernel's own asking would; the kernel writes it back in small
  # letters, so it's kept that way to be compared.
  answers=$(arping -c 1 -w 2 -I "$interface" -s "$source" "$address" 2>/dev/null) || answers=
  mac=$(awk '$1 == "Unicast" && $2 == "reply" {
    gsub(/\[|\]/, "", $5)
    print tolower($5)
    exit
  }' <<<"$answers")
  [ -n "$mac" ] || [ "$following" = 1 ] || unanswered
}

unanswered() {
  unsent 69 "nothing answers for $listener at $address on $interface"
}

send() {
  stop
  mkdir "$target"
  echo "$interface" >"$target/dev_name"
  echo "$source" >"$target/local_ip"
  echo "$address" >"$target/remote_ip"
  echo "$mac" >"$target/remote_mac"
  echo "$port" >"$target/remote_port"
  echo 1 >"$target/extended"
  if ! echo 1 >"$target/enabled" 2>/dev/null; then
    stop
    unsent 75 "the kernel won't send through $interface"
  fi
  echo "netconsole: sending the kernel's log from $source on $interface" \
    "to $listener at $address:$port, $mac"
}

# Makes the target again when what answers for the listener has moved, or
# when nothing is being sent, as when a making of it was refused or the
# kernel turned it off, so the next asking tries again.
follow() {
  following=1
  if [ -d "$target" ]; then
    read -r sent_from <"$target/local_ip" || sent_from=
    read -r sent_to <"$target/remote_ip" || sent_to=
    read -r sent_mac <"$target/remote_mac" || sent_mac=
    read -r enabled <"$target/enabled" || enabled=0
  fi
  find_listener
  if [ -z "$mac" ]; then
    # Nothing answers. A listener asleep on the link it was sent to on is
    # kept, since its lines are lost either way; anywhere else the target
    # goes.
    if [ "$enabled" = 1 ] && [ "$sent_from" = "$source" ] && [ "$sent_to" = "$address" ]; then
      exit 0
    fi
    unanswered
  fi
  # A remembered address answered by another machine is someone else's now,
  # or another network's.
  if [ "$remembered" = 1 ] && [ "$mac" != "$sent_mac" ]; then
    unsent 69 "no address for $listener, and another machine holds $address"
  fi
  if [ "$enabled" != 1 ] || [ "$sent_to" != "$address" ] || [ "$sent_mac" != "$mac" ]; then
    following=0
    send
  fi
}

case "${1:-}" in
-h | --help)
  cat <<'EOF'
netconsole  send the kernel's log to a listener

  Starts or stops sending the kernel's log through one
  interface, to a listener on the same network, as the
  kernel's target of the interface's name. follow asks
  again, and sends anew only if the listener moved or
  nothing is being sent.

EXAMPLES
  netconsole start eth1 listener 6666
  netconsole follow eth1 listener 6666
  netconsole stop eth1

EXIT
  0   done; a follow that finds nothing to send to
      is done too
  1   failed otherwise
  64  asked wrongly: no such action
  69  the listener isn't found on the interface
  75  the interface has no address yet, or the kernel
      won't send through it
  78  the listener is beyond a gateway
EOF
  ;;
start | follow)
  interface=$2 listener=$3 port=$4
  target=/sys/kernel/config/netconsole/$interface
  if [ "$1" = start ]; then
    find_listener
    send
  else
    follow
  fi
  ;;
stop)
  interface=$2
  target=/sys/kernel/config/netconsole/$interface
  stop
  ;;
*)
  echo "netconsole: no action ${1:-}; it takes start, follow or stop" >&2
  exit 64
  ;;
esac
