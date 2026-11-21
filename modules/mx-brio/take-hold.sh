# shellcheck shell=bash
# Takes hold of an MX Brio already plugged in as the rules that share it
# arrive with a switch. udev reads a rule only as an event comes, so a Brio
# the seat was handed is replayed as removed and then added: the removal
# takes its old watch with its device unit, and the add keeps it from the
# seat, marks it relayed and names its capture node for the relay, which
# systemd then starts. A Brio already held, as at every boot after the
# first, only has its rules read again, so its relay keeps running.
#
# What logind handed the seat stays on the node, since nothing takes an ACL
# back as a tag goes, so it's taken back first, before the replay tells the
# seat's programs the Brio has come; a program that opened it before keeps
# what it holds until it lets go. The seat is handed what holds the uaccess
# tag now, CURRENT_TAGS; TAGS keeps every tag an event added, the one
# systemd's 70-uaccess adds before mx-brio's rule takes it off among them.
# The Brio's devices are replayed by their paths as found, since a removal
# drops the properties a match by its ids would read. Once replayed, each
# node is read again, and the run fails, saying why, if one isn't held.

# Whether the facts udev gives a node say the seat may be handed it.
the_seats() {
  grep -q '^CURRENT_TAGS=.*:uaccess:' <<<"$1"
}

# Takes back what the seat was handed on a node: only a named user's entry
# is the seat's, and the owner's and group's stay.
take_back() {
  local acl
  acl=$(getfacl -cp "$1" 2>/dev/null) || return 0
  grep -q '^user:[^:]' <<<"$acl" || return 0
  setfacl --remove-all "$1" || [ ! -e "$1" ]
  echo "Took back the seat's access to $1."
}

nodes=()
paths=()
held=yes
for node in /dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop; it's skipped.
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  grep -qx 'ID_VENDOR_ID=046d' <<<"$facts" || continue
  grep -qx 'ID_MODEL_ID=091d' <<<"$facts" || continue
  path=$(udevadm info --query=path "$node" 2>/dev/null) || continue
  nodes+=("$node")
  paths+=("/sys$path")
  if ! grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || the_seats "$facts"; then
    held=no
  fi
done

if [ "${#nodes[@]}" = 0 ]; then
  echo "No MX Brio is plugged in, so there's nothing to take hold of."
  exit 0
fi
for node in "${nodes[@]}"; do
  take_back "$node"
done
if [ "$held" = no ]; then
  udevadm trigger --settle --action=remove "${paths[@]}"
  udevadm trigger --settle --action=add "${paths[@]}"
  echo "Replayed the MX Brio's arrival, so its relay has it."
else
  udevadm trigger --settle --action=change "${paths[@]}"
  echo "The MX Brio was already held; its rules were read again."
fi

loose=0
for node in "${nodes[@]}"; do
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  if ! grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || the_seats "$facts"; then
    echo "The MX Brio at $node isn't held:" \
      "udev didn't mark it relayed and keep it from the seat." >&2
    loose=1
  fi
  take_back "$node"
done
if [ "$loose" = 1 ]; then
  echo "Replugging the Brio replays its arrival;" \
    "journalctl -u systemd-udevd says why it wasn't." >&2
  exit 1
fi
