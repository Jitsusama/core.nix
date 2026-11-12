# shellcheck shell=bash
# Starts the shell in compiled-once/ three times, and checks Qt compiles its
# own files once and keeps them (qtdeclarative-caches-what-is-served.patch):
# the first start leaves a compiled unit for each of its three files in the
# disk cache, the second compiles none of them again, and the third, after
# Thing.qml has changed, compiles that file alone. Quickshell serves the
# shell's folder on its own qs: scheme, which stock Qt never caches, so
# without the patch the first start leaves nothing.
# usage: compiled-once.sh <qs> <shell folder, writable>
set -euo pipefail

qs=$1
shell=$2

start() {
  if ! timeout 60 "$qs" -p "$shell"; then
    echo "the shell's $1 start failed" >&2
    exit 1
  fi
}

# What Qt keeps, a line each: its inode, then its name, which is the hash of
# where it was compiled from.
kept() {
  find "$HOME/.cache" \( -name '*.qmlc' -o -name '*.jsc' \) -printf '%i %f\n' | sort
}

start first
first=$(kept)
if [ "$(wc -l <<< "$first")" -lt 3 ] || [ -z "$first" ]; then
  echo "the first start kept no compiled unit for each of the shell's three files:" >&2
  echo "${first:-nothing}" >&2
  exit 1
fi

start second
second=$(kept)
if [ "$second" != "$first" ]; then
  echo "the second start compiled the shell's files again:" >&2
  diff <(echo "$first") <(echo "$second") >&2 || true
  exit 1
fi

echo "// Changed, so it is compiled again." >> "$shell/Thing.qml"
start third
third=$(kept)
compiled=$(comm -13 <(echo "$first") <(echo "$third") | wc -l)
if [ "$compiled" -ne 1 ]; then
  echo "the third start compiled $compiled files again, not Thing.qml alone:" >&2
  diff <(echo "$first") <(echo "$third") >&2 || true
  exit 1
fi
