# shellcheck shell=bash
# Says when the generation booted isn't the newest because a newer one ran
# out of its $TRIES boots: in the journal as a warning, and in the file
# $FELL_BACK, which the session reads. systemd-boot keeps an entry's tries
# in its file name, so only bootctl, which reads the ESP, can say.
said=$(bootctl list --json=short | jq -r --argjson tries "$TRIES" -f "$FELL_BACK_SAID")
if [ -n "$said" ]; then
  echo "<4>$said"
  printf '%s\n' "$said" > "$FELL_BACK"
fi
