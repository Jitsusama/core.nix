# shellcheck shell=bash
# Compiles every QML and JavaScript file under the folder given with Qt's own
# compiler, the one the shell compiles each file with as it loads it, and
# fails naming each file it refused or crashed on. qmllint and a file run
# offscreen can both pass a file this compiler segfaults on, as Qt 6.11's
# does on an optional call through a subscript.
# usage: qml-compiles.sh <qmlcachegen> <folder>
set -euo pipefail

compiler=$1
refused=0
while read -r file; do
  if ! "$compiler" --only-bytecode --resource-path "/$file" "$file" -o compiled.cpp; then
    echo "Qt's compiler couldn't compile $file" >&2
    refused=$((refused + 1))
  fi
done < <(find "$2" \( -name '*.qml' -o -name '*.js' -o -name '*.mjs' \) | sort)

if [ "$refused" -gt 0 ]; then
  echo "$refused of the shell's files didn't compile" >&2
  exit 1
fi
