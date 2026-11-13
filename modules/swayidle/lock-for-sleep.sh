# shellcheck shell=bash
# Before the machine sleeps: locks the screen, then turns the screens off,
# so they go down dark. To write a hibernation's image the kernel wakes
# every device, and the graphics card wakes as it was put down: off, so
# nothing lights while the image is written. The screens go off even when
# the lock wasn't drawn in time, which lock-screen reports by failing.
#
# It takes no words, and refuses any, so --help or a typo never locks the
# screen and turns it off.

if [ "$#" -gt 0 ]; then
  case $1 in
    -h | --help)
      cat << 'EOF'
lock-for-sleep  locks the screen and turns it off, for a sleep

EXAMPLES
  lock-for-sleep   what swayidle runs before every sleep

EXIT
  0   locked, and the screens are off
  1   the lock wasn't drawn in time; the screens went off
  64  asked wrongly
EOF
      exit 0
      ;;
    *)
      printf '%s\n' "lock-for-sleep takes no words" "  try  lock-for-sleep --help" >&2
      exit 64
      ;;
  esac
fi

status=0
lock-screen --reason "the machine sleeps" || status=$?
niri msg action power-off-monitors
exit "$status"
