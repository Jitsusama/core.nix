# shellcheck shell=bash
# Says, as a notice, that a pi host was left down, so Joel hears of it the
# moment it happens rather than when a conversation goes quiet or cadence's
# evening read. host.nix runs it from each host's OnFailure=, which a unit
# that restarts on failure reaches only once it's done restarting: at its
# start limit, or on the exit it isn't restarted for. It takes the host's
# unit name without its suffix, as %N gives it, and says which host, why it
# was left down and how to bring it back, with its conversations still on
# its roster. The journal keeps the words too, for when no notice can show.
name="$1"
unit="$name.service"
result=$(systemctl --user show -p Result --value "$unit")
status=$(systemctl --user show -p ExecMainStatus --value "$unit")
# A host's unit is pi-host-<realm>-<kind>, and a realm may hold a hyphen.
kind="${name##*-}"
realm="${name#pi-host-}"
realm="${realm%-"$kind"}"
case "$result/$status" in
  start-limit-hit/*) why="It kept failing, so it was left down." ;;
  exit-code/78) why="It refused what it was given to run, so it waits for a fix." ;;
  *) why="It failed ($result) and wasn't started again." ;;
esac
who="The $realm realm's $kind pi host"
back="systemctl --user reset-failed $unit and systemctl --user start $unit"
body="$why Its conversations wait on its roster, and come back once $back start it."
body+=" journalctl --user -u $unit says what happened."
echo "$who is down. $body"
# With no shell to show it, as before a login, the journal's line above is
# all there is, which is no reason to count this as failed too.
notify-send --app-name=pi --urgency=critical "$who is down" "$body" ||
  echo "No notice could be shown."
