# shellcheck shell=bash
# Tells the session what the boot said in $FELL_BACK, once something shows
# notices: the account's manager starts before anything takes the name.
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
boot-fell-back-told  tell the session the boot fell back

  Waits for something to show notices, then shows what
  the boot said in the file FELL_BACK.

EXAMPLES
  FELL_BACK=/run/boot-fell-back boot-fell-back-told

EXIT
  0   told
  else gdbus's or notify-send's own
EOF
  exit 0
  ;;
*) ;;
esac
gdbus wait --session org.freedesktop.Notifications
notify-send --urgency=critical --app-name=Boot "Booted the last good generation" "$(< "$FELL_BACK")"
