# shellcheck shell=bash
# Writes a harness's state into the folder given, as its fleets and review
# rounds leave it, for tests/runs.qml to find through pi-runs: a fleet still
# at work, its member's process the one given, beside a member done; a fleet
# whose process is gone; one older than a day; and a council, named for its
# change by the review file listing it.
# usage: runs-state.sh <state> <live pid>
set -euo pipefail

state=$1
live=$2
fleets=$state/subagent-workflow/runs
rounds=$state/review/transcripts/runs

# A time, in the form the harness writes, from a date's description.
at() {
  date -u -d "$1" +%FT%T.000Z
}
# A member of a fleet or a round: its folder, its process, its state and
# what it's doing.
member() {
  mkdir -p "$1"
  echo '{"cwd":"/src/thing","args":["cli.js","--model","anthropic/opus","--thinking","high"]}' \
    > "$1/request.json"
  echo "{\"parentPid\":4242,\"childPid\":$2}" > "$1/lease.json"
  echo "{\"state\":\"$3\",\"activity\":\"$4\"}" > "$1/progress.json"
}
# A member's result: its folder, when it started and finished, what it cost
# and its answer, as JSON's string escapes write it.
result() {
  printf '{"state":"complete","startedAt":"%s","completedAt":"%s",%s,"finalAssistantText":"%s"}' \
    "$2" "$3" "\"usage\":{\"cost\":{\"total\":$4}}" "$5" > "$1/result.json"
}

member "$fleets/fleet-live/reviewers/a" "$live" running "reading checks.nix"
member "$fleets/fleet-live/reviewers/b" 1 complete ""
result "$fleets/fleet-live/reviewers/b" "$(at '-10 min')" "$(at now)" 1.25 \
  'All good.\n\n- One \"finding\".'
member "$fleets/fleet-dead/reviewers/y" 4000000000 running ""
member "$fleets/fleet-old/reviewers/x" 1 complete ""
result "$fleets/fleet-old/reviewers/x" "$(at '-3 days')" "$(at '-3 days')" 1 ""
touch -d '2 days ago' "$fleets"/fleet-old/reviewers/x/*

council='council-20261022T100000000-000001'
for who in tests safety; do
  member "$rounds/$council/reviewers/$who" 1 complete ""
done
result "$rounds/$council/reviewers/tests" 2026-10-22T10:00:00.000Z 2026-10-22T10:02:00.000Z 0.5 ""
result "$rounds/$council/reviewers/safety" 2026-10-22T10:00:30.000Z 2026-10-22T10:03:00.000Z \
  0.75 ""
mkdir -p "$state/review/runs"
echo "{\"runs\":[{\"id\":\"$council\",\"round\":\"council\"}]}" \
  > "$state/review/runs/github_github_Jitsusama_core.nix_380.json"
