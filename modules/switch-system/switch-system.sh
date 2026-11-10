# shellcheck shell=bash
# Switches the machine to a NixOS system already built, such as what
# `nixos-rebuild build` leaves in ./result: polkit lets the machine's
# administrators start switch-system@.service and nothing more, with no
# password for a system the machine has run and an administrator's
# authentication for any other (nixos.nix). Run from a terminal, systemctl
# asks for it there; with none, the switch is refused. It returns once the
# switch is done, and on a failure shows what the switch said.
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
