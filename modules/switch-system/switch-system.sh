# shellcheck shell=bash
# Switches the machine to a NixOS system already built, such as what
# `nixos-rebuild build` leaves in ./result: polkit lets the machine's
# administrators start switch-system@.service and nothing more, with no
# password for a system the machine has run and an administrator's
# authentication for any other (nixos.nix). Run from a terminal, systemctl
# asks for it there; with none, the switch is refused. It returns once the
# switch is done, and on a failure shows what the switch said. Its help and
# failures take the shape every command of ours does (docs/commands.md), by
# hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
switch-system  switch the machine to a system already built

  A system the machine has run switches with no password;
  any other asks an administrator once.

EXAMPLES
  switch-system ./result  switch to what was just built

EXIT
  0   switched
  1   the switch failed, and what it said is shown
  64  asked wrongly
  66  there's no system there
EOF
  exit 0
  ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "switch-system takes one system" "  try  switch-system --help" >&2
  exit 64
fi

if ! system=$(realpath --canonicalize-existing "$1" 2> /dev/null); then
  printf '%s\n' "there's no system at $1" "  try  switch-system ./result, after a build" >&2
  exit 66
fi
unit="switch-system@$(systemd-escape --path "$system").service"

if ! systemctl start "$unit"; then
  journalctl --unit "$unit" --invocation=0 --no-pager --output=cat >&2 || true
  exit 1
fi
