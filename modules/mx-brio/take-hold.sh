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
# drops the properties a match by its ids would read. The add follows the
# removal however the run ends, since a removal alone leaves a Brio nobody
# can open and no later run can find. The old watch is let end before the
# add, since as it ends it removes the camera's holders file, which the
# loopback's watch, mx-brio-holders, writes under the same name once the
# relay starts. Once replayed, each node is read again, and a warning says
# why if one isn't held or the seat can still open it. The run doesn't fail
# for it: a machine that updates itself goes back a generation when a unit
# fails, undoing every other change, and the one before reads the same rules,
# so going back would leave the Brio as it is.

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
  # One that fails is let pass, so the check after the replay says so.
  if ! setfacl --remove-all "$1" && [ -e "$1" ]; then
    echo "Couldn't take back the seat's access to $1." >&2
    return 0
  fi
  echo "Took back the seat's access to $1."
}

# Whether the seat can still open a node: a named user's entry is left.
the_seat_can_open() {
  local acl
  acl=$(getfacl -cp "$1" 2>/dev/null) || return 1
  grep -q '^user:[^:]' <<<"$acl"
}

# The Brio's devices still there, since one unplugged mid-run can't be
# replayed, and the kernel's own removal has already done what's needed.
still_there() {
  local path
  for path in "${paths[@]}"; do
    if [ -e "$path" ]; then
      echo "$path"
    fi
  done
}

# Replays to what's still there an `$1` of the Brio's devices, waiting on
# udev with the rest of `$@`.
replay() {
  local action=$1 there
  shift
  there=$(still_there)
  if [ -n "$there" ]; then
    mapfile -t there <<<"$there"
    udevadm trigger "$@" --action="$action" "${there[@]}"
  fi
}

# Whether a node's facts are the Brio's: the same match as mx-brio's rule
# in nixos.nix; change both together.
the_brio() {
  grep -qx 'ID_USB_DRIVER=uvcvideo' <<<"$1" &&
    grep -qx 'ID_VENDOR_ID=046d' <<<"$1" &&
    grep -qx 'ID_MODEL_ID=091d' <<<"$1"
}

nodes=()
paths=()
watches=()
held=yes
removed=no
for node in /dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop; it's skipped.
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  the_brio "$facts" || continue
  path=$(udevadm info --query=path "$node" 2>/dev/null) || continue
  nodes+=("$node")
  paths+=("/sys$path")
  watches+=("usb-camera-holders@${node#/dev/}.service")
  if ! grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || the_seats "$facts"; then
    held=no
  fi
done

if [ "${#nodes[@]}" = 0 ]; then
  echo "No MX Brio is plugged in, or udev hasn't read one yet," \
    "so there's nothing to take hold of."
  exit 0
fi
for node in "${nodes[@]}"; do
  take_back "$node"
done
if [ "$held" = no ]; then
  # Stopped, timed out or failed past here, the run still adds the Brio
  # back. That add doesn't wait on udev, since a timeout is a udev that
  # stopped answering; the add is queued for when it answers again.
  trap '[ "$removed" = no ] || replay add' EXIT
  echo "Replaying the MX Brio's arrival."
  removed=yes
  replay remove --settle
  # A watch already gone with its device is what's wanted, so its stop
  # failing for that is no failure.
  systemctl stop "${watches[@]}" || true
  replay add --settle
  removed=no
  echo "Replayed the MX Brio's arrival, so its relay has it."
else
  replay change --settle
  echo "The MX Brio was already held; its rules were read again."
fi

for node in "${nodes[@]}"; do
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  # Replugged mid-run, the name can now be another camera's, which stays
  # the seat's.
  the_brio "$facts" || continue
  if ! grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || the_seats "$facts"; then
    # <4> has the journal keep it as a warning.
    echo "<4>The MX Brio at $node isn't held:" \
      "udev didn't mark it relayed and keep it from the seat." \
      "Replugging it replays its arrival; journalctl -u systemd-udevd says why it wasn't." >&2
  fi
  take_back "$node"
  if the_seat_can_open "$node"; then
    echo "<4>The seat can still open the MX Brio at $node: its access couldn't be taken back." \
      "getfacl -p $node shows who holds it." >&2
  fi
done
