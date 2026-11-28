#!/usr/bin/env bash
# Samples how long the node's site takes to come live, as cadence-site runs
# it every ten minutes: it opens the feed as the site's page does, says
# hello, then watches each kind the hello lists, all at once as the page
# does, and times each watch to its first records. A line of JSON in the
# day's file of $XDG_STATE_HOME/cadence/site holds each kind's seconds, or
# null for one refused, said broken or that sent none within the wait, with
# what the feed said of each refused or broken one, for `cadence hour`
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
  64  asked wrongly: no ws feed naming its port
EOF
    exit 0
    ;;
  wss://*)
    # The local sites' door admits only a page of http with its port, so
    # a wss feed's https origin would be refused every run and read as a
    # slow node.
    printf '%s\n' "cadence-site takes a ws feed, since the sites' door refuses wss: ${1}" \
      "  try  cadence-site ws://${1#wss://}" >&2
    exit 64
    ;;
  ws://*) ;;
  '')
    printf '%s\n' "cadence-site takes the feed to sample" \
      "  try  cadence-site --help" >&2
    exit 64
    ;;
  *)
    printf '%s\n' "cadence-site takes a ws feed, not ${1}" \
      "  try  cadence-site --help" >&2
    exit 64
    ;;
esac

feed=$1
# The page's own origin, which the node's feed asks a browser for: the
# feed's host under the page's scheme.
rest=${feed#ws://}
if [[ ${rest%%/*} != *:[0-9]* ]]; then
  printf '%s\n' "cadence-site takes a feed with its port, as the sites' door admits: ${feed}" \
    "  try  cadence-site --help" >&2
  exit 64
fi
origin="http://${rest%%/*}"

folder="${XDG_STATE_HOME:-${HOME}/.local/state}/cadence/site"
mkdir -p "${folder}"
out="${folder}/$(date +%F).jsonl"
timed=$(mktemp -d)
trap 'rm -rf "${timed}"' EXIT

# One connection to the feed, as the page's: what is piped in is asked, and
# it ends after `$1` answers or the wait, whichever comes first. A feed
# refused or slow answers what it sent, if anything, which the sample
# reads as such, so its failure ends no run; what websocat said of it goes
# to `$2`, for the sample to say why.
socket() {
  timeout "${WAIT}" websocat --no-close --buffer-size 16777216 --max-messages-rev "$1" --text \
    --origin "${origin}" "${feed}" 2>"$2" || true
}

at=$(date +%s)
hello=$(socket 1 "${timed}/hello.said" <<<'{"id":1,"ask":"hello"}')
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
  # websocat's first line says why the connection failed, where it did.
  said=$(head -n 1 "${timed}/hello.said" | cut -c 1-200)
  jq --null-input --compact-output --argjson at "${at}" \
    --arg error "no hello naming its kinds, refused or past ${WAIT} s${said:+: ${said}}" \
    '{at: $at, error: $error}' >>"${out}"
  exit 0
fi

for kind in "${kinds[@]}"; do
  (
    start=$(date +%s%N)
    # Room for the hello's answer and any state lines before the records;
    # the first records end the read long before this many.
    printf '{"id":1,"ask":"hello"}\n{"id":2,"ask":"watch","kind":"%s"}\n' "${kind}" |
      socket 64 /dev/null |
      # jq reads each of the watch's lines as JSON, whatever order its keys
      # come in, and says it as a word and its reason as it comes, so the
      # clock stops as the records arrive rather than after bash has read
      # a large line a byte at a time. A line that isn't an object is
      # passed over, not the read's end.
      jq --null-input --raw-input --raw-output --unbuffered 'inputs | fromjson? |
        objects | select(.id == 2) |
        def why: tostring | gsub("[\t\n]"; " ") | .[:200];
        if has("records") then "records"
        elif has("error") then "error\t\(.error | why)"
        elif .state == "live" then "live"
        elif .state == "broken" then "broken\t\(.reason // "no reason" | why)"
        else empty end' 2>/dev/null |
      {
        # A fresh watch says it's live before its records, which then end
        # its wait. One another page holds is told the records the server
        # holds first, then the watch's state: records followed by broken
        # are what the page shows as its node down, so no answer, as is a
        # refusal. Either ends the wait untimed, and the hour counts it as
        # the whole wait, saying why.
        live=""
        records=""
        while IFS=$'\t' read -r said why; do
          case ${said} in
            live)
              live=yes
              [[ -n ${records} ]] && break
              ;;
            records)
              [[ -n ${records} ]] || records=$(date +%s%N)
              [[ -n ${live} ]] && break
              ;;
            broken | error)
              records=""
              printf '%s\n' "${said}: ${why}" >"${timed}/${kind}.why"
              break
              ;;
            # jq says no other word.
            *) ;;
          esac
        done
        if [[ -n ${records} ]]; then
          echo $(((records - start) / 1000000)) >"${timed}/${kind}"
        fi
      }
  ) &
done
wait

answered='{}'
why='{}'
for kind in "${kinds[@]}"; do
  if [[ -s "${timed}/${kind}" ]]; then
    ms=$(<"${timed}/${kind}")
    answered=$(jq --compact-output --arg kind "${kind}" --argjson ms "${ms}" \
      '.[$kind] = ($ms / 1000)' <<<"${answered}")
  else
    answered=$(jq --compact-output --arg kind "${kind}" '.[$kind] = null' <<<"${answered}")
  fi
  if [[ -s "${timed}/${kind}.why" ]]; then
    why=$(jq --compact-output --arg kind "${kind}" --rawfile said "${timed}/${kind}.why" \
      '.[$kind] = ($said | rtrimstr("\n"))' <<<"${why}")
  fi
done
jq --null-input --compact-output --argjson at "${at}" --argjson kinds "${answered}" \
  --argjson why "${why}" '{at: $at, kinds: $kinds} + if $why == {} then {} else {why: $why} end' \
  >>"${out}"
