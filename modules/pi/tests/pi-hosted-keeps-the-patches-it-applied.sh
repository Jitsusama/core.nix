# shellcheck shell=bash
# Holds that a hosted pi keeps a copy of each patch it applied, every one of
# the folder's, reading as the patch does, and only those, at the root above
# the package directory a host starts with, where cadence reads them.
# Takes the folder of patches, the hosted pi's store path, and the package
# directory a host starts with, which has to sit beneath that root.
set -euo pipefail
patches=$1
hosted=$2
package=$3
kept=$hosted/share/pi-host/patches
case $package in
  "$hosted"/*) test -d "$package" ;;
  *)
    echo "the package directory $package isn't beneath $hosted" >&2
    exit 1
    ;;
esac
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
