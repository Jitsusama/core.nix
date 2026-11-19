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
# (UNIT= from PID 1) from $WORK/systemd, each "message id|line", the last
# as many as asked for; given MESSAGE_ID=, only systemd's lines with one
# of those ids. --unit= gives both, as journalctl's does, with the unit's
# own last, so a summary that takes its last line there can be given a
# blank.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
lines=10 own='' systemd='' unit='' pid='' ids=' ' boot=''
for arg; do
  case $arg in
  --boot) boot=1 ;;
  --lines=*) lines=${arg#*=} ;;
  _SYSTEMD_UNIT=*) own=$WORK/own/${arg#*=} ;;
  --unit=*) systemd=$WORK/systemd/${arg#*=} own=$WORK/own/${arg#*=} ;;
  UNIT=*) unit=${arg#*=} ;;
  _PID=1) pid=1 ;;
  MESSAGE_ID=*) ids+="${arg#*=} " ;;
  esac
done
[[ -z $unit || -z $pid ]] || systemd=$WORK/systemd/$unit
# Every line it gives is this boot's, so a read without --boot, which a
# real journal answers with earlier boots' lines, is refused.
[[ -n $boot ]] || exit 1
{
  if [[ -f $systemd ]]; then
    while IFS='|' read -r id line; do
      [[ $ids != ' ' && $ids != *" $id "* ]] || printf '%s\n' "$line"
    done < "$systemd"
  fi
  [[ ! -f $own ]] || cat "$own"
} | tail -n "$lines"
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
# Its last seventeen lines draw nothing, so the three that say why are
# as far back as twenty reach, and the one before them past it: braille's
# blank and a direction mark, spelled in UTF-8's bytes, since bash's \u
# spells a character in the locale's encoding.
blank=$'\xe2\xa0\x80' mark=$'\xe2\x80\x8e'
printf '%s\n' "Mounting /home" "probing the disk" $'reading its label\a' \
  "mount: /home: wrong fs type" "$blank$blank" $'\e[0m' "$mark" > "$work/own/home.mount"
for _ in {1..14}; do echo "$mark"; done >> "$work/own/home.mount"
printf '%s\n' "$blank" $'\e[31m\e[0m' > "$work/own/blank.service"
# systemd's lines with the ids sd-messages.h gives a process's exit, a
# unit's result, what it cost and a job that failed. quiet.service
# restarts into its start limit, as systemd writes it, so its last two
# results come with no exit between.
exited=98e322203f7a4ed290d09fe03c09fe15 result=d9b373ed55a64feb8242e02dbe79a49c
cost=ae8f7b866b0347b9af31fe1c80b127c0 job=be02cf6855d2428ba40df7e9d022f03d
printf '%s\n' "$result|home.mount: Failed with result 'exit-code'." > "$work/systemd/home.mount"
printf '%s\n' "$result|blank.service: Failed with result 'exit-code'." \
  "$cost|blank.service: Consumed 4ms CPU time." > "$work/systemd/blank.service"
printf '%s\n' "$exited|quiet.service: Main process exited, code=exited, status=1/FAILURE" \
  "$result|quiet.service: Failed with result 'exit-code'." \
  "|quiet.service: Scheduled restart job, restart counter is at 5." \
  "$exited|quiet.service: Main process exited, code=killed, status=9/KILL" \
  "$result|quiet.service: Failed with result 'signal'." \
  "$cost|quiet.service: Consumed 9ms CPU time." \
  "|quiet.service: Start request repeated too quickly." \
  "$result|quiet.service: Failed with result 'start-limit-hit'." > "$work/systemd/quiet.service"
# nameless.service's only line carries neither id, so the summary falls
# back to systemd's last line of the unit.
printf '%s\n' "$job|Failed to start nameless.service." > "$work/systemd/nameless.service"
said
says "  x  home.mount  Mount /home"
says "     probing the disk"
says "     reading its label"
says "     mount: /home: wrong fs type"
grep -q -F "Mounting /home" "$work/said" && fails "it said more than the last three lines"
grep -q -F "home.mount: Failed" "$work/said" && fails "it said systemd's line past the unit's own"
says "  x  quiet.service  A program that said nothing"
says "     quiet.service: Main process exited, code=killed, status=9/KILL"
says "     quiet.service: Failed with result 'start-limit-hit'."
grep -q -F "Consumed" "$work/said" && fails "it said what a unit cost, not why it failed"
grep -q -F -e "Scheduled restart" -e "'signal'" -e "status=1/" -e "repeated too quickly" \
  "$work/said" && fails "it said more than why systemd last failed it"
says "  x  nameless.service"
says "     Failed to start nameless.service."
# Each line under its own unit, and a process's exit before its result.
at=0
for line in "  x  home.mount  Mount /home" "     mount: /home: wrong fs type" \
  "  x  quiet.service  A program that said nothing" \
  "     quiet.service: Main process exited, code=killed, status=9/KILL" \
  "     quiet.service: Failed with result 'start-limit-hit'." "  x  nameless.service" \
  "     Failed to start nameless.service." "  x  blank.service  A program that drew nothing"; do
  next=$(grep -n -x -F -- "$line" "$work/said" | cut -d: -f1 | head -n 1) ||
    fails "no line \"$line\""
  ((next > at)) || fails "\"$line\" isn't where it belongs"
  at=$next
done
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
