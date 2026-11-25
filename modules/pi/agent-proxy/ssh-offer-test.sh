# shellcheck shell=bash
# Proves the machine's ssh block as ssh itself reads it: the OS Login key
# is offered to compute.* hosts, and to no other.
#
#   ssh-offer-test.sh PATH-TO-THE-MACHINE'S-SSH-BLOCK

set -euo pipefail
work=$(mktemp -d)
# The build sandbox owns the store as nobody, and ssh refuses an included
# file it doesn't trust, so the store's includes are left out here. On a
# machine root owns them, and ssh reads them as it should.
block="$work/ssh_config"
: >"$block"
while IFS= read -r line; do
  [[ $line == "Include /nix/store/"* ]] || printf '%s\n' "$line" >>"$block"
done <"$1"

offered=$(ssh -G -F "$block" compute.1)
if ! grep -qx 'identityfile /run/user/%i/pi-agent/os-login.pub' <<<"$offered"; then
  echo "the machine's ssh doesn't offer the OS Login key to compute.* hosts" >&2
  exit 1
fi
elsewhere=$(ssh -G -F "$block" example.org)
if grep -q os-login <<<"$elsewhere"; then
  echo "the machine's ssh offers the OS Login key past compute.* hosts" >&2
  exit 1
fi
