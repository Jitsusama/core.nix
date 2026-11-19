#!/usr/bin/env bash
# Runs an account's own chrome-maps-in-memory command on a profile Chrome
# has already filled, as the session starts, and proves each folder Chrome
# writes through shared maps is then a link into the runtime folder, with
# Chrome's old files gone from the disk and nothing else in the profile
# touched; that a second run changes nothing; and that with Chrome running,
# as a switch finds it, nothing is touched at all. The folders are those a
# running Chrome was seen mapping shared and writable, listed here apart
# from the module so one dropped there is caught.
#
# Usage: chrome-maps-land-in-memory.sh COMMAND_FILE
set -euo pipefail

command=$(cat "$1")
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

fill
for pass in first second; do
  run "$pass"
  for folder in "${folders[@]}"; do
    if [ ! -L "$profile/$folder" ]; then
      echo "after the $pass run, $folder is still a folder on the disk, not a link into memory"
      exit 1
    fi
    target=$(readlink "$profile/$folder")
    if [ "$target" != "$runtime/chrome/$folder" ]; then
      echo "after the $pass run, $folder links to $target, not the runtime folder"
      exit 1
    fi
    if [ ! -d "$target" ]; then
      echo "after the $pass run, $folder links to $target, which isn't a folder"
      exit 1
    fi
    if [ -e "$target/index" ]; then
      echo "after the $pass run, $folder in memory holds a file from the disk"
      exit 1
    fi
  done
done
if [ "$(cat "$profile/Default/History")" != kept ]; then
  echo "the command changed the profile's history, which isn't its own"
  exit 1
fi

# Chrome running holds the lock, naming its host and its process.
fill
sleep 60 &
chrome=$!
ln -s "somewhere-$chrome" "$profile/SingletonLock"
run "running Chrome's"
kill "$chrome"
for folder in "${folders[@]}"; do
  if [ -L "$profile/$folder" ] || [ "$(cat "$profile/$folder/index")" != old ]; then
    echo "with Chrome running, the command swapped $folder out from under it"
    exit 1
  fi
done
