# The ErgoDox's command: what can be done to the board from a terminal, and
# what the shell runs when it's asked from the launcher. ERGODOX_FIRMWARE is
# this machine's build of the firmware (firmware.nix).

usage() {
  cat <<'EOF'
Usage: ergodox <command>

Commands:
  flash    write this machine's build of the firmware to the board
  serial   print the serial number the board answers once it runs that build
EOF
}

case "${1-}" in
flash)
  # zapp finds the board and says when to press its bootloader key, Delete's
  # key on either thumb while Numb is held, before it writes.
  exec zapp flash "$ERGODOX_FIRMWARE/ergodox.hex"
  ;;
serial)
  cat "$ERGODOX_FIRMWARE/serial"
  ;;
help | -h | --help)
  usage
  ;;
*)
  usage >&2
  exit 64
  ;;
esac
