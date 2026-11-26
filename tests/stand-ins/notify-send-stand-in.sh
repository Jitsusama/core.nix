# shellcheck shell=bash disable=SC2154 # The check's session sets XDG_RUNTIME_DIR (people.nix).
# notify-send, for the shell's checks that tell notices without a notices
# daemon: each notice's words are kept a line each in
# $XDG_RUNTIME_DIR/told, any line breaks in them kept as spaces, and its
# id is the line it was kept on. A notice notify-send would wait on, its
# flags before any `--` offering an action or telling it to wait, waits as
# it does until the notice closes, which the busctl beside it says by
# writing $XDG_RUNTIME_DIR/closed-<id>.

told="$XDG_RUNTIME_DIR/told"
# One notice at a time takes its line and id, as the shell sends several
# at once.
exec 9>>"$told.lock"
flock 9
words="$*"
printf '%s\n' "${words//$'\n'/ }" >>"$told"
id=$(wc -l <"$told")
flock -u 9
echo "$id"
# Any other notice is told and let go at once, as notify-send does.
waits=false
for word in "$@"; do
  case $word in
  --) break ;;
  --wait | --action=*) waits=true ;;
  *) ;;
  esac
done
if [ "$waits" = true ]; then
  until [ -e "$XDG_RUNTIME_DIR/closed-$id" ]; do
    # notify-send waits on the bus for as long as a notice is shown; this
    # looks for its closing about as often as a frame is drawn.
    sleep 0.05
  done
fi
