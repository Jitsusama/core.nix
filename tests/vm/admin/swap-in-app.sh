# shellcheck shell=sh
# A stand-in for an app such as Chrome, whose window's process runs in a
# scope named for its own pid and starts its renderers in a scope beside
# it, named for that pid too. Run as the account, as
# `swap-in-app.sh SIZE RUNTIME`: it moves itself into app-window-PID, then
# starts a heap of SIZE MiB in app-renderer-PID, writes its own pid to
# RUNTIME/app-window.pid and the heap's to RUNTIME/app-renderer.pid, and
# waits. systemd-run --scope runs its command in its own process, so the
# pid stays the same across the move.
set -eu

size=$1
runtime=$2

if [ "${3-}" != moved ]; then
  exec systemd-run --user --scope --quiet --unit="app-window-$$" \
    sh "$0" "$size" "$runtime" moved
fi
systemd-run --user --scope --quiet --unit="app-renderer-$$" -p MemoryZSwapMax=0 \
  python3 /etc/swap-in/heap.py "$size" "$runtime/app-renderer.pid" &
echo "$$" > "$runtime/app-window.pid"
wait
