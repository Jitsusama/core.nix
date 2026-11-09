# shellcheck shell=bash
# shellcheck disable=SC2154 # XDG_RUNTIME_DIR is the session's, set by systemd.
# pi, as the shell opens it, for the VM tests, which have no pi
# (tests/pi-stand-in.nix): where it started and the quest it was told to
# load, then what it was told, a line each, written down.
echo "$PWD ${QUEST_WORKFLOW_AUTOLOAD_ID:-}" >"$XDG_RUNTIME_DIR/pi-opened"
printf '%s\n' "$@" >>"$XDG_RUNTIME_DIR/pi-opened"
# While a test leaves pi-stays, it stays, its window titled with all it was
# told, so the test can find where it opened, and its process and kitty
# window written down by its session, so a test can say its record as pi
# would (desktop-reopens-under-orchestrators).
if [ -e "$XDG_RUNTIME_DIR/pi-stays" ]; then
  printf '\033]2;pi %s\007' "$*"
  if [ "${1:-}" = --session ]; then
    echo "$$ ${KITTY_LISTEN_ON:-} ${KITTY_WINDOW_ID:-}" >"$XDG_RUNTIME_DIR/pi-owner-$2"
  fi
  # It stands in for a session's pi as long as a test keeps its window, ten
  # minutes at most, and a test's cleanup ends it sooner.
  exec sleep 600
fi
