# shellcheck shell=bash disable=SC2154 # The check's session sets XDG_RUNTIME_DIR (people.nix).
# busctl, for the shell's checks that close notices without a notices
# daemon: each call is kept a line each in $XDG_RUNTIME_DIR/closed, and a
# call's last word, a CloseNotification's id, is written as
# $XDG_RUNTIME_DIR/closed-<id>, which closes that notice for the
# notify-send stand-in waiting on it (notify-send-stand-in.sh).

printf '%s\n' "$*" >>"$XDG_RUNTIME_DIR/closed"
for word in "$@"; do
  last=$word
done
touch "$XDG_RUNTIME_DIR/closed-${last:-}"
