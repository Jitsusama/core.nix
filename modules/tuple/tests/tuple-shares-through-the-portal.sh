#!/usr/bin/env bash
# Runs the switch's own steps that write Tuple's screen-sharing setting,
# from the home's activation script given first, with the data folder given
# second moved into the check's build, then has Tuple read the setting back.
# Tuple ignores a folder it doesn't know and a value not written as it
# writes one, so only its own reading shows the switch wrote it rightly.
set -euo pipefail

activate=$1
data=$2
export HOME=$PWD XDG_DATA_HOME=$PWD/data XDG_RUNTIME_DIR=$PWD/run
mkdir -m 700 run
# Matched as plain text, so no character of the folder's name is read as a
# pattern.
while IFS= read -r line; do
  if [[ $line == "run "* && $line == *"$data/tuple/"* ]]; then
    step=${line#run }
    printf '%s\n' "${step//"$data"/$PWD/data}"
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
timeout -k 5 10 tuple stop > /dev/null 2>&1 || true
if [[ $ended -eq 124 || $ended -eq 137 ]]; then
  echo "tuple settings was cut off (status $ended); it said: $(< said)" >&2
  exit 1
fi
if ! grep -q '^capture | portal |' said; then
  echo "Tuple doesn't read back the portal for capture; it said: $(< said)" >&2
  exit 1
fi
