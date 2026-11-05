# shellcheck shell=bash
# Carries a command from the launcher's command line to the swayimg it was
# opened over, which has no socket to be told things over: the command's
# whole name and the rest of what was typed, as a line with a tab between,
# in a file named for swayimg's process, written aside and renamed over so
# it's never read half written, and then USR1, which has swayimg read it
# (init.lua).
# usage: swayimg-command <swayimg's process> <command> [<the rest>]
set -euo pipefail

pid=$1
name=$2
rest=${3:-}
folder="$XDG_RUNTIME_DIR/swayimg"
mkdir -p "$folder"
printf '%s\t%s\n' "$name" "$rest" > "$folder/.$pid"
mv "$folder/.$pid" "$folder/$pid"
kill -USR1 "$pid"
