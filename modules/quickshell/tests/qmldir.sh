# shellcheck shell=bash
# Writes a qmldir in each folder under the one given, naming the folder's
# QML types, singletons as such, since Quickshell finds a folder's files as
# a module by its path, and qmllint needs a qmldir to say so. A module is
# named for its path, its slashes as dots.
# usage: qmldir.sh <folder>
set -euo pipefail

find "$1" -type d | while read -r dir; do
  {
    echo "module ${dir//\//.}"
    for file in "$dir"/[A-Z]*.qml; do
      [ -e "$file" ] || continue
      name=$(basename "$file" .qml)
      if grep -q '^pragma Singleton' "$file"; then
        echo "singleton $name 1.0 $name.qml"
      else
        echo "$name 1.0 $name.qml"
      fi
    done
  } > "$dir/qmldir"
done
