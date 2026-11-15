# shellcheck shell=bash
# Checks the code site's writer against a stand-in for repos: while repos
# can't read the disk, the writer ends well, says so in a line and leaves the
# last record as it was, with nothing of its own left beside it; once repos
# answers, its record replaces the last one whole, readable by the server.
# usage: code-site-keeps-its-last-record.sh <code-site.sh> <coreutils bin>
set -euo pipefail

writer=$1
coreutils=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/site"

# The stand-in answers as repos --json does, or, while $work/down exists, as
# it does when a root can't be read, having written half a record first.
cat > "$work/bin/repos" << EOF
#!$BASH
if [ "\$*" != "--json" ]; then
  echo "repos: asked '\$*' rather than --json" >&2
  exit 64
fi
if [ -e "$work/down" ]; then
  printf '{"schema":"repos.repositories/1",'
  echo "repos: couldn't read /home/joel/src: permission denied" >&2
  exit 74
fi
echo '{"schema":"repos.repositories/1","repositories":[{"name":"core.nix"}]}'
EOF
chmod +x "$work/bin/repos"
PATH="$work/bin:$coreutils"
export PATH

echo '{"kept":true}' > "$work/site/repos.json"
touch "$work/down"
if ! said=$("$BASH" "$writer" "$work/site"); then
  echo "With repos failing, the writer failed, so its unit would fail every minute." >&2
  exit 1
fi
record=$(< "$work/site/repos.json")
if [ "$record" != '{"kept":true}' ]; then
  echo "With repos failing, the writer changed the last record to '$record'." >&2
  exit 1
fi
if [ -z "$said" ] || [[ $said == *$'\n'* ]]; then
  echo "With repos failing, the writer said '$said' rather than one line." >&2
  exit 1
fi
shopt -s nullglob dotglob
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/repos.json" ]; then
  echo "The writer left ${left[*]} in the site's folder." >&2
  exit 1
fi

rm "$work/down"
"$BASH" "$writer" "$work/site"
record=$(< "$work/site/repos.json")
if [ "$record" != '{"schema":"repos.repositories/1","repositories":[{"name":"core.nix"}]}' ]; then
  echo "Once repos answered, the record read '$record'." >&2
  exit 1
fi
mode=$(stat -c %a "$work/site/repos.json")
if [ "$mode" != 644 ]; then
  echo "The record is mode $mode, which the server may not read." >&2
  exit 1
fi
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/repos.json" ]; then
  echo "Once repos answered, the writer left ${left[*]} in the site's folder." >&2
  exit 1
fi
