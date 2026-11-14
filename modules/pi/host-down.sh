# shellcheck shell=bash
# Says, as a notice, that a pi host was left down, so Joel hears of it the
# moment it happens rather than when a conversation goes quiet or cadence's
# evening read. host.nix runs it from each host's OnFailure=. A crash the
# unit restarts itself from never reaches it, but a restart asked of systemd
# whose stop exits failing does: the unit fails, then starts at once. So it
# says nothing but a journal line while the unit starts again or has a start
# waiting. It takes the host's unit name without its suffix, as %N gives it,
# and says which host, why it was left down and how to bring it back, with
# its conversations still on its roster. The journal keeps the words too,
# for when no notice can show. Its help and failures take the shape every
# command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
pi-host-down  say a pi host was left down

  Each host's OnFailure= runs it with the host's unit
  name. Unless the host is starting again, a notice says
  which host is down, why, and how to bring it back.

EXAMPLES
  pi-host-down pi-host-personal-attended

EXIT
  0   told, or it's starting again
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "pi-host-down takes the host's unit name" \
    "  try  pi-host-down --help" >&2
  exit 64
fi

name="$1"
unit="$name.service"
# systemd hands an OnFailure= unit what the failure was. By the time this
# runs, a unit that started again already says success, so the unit's own
# properties are read only when systemd handed nothing.
result="${MONITOR_SERVICE_RESULT:-$(systemctl --user show -p Result --value "$unit")}"
status="${MONITOR_EXIT_STATUS:-$(systemctl --user show -p ExecMainStatus --value "$unit")}"
# A host's unit is pi-host-<realm>-<kind>, and a realm may hold a hyphen.
kind="${name##*-}"
realm="${name#pi-host-}"
realm="${realm%-"$kind"}"
who="The $realm realm's $kind pi host"
state=$(systemctl --user show -p ActiveState --value "$unit")
waiting=$(systemctl --user list-jobs --no-legend "$unit")
case "$state" in
  activating | active | reloading) back=yes ;;
  *) back="${waiting:+yes}" ;;
esac
if [ -n "$back" ]; then
  echo "$who failed ($result), and is starting again, so nobody is told it's down."
  exit 0
fi
case "$result/$status" in
  start-limit-hit/*) why="It kept failing, so it was left down." ;;
  exit-code/78) why="It refused what it was given to run, so it waits for a fix." ;;
  *) why="It failed ($result) and wasn't started again." ;;
esac
back="systemctl --user reset-failed $unit and systemctl --user start $unit"
body="$why Its conversations wait on its roster, and come back once $back start it."
body+=" journalctl --user -u $unit says what happened."
echo "$who is down. $body"
# With no shell to show it, as before a login, the journal's line above is
# all there is, which is no reason to count this as failed too.
notify-send --app-name=pi --urgency=critical "$who is down" "$body" ||
  echo "No notice could be shown."
