# shellcheck shell=bash
# Says whether a switch-system@ unit would switch to a system the machine
# has run before: one of the system profile's generations, which only root
# makes, through update-to-main or a switch an administrator authenticated.
# polkit runs it for switch-system.rules and lets such a switch go unasked.
#
# A system that let a YubiKey's touch in without the person's own secret
# is never one: such a generation stays in the profile until it's
# collected, and switching back to it unasked would bring the touch back
# for anything running as an administrator. So a system whose PAM names
# pam_u2f anywhere but polkit's, or in polkit's without the check that the
# person is already in (modules/yubikey/administers.nix), asks.
#
# usage: switch-system-known switch-system@<escaped system>.service
# Its status is 0 for a system the profile holds, and 1 for any other.
# SWITCH_SYSTEM_PROFILES names another profiles folder, for its checks.

unit=$1
instance=${unit#switch-system@}
instance=${instance%.service}
system=$(systemd-escape --unescape --path "$instance")
profiles=${SWITCH_SYSTEM_PROFILES:-/nix/var/nix/profiles}

# Whether that system's PAM takes a touch alone anywhere.
touch_alone() {
  local pam=$1/etc/pam.d service
  for service in "$pam"/*; do
    [[ -f $service ]] || continue
    grep -q pam_u2f "$service" || continue
    if [[ ${service##*/} != polkit-1 ]] || ! grep -q yubikey-once-in "$service"; then
      return 0
    fi
  done
  return 1
}

for generation in "$profiles"/system-*-link; do
  [[ -e $generation ]] || continue
  held=$(readlink --canonicalize "$generation")
  if [[ $held == "$system" ]]; then
    if touch_alone "$held"; then
      exit 1
    fi
    exit 0
  fi
done
exit 1
