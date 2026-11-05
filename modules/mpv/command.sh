# shellcheck shell=bash
# Carries a command from the launcher's command line to the mpv it was
# opened over, as a message to vi-keys.lua over mpv's socket: the command's
# whole name and the rest of what was typed.
# usage: mpv-command <mpv's socket> <command> [<the rest>]
set -euo pipefail

socket=$1
jq -cn --arg name "$2" --arg rest "${3:-}" \
  '{command: ["script-message-to", "vi_keys", "command", $name, $rest]}' \
  | socat -t 2 - "UNIX-CONNECT:$socket" > /dev/null
