#!/usr/bin/env bash
# Runs site.sh, given as $1, against a stand-in websocat that answers each
# watch in one of the orders the sites' server can send (modules/mycelium's
# upstream.rs), and fails unless each sample reads as the page would: live
# and records in either order are answered, timed to the records that make
# the watch live; records then broken, with nothing after, are unanswered
# saying why; broken, then live and records within the wait, are answered,
# timed through the outage; a refusal is unanswered with the error's
# message; and a connection that fails is unanswered with what websocat
# said. A feed the sites' door can't admit is refused, and written as a
# failed sample, so the hour reads it.
set -o errexit
set -o nounset
set -o pipefail

site=$1
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
mkdir -p "${work}/bin" "${work}/scenes" "${work}/state"

# Each scene is the lines a watch of its kind is sent, after the hello's
# answer: `sleep S` waits, `die WORDS` says WORDS as websocat would and
# ends, and any other line is sent as it is.
scene() {
  printf '%s\n' "${@:2}" >"${work}/scenes/$1"
}
scene network '{"id":2,"state":"live"}' '{"id":2,"records":[]}'
scene cell '{"id":2,"records":[]}' 'not json' '{"id":2,"state":"live"}'
scene machine '{"id":2,"records":[]}' '{"id":2,"state":"broken","reason":"its daemon stopped"}' \
  'sleep 1' '{"id":2,"state":"live"}' '{"id":2,"records":[]}'
scene message '{"id":2,"records":[]}' \
  '{"id":2,"state":"broken","reason":"its daemon did not answer"}'
scene tie '{"id":2,"error":{"code":"unknown","message":"no such kind"}}'
scene murmur 'die websocat: Connection refused (os error 111)'

cat >"${work}/bin/websocat" <<EOF
#!${BASH}
# A stand-in for websocat: what it's piped is read whole, the hello is
# answered, and a watch is sent its kind's scene.
asked=\$(cat)
kind=\$(printf '%s\n' "\${asked}" | jq --raw-output 'select(.ask == "watch") | .kind')
printf '%s\n' '{"id":1,"kinds":["tie","murmur","network","cell","machine","message"]}'
[[ -n \${kind} ]] || exit 0
while IFS= read -r line; do
  case \${line} in
    sleep\ *) sleep "\${line#sleep }" ;;
    die\ *) printf '%s\n' "\${line#die }" >&2; exit 1 ;;
    *) printf '%s\n' "\${line}" ;;
  esac
done <"${work}/scenes/\${kind}"
EOF
chmod +x "${work}/bin/websocat"
export PATH="${work}/bin:${PATH}"
export XDG_STATE_HOME="${work}/state"

bash "${site}" ws://sites.localhost:8484/feed/node
sample=$(cat "${work}/state/cadence/site/"*.jsonl)

failed=""
# Fails the check, saying what was read, unless `test` holds of the sample.
holds() {
  if ! jq --exit-status "$2" <<<"${sample}" >/dev/null; then
    printf '%s\n' "site.sh $1; it wrote: ${sample}" >&2
    failed=yes
  fi
}
holds "didn't time a fresh watch, live then records" '.kinds.network | numbers'
holds "didn't time a held watch, records then live, or stopped at a line not JSON" \
  '.kinds.cell | numbers'
holds "didn't wait through a broken watch to its records and live again" \
  '.kinds.machine | numbers | . >= 1'
holds "kept why a watch was broken once it came live" '.why.machine == null'
holds "read records then broken as answered" '.kinds.message == null'
holds "didn't say why records then broken went unanswered" \
  '.why.message == "broken: its daemon did not answer"'
holds "didn't say a refusal by its message" \
  '.kinds.tie == null and .why.tie == "error: no such kind"'
holds "didn't say why a watch's connection failed" \
  '.kinds.murmur == null and .why.murmur == "connection: websocat: Connection refused (os error 111)"'

# Each feed the door can't admit is refused, its refusal written as a
# failed sample the hour reads.
for feed in wss://sites.localhost:8484/feed/node ws://sites.localhost/feed/node \
  'ws://someone@sites.localhost:8484/feed/node' 'ws://[::1]/feed/node'; do
  rm -f "${work}/state/cadence/site/"*.jsonl
  code=0
  bash "${site}" "${feed}" 2>/dev/null || code=$?
  sample=$(cat "${work}/state/cadence/site/"*.jsonl 2>/dev/null || true)
  if ((code != 64)); then
    printf '%s\n' "site.sh answered ${code}, not 64, for ${feed}" >&2
    failed=yes
  fi
  holds "wrote no failed sample for ${feed}" '.error | strings'
done

[[ -z ${failed} ]]
