# shellcheck shell=bash
# Builds what lag-watch reads after a busy hour, for commands-agree to set
# its bare run and its report at each width from: the last window, in
# which apps stalled on the disk under a unit with a long name, and an
# hour of windows across two generations and every band. Times are
# written @NOW-N@, which the check sets as it renders, so the windows are
# always this hour's. It's built rather than kept as files, since each
# folder kept would need a note.
# usage: a-busy-hour.sh <folder to build it in>
set -euo pipefail

out=$1
mkdir -p "$out/runtime/lag-watch" "$out/state/lag-watch"

# A window that ended N seconds ago on generation G, in band B, with apps
# stalled S% on the disk, said as TROUBLE (empty for none).
window() {
  local ago=$1 generation=$2 band=$3 stalled=$4 trouble=$5 worst='' who='' since=null
  local said='[]'
  if [ -n "$trouble" ]; then
    said="[\"$trouble\"]" worst=disk who=apps since="@NOW-$ago@"
  fi
  local group="{\"cpu_some\": 0.2, \"cpu_full\": 0.1, \"memory_some\": 0.0,
      \"memory_full\": 0.0, \"io_some\": $stalled, \"io_full\": $stalled}"
  local kept='{"swap_mib": 0, "refaults": 0, "sock_throttled": 0}'
  printf '%s\n' "{\"ended\": @NOW-$ago@, \"seconds\": 10.0,
    \"system\": \"/nix/store/00000000000000000000000000000000-nixos-system-a-laptop-$generation\",
    \"band\": \"$band\",
    \"stalls\": {\"app\": $group, \"build\": $group, \"jobs\": $group, \"session\": $group},
    \"memory\": {\"app\": $kept, \"build\": $kept, \"jobs\": $kept, \"session\": $kept},
    \"waits\": {\"niri\": {\"mean_us\": 56.6, \"worst_us\": 80.1},
      \"pipewire\": {\"mean_us\": 5.0, \"worst_us\": 7.2}},
    \"niri\": {\"p50_ms\": 0.4, \"p99_ms\": 0.65, \"max_ms\": 0.65, \"asked\": 5,
      \"unanswered\": 0},
    \"trouble\": $said, \"worst\": \"$worst\", \"who\": \"$who\", \"since\": $since}" \
    | tr -d '\n'
  echo
}

long='apps stalled on the disk 2.2% of the time, most in'
long="$long app-a-program-with-a-long-name-and-a-longer-instance.scope (2.2%)"
window 3 26.11.20261112.4fe25e1 busy 2.2 "$long" > "$out/runtime/lag-watch/status.json.in"
{
  for ago in 3500 3000 2500; do
    window "$ago" 26.11.20261106.1c0ffee quiet 0.1 ''
  done
  window 2000 26.11.20261106.1c0ffee loaded 6.5 "$long"
  for ago in 1500 1000 500; do
    window "$ago" 26.11.20261112.4fe25e1 busy 0.4 ''
  done
  window 3 26.11.20261112.4fe25e1 busy 2.2 "$long"
} > "$out/state/lag-watch/history.jsonl.in"
