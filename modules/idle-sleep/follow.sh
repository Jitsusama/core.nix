# shellcheck shell=bash
# Tells logind whether an idle machine sleeps: on battery it does, and on the
# charger it only locks. logind's own settings are the charger's (nixos.nix
# says why), so on battery this writes the battery's as a drop-in, the file
# its argument names, and on the charger takes it away. systemd-ac-power
# answers yes or no only when it can read the power supplies, and anything
# but no counts as the charger, so a machine that can't tell never sleeps by
# itself.
#
# udev restarts this on every power supply event, and a restart stops a run
# still going, perhaps after it changed the file but before logind heard. So
# whether logind reloads is decided by what logind itself has, not by
# whether the file changed: the run that's left always ends with logind on
# the charger's word or the battery's.
set -euo pipefail

setting=$1
power=$(systemd-ac-power --verbose || true)

if [[ $power == no ]]; then
  if [[ ! -e $setting ]]; then
    # Written whole beside it first, since a reload of logind's own, from a
    # switch, could otherwise read half of it.
    printf '[Login]\nIdleAction=suspend\n' > "$setting.new"
    mv "$setting.new" "$setting"
  fi
else
  rm -f "$setting"
fi

# logind reads its drop-ins as it starts, so one that isn't running hears of
# the change then.
systemctl --quiet is-active systemd-logind.service || exit 0

heard=$(busctl get-property org.freedesktop.login1 /org/freedesktop/login1 \
  org.freedesktop.login1.Manager IdleAction)
if [[ -e $setting ]]; then
  [[ $heard == 's "suspend"' ]] && exit 0
else
  [[ $heard != 's "suspend"' ]] && exit 0
fi
systemctl reload systemd-logind.service
