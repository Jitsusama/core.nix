# shellcheck shell=bash
# Fails when a shell file imports a file or folder by its path that isn't
# laid out beside it, as when a script is added to the source but not to
# home.nix's list of what's installed. Quickshell only warns and leaves the
# component that needs it unmade, and qmllint doesn't count it among its
# warnings, so only a VM test found one, after 25 minutes (2026-10-30).
# usage: imports-installed.sh <folder>
set -euo pipefail

missing=0
while IFS= read -r -d '' file; do
  dir=$(dirname "$file")
  while IFS= read -r path; do
    if [ ! -e "$dir/$path" ]; then
      echo "${file#"$1"/} imports $path, which isn't installed beside it"
      missing=1
    fi
  done < <(sed -n -E 's/^[[:space:]]*\.?import[[:space:]]+"([^"]+)".*/\1/p' "$file")
done < <(find "$1" \( -name '*.qml' -o -name '*.js' \) -print0)
exit "$missing"
