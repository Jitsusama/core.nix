#!/usr/bin/env bash
# Runs an account's own chrome-maps-in-memory command on a profile Chrome
# has already filled, and proves each folder Chrome writes through shared
# maps is then a link into the runtime folder, with Chrome's old files gone
# from the disk and nothing else in the profile touched. Then, each from
# where it stands: that a second run changes nothing; that after a boot,
# with the links there and the runtime folder empty, the folders in memory
# are made again; that a lock Chrome left behind, its process gone or now
# another program's, doesn't stop it; that a Chrome which has dropped its
# lock but still maps a file in the profile is waited for; and that while
# a Chrome holds the profile, on this host or another, nothing in the
# profile is touched, though the folders in memory are still made; and
# that folders reaching outside the profile are refused. The folders are those a
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
here=$(uname -n)

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
  history=$(cat "$profile/Default/History")
  if [ "$history" != kept ]; then
    echo "$1, the profile's history changed, which isn't the command's"
    exit 1
  fi
}

# Every folder where Chrome had it, as it was, with its folder in memory
# made all the same.
untouched() {
  for folder in "${folders[@]}"; do
    index=$(cat "$profile/$folder/index" 2>/dev/null) || index=
    if [ -L "$profile/$folder" ] || [ "$index" != old ]; then
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
before=$(stat -c '%i %Y' "${folders[@]/#/$profile/}")
run second
after=$(stat -c '%i %Y' "${folders[@]/#/$profile/}")
if [ "$before" != "$after" ]; then
  echo "a second run made the links again, where it should leave them be"
  exit 1
fi
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
ln -s "$here-$gone" "$profile/SingletonLock"
run "stale lock's"
in_memory "with a lock whose Chrome is gone"

# The same lock once its process number is another program's.
fill
sleep 60 &
other=$!
ln -s "$here-$other" "$profile/SingletonLock"
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
ln -s "$here-$chrome" "$profile/SingletonLock"
run "running Chrome's"
kill "$chrome"
untouched "with Chrome running"

# A Chrome on another host, sharing the profile, holds it too.
fill
ln -s "elsewhere.example-1" "$profile/SingletonLock"
run "other host's"
untouched "with Chrome running on another host"

# A Chrome quitting has dropped its lock but still maps a file in the
# profile: a copy of the shell, named chrome and run from the profile,
# maps itself there, as Chrome's last processes map their caches.
fill
mkdir -p "$profile/Crashpad"
cp "$BASH" "$profile/Crashpad/chrome"
"$profile/Crashpad/chrome" -c 'sleep 3; true' &
quitting=$!
# The copy's map is there once its own name is, a moment after the fork.
name=
while [ "$name" != chrome ]; do
  sleep 0.1
  name=$(cat "/proc/$quitting/comm")
done
run "quitting Chrome's"
if kill -0 "$quitting" 2>/dev/null; then
  echo "the command moved the folders while a quitting Chrome still mapped the profile"
  exit 1
fi
in_memory "once a quitting Chrome has gone"

# A folder that reaches outside the profile is refused, the profile kept.
fill
if "${command%% *}" "$profile" "$runtime/chrome" ../outside >run.log 2>&1; then
  echo "the command took a folder outside the profile"
  exit 1
fi
for folder in "${folders[@]}"; do
  index=$(cat "$profile/$folder/index")
  if [ "$index" != old ]; then
    echo "after a folder outside the profile, the command touched $folder"
    exit 1
  fi
done
