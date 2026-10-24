# shellcheck shell=bash
# Runs a service that follows the battery against stand-ins for UPower, niri
# and power-profiles-daemon (the scripts beside this), and checks it made
# exactly the calls expected, each after the battery's state it was made in.
# UPower answers each question whether the machine is on battery with the
# next of the steps given, "on-battery percent" a line, and says the
# battery changed once for each step after the first, then goes, which ends
# the service's monitor.
# usage: run.sh <service's command line> <steps> <expected calls> <what follows> <PATH>
set -euo pipefail

# The service's ExecStart, a program and its arguments.
read -ra service <<< "$1"
steps=$2
expected=$3
follows=$4
here=$(dirname "$0")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export STATE=$work

cp "$steps" "$work/steps"
chmod u+w "$work/steps"
mkdir "$work/bin"
for stand_in in busctl gdbus niri powerprofilesctl; do
  printf '#!%s\nexec %s %s "$@"\n' "$BASH" "$BASH" "$here/$stand_in.sh" > "$work/bin/$stand_in"
  chmod +x "$work/bin/$stand_in"
done
export PATH=$work/bin:$5

if "${service[@]}"; then
  echo "The service ended without its monitor failing." >&2
  exit 1
fi
if [ -e "$work/errors" ] || [ -s "$work/steps" ] || ! diff "$expected" "$work/calls"; then
  echo "$follows didn't follow the battery as listed." >&2
  exit 1
fi
