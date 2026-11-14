# shellcheck shell=bash
# Carries a command from the launcher's command line to the swayimg it was
# opened over, which has no socket to be told things over: the command's
# whole name and the rest of what was typed, as a line with a tab between,
# in a file named for swayimg's process, written aside and renamed over so
# it's never read half written, and then USR1, which has swayimg read it
# (init.lua). Its help and failures take the shape every command of ours
# does (docs/commands.md), by hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
swayimg-command  carry a command to the swayimg it's for

  The launcher's command line runs it over a picture, with
  swayimg's process, the command's whole name and the rest
  of what was typed.

EXAMPLES
  swayimg-command 4211 rotate 90

EXIT
  0   carried
  1   swayimg wasn't there to take it
  64  asked wrongly
EOF
  exit 0
  ;;
esac

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  printf '%s\n' "swayimg-command takes swayimg's process and a command" \
    "  try  swayimg-command --help" >&2
  exit 64
fi

pid=$1
name=$2
rest=${3:-}
folder="$XDG_RUNTIME_DIR/swayimg"
mkdir -p "$folder"
printf '%s\t%s\n' "$name" "$rest" > "$folder/.$pid"
mv "$folder/.$pid" "$folder/$pid"
kill -USR1 "$pid"
