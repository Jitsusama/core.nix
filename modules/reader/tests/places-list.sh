# shellcheck shell=bash
# What reader --places prints for the-launcher-reads-on-where-the-reader-left,
# newest left first: what a reader shows now, a file left under a heading
# with two marks, one under none, one read to its end two days ago, and a
# line it never would. Each run is counted in $HOME/listed, so the check can
# say the launcher asks once per opening. NOW is the check's moment.
set -euo pipefail

now=${NOW:?the moment the check runs at, in seconds}
listed=0
if [[ -f "$HOME/listed" ]]; then
  listed=$(< "$HOME/listed")
fi
echo $((listed + 1)) > "$HOME/listed"
place() {
  printf '{"file":"%s","left":%s,"at_end":%s%s,"marks":[%s]}\n' \
    "$HOME/notes/$1.md" "$2" "$3" "$4" "$5"
}
place now $((now - 60)) false ',"heading":"Now","through":0.1' ""
place guide $((now - 720)) false ',"heading":"Installing","through":0.4' \
  '{"letter":"b"},{"letter":"a","heading":"Using It","through":0.625}'
place old $((now - 3000)) false ',"through":0.25' ""
place 'done' $((now - 172800)) true ',"heading":"Last Words","through":1' ""
echo 'not a place'
