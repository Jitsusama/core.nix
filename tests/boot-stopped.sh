# Runs a failed boot's summary (modules/console/stopped.sh, as a machine
# builds it) with stand-ins for systemctl and journalctl, and checks what it
# says: nothing when nothing failed; each failed unit with its description
# and the last its own programs said, wrapped between words; what systemd
# said of a unit whose programs said nothing; at most four units; and the
# way out.
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

# journalctl gives a unit's own lines from $WORK/own and systemd's of it from
# $WORK/systemd, the last as many as asked for.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
lines=10
for arg; do
  case $arg in
  --lines=*) lines=${arg#*=} ;;
  _SYSTEMD_UNIT=*) from=$WORK/own/${arg#*=} ;;
  --unit=*) from=$WORK/systemd/${arg#*=} ;;
  esac
done
[[ -f $from ]] && tail -n "$lines" "$from"
exit 0
EOF
} > "$work/bin/journalctl"
chmod +x "$work/bin/systemctl" "$work/bin/journalctl"

# The summary with the stand-ins ahead of what it names on its PATH.
sed "s|^export PATH=\"|export PATH=\"$work/bin:|" "$1" > "$work/stopped"
said() {
  "$BASH" "$work/stopped" < /dev/null > "$work/raw"
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
long.service|A description far too long to sit beside its unit's name on a console eighty wide
EOF
printf '%s\n' "Mounting /home" "probing the disk" $'reading its label\a' \
  "mount: /home: wrong fs type, bad option, bad superblock on /dev/mapper/data, missing codepage or helper program, or other error." \
  > "$work/own/home.mount"
echo "quiet.service: Main process exited, code=killed, status=9/KILL" > "$work/systemd/quiet.service"
echo "quiet.service: Failed with result 'signal'." >> "$work/systemd/quiet.service"
said
says "  x  home.mount  Mount /home"
says "     probing the disk"
says "     reading its label"
grep -q -F "Mounting /home" "$work/said" && fails "it said more than the last three lines"
says "     mount: /home: wrong fs type, bad option, bad superblock on"
says "     /dev/mapper/data, missing codepage or helper program, or other error."
says "  x  quiet.service  A program that said nothing"
says "     quiet.service: Failed with result 'signal'."
says "  x  nameless.service"
says "  x  long.service"
says "     Ctrl+Alt+Delete restarts; hold Space as it starts for an older generation."
awk 'length > 80 { exit 1 }' "$work/said" || fails "a line is wider than the console"
grep -q ' $' "$work/said" && fails "a line ends in a space"
grep -q -F $'\e[31mx\e[0m' "$work/raw" || fails "the mark isn't in the alert colour"
grep -q $'\a' "$work/raw" && fails "it passed on a control character"

for unit in a b c d e; do echo "$unit.service|$unit" >> "$work/failed"; done
said
[[ $(grep -c '^  x  ' "$work/said") == 4 ]] || fails "it listed other than four units"
says "     and 5 more"
