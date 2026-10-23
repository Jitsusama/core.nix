# shellcheck shell=bash
# Switches the machine to the NixOS system it's given, as root, for
# switch-system@.service: the system profile points at it, so it boots next
# too, and then it's made the running system. What isn't a NixOS system in
# the store is refused before anything changes.
#
# usage: switch-system-as-root /nix/store/<hash>-<name>

system=$1

# A path straight under the store, so nothing outside it, and nothing it
# links to, is switched to.
if [[ ! $system =~ ^/nix/store/[0-9a-z]{32}-[^/]+$ ]]; then
  echo "not a path in the Nix store: $system" >&2
  exit 64
fi
if [[ ! -x $system/bin/switch-to-configuration || ! -f $system/nixos-version ]]; then
  echo "not a NixOS system: $system" >&2
  exit 64
fi

nix-env --profile /nix/var/nix/profiles/system --set "$system"
exec "$system/bin/switch-to-configuration" switch
