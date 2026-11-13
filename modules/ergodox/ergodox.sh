# shellcheck shell=bash
# shellcheck disable=SC2310 # channel and send say whether the board is there.
# The ErgoDox's command: what can be done to the board from a terminal, and
# what the shell runs when it's asked from the launcher. ERGODOX_FIRMWARE is
# this machine's build of the firmware (firmware.nix), and ERGODOX_LAYERS
# the layers' names in its order (layers.nix). The board's commands
# go over Oryx's raw HID channel (keymap/host.c), which only Linux's hidraw
# reaches from a script; ERGODOX_ROOT stands in for / when a check gives it
# a copy of sysfs.
# writeShellApplication sets these already; they're said here so a reader,
# and shellcheck, see what the script runs under.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ergodox <command>

Commands:
  flash              write this machine's build of the firmware to the board
  serial             print the serial number the board answers once it runs that build
  bootloader         restart the board ready to be flashed
  layer <layer>      lock the board on a layer, by its name or number, until the
                     layer lock key lets it go; the first layer locks nothing
  lights <on|off>    turn the board's lights on or off, until it restarts
  brighter, dimmer   step the lights' brightness, until it restarts
EOF
}

# Help is answered wherever it's asked, and a command given more words than
# it takes is refused, so `ergodox flash --help` never flashes the board
# and `ergodox bootloader now` never restarts it.
for word in "$@"; do
  case $word in
    -h | --help)
      usage
      exit 0
      ;;
    *) ;;
  esac
done
# Refuses the command unless it was given exactly this many words, itself
# included.
words=$#
asked=${1-}
takes() {
  [ "$words" -eq "$1" ] && return 0
  case $1 in
    1) printf '%s\n' "ergodox $asked takes no more words" >&2 ;;
    *) printf '%s\n' "ergodox $asked takes one more word" >&2 ;;
  esac
  printf '%s\n' "  try  ergodox --help" >&2
  exit 64
}

root=${ERGODOX_ROOT-}

# The board's channel is the hidraw node whose device is ZSA's and whose
# report descriptor opens on usage page 0xFF60, as Oryx's does.
channel() {
  for node in "$root"/sys/class/hidraw/*; do
    grep -q '^HID_ID=0003:00003297:' "$node/device/uevent" 2>/dev/null || continue
    descriptor=$(od -An -tx1 -N3 "$node/device/report_descriptor" | tr -d ' \n') || continue
    [ "$descriptor" = 0660ff ] || continue
    printf '%s\n' "$root/dev/${node##*/}"
    return 0
  done
  return 1
}

# A command is one 32-byte report, its first byte saying which and its second
# what with, written in a single write behind hidraw's report number of 0.
send() {
  local node
  node=$(channel) || return 1
  {
    printf '\0%b%b' "\\0$(printf %o "$1")" "\\0$(printf %o "${2-0}")"
    head -c 30 /dev/zero
  } | dd of="$node" bs=33 count=1 iflag=fullblock status=none
}

# What a command is sent when a board is there to hear it, and the reason
# nothing was sent when one isn't.
ask() {
  send "$@" && return 0
  echo "ergodox: no ErgoDox is plugged in, or this machine can't reach its channel" >&2
  exit 69
}

case "${1-}" in
flash)
  takes 1
  # A board running this firmware restarts itself, and zapp finds it waiting.
  # One running anything else ignores the command, so zapp says when to press
  # its bootloader key instead: Delete's key on either thumb while Numb is
  # held.
  send 128 || true
  exec zapp flash "${ERGODOX_FIRMWARE:?the package gives its firmware}/ergodox.hex"
  ;;
serial)
  takes 1
  cat "${ERGODOX_FIRMWARE:?the package gives its firmware}/serial"
  ;;
bootloader)
  takes 1
  ask 128
  ;;
layer)
  takes 2
  index=0
  for name in ${ERGODOX_LAYERS:?the package gives the layer names}; do
    if [ "${2-}" = "$index" ] || [ "${2,,}" = "${name,,}" ]; then
      ask 130 "$index"
      exit 0
    fi
    index=$((index + 1))
  done
  usage >&2
  exit 64
  ;;
lights)
  takes 2
  case "${2-}" in
  on) ask 131 1 ;;
  off) ask 131 0 ;;
  *)
    usage >&2
    exit 64
    ;;
  esac
  ;;
brighter)
  takes 1
  ask 132 1
  ;;
dimmer)
  takes 1
  ask 132 0
  ;;
help | -h | --help)
  usage
  ;;
*)
  usage >&2
  exit 64
  ;;
esac
