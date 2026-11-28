#!/usr/bin/env bash
# Runs the switch's own steps that write Tuple's screen-sharing setting,
# from the home's activation script given second, with the data folder given
# third moved into the folder of its own that
# tuple-in-a-folder-of-its-own.sh, given first, makes, then has Tuple read
# the setting back.
# Tuple ignores a folder it doesn't know and a value not written as it
# writes one, so only its own reading shows the switch wrote it rightly.
set -euo pipefail

# A whole path, since the folder of its own becomes the current one; the
# data folder is only text to find in its steps.
activate=$(realpath -- "$2")
data=$3
# An empty one would match every step's folder and move none of them, so
# the steps would write into the person's own Tuple folder.
if [[ -z $data ]]; then
  echo "the data folder to find in the steps is empty" >&2
  exit 64
fi
# The library is handed in by its store path; shellcheck reads it from here.
# shellcheck source=SCRIPTDIR/tuple-in-a-folder-of-its-own.sh
source "$1"
# Matched and put back as plain text, so no character of either folder's
# name is read as a pattern.
while IFS= read -r line; do
  if [[ $line == "run "* && $line == *"$data/tuple/"* ]]; then
    step=${line#run }
    printf '%s\n' "${step//"$data"/"$XDG_DATA_HOME"}"
  fi
done < "$activate" > steps
if [[ ! -s steps ]]; then
  echo "the activation script has no step writing into $data/tuple/" >&2
  exit 1
fi
sh -e steps

# Any tuple command may start Tuple's daemon, so each is held to a time, as
# tuple-ends-its-flags.sh says: settings reads Tuple's own files, so it's
# given room, and a cut-off says so rather than reading as a setting unread.
ended=0
timeout -k 5 30 tuple settings > said 2>&1 < /dev/null || ended=$?
if [[ $ended -eq 124 || $ended -eq 137 ]]; then
  echo "tuple settings was cut off (status $ended); it said: $(< said)" >&2
  exit 1
fi
if ! grep -q '^capture | portal |' said; then
  echo "Tuple doesn't read back the portal for capture; it said: $(< said)" >&2
  exit 1
fi
