# shellcheck shell=bash
# Holds that a hosted pi keeps a copy of each patch it applied, every one of
# the folder's, reading as the patch does, and only those, at the root above
# the package directory a host starts with, where cadence reads them.
# Takes the folder of patches and the hosted pi's store path.
set -euo pipefail
patches=$1
hosted=$2
kept=$hosted/share/pi-host/patches
test -d "$hosted/lib/node_modules/pi-monorepo"
count=0
for patch in "$patches"/*.patch; do
  cmp "$patch" "$kept/$(basename "$patch")"
  count=$((count + 1))
done
kept_count=$(find "$kept" -mindepth 1 -maxdepth 1 | wc -l)
if [ "$kept_count" != "$count" ]; then
  echo "the runtime keeps $kept_count copies, but applied $count patches" >&2
  exit 1
fi
