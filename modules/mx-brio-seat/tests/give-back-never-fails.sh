#!/usr/bin/env bash
# Runs give-back.sh against a stand-in udevadm (./udevadm) over a made-up
# /dev and /sys, so its promises show without a machine: it replays only the
# MX Brio, never another camera a relay holds; it ends well whether or not
# udev answers; and it says the Brio was given back only when udev's record
# shows it was.
#
# Usage: give-back-never-fails.sh GIVE_BACK_SCRIPT
set -euo pipefail
export LC_ALL=C

script=$1
here=${BASH_SOURCE[0]%/*}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cp "$here/udevadm" "$work/bin/udevadm"
chmod +x "$work/bin/udevadm"

brio='ID_USB_DRIVER=uvcvideo
ID_VENDOR_ID=046d
ID_MODEL_ID=091d'
held="$brio
CAMERA_RELAYED=1
CURRENT_TAGS=:seat:systemd:"
given="$brio
CURRENT_TAGS=:seat:systemd:uaccess:"
other='ID_USB_DRIVER=uvcvideo
ID_VENDOR_ID=1bcf
ID_MODEL_ID=2284
CAMERA_RELAYED=1
CURRENT_TAGS=:seat:systemd:'

# A fresh machine: video0 the Brio with the first record, video2 another
# camera a relay holds, and the second record what udev keeps of the Brio
# once it's added again.
machine() {
  rm -rf "$work/root"
  mkdir -p "$work/root"/{dev,sys/devices,facts,after}
  local node
  for node in video0 video2; do
    touch "$work/root/dev/$node" "$work/root/sys/devices/$node"
  done
  echo "$1" >"$work/root/facts/video0"
  echo "$other" >"$work/root/facts/video2"
  echo "$2" >"$work/root/after/video0"
  touch "$work/root/triggers"
}

# Runs the give-back, failing the test on anything but a clean end.
give_back() {
  local ended=0
  MX_BRIO_ROOT="$work/root" PATH="$work/bin:$PATH" \
    bash "$script" >"$work/said" 2>"$work/warned" || ended=$?
  if [ "$ended" != 0 ]; then
    echo "the give-back ended $ended in: $1" >&2
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

machine "$held" "$given"
give_back "a Brio held for a relay"
expect said "Gave the seat back the MX Brio."
expect root/triggers "remove video0"
expect root/triggers "add video0"
refuse root/triggers video2

machine "$held" "$given"
touch "$work/root/fails"
give_back "a udev that refuses each replay"
expect warned "udev didn't settle the MX Brio's removal"
expect warned "udev didn't settle the MX Brio's arrival"
expect warned "still kept from the seat"
refuse said "Gave the seat back"

machine "$held" "$given"
touch "$work/root/hangs"
give_back "a udev that never settles"
expect warned "udev didn't settle the MX Brio's removal"
expect root/triggers "add video0"
expect said "Gave the seat back the MX Brio."

machine "$held" "$held"
give_back "a replay read under the old rules"
expect warned "The MX Brio at $work/root/dev/video0 is still kept from the seat."
expect warned "systemctl restart mx-brio-seat"
refuse said "Gave the seat back"

machine "$given" "$given"
give_back "a Brio the seat has"
expect said "No MX Brio is held for a relay"
refuse root/triggers video

echo "the give-back replays only the MX Brio and never fails"
