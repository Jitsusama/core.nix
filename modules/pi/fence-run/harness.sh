# shellcheck shell=bash
# Lays the harness's source out with yaml beside it, where node finds a
# package for a file by its real path, refusing a harness whose lock no
# longer locks the yaml given, so the two can't drift apart unseen.
# Usage: harness.sh HARNESS YAML_TARBALL YAML_VERSION OUT
set -o errexit
set -o nounset
set -o pipefail

harness=$1
yaml=$2
version=$3
out=$4

# The lock names the harness's own version of each package only in its
# importers, as a `version:` line two below the package's name.
if ! sed -n '/^importers:/,/^packages:/p' "$harness/pnpm-lock.yaml" |
  grep -A2 -x '      yaml:' | grep -qx "        version: $version"; then
  echo "the harness's lock no longer locks yaml $version" >&2
  exit 1
fi
cp -r "$harness" "$out"
chmod -R u+w "$out"
mkdir -p "$out/node_modules/yaml"
tar -xzf "$yaml" -C "$out/node_modules/yaml" --strip-components=1
