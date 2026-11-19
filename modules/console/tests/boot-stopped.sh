# shellcheck shell=bash
# Runs a failed boot's summary (modules/console/stopped/, as a machine
# builds it, through its wrapper) with stand-ins for systemctl and
# journalctl, and checks what it says: nothing when nothing failed; each
# failed unit with its description and the last three lines its own
# programs said that show something, past lines that show nothing; what
# systemd said of a unit whose programs said nothing, or nothing that
# shows; at most four units, and fewer where a terminal's rows leave no
# room; the account's theme and the console's marks;
# the way out; why a journal that couldn't be read gave a unit nothing;
# a file the journal skipped said once, above the way out;
# each unit described when the journal is slow; and each unit and the
# way out in time when the journal stalls. How it lays itself out at the
# console's width is the crate's own tests'.
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
# blank. With $WORK/stall there, it stalls, as a journal on a failing
# disk can; with $WORK/slow, it answers each read late; with
# $WORK/broken, it can't open the journal at all; with $WORK/skipping,
# it skips a file a crash left truncated, as every read would.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
# A stalled journal answers later than the summary waits for any read.
[[ ! -f $WORK/stall ]] || exec sleep 60
if [[ -f $WORK/broken ]]; then
  echo "Failed to open files: Input/output error" >&2
  exit 1
fi
if [[ -f $WORK/skipping ]]; then
  echo "Journal file /j/system@a.journal~ is truncated, ignoring file." >&2
fi
# A slow journal answers well inside a read's five seconds, but too
# slowly for every read to fit in the summary's ten.
[[ ! -f $WORK/slow ]] || sleep 2
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
  "     Failed to start nameless.service." "  x  blank.service  A program that drew nothing" \
  "     blank.service: Failed with result 'exit-code'."; do
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

# On a terminal of sixteen rows, which a console's LINES can say, it
# leaves eight for the emergency shell's own words, so it fits what
# matters most in the rest: a unit, the count of the others and the way
# out. util-linux's script gives it the terminal; into a pipe, as above,
# there's no height to fit.
LINES=16 SHELL=$BASH TERM=linux FORCE_COLOR=1 script -q -e \
  -c "$BASH $work/stopped < /dev/null" /dev/null < /dev/null > "$work/raw"
sed -e 's/\x1b\[[0-9;]*m//g' -e 's/\r$//' "$work/raw" > "$work/said"
rows=$(wc -l < "$work/said")
((rows <= 8)) || fails "it drew $rows rows where the console leaves it eight"
says "  x  home.mount  Mount /home"
grep -q -E '^     and [0-9]+ more$' "$work/said" || fails "it didn't count the units left out"
says "     Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation."

# A file the journal skipped is said once, for every unit, since any
# unit's lines may have been in it, and is no unit's reason: the units
# still show what the rest of the journal held, and nameless.service,
# with nothing in it here, shows nothing under its name.
touch "$work/skipping"
mv "$work/systemd/nameless.service" "$work/nameless"
said
mv "$work/nameless" "$work/systemd/nameless.service"
says "     mount: /home: wrong fs type"
under=$(grep -A 1 -x -F "  x  nameless.service" "$work/said" || true)
# The row after its name is blank, and $(...) drops a blank last row.
[[ $under == "  x  nameless.service" ]] ||
  fails "a skipped file was taken for nameless.service's reason"
at=0
for line in "     and 5 more" \
  "     journalctl skipped a journal file, so lines may be missing:" \
  "     Journal file /j/system@a.journal~ is truncated, ignoring file." \
  "     Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation."; do
  next=$(grep -n -x -F -- "$line" "$work/said" | cut -d: -f1 | head -n 1) ||
    fails "no line \"$line\""
  ((next > at)) || fails "\"$line\" isn't where it belongs"
  at=$next
done
rows=$(grep -c -F "ignoring file" "$work/said" || true)
[[ $rows == 1 ]] || fails "the skipped file was said other than once"
rm "$work/skipping"

# A journal that can't be opened says why, in its own words, under each
# unit it couldn't speak for.
touch "$work/broken"
said
says "  x  home.mount  Mount /home"
says "     its journal couldn't be read: Failed to open files: Input/output error"
rows=$(grep -c -F "its journal couldn't be read" "$work/said" || true)
[[ $rows == 4 ]] || fails "it didn't say why under each unit the journal couldn't speak for"
rm "$work/broken"

# A slow journal spends the time before the last unit is read, so each
# unit is described before any journal is read, and the last says it had
# no time, rather than that the journal stalled.
touch "$work/slow"
said
says "  x  home.mount  Mount /home"
says "     mount: /home: wrong fs type"
under=$(grep -A 1 -x -F "  x  blank.service  A program that drew nothing" "$work/said" || true)
[[ $under == *$'\n'"     its journal couldn't be read: no time was left to ask journalctl" ]] ||
  fails "blank.service wasn't described, or didn't say it had no time"
if grep -q -F "wasn't asked again" "$work/said"; then
  fails "a journal that was only slow was said to stall"
fi
rm "$work/slow"

# A journal that stalls is stopped and not asked again, so the summary
# still names each unit by what systemd calls it, and the way out, well
# before systemd gives up on the shell.
touch "$work/stall"
started=$SECONDS
said
((SECONDS - started < 8)) || fails "a stalled journal held it $((SECONDS - started)) s"
# It waits out the journal's five seconds once, so a journal that
# failed at once rather than stalling would show here.
((SECONDS - started >= 4)) || fails "the journal answered at once rather than stalling"
says "  x  home.mount  Mount /home"
says "  x  blank.service  A program that drew nothing"
says "     Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation."
says "     its journal couldn't be read: journalctl didn't answer in time, so it was stopped"
says "     its journal couldn't be read: journalctl didn't answer an earlier ask in time, so it \
wasn't asked again"
# Under the units only why each said nothing, the count of the rest and
# the way out: a journal that didn't answer gives a unit nothing to show.
rows=$(grep -c '^     ' "$work/said" || true)
[[ $rows == 6 ]] || fails "it showed lines a stalled journal can't give"
if grep -q -F "wrong fs type" "$work/said"; then
  fails "the journal didn't stall, so this shows nothing"
fi
