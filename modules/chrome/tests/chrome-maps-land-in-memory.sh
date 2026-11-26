#!/usr/bin/env bash
# Runs an account's own chrome-maps-in-memory command on a profile Chrome
# has already filled, and proves each folder Chrome writes through shared
# maps is then a link into the runtime folder, with Chrome's old files gone
# from the disk and nothing else in the profile touched. Then, each from
# where it stands: that a second run changes nothing; that after a boot,
# with the links there and the runtime folder empty, the folders in memory
# are made again; that a lock Chrome left behind, its process gone or now
# another program's, doesn't stop it; that a Chrome which has dropped its
# lock but still maps a file in the profile is waited for, as is one whose
# maps can't be read; that a Chrome started again while the old one quits
# stops the move; that one still quitting after the half minute, with a
# histogram mapped, is given up on for BrowserMetrics and the caches
# alike, and the run looks once more and starts over once it has quit,
# or ends within its minute, moving nothing, if it quits only after;
# and that while a Chrome holds the profile nothing in it is touched,
# though the folders in memory are still made, but for BrowserMetrics,
# which Chrome deletes itself as it runs and which moves whenever nothing
# of Chrome's maps or holds open a file still in it: a file it maps or
# holds open there keeps it on the disk, one it deleted doesn't, whether
# mapped or held open, nor one in a folder whose name only begins with
# it, a quitting Chrome mapping one is waited for, what a session left in
# memory goes unless it's mapped, and it moves when named after caches a
# held profile keeps; that a Chrome on another host keeps BrowserMetrics
# too, since its processes can't be seen; and that folders reaching
# outside the profile are refused. The
# folders are those a running Chrome was seen mapping shared and
# writable, listed here apart from the module so one dropped there is
# caught, as is BrowserMetrics losing its mark.
#
# Usage: chrome-maps-land-in-memory.sh COMMAND_FILE UNREADABLE_CHROME
set -euo pipefail

command=$(cat "$1")
unreadable=$2
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
  # The last case's log, which a case that reads its run's log as it goes
  # would otherwise read.
  rm -rf "$home" "$runtime" "$work/run.log"
  mkdir -p "$runtime"
  for folder in "${folders[@]}"; do
    mkdir -p "$profile/$folder"
    echo old >"$profile/$folder/index"
  done
  echo kept >"$profile/Default/History"
}

# Runs the command, failing rather than hanging the build on a run that
# never ends: a run waits at most a minute, half for its pass and half
# for its last look, before it ends or starts over on a profile nothing
# holds, so two minutes is past any run that stops.
run() {
  local status=0
  # The command is the unit's, words split as systemd splits them.
  # shellcheck disable=SC2086
  timeout 120 $command >run.log 2>&1 || status=$?
  if [ "$status" = 124 ]; then
    # One that loops says the same few lines over and over, so its first
    # are enough.
    echo "the account's command was still going two minutes into its $1 run, first saying:"
    head -n 20 run.log
    exit 1
  fi
  if [ "$status" != 0 ]; then
    echo "the account's command fails on its $1 run, saying:"
    cat run.log
    exit 1
  fi
}

# A case that fails leaves its quitting stand-in waiting for this file, and
# its running stand-in looping, so both end as the test ends, however it
# ends.
trap 'touch "$work/quit"; [ -z "${chrome-}" ] || kill "$chrome" 2>/dev/null || true' EXIT

# Waits until a stand-in's process has taken Chrome's name, which it does a
# moment after the fork.
named() {
  local name=''
  while [ "$name" != chrome ]; do
    sleep 0.1
    name=$(cat "/proc/$1/comm")
  done
}

# Starts a Chrome quitting, which has dropped its lock but still maps a
# file in the profile: a copy of the shell, named chrome and run from the
# profile, maps itself there, as Chrome's last processes map their caches.
# It quits only once the file it's given exists, so a run is seen waiting
# whatever the machine's load. It runs from the folder it's given second,
# Crashpad unless told, and its process is left in $quitting.
quit_later() {
  local from=$profile/${2:-Crashpad}
  mkdir -p "$from"
  cp "$BASH" "$from/chrome"
  rm -f "$1"
  # shellcheck disable=SC2016 # The stand-in expands its own argument.
  "$from/chrome" -c 'until [ -e "$0" ]; do sleep 0.1; done' "$1" &
  quitting=$!
  named "$quitting"
}

# Fails unless the run in the background is still going, and so waiting.
waiting() {
  if ! kill -0 "$running" 2>/dev/null; then
    echo "the command didn't wait for $1"
    exit 1
  fi
}

# Waits until the run in the background says it waits for a quitting
# Chrome, so it has reached the caches' wait however loaded the machine
# is, and a run that skips the wait fails here every time rather than
# racing the looks after. It fails if the run ends first or takes a
# minute.
reached_caches() {
  local started=$SECONDS
  # The caches' own line, which BrowserMetrics' wait doesn't share.
  until grep -q 'still quitting, so this waits' run.log 2>/dev/null; do
    waiting "$1"
    if [ $((SECONDS - started)) -gt 60 ]; then
      echo "the command hadn't said it waits a minute into waiting for $1"
      exit 1
    fi
    # A tenth of a second between looks at a move that takes a moment.
    sleep 0.1
  done
}

# Fails unless BrowserMetrics is still the folder Chrome filled.
metrics_kept() {
  local index
  index=$(cat "$profile/BrowserMetrics/index" 2>/dev/null || true)
  if [ -L "$profile/BrowserMetrics" ] || [ "$index" != old ]; then
    echo "$1, the command moved BrowserMetrics"
    exit 1
  fi
}

# Fails unless the run's log says what it was given.
said() {
  if ! grep -qF -- "$1" run.log; then
    echo "$2, the command didn't say '$1', saying:"
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

# BrowserMetrics in memory, with nothing of Chrome's open in it, and every
# other folder where Chrome had it, as it was, with its folder in memory
# made all the same.
untouched() {
  metrics "$1"
  caches_untouched "$1"
}

# Every folder but BrowserMetrics where Chrome had it, as it was, with its
# folder in memory made all the same.
caches_untouched() {
  for folder in "${folders[@]:1}"; do
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

# BrowserMetrics a link into memory, its old files gone.
metrics() {
  local link
  link=$(readlink "$profile/BrowserMetrics" || true)
  if [ "$link" != "$runtime/chrome/BrowserMetrics" ]; then
    echo "$1, BrowserMetrics isn't a link into memory, though nothing of Chrome's was open in it"
    exit 1
  fi
  if [ -e "$runtime/chrome/BrowserMetrics/index" ]; then
    echo "$1, BrowserMetrics' old files went into memory with it"
    exit 1
  fi
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
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
run "running Chrome's"
kill "$chrome"
untouched "with Chrome running"

# A Chrome on another host, sharing the profile, holds it too.
fill
ln -s "elsewhere.example-1" "$profile/SingletonLock"
# What a session before left in memory goes whatever holds the profile.
mkdir -p "$runtime/chrome/BrowserMetrics"
echo left >"$runtime/chrome/BrowserMetrics/left.pma"
run "other host's"
metrics_kept "with Chrome running on another host, whose processes can't be seen"
if [ -e "$runtime/chrome/BrowserMetrics/left.pma" ]; then
  echo "a file a session left in memory stayed while another host's Chrome held the profile"
  exit 1
fi
caches_untouched "with Chrome running on another host"
# The caches' own reason, which BrowserMetrics' line doesn't share.
said "Chrome on elsewhere.example holds the profile, so its caches stay on the disk" \
  "with Chrome running on another host"
# BrowserMetrics' own reason, which the caches' line doesn't share.
said "holds the profile, whose processes can't be seen from here," \
  "with Chrome running on another host, for its histograms"

# A Chrome quitting, which has dropped its lock but still maps a file in
# the profile, is waited for.
fill
quit_later "$work/quit"
run "quitting Chrome's" &
running=$!
reached_caches "a quitting Chrome that still mapped the profile"
# Five seconds more before it quits, so a wait in the pass cut short has
# given up by then, and says so below, rather than the last look catching
# the quit for it.
sleep 5
waiting "a quitting Chrome that still mapped the profile, five seconds on"
untouched "while a quitting Chrome still mapped the profile"
touch "$work/quit"
wait "$quitting"
wait "$running"
if grep -q -e 'holds the profile' -e 'starts over' run.log; then
  echo "the command gave up on a quitting Chrome within the pass's half minute, saying:"
  cat run.log
  exit 1
fi
in_memory "once a quitting Chrome has gone"

# So is a Chrome process of this account whose maps can't be read, as a
# sandboxed one's can't, since what it maps can't be ruled out.
fill
rm -f "$work/quit"
python3 "$unreadable" "$work/quit" &
hidden=$!
named "$hidden"
if grep -q . "/proc/$hidden/maps" 2>/dev/null; then
  echo "the stand-in's maps can still be read, so it stands in for no sandboxed Chrome;"
  echo "this test's account may read processes that aren't dumpable, as root does"
  exit 1
fi
run "unreadable Chrome's" &
running=$!
reached_caches "a Chrome process whose maps couldn't be read"
waiting "a Chrome process whose maps couldn't be read"
untouched "while a Chrome process's maps couldn't be read"
touch "$work/quit"
wait "$hidden"
wait "$running"
in_memory "once a Chrome process whose maps couldn't be read has gone"

# A Chrome started again while the old one quits, as Chrome's own Relaunch
# does, takes the lock, and the run moves nothing.
fill
quit_later "$work/quit"
run "relaunched Chrome's" &
running=$!
reached_caches "a quitting Chrome, before another started"
waiting "a quitting Chrome, before another started"
./chrome &
chrome=$!
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
touch "$work/quit"
wait "$quitting"
wait "$running"
kill "$chrome"
untouched "with Chrome started again as the old one quit"

# A Chrome still quitting after the half minute, mapping a histogram, is
# given up on for BrowserMetrics and then the caches, in one half minute
# between them, and when it quits as the run looks once more, the run
# starts over and moves them all. It waits the whole half minute, since
# that's what it proves, as the two cases after it do.
fill
quit_later "$work/quit" BrowserMetrics
started=$SECONDS
run "long-quitting Chrome's" &
running=$!
until grep -q 'holds the profile' run.log 2>/dev/null; do
  waiting "a Chrome still quitting, ending before it gave up"
  if [ $((SECONDS - started)) -gt 90 ]; then
    echo "the command hadn't given up on a quitting Chrome after a minute and a half"
    exit 1
  fi
  # A tenth of a second between looks at the run's log, which takes half
  # a minute to say it.
  sleep 0.1
done
# Half a minute of looks a second apart, less five seconds since the clock
# counts whole seconds and the run starts a moment after it's read.
if [ $((SECONDS - started)) -lt 25 ]; then
  echo "the command gave up on a quitting Chrome after $((SECONDS - started)) s, not half a minute"
  exit 1
fi
if [ $((SECONDS - started)) -gt 45 ]; then
  echo "the command gave up on a quitting Chrome after $((SECONDS - started)) s,"
  echo "waiting a half minute for BrowserMetrics and another for the caches"
  exit 1
fi
waiting "a Chrome still quitting as it gave up, to see it quit"
metrics_kept "while a Chrome mapping a histogram was still quitting after half a minute"
caches_untouched "while a Chrome was still quitting after half a minute"
said "BrowserMetrics stays on the disk until nothing of Chrome's is open in it" \
  "with a Chrome mapping a histogram still quitting after half a minute"
touch "$work/quit"
wait "$quitting"
wait "$running"
if ! grep -q 'starts over' run.log; then
  echo "the command didn't start over when Chrome quit as it ended, saying:"
  cat run.log
  exit 1
fi
in_memory "once a Chrome that quit as the run ended had gone"

# With the caches already in memory, as after any run before, a Chrome
# still quitting from a BrowserMetrics it made again on the disk is the
# only thing the run waits on, and when it quits as the run looks once
# more, the run starts over and moves it. This case waits the half minute
# too, since only the last look over BrowserMetrics sees that quit.
fill
run "kept histograms' first"
rm -- "$profile/BrowserMetrics"
mkdir "$profile/BrowserMetrics"
echo old >"$profile/BrowserMetrics/index"
quit_later "$work/quit" BrowserMetrics
started=$SECONDS
run "kept histograms'" &
running=$!
until grep -q 'BrowserMetrics stays on the disk until nothing' run.log 2>/dev/null; do
  waiting "a Chrome still quitting from its histograms alone, ending before it gave up"
  if [ $((SECONDS - started)) -gt 90 ]; then
    echo "the command hadn't given up on a Chrome quitting from its histograms after 90 s"
    exit 1
  fi
  # A tenth of a second between looks at the run's log, which takes half
  # a minute to say it.
  sleep 0.1
done
waiting "a Chrome still quitting from its histograms, to see it quit in the last look"
metrics_kept "while a Chrome mapping only its histograms was still quitting"
touch "$work/quit"
wait "$quitting"
wait "$running"
said "starts over" "when a Chrome mapping only its histograms quit as the run looked once more"
in_memory "once a Chrome that mapped only its histograms had gone"

# A Chrome still quitting through the last look too, mapping a histogram,
# ends the run within its minute, half for the pass and half for the last
# look, with everything it kept still on the disk, each wait said once with
# no more than its half minute. Then, once it has quit, a run moves it all.
fill
quit_later "$work/quit" BrowserMetrics
started=$SECONDS
run "outlasting Chrome's"
# A minute of looks a second apart, less five seconds since the clock
# counts whole seconds. A loaded builder stretches each look, so the top
# bound only catches a wait with no end short of the run's two minutes; a
# third half minute says a wait twice, which the count below catches.
took=$((SECONDS - started))
if [ "$took" -lt 55 ] || [ "$took" -gt 105 ]; then
  echo "the command ended after $took s on a Chrome quitting through the last look, not a minute,"
  echo "saying:"
  cat run.log
  exit 1
fi
if ! kill -0 "$quitting" 2>/dev/null; then
  echo "the quitting stand-in ended before the run did, so nothing outlasted the last look"
  exit 1
fi
metrics_kept "while a Chrome mapping a histogram was still quitting after the last look"
caches_untouched "while a Chrome was still quitting after the last look"
said "still had what it kept after the last look" "with a Chrome quitting through the last look"
if grep -q 'starts over' run.log; then
  echo "the command started over with a Chrome still quitting, saying:"
  cat run.log
  exit 1
fi
for line in 'as it quits, so this waits' 'still quitting, so this waits'; do
  count=$(grep -c "$line" run.log || true)
  if [ "$count" != 1 ]; then
    echo "the command didn't say once that it waits, as '$line', saying:"
    cat run.log
    exit 1
  fi
done
if grep -o 'so this waits up to [0-9]* s' run.log | grep -q -v -E 'to ([0-9]|[12][0-9]|30) s'; then
  echo "the command said it waits longer than its half minute, saying:"
  cat run.log
  exit 1
fi
touch "$work/quit"
wait "$quitting"
run "outlasting Chrome's second"
in_memory "once a Chrome that outlasted the last look had gone"

# A running Chrome mapping a file in BrowserMetrics, as its browser maps
# its histograms, keeps it on the disk, and the run says why.
fill
cp "$BASH" "$profile/BrowserMetrics/chrome"
"$profile/BrowserMetrics/chrome" -c 'while :; do sleep 1; done' &
chrome=$!
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
run "mapping Chrome's"
index=$(cat "$profile/BrowserMetrics/index" || true)
if [ -L "$profile/BrowserMetrics" ] || [ "$index" != old ]; then
  echo "the command moved BrowserMetrics from under a Chrome mapping a file in it"
  exit 1
fi
if ! grep -q "process $chrome, maps $profile/BrowserMetrics/chrome" run.log; then
  echo "the command kept BrowserMetrics on the disk without saying what mapped it:"
  cat run.log
  exit 1
fi

# Once Chrome has deleted the folder, its file with it, and made another,
# the map names a deleted file, and the folder moves.
rm -rf "$profile/BrowserMetrics"
mkdir "$profile/BrowserMetrics"
echo old >"$profile/BrowserMetrics/index"
run "deleted map's"
kill "$chrome"
metrics "with Chrome mapping a file it had deleted"

# A running Chrome holding a file in BrowserMetrics open, without mapping
# it, keeps it on the disk too.
fill
mkdir -p "$work/holding"
# shellcheck disable=SC2016 # The stand-in expands its own argument.
printf '#!%s\nexec 3<"$1"\nwhile :; do sleep 1; done\n' "$BASH" >"$work/holding/chrome"
chmod +x "$work/holding/chrome"
"$work/holding/chrome" "$profile/BrowserMetrics/index" &
chrome=$!
named "$chrome"
started=$SECONDS
until [ -e "/proc/$chrome/fd/3" ]; do
  if ! kill -0 "$chrome" 2>/dev/null || [ $((SECONDS - started)) -gt 60 ]; then
    echo "the stand-in Chrome never held its histograms' index open"
    exit 1
  fi
  # It opens the file a moment after it takes Chrome's name.
  sleep 0.1
done
ln -s "$here-$chrome" "$profile/SingletonLock"
run "holding Chrome's"
metrics_kept "with a Chrome holding a file in it open"
said "process $chrome, holds $profile/BrowserMetrics/index open" \
  "with a Chrome holding a file in it open"
# A running Chrome was said already, and the last look waited on nothing.
if grep -q 'still had what it kept' run.log; then
  echo "a run while Chrome ran ended on a last look that looked at nothing, saying:"
  cat run.log
  exit 1
fi

# Once Chrome has deleted the folder, the file it holds open with it, the
# descriptor names a deleted file, and the folder moves.
rm -rf "$profile/BrowserMetrics"
mkdir "$profile/BrowserMetrics"
echo old >"$profile/BrowserMetrics/index"
run "deleted open file's"
kill "$chrome"
metrics "with Chrome holding open a file it had deleted"

# BrowserMetrics moves while Chrome holds the profile even when named
# after the caches it keeps, which are passed over rather than ending the
# pass. The unit names it first, so the command is run here by hand.
fill
./chrome &
chrome=$!
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
ordered=0
# Bounded as run bounds a run, so one that never ends fails, ending 124.
timeout 120 "${command%% *}" "$profile" "$runtime/chrome" Default/GPUCache +BrowserMetrics \
  >run.log 2>&1 || ordered=$?
kill "$chrome"
if [ "$ordered" != 0 ]; then
  echo "the command fails with BrowserMetrics named after a cache, ending $ordered, saying:"
  cat run.log
  exit 1
fi
metrics "named after a cache a running Chrome keeps"
index=$(cat "$profile/Default/GPUCache/index" 2>/dev/null) || index=
if [ -L "$profile/Default/GPUCache" ] || [ "$index" != old ]; then
  echo "named before BrowserMetrics, the command swapped GPUCache out from under Chrome"
  exit 1
fi

# A Chrome mapping a file in a folder whose name only begins with
# BrowserMetrics doesn't keep BrowserMetrics on the disk: it holds the
# whole-path match against one that takes a prefix for the folder.
fill
mkdir -p "$profile/BrowserMetrics-spare"
cp "$BASH" "$profile/BrowserMetrics-spare/chrome"
"$profile/BrowserMetrics-spare/chrome" -c 'while :; do sleep 1; done' &
chrome=$!
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
run "neighbour's"
kill "$chrome"
metrics "with Chrome mapping a file beside BrowserMetrics"

# A Chrome quitting, its lock dropped, that still maps a file in
# BrowserMetrics is waited for, and the folder moves once it has gone.
fill
cp "$BASH" "$profile/BrowserMetrics/chrome"
rm -f "$work/quit"
# shellcheck disable=SC2016 # The stand-in expands its own argument.
"$profile/BrowserMetrics/chrome" -c 'until [ -e "$0" ]; do sleep 0.1; done' "$work/quit" &
quitting=$!
named "$quitting"
run "quitting histograms'" &
running=$!
started=$SECONDS
# BrowserMetrics' own line, which the caches' wait doesn't share.
until grep -q 'as it quits, so this waits' run.log 2>/dev/null; do
  waiting "a quitting Chrome that still mapped its histograms"
  if [ $((SECONDS - started)) -gt 60 ]; then
    echo "the command hadn't said it waits for a quitting Chrome's histograms after a minute"
    exit 1
  fi
  # A tenth of a second between looks at the run's log.
  sleep 0.1
done
metrics_kept "while a quitting Chrome still mapped its histograms"
touch "$work/quit"
wait "$quitting"
wait "$running"
metrics "once a quitting Chrome that mapped its histograms had gone"

# What a session before left in memory goes as it moves, but for a file
# that's still mapped.
fill
mkdir -p "$runtime/chrome/BrowserMetrics"
echo left >"$runtime/chrome/BrowserMetrics/left.pma"
cp "$BASH" "$runtime/chrome/BrowserMetrics/chrome"
"$runtime/chrome/BrowserMetrics/chrome" -c 'while :; do sleep 1; done' &
chrome=$!
named "$chrome"
ln -s "$here-$chrome" "$profile/SingletonLock"
run "left files'"
kill "$chrome"
metrics "with files left in memory"
if [ -e "$runtime/chrome/BrowserMetrics/left.pma" ]; then
  echo "a file a session left in memory, which nothing mapped, stayed"
  exit 1
fi
if [ ! -e "$runtime/chrome/BrowserMetrics/chrome" ]; then
  echo "a file in memory that Chrome still mapped was taken"
  exit 1
fi

# A folder that reaches outside the profile is refused as asked wrongly,
# and what's there is kept.
fill
mkdir -p "$home/.config/outside"
echo kept >"$home/.config/outside/index"
refused=0
"${command%% *}" "$profile" "$runtime/chrome" ../outside >run.log 2>&1 || refused=$?
if [ "$refused" != 64 ]; then
  echo "the command took a folder outside the profile, ending $refused rather than 64"
  exit 1
fi
outside=$(cat "$home/.config/outside/index")
if [ -L "$home/.config/outside" ] || [ "$outside" != kept ]; then
  echo "the command touched a folder outside the profile"
  exit 1
fi
