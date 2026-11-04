# shellcheck shell=bash
# Tells logind whether an idle machine sleeps: on battery it does, and on the
# charger it only locks. logind's own settings are the charger's (nixos.nix
# says why), so on battery this writes the battery's as a drop-in, the file
# its argument names, and on the charger takes it away, reloading logind only
# when that changes. systemd-ac-power answers yes or no only when it can read
# the power supplies, and anything but no counts as the charger, so a machine
# that can't tell never sleeps by itself.
set -euo pipefail

setting=$1
power=$(systemd-ac-power --verbose || true)

if [[ $power == no ]]; then
  [[ -e $setting ]] && exit 0
  # Written whole beside it first, since a reload of logind's own, from a
  # switch, could otherwise read half of it.
  printf '[Login]\nIdleAction=suspend\n' > "$setting.new"
  mv "$setting.new" "$setting"
else
  [[ -e $setting ]] || exit 0
  rm "$setting"
fi

# logind reads its drop-ins as it starts, so one that isn't running hears of
# the change then.
if systemctl --quiet is-active systemd-logind.service; then
  systemctl reload systemd-logind.service
fi
