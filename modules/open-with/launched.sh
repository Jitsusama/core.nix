# shellcheck shell=bash
# What GLib runs each program it starts from a desktop entry through, in
# gio-launch-desktop's place (nixos.nix says why): the program gets a scope
# of its own in app.slice, named for its entry as the desktop names one,
# app-<launcher>-<id>-<pid>.scope, rather than staying in whatever opened
# it. Like gio-launch-desktop, it says which process the entry started.
# Where the account's systemd can't be reached it runs the program as it is.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
launched  run an entry's program in a scope of its own

  What GLib starts each program from a desktop entry
  through: the program gets a scope of its own in
  app.slice, named for its entry. Without the account's
  systemd, the program runs as it is.

EXAMPLES
  launched mpv film.mkv

EXIT
  64  no program was named
  else the program's own, or systemd-run's
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -eq 0 ]; then
  printf '%s\n' "launched takes the program to run" \
    "  try  launched --help" >&2
  exit 64
fi

entry=${GIO_LAUNCHED_DESKTOP_FILE:-}
if [ -n "$entry" ]; then
  export GIO_LAUNCHED_DESKTOP_FILE_PID=$$
fi
if [ -S "${XDG_RUNTIME_DIR:-/nonexistent}/systemd/private" ]; then
  id=$(basename "${entry:-$1}" .desktop)
  # A unit's name takes letters, digits and only these marks.
  id=${id//[^A-Za-z0-9:_.-]/_}
  # The scope takes this process's pid, which the program keeps, so the
  # name is unique while it runs.
  exec systemd-run --user --scope --collect --quiet --slice=app.slice \
    --unit="app-gio-$id-$$" -- "$@"
fi
exec "$@"
