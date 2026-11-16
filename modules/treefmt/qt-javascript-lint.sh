# shellcheck shell=bash
# Lints the JavaScript QML imports with biome, which reads only standard
# JavaScript. A file QML imports is a library whose every top-level name QML
# can reach, and it names what it imports with Qt's own `.pragma` and
# `.import` lines. Each file is read as the module that means the same: its
# imports as imports, its top-level names exported. Only the start of a line
# changes, and the copies keep the files' paths, so every finding points at
# the file's own line.
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
qt-javascript-lint  lint the JavaScript QML imports

  Each file is read as the module its .pragma and .import
  lines mean, and biome holds it to core's biome.jsonc.
  Each file is a path below the folder it's run in.
  nix fmt runs it on every .js file.

EXAMPLES
  qt-javascript-lint modules/quickshell/Fuzzy.js

EXIT
  0   every file is clean
  1   biome found something, and said where
EOF
  exit 0
  ;;
*) ;;
esac

copies=$(mktemp -d)
settings=$(mktemp -d)
trap 'rm -rf "$copies" "$settings"' EXIT
# Core's biome.jsonc, which the package names (qt-javascript-lint.nix), is
# read from a folder of its own, since biome takes the folder holding its
# settings for the project it indexes: read from the store, it indexed the
# whole store, 13 GB of memory in six minutes. The copies stay outside that
# project, so biome reads each alone and never resolves an .import, which
# names a file on QML's import path rather than one beside it.
cp "${QT_JAVASCRIPT_LINT_CONFIG:?names no biome.jsonc}" "$settings/biome.jsonc"

for file in "$@"; do
  mkdir -p "$copies/$(dirname "$file")"
  sed -E \
    -e 's|^\.pragma library$|// .pragma library|' \
    -e 's|^\.import "(\.[^"]+)" as ([A-Za-z_]+)$|import * as \2 from "\1";|' \
    -e 's|^\.import "([^"]+)" as ([A-Za-z_]+)$|import * as \2 from "./\1";|' \
    -e 's/^(function|const|let|var) /export \1 /' \
    "$file" > "$copies/$file"
done

cd "$copies" || exit
biome lint --error-on-warnings --max-diagnostics=none --config-path="$settings" .
