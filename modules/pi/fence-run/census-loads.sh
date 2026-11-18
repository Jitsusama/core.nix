# shellcheck shell=bash
# Shows that pi-fence's census, as core builds pi-fence, loads and judges
# a machine: here one with no sessions, which it counts and, with nothing
# proven fenced, says doesn't hold (exit 1). It wants that verdict whole on
# stdout and nothing on stderr, since a census that can't load exits 126
# and one that throws after its first line exits 1 too, both saying why on
# stderr. Usage: census-loads.sh PI_FENCE
set -o errexit
set -o nounset
set -o pipefail

fence=$1
home=$(mktemp -d)
proc=$(mktemp -d)
errors=$(mktemp)
status=0
said=$(HOME=$home "$fence" census --proc "$proc" 2>"$errors") || status=$?
verdict="0 live sessions: 0 proven fenced, 0 claimed, 0 unfenced"
if [ "$status" -ne 1 ] || [ "$said" != "$verdict" ] || [ -s "$errors" ]; then
  printf 'pi-fence census exited %s, saying:\n%s\nand on stderr:\n' "$status" "$said" >&2
  cat "$errors" >&2
  exit 1
fi
