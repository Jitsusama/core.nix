# shellcheck shell=bash
# Runs systemd-tmpfiles with the machine's Btrfs rules over a copy of sysfs
# holding two filesystems, every reclaim setting off, and checks reclaim is
# on for data and metadata in both and left off for the system chunks. Two
# filesystems, since a pattern that matches none passes without a word.
# usage: btrfs-reclaims-its-chunks.sh <tmpfiles rules>
set -euo pipefail

rules=$1
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
filesystems=(one two)
settings=(dynamic_reclaim periodic_reclaim)

for fs in "${filesystems[@]}"; do
  for kind in data metadata system; do
    mkdir -p "$root/sys/fs/btrfs/$fs/allocation/$kind"
    for setting in "${settings[@]}"; do
      echo 0 > "$root/sys/fs/btrfs/$fs/allocation/$kind/$setting"
    done
  done
done

systemd-tmpfiles --root="$root" --create "$rules"

# What a filesystem's setting for a kind of chunk now reads.
reads() {
  cat "$root/sys/fs/btrfs/$1/allocation/$2/$3"
}
for fs in "${filesystems[@]}"; do
  for setting in "${settings[@]}"; do
    for kind in data metadata; do
      if [ "$(reads "$fs" "$kind" "$setting")" != 1 ]; then
        echo "Btrfs's $setting is off for $kind."
        exit 1
      fi
    done
    if [ "$(reads "$fs" system "$setting")" != 0 ]; then
      echo "Btrfs's $setting was turned on for the system chunks."
      exit 1
    fi
  done
done
