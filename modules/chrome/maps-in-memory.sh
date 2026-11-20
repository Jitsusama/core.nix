# shellcheck shell=bash
# Moves the profile folders Chrome writes through shared maps into the
# runtime folder, each left as a link where the folder was, so a store into
# one of their maps never waits on btrfs writing the page back (home.nix
# says why that matters). It runs as the account's manager starts, before
# Chrome, and each time the profile's own folder changes, as when Chrome
# quits and drops its lock. While Chrome runs it leaves the profile as it
# is, since a folder swapped under a Chrome that maps its files would go on
# being written on the disk, unlinked; the run after Chrome quits does it
# instead, before Chrome is started again.
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
# without this. Its help takes the shape every command of ours does
# (docs/commands.md), by hand.

case ${1:-} in
-h | --help)
  cat << 'EOF'
chrome-maps-in-memory  Chrome's mapped folders in memory

EXAMPLES
  chrome-maps-in-memory ~/.config/google-chrome \
      /run/user/1000/chrome BrowserMetrics
      link the profile's BrowserMetrics into memory,
      unless a Chrome holds the profile

EXIT
  0   linked, or left alone while a Chrome runs
  1   a folder couldn't be moved or linked
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

set -euo pipefail

if [ $# -lt 3 ]; then
  echo "✕ chrome-maps-in-memory needs a profile, a folder in memory and a folder" >&2
  echo "  try  chrome-maps-in-memory --help" >&2
  exit 64
fi
profile=$1 memory=$2
shift 2
here=$(uname -n)

# Says which Chrome holds the profile, or nothing when none does.
holder() {
  local lock host pid comm
  lock=$(readlink "$profile/SingletonLock" 2>/dev/null) || return 0
  host=${lock%-*} pid=${lock##*-}
  if [ "$host" != "$here" ]; then
    echo "Chrome on $host"
    return 0
  fi
  comm=$(cat "/proc/$pid/comm" 2>/dev/null) || return 0
  if [ "$comm" = chrome ]; then
    echo "Chrome, as process $pid,"
  fi
}

for folder in "$@"; do
  mkdir -p "$memory/$folder"
done

for folder in "$@"; do
  link=$(readlink "$profile/$folder" 2>/dev/null) || link=
  if [ "$link" = "$memory/$folder" ]; then
    continue
  fi
  chrome=$(holder)
  if [ -n "$chrome" ]; then
    echo "$chrome holds the profile, so $folder and those after it stay on the disk"
    echo "until Chrome quits and this runs again"
    exit 0
  fi
  # What was there is a cache Chrome fills again, or its histograms, which
  # feed only its usage reports to Google.
  rm -rf "${profile:?}/$folder"
  parent=$(dirname "$profile/$folder")
  mkdir -p "$parent"
  ln -s "$memory/$folder" "$profile/$folder"
  echo "$folder is in memory now"
done
