# shellcheck shell=bash
# Checks that lock-screen, lock-for-sleep and swayidle-away answer --help
# and refuse a word they don't know, exiting 64, without locking, turning
# a screen off or starting a watcher: given stand-ins for everything they
# act through, which write what they're asked, nothing may be asked.
# lock-screen once locked Joel's screen for `lock-screen --help`.
# usage: asked-wrongly-test.sh <lock-screen.sh> <lock-for-sleep.sh> <away.sh> <a folder>
set -euo pipefail

lock_screen=$1
lock_for_sleep=$2
away=$3
stand_ins=$4
said=$stand_ins/said
mkdir -p "$stand_ins/bin"
export STAND_IN_SAID=$said XDG_RUNTIME_DIR=$stand_ins
: > "$said"

fails() { echo "$1" >&2; exit 1; }

for stand_in in qs niri swayidle inotifywait lock-screen; do
  # The stand-in expands its own words, when it's run, not these.
  # shellcheck disable=SC2016
  printf '#!%s\necho "%s $*" >> "$STAND_IN_SAID"\n' "$BASH" "$stand_in" \
    > "$stand_ins/bin/$stand_in"
  chmod +x "$stand_ins/bin/$stand_in"
done
export PATH=$stand_ins/bin:$PATH

# Runs a script with its words, and checks it exits as expected, opening
# its help with its name when that's what was asked, and asks nothing.
expect() {
  local want=$1 name=$2 script=$3
  shift 3
  local status=0
  # Bounded, so a script that loops on a word it doesn't take fails here
  # rather than holding the build.
  timeout 10 "$BASH" "$script" "$@" > "$stand_ins/out" 2> "$stand_ins/err" || status=$?
  local err out asked
  err=$(< "$stand_ins/err")
  out=$(< "$stand_ins/out")
  asked=$(< "$said")
  [[ $status == "$want" ]] || fails "$name $* exited $status, not $want: $err"
  if [[ $want == 0 ]]; then
    [[ $out == "$name"* ]] || fails "$name $* didn't open its help with its name"
  else
    [[ -n $err ]] || fails "$name $* was refused without a word why"
  fi
  [[ -z $asked ]] || fails "$name $* acted: $asked"
}

expect 0 lock-screen "$lock_screen" --help
expect 0 lock-screen "$lock_screen" -h
expect 64 lock-screen "$lock_screen" lid
expect 64 lock-screen "$lock_screen" --halp
expect 64 lock-screen "$lock_screen" --reason
expect 64 lock-screen "$lock_screen" --reason lid extra

expect 0 lock-for-sleep "$lock_for_sleep" --help
expect 64 lock-for-sleep "$lock_for_sleep" now

expect 0 swayidle-away "$away" --help
expect 64 swayidle-away "$away"
expect 64 swayidle-away "$away" 120 300
expect 64 swayidle-away "$away" 120 300 soon
expect 64 swayidle-away "$away" 120 300 600 900
