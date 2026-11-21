# shellcheck shell=bash
# Stands in for Tuple's command as tuple-call runs it (modules/tuple/call),
# since Tuple can't sign in to a call in the VM: its stream says each line
# a test writes to tuple-stream in the runtime folder, from the first, and
# every other command is kept in tuple-asked there, a line each. Leaving
# ends the call, as Tuple's stream says once it has, and opening Joel's
# room says a room's link, as Tuple's does.
stream=$XDG_RUNTIME_DIR/tuple-stream
if [ "$3 $4" = "state follow" ]; then
  touch "$stream"
  exec tail -n +1 -F "$stream"
fi
echo "$*" >>"$XDG_RUNTIME_DIR/tuple-asked"
case "$*" in
"call leave") echo '{"type":"current_call","data":null}' >>"$stream" ;;
new) echo "Joining https://tuple.app/c/stand-in" ;;
esac
