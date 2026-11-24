#!/usr/bin/env bash
# The test that the Mac's builder reuses its network: vzvm's patches, in the
# order given, apply to a copy of its source with no fuzz; the network made
# is kept; and the attachment hands a kept one back, once, before vmnet is
# asked for another. Usage: SOURCE PATCH...
set -euo pipefail

source=$1
shift
cp -r "${source}" source
chmod -R u+w source
cd source
for patch in "$@"; do
  if ! patch -p1 -F0 --no-backup-if-mismatch --quiet <"${patch}"; then
    echo "${patch} no longer applies to vzvm's source." >&2
    exit 1
  fi
done

file=Sources/vzvm/VirtualMachineBuilder.swift
# The line numbers holding a text, which must be exactly one; grep finding
# nothing is an answer here, not an error to stop on.
only() {
  local lines
  lines=$(grep -n -F "$1" "${file}" | cut -d: -f1 || true)
  if [[ -z ${lines} || ${lines} == *$'\n'* ]]; then
    echo "vzvm's ${file} holds '$1' at lines '${lines//$'\n'/ }', not at one." >&2
    exit 1
  fi
  echo "${lines}"
}
kept=$(only 'network = created')
guarded=$(only 'if let network {')
reused=$(only 'return VZVmnetNetworkDeviceAttachment(network: network)')
made=$(only 'vmnet_network_create(')
if ((guarded >= reused || reused >= made || made >= kept)); then
  echo "A later boot isn't given the network the process made: guarded at" \
    "${guarded}, handed back at ${reused}, made at ${made}, kept at ${kept}." >&2
  exit 1
fi
