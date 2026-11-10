#!/usr/bin/env bash
# The test of collect.sh, the only thing a collecting key may run in the
# Linux builder's VM: each thing it's asked collects what it says and
# prints the store's free bytes, and anything else runs nothing. nix-store
# and df are stand-ins that write down what they were asked.
#
#   collect-test.sh COLLECT
set -euo pipefail

collect=$1
bin=${PWD}/bin
mkdir -p "${bin}"
: >"${PWD}/asked"
printf '#!%s\necho "$*" >> %q\n' "${BASH}" "${PWD}/asked" >"${bin}/nix-store"
printf '#!%s\necho Avail; echo 1024\n' "${BASH}" >"${bin}/df"
chmod +x "${bin}/nix-store" "${bin}/df"
export PATH=${bin}:${PATH}

# Says what was wrong, and everything the stand-in nix-store was asked.
fail() {
  echo "collect-test: $*; nix-store was asked:" >&2
  cat "${PWD}/asked" >&2
  exit 1
}

said=$(env -u SSH_ORIGINAL_COMMAND "${BASH}" "${collect}")
[[ ${said} == 1024 ]] || fail "a bare ssh printed '${said}', not the store's free bytes"
last=$(tail -n 1 "${PWD}/asked")
[[ ${last} == --gc ]] || fail "a bare ssh didn't ask nix-store to collect all"

said=$(SSH_ORIGINAL_COMMAND="collect 5" "${BASH}" "${collect}")
[[ ${said} == 1024 ]] || fail "collect 5 printed '${said}', not the store's free bytes"
last=$(tail -n 1 "${PWD}/asked")
[[ ${last} == "--gc --max-freed 5368709120" ]] ||
  fail "collect 5 didn't ask nix-store to free at most 5 GiB"

: >"${PWD}/asked"
said=$(SSH_ORIGINAL_COMMAND=free "${BASH}" "${collect}")
[[ ${said} == 1024 ]] || fail "free printed '${said}', not the store's free bytes"
[[ ! -s ${PWD}/asked ]] || fail "free asked nix-store something, where it should collect nothing"

for refused in "sh" "collect 0" "collect -1" "collect 1234567" "collect 5; sh" "free; sh" ""; do
  status=0
  SSH_ORIGINAL_COMMAND=${refused} "${BASH}" "${collect}" >/dev/null 2>&1 || status=$?
  [[ ${status} == 64 ]] || fail "'${refused}' ended ${status}, not refused with 64"
  [[ ! -s ${PWD}/asked ]] || fail "'${refused}' asked nix-store something, not refused first"
done
echo "collect-test: every ask did what it says, and nothing else ran"
