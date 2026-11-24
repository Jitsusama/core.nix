# shellcheck shell=bash
# Gives the seat back an MX Brio that ../mx-brio's sharing kept from it. udev
# reads a rule only as an event comes, and keeps the record the sharing's rule
# gave the Brio, its seat's tag gone and its nodes marked relayed, until the
# Brio arrives again. So each of the Brio's nodes still marked relayed has its
# arrival replayed: removed, then added under the machine's rules as they are
# now, which hand it to the seat and start the seat's own camera watch on it.
# A change event wouldn't do, since udev carries a node's old marks into one,
# and the relayed mark keeps the seat's watch off.
#
# A Brio already the seat's, as at every boot after the first, is left alone,
# and so is any other camera a relay holds, which its own rule marks again.
# The add follows the removal however the run ends, since a removal alone
# leaves a camera nobody can open. The run never fails, even when a replay
# does: a machine that updates itself goes back a generation when a unit
# fails, and going back hides the Brio again. It warns instead, naming the
# way out, and bounds each wait on udev well inside the unit's own limit,
# since the unit timing out fails it as surely as an error.
#
# Its one argument, which the unit never gives, roots /dev and /sys for its
# tests, so nothing in a unit's environment can steer what root replays.

root=${1:-}
# How long udev may take to settle each replay, twice over within the
# unit's TimeoutStartSec.
settling=25

# Whether udev holds a record of a node, which it writes as an add is
# handled, and deletes as a removal is, leaving only the kernel's own keys.
recorded() {
  grep -q '^USEC_INITIALIZED=' <<<"$1"
}

# Whether a node's facts are the Brio's: the same match as ../mx-brio's rule
# and its take-hold.sh; change all three together.
the_brio() {
  grep -qx 'ID_USB_DRIVER=uvcvideo' <<<"$1" &&
    grep -qx 'ID_VENDOR_ID=046d' <<<"$1" &&
    grep -qx 'ID_MODEL_ID=091d' <<<"$1"
}

nodes=()
paths=()
unread=()
unrecorded=()
for node in "$root"/dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop, and is skipped;
  # one still there that udev can't say of may be the Brio, held, and so
  # may one udev removed and never added again. With no camera at all, the
  # pattern itself comes back, and isn't there either. udevadm says why it
  # couldn't read a node, into the unit's journal.
  if ! facts=$(udevadm info --query=property "$node"); then
    if [ -e "$node" ]; then
      unread+=("$node")
    fi
    continue
  fi
  if ! recorded "$facts"; then
    unrecorded+=("$node")
    continue
  fi
  the_brio "$facts" || continue
  grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || continue
  path=$(udevadm info --query=path "$node" 2>/dev/null) || continue
  nodes+=("$node")
  paths+=("$root/sys$path")
done

# <4> has the journal keep it as a warning.
if [ "${#unread[@]}" -gt 0 ]; then
  echo "<4>udev couldn't say how the cameras at ${unread[*]} stand, so an MX Brio" \
    "among them may still be kept from the seat; udevadm's words above say why." >&2
fi
if [ "${#unrecorded[@]}" -gt 0 ]; then
  echo "<4>udev holds no record of the cameras at ${unrecorded[*]}, as after a removal" \
    "it never added again, so an MX Brio among them is kept from the seat until it's" \
    "replugged or udevadm trigger --action=add adds it." >&2
fi
if [ "${#paths[@]}" = 0 ]; then
  if [ "${#unread[@]}" = 0 ] && [ "${#unrecorded[@]}" = 0 ]; then
    echo "No MX Brio is held for a relay, so the seat has it already."
  fi
  exit 0
fi

# Only what's still there is replayed, since one unplugged mid-run is gone
# already and comes back under the new rules when it's plugged in.
replay() {
  local action=$1 path there=()
  shift
  for path in "${paths[@]}"; do
    if [ -e "$path" ]; then
      there+=("$path")
    fi
  done
  if [ "${#there[@]}" -gt 0 ]; then
    udevadm trigger --action="$action" "${there[@]}"
  fi
}

# Waits for udev to finish what a replay queued, for at most $settling s.
settled() {
  udevadm settle --timeout="$settling"
}

echo "Giving the seat back the MX Brio's ${#paths[@]} nodes held for a relay."
trap 'replay add || true' EXIT
if ! replay remove || ! settled; then
  echo "<4>udev didn't settle the MX Brio's removal; adding it again." >&2
fi
arrived=yes
if ! replay add; then
  echo "<4>udev refused the MX Brio's arrival." >&2
elif ! settled; then
  arrived=no
fi
trap - EXIT

# udev handles a node's events in order, so an arrival it hasn't finished
# may still give the seat the Brio, and its record now says nothing yet.
if [ "$arrived" = no ]; then
  echo "<4>udev didn't finish the MX Brio's arrival in ${settling} s, so whether the" \
    "seat has it isn't known yet; udevadm info ${nodes[*]} says once udev has." >&2
  exit 0
fi

# udev's record of each node as the add left it, so a replay read under the
# old rules says so rather than passing for one that worked.
given=yes
for node in "${nodes[@]}"; do
  if [ ! -e "$node" ]; then
    given=no
    echo "The MX Brio was unplugged as it was given back; plugged in, it's the seat's."
    continue
  fi
  if ! facts=$(udevadm info --query=property "$node"); then
    given=no
    echo "<4>udev couldn't say how the MX Brio at $node stands after its arrival;" \
      "udevadm's words above say why." >&2
    continue
  fi
  # An arrival udev finished with no record left means the add never came.
  if ! recorded "$facts"; then
    given=no
    echo "<4>udev holds no record of the MX Brio at $node since its removal, so it's" \
      "kept from the seat until it's replugged or udevadm trigger --action=add adds it;" \
      "journalctl -u systemd-udevd says why the add didn't come." >&2
    continue
  fi
  if grep -qx 'CAMERA_RELAYED=1' <<<"$facts" ||
    ! grep -q '^CURRENT_TAGS=.*:uaccess:' <<<"$facts"; then
    given=no
    echo "<4>The MX Brio at $node is still kept from the seat." \
      "systemctl restart mx-brio-seat, or replugging it, replays its arrival;" \
      "journalctl -u systemd-udevd says why it wasn't." >&2
  fi
done
if [ "$given" = yes ]; then
  echo "Gave the seat back the MX Brio."
fi
exit 0
