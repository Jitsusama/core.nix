# shellcheck shell=bash
# Opens a document in zathura, in a compositor of its own, and reads back
# what it has open by the command the launcher reads zathura's Recent by.
# Run inside cage with a session bus, as `open-now-test.sh DOCUMENT
# COMMAND...`, DOCUMENT named from the folder it's in. It leaves ./passed
# once the command said zathura's process and the document's whole path,
# and then nothing once zathura quit, since cage ends 0 whatever the script
# inside it does.
set -eu

document=$1
shift
reading=("$@")
zathura "$document" > said 2>&1 &
zathura=$!

read_now() { "${reading[@]}" 2>&1 || true; }

# Waits up to two minutes for what the command reads to be `wanted`,
# reading it a tenth of a second apart, and says what it read when it never
# is: a busy host can take many seconds to open a document.
reads() {
  what=$1
  wanted=$2
  for _ in $(seq 1200); do
    if [ "$(read_now)" = "$wanted" ]; then return 0; fi
    sleep 0.1
  done
  echo "never read $what; it read: $(read_now)" >&2
  cat said >&2
  kill "$zathura" 2> /dev/null || true
  exit 1
}

reads "zathura's process and the document's whole path" \
  "$(printf '%s\t%s' "$zathura" "$PWD/$document")"
kill "$zathura"
wait "$zathura" || true
reads "nothing once zathura quit" ""
touch passed
