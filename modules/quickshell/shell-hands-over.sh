# shellcheck shell=bash disable=SC2154 # XDG_RUNTIME_DIR and MAINPID come from the user manager.
# Hands the shell over to a new one as a switch reloads its unit, so the
# launcher answers all through it: the new shell loads held, drawing
# nothing and serving no socket, while the old one still draws and answers;
# once it's ready and has kept home, it becomes the unit's main process, the
# old one ends, and the new one shows its windows and takes the sockets. Run
# as the unit's ExecReload, with the shell's command as its arguments.
#
# A shell that isn't ready within its time is ended and the old one kept,
# and the reload fails, which a switch reports. A locked screen restarts
# instead, since a lock is taken by one shell at a time and a held one
# can't take it.
#
# Its help takes the shape every command of ours does (docs/commands.md), by
# hand, since a script has no kit.

case ${1:-} in
  -h | --help)
    cat << 'EOF'
quickshell-hands-over  hands the shell to a new one

EXAMPLES
  quickshell-hands-over quickshell    as the unit reloads

EXIT
  0   handed over, or restarted when locked
  1   the new shell failed; the old one stays
EOF
    exit 0
    ;;
  *) ;;
esac

ready="$XDG_RUNTIME_DIR/quickshell-held"
locked="$XDG_RUNTIME_DIR/quickshell-locked"
socket="$XDG_RUNTIME_DIR/launcher.sock"
aside="$socket.handing-over"

# The old shell's socket, taken aside below, goes back where launcher-say
# finds it, unless a socket is there already.
put_back() {
  if [ -e "$aside" ] && [ ! -e "$socket" ]; then
    mv -f "$aside" "$socket"
  fi
}

restart_if_locked() {
  local state=""
  read -r state 2>/dev/null <"$locked" || true
  if [ "$state" = locked ]; then
    echo "the screen is locked, so the shell restarts instead"
    put_back
    exec systemctl --user --no-block restart quickshell.service
  fi
}
restart_if_locked

old="$MAINPID"
rm -f "$ready"
QS_HELD="$ready" "$@" &
new=$!

# The shell's own start, about a second on a lit machine, with room for a
# busy one; past it, the old shell stays.
for ((i = 0; i < 100; i++)); do
  [ -s "$ready" ] && break
  if ! kill -0 "$new" 2>/dev/null; then
    echo "the new shell ended as it loaded, so the old one stays"
    exit 1
  fi
  sleep 0.1
done
if [ ! -s "$ready" ]; then
  echo "the new shell wasn't ready in 10 s, so the old one stays"
  kill -TERM "$new"
  exit 1
fi

# Loaded, a shell is busy for most of a second more making the launcher's
# home, and a chord it heard then waited on that. So the old one answers
# until the new one says it holds home kept, which took about 4 s on a
# software-rendered nested niri; past 10 s it's handed over anyway, since
# the new shell has loaded and only its first opening would be slower.
for ((i = 0; i < 100; i++)); do
  shown="$("$1" ipc --pid "$new" call launcher showing 2>/dev/null || true)"
  jq -e .kept <<<"$shown" >/dev/null 2>&1 && break
  sleep 0.1
done

# Ended open, the old launcher would vanish under Joel, and Super+S's line
# with what he was saying in it, so it's handed over once he has closed
# them, for up to a minute, past which a switch has waited long enough.
# The new shell wrote "closed" in the line's mark as it loaded, and the old
# one writes only as its line changes, so "open" there now is the old
# shell's line, come up since.
talking="$XDG_RUNTIME_DIR/quickshell-talk-line"
shut() {
  local shown open line=""
  shown="$("$1" ipc --pid "$old" call launcher showing 2>/dev/null || true)"
  open="$(jq -r .open <<<"$shown" 2>/dev/null || true)"
  read -r line 2>/dev/null <"$talking" || true
  [ "$open" != true ] && [ "$line" != open ]
}
# Found shut, the old shell could still hear a chord before its socket
# went, open, and be ended under it: on a busy builder the fourth of four
# chords drew the old launcher and was never presented (2026-11-14,
# e777b33d). So the socket is taken aside, which launcher-say waits out,
# and the shell asked again once a chord it had already been sent has
# opened it; open then, the socket goes back and the wait goes on.
for ((i = 0; i < 600; i++)); do
  if shut "$1"; then
    mv -f "$socket" "$aside" 2>/dev/null || true
    # A chord read before the socket went is opening within this.
    sleep 0.05
    shut "$1" && break
    put_back
  fi
  sleep 0.1
done

# Locked while the new shell loaded, the old one holds the lock, and the
# restart ends the new one with the unit.
restart_if_locked

# The main process moves first: had the old one ended first, systemd would
# take the unit as stopped and end the new shell with it.
if ! systemd-notify --pid="$new"; then
  echo "systemd didn't take the new shell as the unit's, so the old one stays"
  put_back
  kill -TERM "$new"
  exit 1
fi

# A chord said as the old shell ends would be heard by it and lost, so its
# socket goes first, as the unit's stop does, and launcher-say waits for the
# new shell's.
rm -f "$socket" "$aside"

# The old shell's group, so what it started (its Process children) ends
# with it rather than lingering in the unit. One already gone has none.
group="$(ps -o pgid= -p "$old" | tr -d ' ' || true)"
if [ -n "$group" ]; then
  kill -TERM -- "-$group" 2>/dev/null || true
fi
# Polled, as the old shell isn't this script's child to wait on. It ends in
# a fifth of a second or so, and one still there after 3 s is killed, since
# the new one draws nothing until it's gone.
for ((i = 0; i < 300; i++)); do
  kill -0 "$old" 2>/dev/null || break
  sleep 0.01
done
if [ -n "$group" ]; then
  kill -KILL -- "-$group" 2>/dev/null || true
fi

kill -USR1 "$new"
rm -f "$ready"
