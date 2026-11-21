# shellcheck shell=bash
# Takes hold of an MX Brio already plugged in as the rules that share it
# arrive with a switch. udev reads a rule only as an event comes, so this
# replays the Brio's add: its nodes are kept from the seat and marked
# relayed, and its capture node is named for the relay, which systemd then
# starts. What logind handed the seat before stays on the node, since
# nothing takes an ACL back as a tag goes, so it's taken back here. The
# Brio is found by its model alone, since udevadm takes a device matching
# any one of the properties given it; the rules check its vendor too.
udevadm trigger --action=add --settle --subsystem-match=video4linux \
  --property-match=ID_MODEL_ID=091d
for node in /dev/video*; do
  [ -c "$node" ] || continue
  relayed=$(udevadm info --query=property --property=CAMERA_RELAYED --value "$node")
  if [ "$relayed" = 1 ]; then
    setfacl --remove-all "$node"
  fi
done
