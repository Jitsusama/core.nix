#!/usr/bin/env bash
# Gives cargo's builds a filesystem of their own: an ext4 image in a file on
# the system's disk, attached with direct IO and mounted where every
# account's cargo puts its intermediates. nixos.nix says why.
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
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

# The size in bytes to make or grow IMAGE to: the one asked, or half what
# its disk would have free without it, when that's less. The image holds
# only what its builds write, so the size is a ceiling, and on a small
# disk, such as a stick's, half keeps builds from filling it. What the
# image holds counts as free, and half of that stays put once reached, so
# attaching again at each boot doesn't creep the image up.
room() {
  local image=$1 asked has=0 blocks block most disk
  asked=$(numfmt --from=iec "$2")
  disk=$(dirname "$image")
  # Btrfs counts a file's new blocks at once but its free space only once
  # they're committed, so the two are read after a sync, or what was just
  # written would count twice.
  sync -f "$disk"
  if [ -e "$image" ]; then
    has=$(($(stat -c '%b * %B' "$image")))
  fi
  read -r blocks block < <(stat -f -c '%a %S' "$disk")
  most=$(((blocks * block + has) / 2))
  echo $((asked < most ? asked : most))
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
    # time Btrfs lets it be, so a block the filesystem inside writes over is
    # written in place. Other filesystems never copy on write.
    touch "$image.new"
    if [ "$(stat -f -c %T "$(dirname "$image")")" = btrfs ]; then
      chattr +C "$image.new"
    fi
    # Sparse: the file is its size but holds no blocks until written.
    truncate -s "$(room "$image.new" "$size")" "$image.new"
    mkfs.ext4 -q -L cargo-builds "$image.new"
    mv "$image.new" "$image"
  fi
  # A size raised since the image was made, or room freed since it was made
  # smaller than asked, grows it in place; one lowered leaves it as it is,
  # since shrinking would lose builds for no room.
  local want made
  want=$(room "$image" "$size")
  made=$(stat -c %s "$image")
  if [ "$want" -gt "$made" ]; then
    truncate -s "$want" "$image"
  fi
  # Discard hands each block a build deletes back to the system's disk as
  # the filesystem frees it: the loop device punches it out of the image.
  if ! mountpoint -q "$folder"; then
    local loop
    # Direct IO, so a block isn't cached twice, once for the image file and
    # once for the filesystem inside it.
    loop=$(losetup --find --show --direct-io=on "$image")
    mount -o noatime,discard "$loop" "$folder"
    resize2fs "$loop" >/dev/null 2>&1 || true
  else
    mount -o remount,noatime,discard "$folder"
    # A loop device keeps the size its file had when attached, so an image
    # grown while mounted, as at a switch, is read again and its
    # filesystem grown online; otherwise the room came only at a reboot.
    loop=$(findmnt -n -o SOURCE --target "$folder")
    losetup -c "$loop"
    resize2fs "$loop" >/dev/null 2>&1 || true
  fi
  # One trim hands back what was freed before discard was on, as in an
  # image made with every byte reserved, which older machines hold.
  fstrim "$folder" || true
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
  -h | --help)
    cat << 'EOF'
cargo-builds  give cargo's builds a filesystem of their own

  attach makes or grows an ext4 image of at most SIZE,
  or half the disk's room, and mounts it at FOLDER with
  a folder for each ACCOUNT:GROUP; detach unmounts it.
  Its unit runs both.

EXAMPLES
  cargo-builds attach /var/cache/cargo-builds.img 96G \
    /var/cache/cargo-builds jitsusama:users
  cargo-builds detach /var/cache/cargo-builds

EXIT
  0   attached or detached
  64  asked wrongly
  else what failed: losetup, mkfs, e2fsck or mount
EOF
    ;;
  attach) shift && attach "$@" ;;
  detach) shift && detach "$@" ;;
  *)
    printf '%s\n' "cargo-builds takes attach or detach" \
      "  try  cargo-builds --help" >&2
    exit 64
    ;;
esac
