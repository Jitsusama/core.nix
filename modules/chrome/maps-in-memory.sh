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
# again before each folder, so a Chrome started partway stops the caches
# after it.
#
# Chrome drops its lock before its last processes are gone, so a Chrome
# process still mapping a file in the profile holds it too, and is waited
# for rather than left to the next quit: up to half a minute in all for a
# pass over the folders, however many it waits on. A run that left a
# folder where it was for a Chrome that held or was quitting looks once
# more as it ends, for up to another half minute, and starts over if that
# Chrome went meanwhile: the watch that starts it misses a change made
# while it runs, so a quit landing then would otherwise wait for the next.
# So a run waits a minute at most before it ends or starts over.
#
# A folder named with a leading + is one Chrome empties itself as it runs:
# BrowserMetrics, which it deletes whole, link and all, a while into each
# session when usage reports are off, leaving a folder on the disk for the
# next start to map from. That one moves whenever no Chrome process maps
# or holds open a file still in it, even while Chrome holds the profile,
# and a held profile keeps only the caches where they are. A Chrome
# quitting with a file in it open is waited for, as for the caches, but a
# Chrome on another host keeps it too, since its processes can't be seen.
# Only the browser process maps its histograms, and its maps are readable,
# so a sandboxed process whose maps can't be read doesn't stop it. A file
# Chrome deleted is in no folder, so its map doesn't count. The files a
# session left in its folder in memory, which nothing maps, go before it's
# looked at, whatever holds the profile: nothing reads them where it is on
# the disk, and going first they leave no time between the look that
# clears the folder and its move for a Chrome to start in.
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
  chrome-maps-in-memory ~/.config/google-chrome \
      /run/user/1000/chrome +BrowserMetrics
      the same, and while a Chrome holds the profile too,
      once nothing of Chrome's is open in it

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
# Chrome makes its profile readable by its owner alone, and so does this.
umask 077

if [ $# -lt 3 ]; then
  echo "✕ chrome-maps-in-memory needs a profile, a folder in memory and a folder" >&2
  echo "  try  chrome-maps-in-memory --help" >&2
  exit 64
fi
profile=$1 memory=$2
shift 2
here=$(uname -n)
# A process's maps name the files' real paths, past any link on the way.
real=$(realpath -m -- "$profile")
for folder in "$@"; do
  case /${folder#+}/ in
  // | */../* | */./* | //*)
    echo "✕ chrome-maps-in-memory needs folders inside the profile, not '$folder'" >&2
    echo "  try  chrome-maps-in-memory --help" >&2
    exit 64
    ;;
  *) ;;
  esac
done

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

# Says which Chrome process still maps a file in the profile, or nothing.
# Each process's name is read by the shell itself, since a machine runs
# thousands, and only Chrome's maps are searched. A live Chrome process
# whose maps can't be read counts as mapping the profile, since what
# can't be seen can't be ruled out. Only this account's count, by the
# real user its status names, since another's can't be this profile's.
quitting() {
  local proc comm key uid found
  for proc in /proc/[0-9]*; do
    read -r comm 2>/dev/null <"$proc/comm" || continue
    [ "$comm" = chrome ] || continue
    uid=
    {
      while read -r key uid _; do
        [ "$key" != Uid: ] || break
      done <"$proc/status"
    } 2>/dev/null || continue
    [ "$uid" = "$UID" ] || continue
    found=0
    grep -qF " $real/" "$proc/maps" 2>/dev/null || found=$?
    if [ "$found" = 0 ] || { [ "$found" = 2 ] && [ -e "$proc" ]; }; then
      echo "Chrome, as process ${proc#/proc/}, still quitting,"
      return 0
    fi
  done
}

# Says what holds the profile once a quitting Chrome has had until the
# pass's deadline, or nothing when nothing does.
# The lock is read last, after the slower look through the processes, so
# a Chrome starting meanwhile is seen before anything is moved.
held() {
  local chrome lock first=yes
  while :; do
    chrome=$(quitting)
    lock=$(holder)
    if [ -n "$lock" ]; then
      echo "$lock"
      return 0
    fi
    [ -n "$chrome" ] || return 0
    [ "$SECONDS" -lt "$deadline" ] || break
    # Said once, to the log rather than the answer, so a pause is explained.
    if [ -n "$first" ]; then
      first=
      echo "$chrome so this waits up to half a minute for it" >&2
    fi
    # A second between looks lets Chrome's last processes go.
    sleep 1
  done
  echo "$chrome"
}

# Says which Chrome process maps or holds open `$1`, a real path, or a file
# still under it, or nothing when none does. A map or a descriptor of a deleted
# file names its old path with " (deleted)" after it, and isn't counted.
# A process whose maps can't be read is passed over (see the top).
open_in() {
  local proc comm key uid line
  for proc in /proc/[0-9]*; do
    read -r comm 2>/dev/null <"$proc/comm" || continue
    [ "$comm" = chrome ] || continue
    uid=
    {
      while read -r key uid _; do
        [ "$key" != Uid: ] || break
      done <"$proc/status"
    } 2>/dev/null || continue
    [ "$uid" = "$UID" ] || continue
    # A map's path is its sixth field to the line's end, spaces and all,
    # so a folder whose name begins another's isn't taken for it.
    while read -r _ _ _ _ _ line; do
      case $line in
      *" (deleted)") ;;
      "$1" | "$1"/*)
        echo "Chrome, as process ${proc#/proc/}, maps $line"
        return 0
        ;;
      *) ;;
      esac
    done 2>/dev/null <"$proc/maps" || true
    for line in "$proc"/fd/*; do
      line=$(readlink "$line" 2>/dev/null) || continue
      case $line in
      *" (deleted)") ;;
      "$1" | "$1"/*)
        echo "Chrome, as process ${proc#/proc/}, holds $line open"
        return 0
        ;;
      *) ;;
      esac
    done
  done
}

# Says which Chrome process maps or holds open `$1` once a quitting Chrome
# has had until the pass's deadline, or nothing when none does, as held
# does for the profile. A Chrome that takes the lock again meanwhile is
# running, not quitting, so it's said at once.
let_go() {
  local using lock first=yes
  while :; do
    using=$(open_in "$1")
    [ -n "$using" ] || return 0
    lock=$(holder)
    [ -z "$lock" ] || break
    [ "$SECONDS" -lt "$deadline" ] || break
    # Said once, to the log rather than the answer, so a pause is explained.
    if [ -n "$first" ]; then
      first=
      echo "$using as it quits, so this waits up to half a minute for it" >&2
    fi
    # A second between looks lets Chrome's last processes go.
    sleep 1
  done
  echo "$using"
}

for folder in "$@"; do
  mkdir -p "$memory/${folder#+}"
done

while :; do
  # The half minute every wait in a pass shares (see the top).
  deadline=$((SECONDS + 30))
  chrome=
  looked=
  # The marked folders a quitting Chrome kept, for the last look.
  kept=()
  for marked in "$@"; do
    folder=${marked#+}
    link=$(readlink "$profile/$folder" 2>/dev/null) || link=
    if [ "$link" = "$memory/$folder" ]; then
      continue
    fi
    if [ "$marked" != "$folder" ]; then
      # What sessions before left in memory, which nothing maps any more.
      for left in "$memory/$folder"/*; do
        [ -e "$left" ] || continue
        whole=$(realpath -m -- "$left")
        using=$(open_in "$whole")
        [ -n "$using" ] || rm -rf -- "$left"
      done
      # Another host's Chrome maps its files where this can't look.
      lock=$(holder)
      case $lock in
      "Chrome on "*)
        echo "$lock holds the profile, whose processes can't be seen from here,"
        echo "so $folder stays on the disk"
        continue
        ;;
      *) ;;
      esac
      if [ -n "$lock" ]; then
        using=$(open_in "$real/$folder")
      else
        using=$(let_go "$real/$folder")
      fi
      if [ -n "$using" ]; then
        echo "$using, so $folder stays on the disk until nothing of Chrome's is open in it"
        # A running Chrome keeps it until it next quits, which starts a run.
        [ -n "$lock" ] || kept+=("$folder")
        continue
      fi
    else
      # The profile is asked about once a pass, and a held one keeps every
      # cache where it is while the marked folders after them still move.
      if [ -z "$looked" ]; then
        looked=yes
        chrome=$(held)
        if [ -n "$chrome" ]; then
          echo "$chrome holds the profile, so its caches stay on the disk until this runs"
          echo "again, as Chrome next quits or the account's manager next starts"
        fi
      fi
      [ -z "$chrome" ] || continue
      # A Chrome started since that look takes the rest, so its lock is
      # read again before each, which costs a readlink.
      lock=$(holder)
      if [ -n "$lock" ]; then
        chrome=$lock
        echo "$lock took the profile as this ran,"
        echo "so $folder and the caches after it stay on the disk"
        continue
      fi
    fi
    # What was there is a cache Chrome fills again, or its histograms, which
    # feed only its usage reports to Google.
    rm -rf "${profile:?}/$folder"
    parent=$(dirname "$profile/$folder")
    mkdir -p "$parent"
    # A folder Chrome made again since the rm fails the link, rather than
    # taking it inside.
    ln -sT "$memory/$folder" "$profile/$folder"
    echo "$folder is in memory now"
  done
  [ -n "$chrome" ] || [ "${#kept[@]}" != 0 ] || exit 0
  # The last look, with a half minute of its own, starts over once
  # anything that kept a folder where it was has gone.
  deadline=$((SECONDS + 30))
  gone=
  if [ -n "$chrome" ]; then
    chrome=$(held)
    [ -n "$chrome" ] || gone=yes
  fi
  for folder in "${kept[@]}"; do
    using=$(let_go "$real/$folder")
    [ -n "$using" ] || gone=yes
  done
  [ -n "$gone" ] || exit 0
  echo "Chrome quit as this ran, so it starts over"
done
