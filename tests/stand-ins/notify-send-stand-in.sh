# shellcheck shell=bash
# notify-send, for the shell's checks that tell notices without a notices
# daemon: each notice's words are kept a line each in
# $XDG_RUNTIME_DIR/told, and its id is the line it was kept on. A notice
# notify-send would wait on, offering an action or told to wait, waits as
# it does until the notice closes, which the busctl beside it says by
# writing $XDG_RUNTIME_DIR/closed-<id>.

told="$XDG_RUNTIME_DIR/told"
# One notice at a time takes its line and id, as the shell sends several
# at once.
exec 9>>"$told.lock"
flock 9
printf '%s\n' "$*" >>"$told"
id=$(wc -l <"$told")
flock -u 9
echo "$id"
case " $* " in
*" --wait "* | *" --action="*)
  until [ -e "$XDG_RUNTIME_DIR/closed-$id" ]; do
    # notify-send waits on the bus for as long as a notice is shown; this
    # looks for its closing about as often as a frame is drawn.
    sleep 0.05
  done
  ;;
esac
