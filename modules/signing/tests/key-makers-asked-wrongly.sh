# shellcheck shell=bash
# Checks that each key maker answers --help and refuses a word it doesn't
# take with exit 64, before it makes a key or asks a YubiKey for a touch:
# run with an empty home, nothing may appear in it.
# usage: key-makers-asked-wrongly.sh <a home> <key maker>...
set -euo pipefail

export HOME=$1
shift
mkdir -p "$HOME"

for maker in "$@"; do
  name=${maker##*/}
  help=$("$maker" --help) || { echo "$name --help failed" >&2; exit 1; }
  [[ $help == "$name "* ]] || { echo "$name --help didn't open with its name" >&2; exit 1; }
  for word in --halp make; do
    status=0
    "$maker" "$word" 2> /dev/null || status=$?
    [[ $status == 64 ]] || { echo "$name $word exited $status, not 64" >&2; exit 1; }
  done
done

made=$(ls -A "$HOME")
[[ -z $made ]] || { echo "Asked wrongly, a key maker made $made" >&2; exit 1; }
