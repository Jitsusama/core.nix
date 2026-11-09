# shellcheck shell=bash
# Lints the JavaScript QML imports with biome, which reads only standard
# JavaScript. A file QML imports is a library whose every top-level name QML
# can reach, and it names what it imports with Qt's own `.pragma` and
# `.import` lines. Each file is read as the module that means the same: its
# imports as imports, its top-level names exported. Only the start of a line
# changes, and the copies keep the files' paths, so every finding points at
# the file's own line.
#
# usage: qt-javascript-lint <file.js>...   (paths below the folder it's run in)

config=$PWD/biome.jsonc
copies=$(mktemp -d)
trap 'rm -rf "$copies"' EXIT

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
biome lint --error-on-warnings --max-diagnostics=none --config-path="$config" .
