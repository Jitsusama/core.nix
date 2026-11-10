# shellcheck shell=bash
# Says whether a switch-system@ unit would switch to a system the machine
# has run before: one of the system profile's generations, which only root
# makes, through update-to-main or a switch an administrator authenticated.
# polkit runs it for switch-system.rules and lets such a switch go unasked.
#
# usage: switch-system-known switch-system@<escaped system>.service
# Its status is 0 for a system the profile holds, and 1 for any other.

unit=$1
instance=${unit#switch-system@}
instance=${instance%.service}
system=$(systemd-escape --unescape --path "$instance")

for generation in /nix/var/nix/profiles/system-*-link; do
  if [[ -e $generation && $(readlink --canonicalize "$generation") == "$system" ]]; then
    exit 0
  fi
done
exit 1
