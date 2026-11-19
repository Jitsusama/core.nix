# shellcheck shell=bash
# Runs a failed boot's summary (modules/console/stopped/, as a machine
# builds it, through its wrapper) with stand-ins for systemctl and
# journalctl, and checks what it says: nothing when nothing failed; each
# failed unit with its description and the last three lines its own
# programs said that show something, past lines that show nothing; what
# systemd said of a unit whose programs said nothing, or nothing that
# shows; at most four units; the account's theme and the console's marks;
# and the way out. How it lays itself out at the console's width is the
# crate's own tests'.
# usage: boot-stopped.sh <boot-stopped>
set -euo pipefail

work=$(mktemp -d)
export WORK=$work
mkdir "$work/bin" "$work/own" "$work/systemd"
: > "$work/failed"

# systemctl lists the units in $WORK/failed, one "unit|description" a line,
# and shows each one's description; a unit with none is described by name.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
case $1 in
list-units)
  while IFS='|' read -r unit _; do echo "$unit loaded failed failed"; done < "$WORK/failed"
  ;;
show)
  while IFS='|' read -r unit about; do
    [[ $unit != "$5" ]] || echo "${about:-$unit}"
  done < "$WORK/failed"
  ;;
esac
EOF
} > "$work/bin/systemctl"

# journalctl gives a unit's own lines from $WORK/own and systemd's of it
# (UNIT= from PID 1) from $WORK/systemd, the last as many as asked for.
# --unit= gives both, as journalctl's does, with the unit's own last, so a
# summary that takes its last line there can be given a blank.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
lines=10 from=() unit='' pid=''
for arg; do
  case $arg in
  --lines=*) lines=${arg#*=} ;;
  _SYSTEMD_UNIT=*) from=("$WORK/own/${arg#*=}") ;;
  --unit=*) from=("$WORK/systemd/${arg#*=}" "$WORK/own/${arg#*=}") ;;
  UNIT=*) unit=${arg#*=} ;;
  _PID=1) pid=1 ;;
  esac
done
[[ -z $unit || -z $pid ]] || from=("$WORK/systemd/$unit")
((${#from[@]})) && cat "${from[@]}" 2> /dev/null | tail -n "$lines"
exit 0
EOF
} > "$work/bin/journalctl"
chmod +x "$work/bin/systemctl" "$work/bin/journalctl"

# The summary with the stand-ins ahead of what its wrapper names on its
# PATH, inked as the console inks it.
sed "s|^export PATH=\"|export PATH=\"$work/bin:|" "$1" > "$work/stopped"
said() {
  TERM=linux FORCE_COLOR=1 "$BASH" "$work/stopped" < /dev/null > "$work/raw"
  sed 's/\x1b\[[0-9;]*m//g' "$work/raw" > "$work/said"
}
fails() {
  echo "a failed boot's summary: $1" >&2
  cat "$work/said" >&2
  exit 1
}
says() {
  grep -q -x -F -- "$1" "$work/said" || fails "no line \"$1\""
}

said
[[ ! -s $work/said ]] || fails "it said something with nothing failed"

cat > "$work/failed" <<'EOF'
home.mount|Mount /home
quiet.service|A program that said nothing
nameless.service|
blank.service|A program that drew nothing
EOF
# Its last lines draw nothing, so the three that say why are further back:
# braille's blank and a direction mark, spelled in UTF-8's bytes, since
# bash's \u spells a character in the locale's encoding.
blank=$'\xe2\xa0\x80' mark=$'\xe2\x80\x8e'
printf '%s\n' "Mounting /home" "probing the disk" $'reading its label\a' \
  "mount: /home: wrong fs type" "$blank$blank" $'\e[0m' "$mark" > "$work/own/home.mount"
printf '%s\n' "$blank" $'\e[31m\e[0m' > "$work/own/blank.service"
echo "blank.service: Failed with result 'exit-code'." > "$work/systemd/blank.service"
echo "quiet.service: Main process exited, code=killed, status=9/KILL" > "$work/systemd/quiet.service"
echo "quiet.service: Failed with result 'signal'." >> "$work/systemd/quiet.service"
said
says "  x  home.mount  Mount /home"
says "     probing the disk"
says "     reading its label"
says "     mount: /home: wrong fs type"
grep -q -F "Mounting /home" "$work/said" && fails "it said more than the last three lines"
says "  x  quiet.service  A program that said nothing"
says "     quiet.service: Failed with result 'signal'."
grep -q -F "status=9/KILL" "$work/said" && fails "it said more than systemd's last line"
says "  x  nameless.service"
says "  x  blank.service  A program that drew nothing"
says "     blank.service: Failed with result 'exit-code'."
says "     Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation."
grep -q -F $'\e[31mx\e[0m' "$work/raw" || fails "the mark isn't in the console's red"
# Without a theme the kit draws its sixteen own colours wherever it draws,
# so only a theme's colours give a terminal of millions a colour by its
# channels: that shows the wrapper pointed it at the account's theme.
TERM=xterm-256color COLORTERM=truecolor FORCE_COLOR=1 "$BASH" "$work/stopped" \
  < /dev/null > "$work/inked"
grep -q $'\e\\[38;2;' "$work/inked" || fails "it didn't read the account's theme"
grep -q $'\a' "$work/raw" && fails "it passed on a control character"

for unit in a b c d e; do echo "$unit.service|$unit" >> "$work/failed"; done
said
[[ $(grep -c '^  x  ' "$work/said") == 4 ]] || fails "it listed other than four units"
says "     and 5 more"
