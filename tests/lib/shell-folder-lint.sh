# shellcheck shell=bash
# Lints the shell a check has laid out in the folder it's run in, as
# quickshell-passes-qmllint lints the account's: as the module qs, against
# Quickshell's type files (the first argument), and any warning fails. A
# check's own QML, its cases and stand-ins, is held to what the shell is.
# The same two kinds of report are left out, since each is a gap in
# Quickshell's files rather than the shell (checks.nix says which). Each
# message qmllint counts is printed, as its own report can leave one out
# (qmllint-counted.sh).
#
# usage: shell-folder-lint <Quickshell's types> <Qt's QML modules>
set -euo pipefail

types=$1
qt=$2
module=$(mktemp -d)
trap 'rm -rf "$module"' EXIT

# Quickshell finds a folder's files as a module by its path; qmllint needs
# a qmldir saying so, with each singleton named one.
mkdir "$module/qs"
find . -path ./runtime -prune -o \( -name '*.qml' -o -name '*.js' \) -print |
  while read -r file; do
    mkdir -p "$module/qs/$(dirname "$file")"
    cp "$file" "$module/qs/$file"
  done
find "$module/qs" -type d | while read -r dir; do
  {
    echo "module ${dir#"$module/"}" | tr / .
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

# Each file is named by its real, whole path, as Qt 6.12's qmllint names a
# qmldir's entries: named any other way, the file it lints and the qmldir's
# entry are two types to it, the first never told it's a singleton.
cd "$module"
here=$(pwd -P)
find "$here/qs" -name '*.qml' -print0 | sort -z |
  xargs -0 qmllint-counted --ignore-settings --max-warnings 0 \
    -I "$types" -I "$qt" -I . \
    --unresolved-type disable --signal-handler-parameters disable
