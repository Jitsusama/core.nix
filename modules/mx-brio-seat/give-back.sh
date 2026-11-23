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
# way out.

# Whether a node's facts are the Brio's: the same match as ../mx-brio's rule
# and its take-hold.sh; change all three together.
the_brio() {
  grep -qx 'ID_USB_DRIVER=uvcvideo' <<<"$1" &&
    grep -qx 'ID_VENDOR_ID=046d' <<<"$1" &&
    grep -qx 'ID_MODEL_ID=091d' <<<"$1"
}

nodes=()
paths=()
for node in /dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop; it's skipped.
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  the_brio "$facts" || continue
  grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || continue
  path=$(udevadm info --query=path "$node" 2>/dev/null) || continue
  nodes+=("$node")
  paths+=("/sys$path")
done

if [ "${#paths[@]}" = 0 ]; then
  echo "No MX Brio is held for a relay, so the seat has it already."
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
    udevadm trigger "$@" --action="$action" "${there[@]}"
  fi
}

echo "Giving the seat back the MX Brio's ${#paths[@]} nodes held for a relay."
trap 'replay add || true' EXIT
# <4> has the journal keep it as a warning.
replay remove --settle ||
  echo "<4>udev didn't settle the MX Brio's removal; adding it again." >&2
replay add --settle ||
  echo "<4>udev didn't settle the MX Brio's arrival." >&2
trap - EXIT

# udev's record of each node as the add left it, so a replay read under the
# old rules says so rather than passing for one that worked.
given=yes
for node in "${nodes[@]}"; do
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  if grep -qx 'CAMERA_RELAYED=1' <<<"$facts" ||
    ! grep -q '^CURRENT_TAGS=.*:uaccess:' <<<"$facts"; then
    given=no
    echo "<4>The MX Brio at $node is still kept from the seat." \
      "Replugging it replays its arrival; journalctl -u systemd-udevd says why it wasn't." >&2
  fi
done
if [ "$given" = yes ]; then
  echo "Gave the seat back the MX Brio."
fi
exit 0
