# shellcheck shell=bash
# Carries a command from the launcher's command line to the mpv it was
# opened over, as a message to vi-keys.lua over mpv's socket: the command's
# whole name and the rest of what was typed. Its help and failures take the
# shape every command of ours does (docs/commands.md), by hand.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
mpv-command  carry a command to the mpv it's for

  The launcher's command line runs it over a video, with
  mpv's socket, the command's whole name and the rest of
  what was typed.

EXAMPLES
  mpv-command /run/user/1000/mpv/4211 goto 12:30

EXIT
  0   carried
  1   mpv wasn't there to take it
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
  printf '%s\n' "mpv-command takes mpv's socket and a command" \
    "  try  mpv-command --help" >&2
  exit 64
fi

socket=$1
jq -cn --arg name "$2" --arg rest "${3:-}" \
  '{command: ["script-message-to", "vi_keys", "command", $name, $rest]}' \
  | socat -t 2 - "UNIX-CONNECT:$socket" > /dev/null
