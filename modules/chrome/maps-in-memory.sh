# shellcheck shell=bash
# Moves the profile folders Chrome writes through shared maps into the
# runtime folder, each left as a link where the folder was, so a store into
# one of their maps never waits on btrfs writing the page back (home.nix
# says why that matters). It runs as the session starts, before Chrome, and
# again after a switch changes it, when Chrome is most likely running: then
# it leaves everything as it is, since a folder swapped under a Chrome that
# maps its files would go on being written on the disk, unlinked, and the
# next login does it instead. Chrome holds its profile's SingletonLock, a
# link naming its host and process, while it runs.
#
# Usage: chrome-maps-in-memory PROFILE MEMORY FOLDER...
set -euo pipefail

profile=$1 memory=$2
shift 2

if lock=$(readlink "$profile/SingletonLock" 2>/dev/null); then
  pid=${lock##*-}
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    echo "Chrome runs as process $pid, so its folders stay where they are until the next login"
    exit 0
  fi
fi

for folder in "$@"; do
  mkdir -p "$memory/$folder"
  if [ "$(readlink "$profile/$folder" 2>/dev/null)" = "$memory/$folder" ]; then
    continue
  fi
  # What was there is a cache Chrome fills again, or its histograms, which
  # feed only its usage reports to Google.
  rm -rf "${profile:?}/$folder"
  mkdir -p "$(dirname "$profile/$folder")"
  ln -s "$memory/$folder" "$profile/$folder"
  echo "$folder is in memory now"
done
