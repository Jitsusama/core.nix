# shellcheck shell=bash
# Matches each udev rule's patterns, as udev matches them, against the I2C
# buses of a laptop with a screen behind a USB-C hub, and checks only the
# buses that reach external DisplayPort screens, a connector's AUX channel
# and an MST branch's, are tagged for the seat: not the panel's eDP, not
# HDMI's, not the touchpad's. The rules must also come before systemd's
# 73-seat-late.rules, which reads the tag.
# usage: only-external-screens-reach-the-seat.sh <rules file>
set -euo pipefail

rules=$1

case $(basename "$rules") in
  [0-6]* | 7[0-2]*) ;;
  *)
    echo "the rules come after 73-seat-late.rules"
    exit 1
    ;;
esac

# The value a rule gives a key, or nothing when it names none.
key() {
  sed -nE "s/.*$1==\"([^\"]*)\".*/\\1/p" <<< "$2"
}

# Whether a bus, by its name and its devices up to the PCI root, is tagged
# for the seat by a rule: SUBSYSTEM, KERNELS and ATTR{name} matched as
# fnmatch does, each key a rule names having to match.
seated() {
  local name=$1 rule subsystem kernels attr device found
  shift
  while IFS= read -r rule; do
    case $rule in "#"* | "") continue ;; esac
    case $rule in *'TAG+="uaccess"'*) ;; *) continue ;; esac
    subsystem=$(key SUBSYSTEM "$rule")
    kernels=$(key KERNELS "$rule")
    attr=$(key 'ATTR\{name\}' "$rule")
    # A rule's values are patterns, so each is matched unquoted, as a glob.
    # shellcheck disable=SC2053
    [[ i2c-dev == $subsystem ]] || continue
    # shellcheck disable=SC2053
    if [ -n "$attr" ] && [[ $name != $attr ]]; then
      continue
    fi
    if [ -n "$kernels" ]; then
      found=
      for device in "$@"; do
        # shellcheck disable=SC2053
        [[ $device == $kernels ]] && found=1
      done
      [ -n "$found" ] || continue
    fi
    return 0
  done < "$rules"
  return 1
}

wrong=0
# Says where a bus named so, above the devices given, should go: the seat or
# root alone.
expect() {
  local want=$1 got
  shift
  if seated "$@"; then got=seat; else got=root; fi
  if [ "$got" != "$want" ]; then
    echo "the bus named '$1' above $2 goes to $got, not $want"
    wrong=1
  fi
}
expect seat "AUX USBC1/DDI TC1/PHY TC1" i2c-10 card0-DP-1 card0 0000:00:02.0 pci0000:00
expect seat "AUX USBC3/DDI TC3/PHY TC3" i2c-12 card1-DP-12 card1 0000:03:00.0 pci0000:00
expect seat DPMST i2c-13 0000:00:02.0 pci0000:00
expect root "AUX A/DDI A/PHY A" i2c-9 card0-eDP-1 card0 0000:00:02.0 pci0000:00
expect root "AUX B/DDI B/PHY B" i2c-14 card0-HDMI-A-1 card0 0000:00:02.0 pci0000:00
expect root "i915 gmbus dpa" i2c-3 0000:00:02.0 pci0000:00
expect root "Synopsys DesignWare I2C adapter" i2c-0 i2c_designware.0 0000:00:15.0 pci0000:00
[ "$wrong" = 0 ]
