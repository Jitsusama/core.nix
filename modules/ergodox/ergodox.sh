# The ErgoDox's command: what can be done to the board from a terminal, and
# what the shell runs when it's asked from the launcher. ERGODOX_FIRMWARE is
# this machine's build of the firmware (firmware.nix), and ERGODOX_LAYERS
# the layers' names in its order (layers.nix). The board's commands
# go over Oryx's raw HID channel (keymap/host.c), which only Linux's hidraw
# reaches from a script; ERGODOX_ROOT stands in for / when a check gives it
# a copy of sysfs.

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

root=${ERGODOX_ROOT-}

# The board's channel is the hidraw node whose device is ZSA's and whose
# report descriptor opens on usage page 0xFF60, as Oryx's does.
channel() {
  for node in "$root"/sys/class/hidraw/*; do
    grep -q '^HID_ID=0003:00003297:' "$node/device/uevent" 2>/dev/null || continue
    [ "$(od -An -tx1 -N3 "$node/device/report_descriptor" | tr -d ' \n')" = 0660ff ] || continue
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
  # A board running this firmware restarts itself, and zapp finds it waiting.
  # One running anything else ignores the command, so zapp says when to press
  # its bootloader key instead: Delete's key on either thumb while Numb is
  # held.
  send 128 || true
  exec zapp flash "$ERGODOX_FIRMWARE/ergodox.hex"
  ;;
serial)
  cat "$ERGODOX_FIRMWARE/serial"
  ;;
bootloader)
  ask 128
  ;;
layer)
  index=0
  for name in $ERGODOX_LAYERS; do
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
  ask 132 1
  ;;
dimmer)
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
