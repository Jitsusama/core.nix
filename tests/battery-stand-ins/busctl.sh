# shellcheck shell=bash
# Stands in for busctl asking UPower about the battery and power-profiles-daemon
# about the profile. Each time it's asked whether the machine is on battery,
# it answers with the next of the states in $STATE/steps, "on-battery percent
# [refused]" a line, and the percentage is the one that state gave. Asked once
# more than there are states, it says so in $STATE/errors and fails.
#
# The daemon offers the three profiles with balanced in use, and writes each
# profile it's asked to set to $STATE/calls, after the state it was set in. A
# profile the state names as refused fails as the kernel's refusal did, and is
# written as refused.
# usage: busctl.sh <busctl's arguments>
set -euo pipefail

profile() {
  if [ -e "$STATE/active" ]; then
    cat "$STATE/active"
  else
    echo balanced
  fi
}

# The daemon's interface, org.freedesktop.UPower.PowerProfiles, holds the name
# of one of its properties, so the daemon's are told apart by the arguments
# after it.
case "$*" in
  *OnBattery*)
    if [ ! -s "$STATE/steps" ]; then
      echo "asked once too often" >> "$STATE/errors"
      exit 1
    fi
    head -n 1 "$STATE/steps" > "$STATE/now"
    tail -n +2 "$STATE/steps" > "$STATE/rest"
    mv "$STATE/rest" "$STATE/steps"
    read -r battery _ < "$STATE/now"
    echo "b $battery"
    ;;
  *Percentage*)
    read -r _ percent _ < "$STATE/now"
    echo "d $percent.6"
    ;;
  *" Profiles")
    echo 'aa{sv} 3 1 "Profile" s "power-saver" 1 "Profile" s "balanced" 1 "Profile" s "performance"'
    ;;
  *set-property*" ActiveProfile s "*)
    wanted=${*: -1}
    read -r _ _ refused < "$STATE/now"
    if [ "$wanted" = "${refused:-}" ]; then
      echo "$(cat "$STATE/now"): refused $wanted" >> "$STATE/calls"
      echo "Operation not supported" >&2
      exit 1
    fi
    echo "$wanted" > "$STATE/active"
    echo "$(cat "$STATE/now"): $wanted" >> "$STATE/calls"
    ;;
  *" ActiveProfile")
    echo "s \"$(profile)\""
    ;;
esac
