#!/usr/bin/env bash
# Samples how long the node's site takes to come live, as cadence-site runs
# it every ten minutes: it opens the feed as the site's page does, says
# hello, then watches each kind the hello lists, all at once as the page
# does, and times each watch to its first records. A line of JSON in the
# day's file of $XDG_STATE_HOME/cadence/site holds each kind's seconds, or
# null for one that sent none within the wait, for `cadence hour` to hold
# the hour's median wait for the slowest to its ceiling
# (src/hour/measures/site.rs). On 26 November at 22:40 the message watch
# took 3.5 s and the cell watch 2.8 s, where seven minutes before both took
# under 1 s, and the page showed nothing new while they waited.
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
set -o errexit
set -o nounset
set -o pipefail

# As long as site.rs counts an unanswered watch's wait.
readonly WAIT=20

case ${1-} in
  -h | --help)
    cat <<'EOF'
cadence-site  time the node's site to live

  Opens FEED, the node's feed as the site's page opens it,
  says hello and watches each kind the hello lists at once,
  timing each to its first records, and adds a line of JSON
  to the day's file in the state folder's cadence/site, for
  cadence hour to judge.

EXAMPLES
  cadence-site ws://sites.localhost:8484/feed/node

EXIT
  0   sampled, a failed sample written too
  64  asked wrongly: no feed, or one that isn't ws or wss
EOF
    exit 0
    ;;
  ws://* | wss://*) ;;
  '')
    printf '%s\n' "cadence-site takes the feed to sample" \
      "  try  cadence-site --help" >&2
    exit 64
    ;;
  *)
    printf '%s\n' "cadence-site takes a ws or wss feed, not ${1}" \
      "  try  cadence-site --help" >&2
    exit 64
    ;;
esac

feed=$1
# The page's own origin, which the node's feed asks a browser for: the
# feed's host under the page's scheme.
scheme=http
[[ ${feed} == wss://* ]] && scheme=https
rest=${feed#*://}
origin="${scheme}://${rest%%/*}"

folder="${XDG_STATE_HOME:-${HOME}/.local/state}/cadence/site"
mkdir -p "${folder}"
out="${folder}/$(date +%F).jsonl"
timed=$(mktemp -d)
trap 'rm -rf "${timed}"' EXIT

# One connection to the feed, as the page's: what is piped in is asked, and
# it ends after `$1` answers or the wait, whichever comes first. A feed
# refused or slow answers what it sent, if anything, which the sample
# reads as such, so its failure ends no run.
socket() {
  timeout "${WAIT}" websocat --no-close --buffer-size 16777216 --max-messages-rev "$1" --text \
    --origin "${origin}" "${feed}" 2>/dev/null || true
}

at=$(date +%s)
hello=$(socket 1 <<<'{"id":1,"ask":"hello"}')
listed=$(jq --raw-output '.kinds[]?' <<<"${hello}" 2>/dev/null) || listed=""
kinds=()
if [[ -n ${listed} ]]; then
  mapfile -t kinds <<<"${listed}"
fi
if ((${#kinds[@]} == 0)); then
  jq --null-input --compact-output --argjson at "${at}" \
    --arg error "no hello naming its kinds, refused or past ${WAIT} s" \
    '{at: $at, error: $error}' >>"${out}"
  exit 0
fi

for kind in "${kinds[@]}"; do
  (
    start=$(date +%s%N)
    printf '{"id":1,"ask":"hello"}\n{"id":2,"ask":"watch","kind":"%s"}\n' "${kind}" |
      socket 4 |
      while IFS= read -r line; do
        # Its first records, or its refusal, end the watch's wait.
        if [[ ${line} == '{"id":2,"records"'* || ${line} == *'"error"'* ]]; then
          end=$(date +%s%N)
          echo $(((end - start) / 1000000)) >"${timed}/${kind}"
          break
        fi
      done
  ) &
done
wait

answered='{}'
for kind in "${kinds[@]}"; do
  if [[ -s "${timed}/${kind}" ]]; then
    ms=$(<"${timed}/${kind}")
    answered=$(jq --compact-output --arg kind "${kind}" --argjson ms "${ms}" \
      '.[$kind] = ($ms / 1000)' <<<"${answered}")
  else
    answered=$(jq --compact-output --arg kind "${kind}" '.[$kind] = null' <<<"${answered}")
  fi
done
jq --null-input --compact-output --argjson at "${at}" --argjson kinds "${answered}" \
  '{at: $at, kinds: $kinds}' >>"${out}"
