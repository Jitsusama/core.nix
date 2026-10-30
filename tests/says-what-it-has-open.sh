# shellcheck shell=bash
# Opens a file in a program, in a compositor of its own, and reads back what
# the program has open by the command its Recent in the launcher is led by
# (a file source's `opened`, ../modules/quickshell/recents.nix). Run inside
# cage, with a session bus where the program wants one and wtype on the
# path, as `says-what-it-has-open.sh FILE PROGRAM... -- COMMAND...`, FILE
# named from the folder it's in. It leaves ./passed once the command said
# the program's process and the file's whole path, and then nothing once q
# closed it, since cage ends 0 whatever the script inside it does.
set -eu

file=$1
shift
program=()
while [ "$1" != -- ]; do
  program+=("$1")
  shift
done
shift
reading=("$@")

"${program[@]}" "$file" > said 2>&1 &
opened=$!

read_now() { "${reading[@]}" 2>&1 || true; }

# Waits up to two minutes for what the command reads to be `wanted`,
# reading it a tenth of a second apart and doing what's left of the words
# before each reading, and says what it read when it never is: a busy host
# can take many seconds to open a file.
reads() {
  what=$1
  wanted=$2
  shift 2
  for _ in $(seq 1200); do
    "$@"
    if [ "$(read_now)" = "$wanted" ]; then return 0; fi
    sleep 0.1
  done
  echo "never read $what; it read: $(read_now)" >&2
  cat said >&2
  kill "$opened" 2> /dev/null || true
  exit 1
}

reads "the program's process and the file's whole path" \
  "$(printf '%s\t%s' "$opened" "$PWD/$file")" true
# The program is open before its window has the keyboard, and a key sent
# before then goes nowhere, so q is pressed until it's closed.
reads "nothing once q closed it" "" wtype q
wait "$opened" || true
touch passed
