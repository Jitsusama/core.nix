# shellcheck shell=bash
# shellcheck disable=SC2154 # XDG_RUNTIME_DIR is the session's, set by systemd.
# pi, as the shell opens it, for the VM tests, which have no pi
# (tests/pi-stand-in.nix): where it started and the quest it was told to
# load, then what it was told, a line each, written down.
echo "$PWD ${QUEST_WORKFLOW_AUTOLOAD_ID:-}" >"$XDG_RUNTIME_DIR/pi-opened"
printf '%s\n' "$@" >>"$XDG_RUNTIME_DIR/pi-opened"
# While a test leaves pi-moves-in-place, a held session's pi moves into a
# pi host in its own window, as the harness's does (its mesh-workflow's
# in-place.ts and host-join.ts), each step a start of its holder's on the
# same terminal (desktop-moves-keep-their-window): pi draws its frame and
# takes keys, then on ~/move-ID names the join as what its holder starts
# next and exits as a park does, drawing nothing more; the join draws
# nothing and, on ~/hosted-ID, names the host's viewer, or on
# ~/refused-ID, plain pi on the session's log; and either draws the same
# frame again, as a viewer catching up does, and takes the keys from there.
if [ -e "$XDG_RUNTIME_DIR/pi-moves-in-place" ]; then
  frame() {
    # The same frame each time, every line placed, so one drawn again over
    # it changes no pixel; in one synchronized update, as pi's are.
    printf '\033[?2026h\033[H\033[1;1Hpi, held in its window: %s\033[2;1H%s\033[?2026l' \
      "$1" 'Every key typed here is kept, whatever starts next.'
  }
  keys() {
    # Each key as it comes, to the file the test reads, a byte a read, so
    # none is taken from the terminal but those written down.
    while [ ! -e "$HOME/move-$1" ]; do
      if IFS= read -r -s -n 1 -t 0.2 key; then printf '%s' "$key" >>"$HOME/typed-$1"; fi
    done
  }
  case "${1:-} ${2:-}" in
  "--session "*/*)
    # Plain pi again on the session's log, as the join falls back to.
    session=$(basename "$2" .jsonl)
    stty raw -echo
    frame "$session"
    echo "$$ again" >"$XDG_RUNTIME_DIR/moves-$session"
    cat >>"$HOME/typed-$session"
    exit 0
    ;;
  "--session "*)
    session=$2
    printf '\033]2;pi --session %s\007\033[?1049h\033[?25l\033[2J' "$session"
    stty raw -echo
    frame "$session"
    echo "$$ ${MYCELIUM_WAKE:-}" >"$XDG_RUNTIME_DIR/moves-$session"
    keys "$session"
    printf '["pi","join","--session","%s"]' "$session" >"$MYCELIUM_AGAIN"
    : >"$MYCELIUM_WAKE"
    exit 75
    ;;
  "join --session")
    session=$3
    echo "$$ join" >"$XDG_RUNTIME_DIR/moves-$session"
    until [ -e "$HOME/hosted-$session" ] || [ -e "$HOME/refused-$session" ]; do
      sleep 0.1
    done
    if [ -e "$HOME/hosted-$session" ]; then
      printf '["pi","attach","--host","harbour-attended","--session","%s"]' \
        "$session" >"$MYCELIUM_AGAIN"
    else
      printf '["pi","--session","%s/%s.jsonl"]' "$HOME" "$session" >"$MYCELIUM_AGAIN"
    fi
    : >"$MYCELIUM_WAKE"
    exit 75
    ;;
  "attach --host")
    session=$5
    stty raw -echo
    frame "$session"
    echo "$$ attach" >"$XDG_RUNTIME_DIR/moves-$session"
    cat >>"$HOME/typed-$session"
    exit 0
    ;;
  *) ;;
  esac
fi
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
