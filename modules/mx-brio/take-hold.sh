# shellcheck shell=bash
# Takes hold of an MX Brio already plugged in as the rules that share it
# arrive with a switch. udev reads a rule only as an event comes, and an
# added tag sticks to a device until it's removed, so a Brio the seat was
# handed is replayed as removed and then added: the removal takes its old
# watch with its device unit and drops what udev knew of it, uaccess tag and
# all, and the add keeps it from the seat, marks it relayed and names its
# capture node for the relay, which systemd then starts. A Brio already held,
# as at every boot after the first, only has its rules read again, so its
# relay keeps running. What logind handed the seat stays on the node, since
# nothing takes an ACL back as a tag goes, so it's taken back here; a program
# that opened the Brio before keeps what it holds until it lets go. The Brio
# is replayed by its model alone, since udevadm takes a device matching any
# one of the properties given it; the rules check its vendor too.
replay=(--settle --subsystem-match=video4linux --property-match=ID_MODEL_ID=091d)

brios=0
held=yes
for node in /dev/video*; do
  # A node can go as it's read, the Brio unplugged mid-loop; it's skipped.
  facts=$(udevadm info --query=property "$node" 2>/dev/null) || continue
  grep -qx 'ID_VENDOR_ID=046d' <<<"$facts" || continue
  grep -qx 'ID_MODEL_ID=091d' <<<"$facts" || continue
  brios=$((brios + 1))
  if ! grep -qx 'CAMERA_RELAYED=1' <<<"$facts" || grep -q '^TAGS=.*:uaccess:' <<<"$facts"; then
    held=no
  fi
done

if [ "$brios" = 0 ]; then
  echo "No MX Brio is plugged in, so there's nothing to take hold of."
  exit 0
fi
if [ "$held" = no ]; then
  udevadm trigger --action=remove "${replay[@]}"
  udevadm trigger --action=add "${replay[@]}"
  echo "Replayed the MX Brio's arrival, so its relay has it."
else
  udevadm trigger --action=change "${replay[@]}"
  echo "The MX Brio was already held; its rules were read again."
fi

for node in /dev/video*; do
  relayed=$(udevadm info --query=property --property=CAMERA_RELAYED --value "$node" 2>/dev/null) ||
    continue
  [ "$relayed" = 1 ] || continue
  # Only a named user's entry is the seat's; the owner's and group's stay.
  if getfacl -cp "$node" 2>/dev/null | grep -q '^user:[^:]'; then
    setfacl --remove-all "$node" || [ ! -e "$node" ]
    echo "Took back the seat's access to $node."
  fi
done
