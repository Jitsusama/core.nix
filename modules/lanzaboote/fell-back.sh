# shellcheck shell=bash
# Says when the generation booted isn't the newest because a newer one ran
# out of its $TRIES boots: in the journal as a warning, and in the file
# $FELL_BACK, which the session reads. systemd-boot keeps an entry's tries
# in its file name, so only bootctl, which reads the ESP, can say. Its help
# takes the shape every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
boot-fell-back  say if the boot fell back a generation

  When the generation booted isn't the newest, because a
  newer one ran out of its TRIES boots, it says so in the
  journal as a warning and in the file FELL_BACK, which
  the session reads. Every boot runs it.

EXAMPLES
  boot-fell-back

EXIT
  0   said, or there was nothing to say
  else bootctl's or jq's own
EOF
  exit 0
  ;;
*) ;;
esac
said=$(bootctl list --json=short | jq -r --argjson tries "$TRIES" -f "$FELL_BACK_SAID")
if [ -n "$said" ]; then
  echo "<4>$said"
  printf '%s\n' "$said" > "$FELL_BACK"
fi
