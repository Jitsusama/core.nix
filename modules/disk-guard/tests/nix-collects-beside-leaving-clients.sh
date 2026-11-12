# shellcheck shell=bash
# The guard's Nix collects garbage while other programs connect to its
# roots socket and leave at once. Nix 2.34's collector takes each client
# on a thread of its own, and one that left before its thread was
# registered left a stale entry; the next client given the same
# descriptor ended the collector with std::terminate. On 11 November that
# aborted the machine guard's deletes at 06:16 and 11:54, and an
# unpatched Nix aborts here in every round.
#
# Usage: nix-collects-beside-leaving-clients.sh NIX-BIN-FOLDER PYTHON
# CLIENT, where CLIENT is leave-at-once.py beside this.
set -euo pipefail

bin=$1
python=$2
client=$3
# A store's root may lie behind no symlink.
made=$(mktemp -d)
work=$(realpath "${made}")
hammer=
stop() {
  [[ -z "${hammer}" ]] || kill "${hammer}" 2>/dev/null || true
  chmod -R u+w "${work}"
  rm -rf "${work}"
}
trap stop EXIT
export HOME=${work}/home
mkdir -p "${HOME}" "${work}/in"
store="local?root=${work}/root"
options=(--option min-free 0 --option max-free 0 --extra-experimental-features nix-command)
socket=${work}/root/nix/var/nix/gc-socket/socket

for round in 1 2 3; do
  for i in $(seq 1 1500); do
    echo "${round}.${i}" > "${work}/in/f${i}"
  done
  "${bin}/nix-store" "${options[@]}" --store "${store}" --add "${work}/in/"* > /dev/null
  "${python}" "${client}" "${socket}" "${work}/connected" &
  hammer=$!
  status=0
  "${bin}/nix" "${options[@]}" store gc --store "${store}" > "${work}/gc.log" 2>&1 || status=$?
  kill "${hammer}"
  wait "${hammer}" 2>/dev/null || true
  hammer=
  if ((status != 0)); then
    echo "FAIL: round ${round}: collecting beside clients that leave ended with ${status}" >&2
    tail -5 "${work}/gc.log" >&2
    exit 1
  fi
  # A round whose client never reached the collector proves nothing.
  connected=$(cat "${work}/connected")
  if ((connected < 50)); then
    echo "FAIL: round ${round}: only ${connected} clients reached the collector" >&2
    exit 1
  fi
  echo "round ${round}: ${connected} clients came and went"
done
echo "Nix collects garbage beside clients that connect and leave at once"
