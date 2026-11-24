#!/usr/bin/env bash
# The test that the Mac's builder reuses its network: vzvm's patches, in the
# order given, apply to a copy of its source with no fuzz; past the macOS 26
# guard, the attachment hands a kept network back, once, before vmnet is
# asked for another; the network made is kept; a guest losing its attachment
# stops the next boot reusing it, which lets it go, until another is made;
# and every guest of a process has one hardware address.
# Usage: SOURCE PATCH...
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

# The number of the one line of a file that is a text, leading space aside;
# none, or more than one, fails.
only() {
  local lines
  lines=$(awk -v text="$2" '{ line = $0; sub(/^[ \t]+/, "", line) } line == text { print NR }' "$1")
  if [[ -z ${lines} || ${lines} == *$'\n'* ]]; then
    echo "vzvm's $1 holds '$2' at lines '${lines//$'\n'/ }', not at one." >&2
    exit 1
  fi
  echo "${lines}"
}
builder=Sources/vzvm/VirtualMachineBuilder.swift
newer=$(only "${builder}" 'guard #available(macOS 26.0, *) else {')
guarded=$(only "${builder}" 'if let network, !networkLost {')
reused=$(only "${builder}" 'return VZVmnetNetworkDeviceAttachment(network: network)')
let_go=$(only "${builder}" 'network = nil')
configured=$(only "${builder}" \
  'guard let shared = vmnet_network_configuration_create(.VMNET_SHARED_MODE, &status) else {')
made=$(only "${builder}" 'guard let created = vmnet_network_create(shared, &status) else {')
kept=$(only "${builder}" 'network = created')
cleared=$(only "${builder}" 'networkLost = false')
only "${builder}" 'static var networkLost = false' >/dev/null
if ((newer >= guarded || guarded >= reused || reused >= let_go || let_go >= configured ||
  configured >= made || made >= kept || kept >= cleared)); then
  echo "A later boot isn't given the network the process made: past the guard at ${newer}," \
    "guarded at ${guarded}, handed back at ${reused}, a lost one let go at ${let_go}," \
    "configured at ${configured}, made at ${made}, kept at ${kept}, its loss cleared at" \
    "${cleared}." >&2
  exit 1
fi

# One hardware address a process, so the kept network's DHCP server sees one client.
only "${builder}" 'private static let guestAddress = VZMACAddress.randomLocallyAdministered()' \
  >/dev/null
only "${builder}" 'network.macAddress = guestAddress' >/dev/null

guest=Sources/vzvm/Guest.swift
detached=$(only "${guest}" 'attachmentWasDisconnectedWithError error: Error')
lost=$(only "${guest}" 'VirtualMachineBuilder.networkLost = true')
if ((detached >= lost)); then
  echo "A guest losing its network isn't said to the builder: detached at ${detached}," \
    "said at ${lost}." >&2
  exit 1
fi
