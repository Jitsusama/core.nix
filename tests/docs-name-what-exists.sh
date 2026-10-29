# shellcheck shell=bash
# Fails on a doc naming a file or folder that isn't there: a relative link or
# reference definition in any markdown file, read from the file's own folder,
# or a backticked path from one of the tree's top folders, read from the root
# or the file's folder. A mention that names something deliberately absent (an
# example, a path in another project's tree) is listed in the exemptions file
# as `file path`, and a stale exemption fails too, so the list only shrinks.
# usage: docs-name-what-exists.sh <tree> <exemptions, one `file path` a line>
set -euo pipefail

fail() {
  echo "$*" >&2
  exit 1
}

tree=$1
exempt=$2
[ -d "$tree" ] || fail "no tree at $tree"
[ -f "$exempt" ] || fail "no exemptions at $exempt"
cd "$tree"

missing=()
named=()

# The tree's own top folders, so a path is read as one only where the
# repository has a folder by that name, whichever repository it is.
tops=$(find . -mindepth 1 -maxdepth 1 -type d -not -name .git -printf '%f\n' |
  sed 's/\./\\./g' | sort | paste -sd '|')
[ -n "$tops" ] || fail "no folders in $tree"

# Whether a mention in a file is exempt, and if so, that it was seen.
exempted() {
  grep -qxF "$1 $2" "$exempt" || return 1
  named+=("$1 $2")
}

while IFS= read -r file; do
  file=${file#./}
  dir=$(dirname "$file")
  # Code blocks are examples, so only prose is read.
  prose=$(awk '/^(```|~~~)/ { fenced = !fenced; next } !fenced' "$file")
  while IFS= read -r target; do
    [ -n "$target" ] || continue
    [ -e "$dir/$target" ] || exempted "$file" "$target" || missing+=("$file: ($target)")
  done < <(
    {
      grep -oE '\]\([^)#: ]+' <<<"$prose" | sed 's/^](//' || true
      grep -E '^\[[^]]+\]: ' <<<"$prose" | awk '{ print $2 }' | sed 's/#.*//' || true
    } | grep -vE '^(https?|mailto):' || true
  )
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    path=${path%/}
    [ -e "$path" ] || [ -e "$dir/$path" ] || exempted "$file" "$path" || missing+=("$file: \`$path\`")
  done < <(
    # The backticks are markdown's, matched as they're written.
    # shellcheck disable=SC2016
    grep -oE '`('"$tops"')/[^` *<>{}]+`' <<<"$prose" |
      tr -d '`' | sed -E 's/:[0-9]+(-[0-9]+)?$//' || true
  )
done < <(find . -name '*.md' -not -path './.git/*' | sort)

stale=()
while IFS= read -r line; do
  [ -n "$line" ] || continue
  # Read from a file of its own rather than a pipe: grep -q stops at the
  # first match, and with pipefail the printf it cut off would fail the
  # pipe, so a mention that's there read as gone whenever the list was long.
  grep -qxF "$line" <(printf '%s\n' "${named[@]}") || stale+=("$line")
done <"$exempt"

if [ "${#missing[@]}" -gt 0 ]; then
  printf '%s\n' "These docs name what isn't there; name where it is now, or take it out:" \
    "${missing[@]/#/  }" >&2
fi
if [ "${#stale[@]}" -gt 0 ]; then
  printf '%s\n' "These exemptions name a mention that's gone; take them out of checks.nix:" \
    "${stale[@]/#/  }" >&2
fi
[ "${#missing[@]}" -eq 0 ] && [ "${#stale[@]}" -eq 0 ] || exit 1
