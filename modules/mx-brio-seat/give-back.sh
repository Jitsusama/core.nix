# shellcheck shell=bash
# Gives the seat back an MX Brio that ../mx-brio's sharing kept from it. udev
# reads a rule only as an event comes, and keeps the record the sharing's rule
# gave the Brio, its seat's tag gone and its nodes marked relayed, until the
# Brio arrives again. So each node still marked relayed has its arrival
# replayed: removed, then added under the machine's rules as they are now,
# which hand it to the seat and start the seat's own camera watch on it. A
# change event wouldn't do, since udev carries a node's old marks into one,
# and the relayed mark keeps the seat's watch off.
#
# A Brio already the seat's, as at every boot after the first, is left alone.
# The add follows the removal however the run ends, since a removal alone
# leaves a camera nobody can open.

paths=()
for node in /dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop; it's skipped.
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || continue
  path=$(udevadm info --query=path "$node" 2>/dev/null) || continue
  paths+=("/sys$path")
done

if [ "${#paths[@]}" = 0 ]; then
  echo "No camera is held for a relay, so the seat has every one already."
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

echo "Giving the seat back ${#paths[@]} camera nodes held for a relay."
trap 'replay add' EXIT
replay remove --settle
replay add --settle
trap - EXIT
echo "Gave the seat back the camera nodes held for a relay."
