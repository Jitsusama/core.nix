#!/usr/bin/env bash
# Gives cargo's builds a filesystem of their own: an ext4 image in a file on
# the system's disk, attached with direct IO and mounted where every
# account's cargo puts its intermediates. nixos.nix says why.
#
#   builds.sh attach IMAGE SIZE FOLDER [ACCOUNT:GROUP ...]
#   builds.sh detach FOLDER
set -euo pipefail

# Whether the image's filesystem is sound, repairing what e2fsck can: 0 is
# clean and 1 repaired, and anything above means it's past repairing.
sound() {
  local loop status=0
  loop=$(losetup --find --show "$1")
  e2fsck -p "$loop" >/dev/null 2>&1 || status=$?
  losetup -d "$loop"
  [ "$status" -le 1 ]
}

attach() {
  local image=$1 size=$2 folder=$3
  shift 3
  mkdir -p "$(dirname "$image")" "$folder"
  # A machine stopped mid-build leaves the image as a crash would. What it
  # holds is only cache, so one past repairing is made again.
  if [ -e "$image" ] && ! mountpoint -q "$folder" && ! sound "$image"; then
    rm -f "$image"
  fi
  if [ ! -e "$image" ]; then
    # Copy-on-write is turned off while the file is still empty, the only
    # time Btrfs lets it be, so the filesystem inside only ever overwrites
    # blocks the file already holds. Other filesystems never copy on write.
    touch "$image.new"
    if [ "$(stat -f -c %T "$(dirname "$image")")" = btrfs ]; then
      chattr +C "$image.new"
    fi
    fallocate -l "$size" "$image.new"
    mkfs.ext4 -q -L cargo-builds "$image.new"
    mv "$image.new" "$image"
  fi
  # A size raised since the image was made grows it in place; one lowered
  # leaves it as it is, since shrinking would lose builds for no room.
  fallocate -l "$size" "$image" 2>/dev/null || true
  if ! mountpoint -q "$folder"; then
    local loop
    # Direct IO, so a block isn't cached twice, once for the image file and
    # once for the filesystem inside it.
    loop=$(losetup --find --show --direct-io=on "$image")
    mount -o noatime "$loop" "$folder"
    resize2fs "$loop" >/dev/null 2>&1 || true
  fi
  chmod 0755 "$folder"
  local owner
  for owner in "$@"; do
    install -d -o "${owner%%:*}" -g "${owner##*:}" -m 0700 "$folder/${owner%%:*}"
  done
}

detach() {
  local folder=$1 loop
  if mountpoint -q "$folder"; then
    loop=$(findmnt -n -o SOURCE --target "$folder")
    umount "$folder"
    losetup -d "$loop"
  fi
}

case ${1-} in
  attach) shift && attach "$@" ;;
  detach) shift && detach "$@" ;;
  *)
    echo "usage: builds.sh attach IMAGE SIZE FOLDER [ACCOUNT:GROUP ...] | detach FOLDER" >&2
    exit 64
    ;;
esac
