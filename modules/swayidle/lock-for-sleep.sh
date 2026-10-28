# shellcheck shell=bash
# Before the machine sleeps: locks the screen, then turns the screens off,
# so they go down dark. To write a hibernation's image the kernel wakes
# every device, and the graphics card wakes as it was put down: off, so
# nothing lights while the image is written. The screens go off even when
# the lock wasn't drawn in time, which lock-screen reports by failing.

status=0
lock-screen || status=$?
niri msg action power-off-monitors
exit "$status"
