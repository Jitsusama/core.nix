# shellcheck shell=bash
# Lints the shell a check has laid out in the folder it's run in, as
# quickshell-passes-qmllint lints the account's: as the module qs, against
# Quickshell's type files (the first argument), and any warning fails. A
# check's own QML, its cases and stand-ins, is held to what the shell is.
# The same two kinds of report are left out, since each is a gap in
# Quickshell's files rather than the shell (checks.nix says which).
#
# usage: shell-folder-lint <Quickshell's types> <Qt's QML modules>

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

cd "$module" || exit
find qs -name '*.qml' -print0 | sort -z |
  xargs -0 qmllint --ignore-settings --max-warnings 0 \
    -I "$types" -I "$qt" -I . \
    --unresolved-type disable --signal-handler-parameters disable
