# shellcheck shell=bash
# Fails unless the Qt library a program loads, found through the program's own
# run path so it is the Qt the program was built on, still holds the name of
# an environment variable whole. A setting Qt stopped reading by that name
# would otherwise sit in a unit doing nothing, with nothing to say so.
# usage: qt-holds-its-setting.sh <program> <library> <variable>
set -euo pipefail

program=$1
library=$2
variable=$3

if ! path=$(patchelf --print-rpath "$program"); then
  echo "patchelf couldn't read $program's run path, so nothing says Qt holds $variable." >&2
  exit 1
fi
found=""
IFS=: read -ra folders <<<"$path"
for folder in "${folders[@]}"; do
  if [[ -e "$folder/$library" ]]; then
    found="$folder/$library"
    break
  fi
done
if [[ -z "$found" ]]; then
  echo "$program loads no $library through its run path, so nothing says Qt holds $variable." >&2
  exit 1
fi

# Whether the library holds a name as written, between the terminators the
# strings Qt compiles in sit between: held is 0 if it does and 1 if not, and
# the script fails on its own if grep can't search it at all.
search() {
  held=0
  grep -a -q -P "\\x00\\Q$1\\E\\x00" "$found" || held=$?
  if ((held > 1)); then
    echo "grep couldn't search $found for $1 (status $held)." >&2
    exit 1
  fi
}

search "$variable"
if ((held != 0)); then
  echo "$found no longer holds $variable whole: find where Qt reads it now, or" \
    "the setting does nothing." >&2
  exit 1
fi

# The name cut by a letter at either end is in the library only inside the
# whole one, so a search that found either would pass a renamed name too.
for cut in "${variable:0:${#variable}-1}" "${variable:1}"; do
  search "$cut"
  if ((held == 0)); then
    echo "The search found $cut on its own in $found, so finding $variable proves no" \
      "name whole." >&2
    exit 1
  fi
done
