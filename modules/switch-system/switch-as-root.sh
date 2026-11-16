# shellcheck shell=bash
# Switches the machine to the NixOS system it's given, as root, for
# switch-system@.service: the system profile points at it, so it boots next
# too, and then it's made the running system. What isn't a NixOS system in
# the store is refused before anything changes.
#
# usage: switch-system-as-root /nix/store/<hash>-<name>
#
# Its help takes the shape every command of ours does (docs/commands.md), by
# hand, since a script has no kit. Help comes before anything is read, so
# asking for it changes nothing.

if [[ $# -eq 1 && ($1 == --help || $1 == -h) ]]; then
  cat << 'EOF'
switch-system-as-root  switches to a NixOS system, as root

  The system profile points at the system it's given, so it
  boots next too, and then it's made the running system.
  switch-system@.service runs it; people run switch-system.

EXAMPLES
  switch-system-as-root /nix/store/<hash>-<name>

EXIT
  0   switched
  64  not a NixOS system in the store, so nothing changed
  what switch-to-configuration exits, once it runs
EOF
  exit 0
fi

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
