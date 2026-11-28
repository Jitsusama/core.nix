#!/usr/bin/env bash
# Samples how long the node's site takes to come live, as cadence-site runs
# it every ten minutes: it opens the feed as the site's page does, says
# hello, then watches each kind the hello lists, all at once as the page
# does, and times each watch to its first records. A line of JSON in the
# day's file of $XDG_STATE_HOME/cadence/site holds each kind's seconds, or
# null for one refused or that sent none within the wait, for `cadence hour`
# to hold the hour's median wait for the slowest the page waits on to its
# ceiling (src/hour/measures/site.rs). The sites' server shares one watch
# of each kind among its pages, so while a page holds one the sample times
# what a page opened then would wait, the server's held records, rather
# than the node: the page's wait is what this measures. On 26 November at 22:40 the message watch
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
# The watches the page waits on before it lays its map, as site.rs's
# WAITED_ON names them: they come first, so the cap never cuts one.
readonly WAITED='["network","cell","machine","message"]'
# Only plain names, since each names a file here and a field of the line:
# a feed answering anything else gets no watch of it. At most 16, as each
# holds a connection and its buffer for the wait; the node declares 11.
listed=$(jq --raw-output --argjson waited "${WAITED}" \
  '[.kinds[]? | strings | select(test("^[a-z][a-z_-]{0,31}$"))]
    | map(select(IN($waited[]))) + map(select(IN($waited[]) | not)) | .[:16][]' \
  <<<"${hello}" 2>/dev/null) || listed=""
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
    # Room for the hello's answer and any state lines before the records;
    # the first records end the read long before this many.
    printf '{"id":1,"ask":"hello"}\n{"id":2,"ask":"watch","kind":"%s"}\n' "${kind}" |
      socket 64 |
      {
        # The watch's first records end its wait, timed; its refusal ends
        # it untimed, so the hour counts it as the whole wait. jq reads
        # each line as JSON, whatever order its keys come in, and ends at
        # the first that says, so the clock stops as the records arrive
        # rather than after bash has read a large line a byte at a time.
        # A line that isn't an object is passed over, not the read's end.
        said=$(jq --null-input --raw-input --raw-output 'first(inputs | fromjson? |
          objects | select(.id == 2) |
          if has("records") then "records" elif has("error") then "error" else empty end)' \
          2>/dev/null) || said=""
        end=$(date +%s%N)
        if [[ ${said} == records ]]; then
          echo $(((end - start) / 1000000)) >"${timed}/${kind}"
        fi
      }
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
