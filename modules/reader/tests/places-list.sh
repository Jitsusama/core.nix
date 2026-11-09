# shellcheck shell=bash
# What reader --places prints for the-launcher-reads-on-where-the-reader-left,
# newest left first: what a reader shows now, a file left under a heading
# with two marks, one under none, one read to its end two days ago, and a
# line it never would. Each run is counted in $HOME/listed, so the check can
# say the launcher asks once per opening. $NOW is the check's moment.
set -euo pipefail

echo $(($(cat "$HOME/listed" 2> /dev/null || echo 0) + 1)) > "$HOME/listed"
place() {
  printf '{"file":"%s","left":%s,"at_end":%s%s,"marks":[%s]}\n' \
    "$HOME/notes/$1.md" "$2" "$3" "$4" "$5"
}
place now $((NOW - 60)) false ',"heading":"Now","through":0.1' ""
place guide $((NOW - 720)) false ',"heading":"Installing","through":0.4' \
  '{"letter":"b"},{"letter":"a","heading":"Using It","through":0.625}'
place old $((NOW - 3000)) false ',"through":0.25' ""
place 'done' $((NOW - 172800)) true ',"heading":"Last Words","through":1' ""
echo 'not a place'
