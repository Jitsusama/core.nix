#!/usr/bin/env bash
# Runs give-back.sh against a stand-in udevadm (./udevadm.sh) over a made-up
# /dev and /sys, under set -e as its unit runs it, so its promises show
# without a machine: it replays only the MX Brio, never another camera a
# relay holds, even one of the Brio's maker or model; it bounds each wait on
# udev inside the unit's limit, and ends well whether or not udev answers;
# and it says the Brio was given back only when udev's record shows it was.
#
# Usage: give-back-never-fails.sh GIVE_BACK_SCRIPT UNIT_TIMEOUT_SECONDS
set -euo pipefail
export LC_ALL=C

script=$1
export MX_BRIO_TIMEOUT=$2
here=${BASH_SOURCE[0]%/*}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
# Through the bash this runs under, since a build's sandbox has no
# /usr/bin/env for a shebang to find.
printf '#!%s\nexec %q %q "$@"\n' "$BASH" "$BASH" "$here/udevadm.sh" >"$work/bin/udevadm"
chmod +x "$work/bin/udevadm"

brio='ID_USB_DRIVER=uvcvideo
ID_VENDOR_ID=046d
ID_MODEL_ID=091d'
# As udev keeps a held Brio: 70-uaccess's tag is among every tag an event
# added, and gone only from the current ones.
held="$brio
CAMERA_RELAYED=1
TAGS=:seat:systemd:uaccess:
CURRENT_TAGS=:seat:systemd:"
given="$brio
TAGS=:seat:systemd:uaccess:
CURRENT_TAGS=:seat:systemd:uaccess:"
untagged="$brio
TAGS=:seat:systemd:uaccess:
CURRENT_TAGS=:seat:systemd:"
# Another camera of the Brio's maker, and one of its model number from
# another maker, each held for a relay of its own.
kin='ID_USB_DRIVER=uvcvideo
ID_VENDOR_ID=046d
ID_MODEL_ID=082d
CAMERA_RELAYED=1
CURRENT_TAGS=:seat:systemd:'
namesake='ID_USB_DRIVER=uvcvideo
ID_VENDOR_ID=1bcf
ID_MODEL_ID=091d
CAMERA_RELAYED=1
CURRENT_TAGS=:seat:systemd:'

# A fresh machine: video0 the Brio with the first record, video2 and video4
# the other cameras, and the second record what udev keeps of the Brio once
# it's added again.
machine() {
  rm -rf "$work/root"
  mkdir -p "$work/root"/{dev,sys/devices,facts,after}
  local node
  for node in video0 video2 video4; do
    touch "$work/root/dev/$node" "$work/root/sys/devices/$node"
  done
  echo "$1" >"$work/root/facts/video0"
  echo "$kin" >"$work/root/facts/video2"
  echo "$namesake" >"$work/root/facts/video4"
  echo "$2" >"$work/root/after/video0"
  touch "$work/root/triggers"
}

# Runs the give-back as its unit does, under set -e, failing the test on
# anything but a clean end or on anything the stand-in refused.
give_back() {
  local ended=0
  MX_BRIO_ROOT="$work/root" PATH="$work/bin:$PATH" \
    bash -e "$script" "$work/root" >"$work/said" 2>"$work/warned" || ended=$?
  if [ "$ended" != 0 ]; then
    echo "the give-back ended $ended in: $1; it warned: $(<"$work/warned")" >&2
    exit 1
  fi
  if [ -e "$work/root/refused" ]; then
    echo "the give-back asked udev what it must never, in $1: $(<"$work/root/refused")" >&2
    exit 1
  fi
}

# Whether a file of the run holds a line, or doesn't, failing the test if not.
expect() {
  if ! grep -qF "$2" "$work/$1"; then
    echo "expected '$2' in $1, which holds: $(<"$work/$1")" >&2
    exit 1
  fi
}

refuse() {
  if grep -qF "$2" "$work/$1"; then
    echo "didn't expect '$2' in $1, which holds: $(<"$work/$1")" >&2
    exit 1
  fi
}

# The replays made, in order, against those expected.
replayed() {
  local made
  made=$(<"$work/root/triggers")
  if [ "$made" != "$1" ]; then
    echo "expected the replays '$1', but the give-back made '$made'" >&2
    exit 1
  fi
}

machine "$held" "$given"
give_back "a Brio held for a relay"
expect said "Gave the seat back the MX Brio."
replayed $'remove video0\nadd video0'

machine "$held" "$given"
touch "$work/root/fails"
give_back "a udev that refuses each replay"
expect warned "udev didn't settle the MX Brio's removal"
expect warned "udev didn't settle the MX Brio's arrival"
expect warned "still kept from the seat"
refuse said "Gave the seat back"
replayed $'remove video0\nadd video0'

machine "$held" "$held"
touch "$work/root/hangs"
give_back "a udev that never settles"
expect warned "udev didn't settle the MX Brio's removal"
expect warned "udev didn't settle the MX Brio's arrival"
expect warned "still kept from the seat"
refuse said "Gave the seat back"
replayed $'remove video0\nadd video0'

machine "$held" "$held"
give_back "a replay read under the old rules"
expect warned "The MX Brio at $work/root/dev/video0 is still kept from the seat."
expect warned "systemctl restart mx-brio-seat"
refuse said "Gave the seat back"

machine "$held" "$untagged"
give_back "a Brio unmarked but untagged for the seat"
expect warned "still kept from the seat"
refuse said "Gave the seat back"

machine "$held" "$given"
touch "$work/root/after/video0.gone"
give_back "a Brio whose record is lost in its arrival"
expect warned "udev couldn't say how the MX Brio at $work/root/dev/video0 stands"
refuse said "Gave the seat back"

machine "$given" "$given"
give_back "a Brio the seat has"
expect said "No MX Brio is held for a relay"
replayed ""

echo "the give-back replays only the MX Brio and never fails"
