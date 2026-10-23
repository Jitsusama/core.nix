# shellcheck shell=bash
# Switches the machine to a NixOS system already built, such as what
# `nixos-rebuild build` leaves in ./result, with no password: polkit lets the
# machine's administrators start switch-system@.service and nothing more
# (nixos.nix). It returns once the switch is done, and on a failure shows
# what the switch said.
#
# usage: switch-system <system>

if [ "$#" -ne 1 ]; then
  echo "usage: switch-system <system>" >&2
  exit 64
fi

system=$(realpath --canonicalize-existing "$1")
unit="switch-system@$(systemd-escape --path "$system").service"

if ! systemctl start "$unit"; then
  journalctl --unit "$unit" --invocation=0 --no-pager --output=cat >&2 || true
  exit 1
fi
