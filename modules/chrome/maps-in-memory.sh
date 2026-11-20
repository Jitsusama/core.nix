# shellcheck shell=bash
# Moves the profile folders Chrome writes through shared maps into the
# runtime folder, each left as a link where the folder was, so a store into
# one of their maps never waits on btrfs writing the page back (home.nix
# says why that matters). It runs as the account's manager starts, before
# Chrome, and again after a switch changes it, when Chrome is most likely
# running: then it leaves the profile as it is, since a folder swapped under
# a Chrome that maps its files would go on being written on the disk,
# unlinked, and the manager's next start does it instead. That's the next
# boot for an account that lingers, and the next login for one that doesn't.
#
# Chrome holds its profile's SingletonLock, a link naming its host and its
# process, while it runs, and leaves it behind when it crashes or the
# machine stops. A lock counts as Chrome's while its process is one named
# chrome, or while it names another host, as Chrome itself counts one; a
# lock whose process is gone or is some other program is stale. It's read
# again before each folder, so a Chrome started partway stops the rest.
#
# The folders in memory are made first, whatever the lock says, since the
# runtime folder is empty after each boot and the links from the boot
# before would otherwise point at nothing. A link that does is no harm:
# Chrome removes it and makes the cache on the disk again, as it would
# without this.
#
# Usage: chrome-maps-in-memory PROFILE MEMORY FOLDER...
set -euo pipefail

profile=$1 memory=$2
shift 2

# Says which Chrome holds the profile, failing when none does.
holder() {
  local lock host pid
  lock=$(readlink "$profile/SingletonLock" 2>/dev/null) || return 1
  host=${lock%-*} pid=${lock##*-}
  if [ "$host" != "$(uname -n)" ]; then
    echo "Chrome on $host"
    return 0
  fi
  [ "$(cat "/proc/$pid/comm" 2>/dev/null)" = chrome ] || return 1
  echo "Chrome, as process $pid,"
}

for folder in "$@"; do
  mkdir -p "$memory/$folder"
done

for folder in "$@"; do
  if [ "$(readlink "$profile/$folder" 2>/dev/null)" = "$memory/$folder" ]; then
    continue
  fi
  if chrome=$(holder); then
    echo "$chrome holds the profile, so $folder and those after it stay on the disk"
    echo "until this runs again, at the account manager's next start"
    exit 0
  fi
  # What was there is a cache Chrome fills again, or its histograms, which
  # feed only its usage reports to Google.
  rm -rf "${profile:?}/$folder"
  mkdir -p "$(dirname "$profile/$folder")"
  ln -s "$memory/$folder" "$profile/$folder"
  echo "$folder is in memory now"
done
