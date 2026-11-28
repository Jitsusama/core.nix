#!/usr/bin/env bash
# Samples the path to each origin it's given, as cadence-path runs it every
# ten minutes: how long TCP and TLS took to set up and the first byte to
# come, the device the route to it takes, and the answer's code, or the
# sample's failure, a line of JSON each in the day's file of
# $XDG_STATE_HOME/cadence/path, for `cadence hour` to hold each origin's
# median TLS setup to its ceiling (src/hour/measures/way_out/path.rs). On 26
# November TLS to the model origin took 6 to 10 s through a starved WARP
# lane while warp-cli said Connected; a sample asks no more of the path
# than a session's first call does.
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
set -o errexit
set -o nounset
set -o pipefail

# As long as path.rs counts a failed sample's wait.
readonly WAIT=20

case ${1-} in
  -h | --help)
    cat <<'EOF'
cadence-path  sample the path to each origin

  For each ORIGIN, times TCP and TLS setup and the first
  byte over HTTPS, with the device its route takes, and
  adds a line of JSON to the day's file in the state
  folder's cadence/path, for cadence hour to judge.

EXAMPLES
  cadence-path github.com model.example

EXIT
  0   each origin sampled, a failed sample written too
  64  asked wrongly: no origin named
EOF
    exit 0
    ;;
  -*)
    printf '%s\n' "cadence-path takes origins, not ${1}" \
      "  try  cadence-path --help" >&2
    exit 64
    ;;
  '')
    printf '%s\n' "cadence-path takes at least one origin" \
      "  try  cadence-path --help" >&2
    exit 64
    ;;
  *) ;;
esac

folder="${XDG_STATE_HOME:-${HOME}/.local/state}/cadence/path"
mkdir -p "${folder}"
out="${folder}/$(date +%F).jsonl"

for origin in "$@"; do
  at=$(date +%s)
  # The route an address of the origin takes, read before the sample so a
  # failed one still names its path.
  address=$(getent ahosts "${origin}" | awk 'NR == 1 {print $1}') || address=""
  device="unresolved"
  if [[ -n ${address} ]]; then
    # No route is the worst path there is, so it is sampled as one rather
    # than ending the run before its sample is written.
    device=$(ip -o route get "${address}" |
      awk '{for (i = 1; i < NF; i++) if ($i == "dev") print $(i + 1)}') || device="no route"
  fi
  status=0
  # Only the head is asked, so a slow page body never ends a fast setup as
  # a failed sample, and nothing more than a head comes every ten minutes.
  # Each time counts from the start, the lookup and connection included,
  # as a session waits on them too.
  timed=$(curl --silent --head --output /dev/null --max-time "${WAIT}" \
    --write-out '%{time_connect} %{time_appconnect} %{time_starttransfer} %{http_code}' \
    "https://${origin}/") || status=$?
  if ((status == 0)); then
    read -r connect tls first code <<<"${timed}"
    jq --null-input --compact-output --arg origin "${origin}" --arg device "${device}" \
      --argjson at "${at}" --argjson connect "${connect}" --argjson tls "${tls}" \
      --argjson first "${first}" --argjson code "${code}" \
      '{at: $at, origin: $origin, device: $device, connect_s: $connect, tls_s: $tls,
        first_s: $first, code: $code}' >>"${out}"
  else
    jq --null-input --compact-output --arg origin "${origin}" --arg device "${device}" \
      --argjson at "${at}" --arg error "curl exited ${status}" \
      '{at: $at, origin: $origin, device: $device, error: $error}' >>"${out}"
  fi
done
