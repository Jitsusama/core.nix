# shellcheck shell=bash
# What happens as Joel steps away, each after the seconds it's given since
# the last key or touch: `swayidle-away MARK LOCK OFF` stops marking
# someone at the machine, locks the screen and turns the screens off. A key
# or touch after a second without one marks someone there again, so the
# mark comes back at once even when this swayidle has only just started and
# hasn't yet reached MARK.
#
# It runs a swayidle of its own that asks logind nothing, so logind's lock
# against going idle stops none of these. A swayidle that hears logind
# drops every timeout while anything holds that lock, and mycelium holds it
# whenever it holds work, so the screen went unlocked for as long as Joel
# was away (2026-11-10). What does ask the screen to stay holds that lock
# too, so the lock and the screens going off ask screen-kept first. A video
# or a call in Chrome keeps the screen through Wayland's own inhibitor,
# which niri honours and this swayidle hears.

mark=$1
lock=$2
off=$3
here=${XDG_RUNTIME_DIR:?}/someone-at-the-machine

exec swayidle \
  timeout 1 true resume "touch '$here'" \
  timeout "$mark" "rm -f '$here'" \
  timeout "$lock" "screen-kept || lock-screen 'Joel stepped away'" \
  timeout "$off" "screen-kept || niri msg action power-off-monitors"
