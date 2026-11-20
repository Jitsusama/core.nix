#!/usr/bin/env bash
# Proves, over a machine's whole set of udev rules as udev reads them, that
# the MX Brio's rule takes the seat's tag off the Brio after every rule
# that gives a camera that tag and before the rule that hands tagged nodes
# to the seat, so the tag is gone by the handing; that udev reads the rule
# without a complaint; and that it marks the Brio relayed and starts the
# relay on its capture node.
#
# Usage: the-brio-is-kept-from-the-seat.sh RULES_FOLDER
set -euo pipefail
export LC_ALL=C

rules=$1
ours=71-mx-brio.rules
if [ ! -f "$rules/$ours" ]; then
  echo "the machine's udev rules have no $ours"
  exit 1
fi
udevadm verify --resolve-names=never "$rules/$ours"

tagging=''
handing=''
for file in "$rules"/*.rules; do
  name=${file##*/}
  if grep -qE 'SUBSYSTEM=="video4linux".*TAG\+="uaccess"' "$file"; then
    tagging=$name
  fi
  if [ -z "$handing" ] && grep -qF 'RUN{builtin}+="uaccess"' "$file"; then
    handing=$name
  fi
done
if [ -z "$tagging" ] || [ -z "$handing" ]; then
  echo "the machine's udev rules don't tag cameras for the seat or hand them to it"
  exit 1
fi
if [[ ! $tagging < $ours ]]; then
  echo "$tagging gives cameras the seat's tag after $ours takes it off the Brio"
  exit 1
fi
if [[ ! $ours < $handing ]]; then
  echo "$handing hands cameras to the seat before $ours takes the Brio's tag off"
  exit 1
fi

for match in \
  'ENV{ID_VENDOR_ID}=="046d", ENV{ID_MODEL_ID}=="091d", TAG-="uaccess", ENV{CAMERA_RELAYED}="1"' \
  'ATTR{index}=="0", SYMLINK+="mx-brio", TAG+="systemd", ENV{SYSTEMD_WANTS}+="mx-brio.service"'; do
  if ! grep -qF "$match" "$rules/$ours"; then
    echo "the Brio's rule lacks: $match"
    exit 1
  fi
done
