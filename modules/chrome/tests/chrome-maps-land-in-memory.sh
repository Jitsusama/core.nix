#!/usr/bin/env bash
# Runs an account's own chrome-maps-in-memory command on a profile Chrome
# has already filled, and proves each folder Chrome writes through shared
# maps is then a link into the runtime folder, with Chrome's old files gone
# from the disk and nothing else in the profile touched. Then, each from
# where it stands: that a second run changes nothing; that after a boot,
# with the links there and the runtime folder empty, the folders in memory
# are made again; that a lock Chrome left behind, its process gone or now
# another program's, doesn't stop it; and that while a Chrome holds the
# profile, on this host or another, nothing in the profile is touched,
# though the folders in memory are still made. The folders are those a
# running Chrome was seen mapping shared and writable, listed here apart
# from the module so one dropped there is caught.
#
# Usage: chrome-maps-land-in-memory.sh COMMAND_FILE
set -euo pipefail

command=$(cat "$1")
if [ -z "$command" ]; then
  echo "the account has no chrome-maps-in-memory command to run"
  exit 1
fi
work=$PWD
home=$work/home runtime=$work/run
profile=$home/.config/google-chrome
command=${command//%h/$home}
command=${command//%t/$runtime}
folders=(BrowserMetrics Default/GPUCache Default/DawnWebGPUCache Default/DawnGraphiteCache)

fill() {
  rm -rf "$home" "$runtime"
  mkdir -p "$runtime"
  for folder in "${folders[@]}"; do
    mkdir -p "$profile/$folder"
    echo old >"$profile/$folder/index"
  done
  echo kept >"$profile/Default/History"
}

run() {
  # The command is the unit's, words split as systemd splits them.
  # shellcheck disable=SC2086
  if ! $command >run.log 2>&1; then
    echo "the account's command fails on its $1 run, saying:"
    cat run.log
    exit 1
  fi
}

# Every folder a link to its folder in memory, and nothing of Chrome's
# old files left on the disk.
in_memory() {
  for folder in "${folders[@]}"; do
    if [ ! -L "$profile/$folder" ]; then
      echo "$1, $folder is still a folder on the disk, not a link into memory"
      exit 1
    fi
    target=$(readlink "$profile/$folder")
    if [ "$target" != "$runtime/chrome/$folder" ]; then
      echo "$1, $folder links to $target, not the runtime folder"
      exit 1
    fi
    if [ ! -d "$target" ]; then
      echo "$1, $folder links to $target, which isn't a folder"
      exit 1
    fi
  done
  left=$(find "$profile" "$runtime" -name index)
  if [ -n "$left" ]; then
    echo "$1, Chrome's old files are still there: $left"
    exit 1
  fi
  if [ "$(cat "$profile/Default/History")" != kept ]; then
    echo "$1, the profile's history changed, which isn't the command's"
    exit 1
  fi
}

# Every folder where Chrome had it, as it was, with its folder in memory
# made all the same.
untouched() {
  for folder in "${folders[@]}"; do
    if [ -L "$profile/$folder" ] || [ "$(cat "$profile/$folder/index")" != old ]; then
      echo "$1, the command swapped $folder out from under it"
      exit 1
    fi
    if [ ! -d "$runtime/chrome/$folder" ]; then
      echo "$1, $folder has no folder in memory for the boot's links to find"
      exit 1
    fi
  done
}

fill
run first
in_memory "after the first run"

for folder in "${folders[@]}"; do
  echo new >"$runtime/chrome/$folder/written"
done
run second
in_memory "after a second run"
for folder in "${folders[@]}"; do
  if [ ! -e "$runtime/chrome/$folder/written" ]; then
    echo "a second run emptied $folder in memory, which Chrome had written to"
    exit 1
  fi
done

rm -rf "$runtime"
mkdir -p "$runtime"
run "after a boot"
in_memory "after a boot"

# A lock left by a Chrome that's gone names this host and a dead process.
fill
true &
gone=$!
wait "$gone"
ln -s "$(uname -n)-$gone" "$profile/SingletonLock"
run "stale lock's"
in_memory "with a lock whose Chrome is gone"

# The same lock once its process number is another program's.
fill
sleep 60 &
other=$!
ln -s "$(uname -n)-$other" "$profile/SingletonLock"
run "reused lock's"
kill "$other"
in_memory "with a lock whose process is now another program"

# A running Chrome holds the lock, naming this host and its process, which
# a script named chrome stands in for: its process takes the script's name.
fill
printf '#!%s\nwhile :; do sleep 1; done\n' "$BASH" >chrome
chmod +x chrome
./chrome &
chrome=$!
ln -s "$(uname -n)-$chrome" "$profile/SingletonLock"
run "running Chrome's"
kill "$chrome"
untouched "with Chrome running"

# A Chrome on another host, sharing the profile, holds it too.
fill
ln -s "elsewhere.example-1" "$profile/SingletonLock"
run "other host's"
untouched "with Chrome running on another host"
