# shellcheck shell=bash
# Checks the code site's writer against a stand-in for repos: while repos
# fails, ends well having said nothing, or stalls past the writer's wait,
# the writer ends well in its time, says so in
# a line and leaves the last homes as they were, with nothing of its own left
# beside them; once repos answers, its homes replace the last whole,
# readable by the server: each repository at Code's address for it, with or
# without an owner, and each tree still on the disk at its folder's, a gone
# one left out.
# usage: code-site-keeps-its-last-homes.sh <code-site.sh> <bin folders, joined by :>
set -euo pipefail

writer=$1
tools=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/site"

# The stand-in answers as repos --json does, or, while $work/down exists, as
# it does when its answer can't be written whole, half a record out first,
# or, while $work/silent exists, ends well having said nothing at all, or,
# while $work/stalled exists, half a record out, stalls as a reading starved
# on a machine under pressure does.
cat > "$work/bin/repos" << EOF
#!$BASH
if [ "\$*" != "--json" ]; then
  echo "repos: asked '\$*' rather than --json" >&2
  exit 64
fi
if [ -e "$work/down" ]; then
  printf '{"schema":"repos.repositories/1",'
  echo "repos: couldn't write its answer: broken pipe" >&2
  exit 1
fi
if [ -e "$work/silent" ]; then
  exit 0
fi
if [ -e "$work/stalled" ]; then
  printf '{"schema":"repos.repositories/1",'
  exec sleep 60
fi
cat << 'RECORD'
{"schema": "repos.repositories/1", "repositories": [
  {"key": "github:owl/core", "name": "core", "owner": "owl", "checkouts": [
    {"path": "/home/ada/src/core", "trees": [
      {"path": "/home/ada/trees/fresh", "present": true},
      {"path": "/home/ada/trees/lost", "present": false}
    ]}
  ]},
  {"key": "local:~/src/notes", "name": "notes", "checkouts": [{"path": "/home/ada/src/notes", "trees": []}]}
]}
RECORD
EOF
chmod +x "$work/bin/repos"
PATH="$work/bin:$tools"
export PATH

echo '{"kept":true}' > "$work/site/homes.json"
touch "$work/down"
if ! said=$("$BASH" "$writer" "$work/site"); then
  echo "With repos failing, the writer failed, so its unit would fail every minute." >&2
  exit 1
fi
homes=$(< "$work/site/homes.json")
if [ "$homes" != '{"kept":true}' ]; then
  echo "With repos failing, the writer changed the last homes to '$homes'." >&2
  exit 1
fi
if [ -z "$said" ] || [[ $said == *$'\n'* ]]; then
  echo "With repos failing, the writer said '$said' rather than one line." >&2
  exit 1
fi
shopt -s nullglob dotglob
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/homes.json" ]; then
  echo "The writer left ${left[*]} in the site's folder." >&2
  exit 1
fi

rm "$work/down"
touch "$work/silent"
"$BASH" "$writer" "$work/site" > /dev/null
homes=$(< "$work/site/homes.json")
if [ "$homes" != '{"kept":true}' ]; then
  echo "With repos saying nothing, the writer changed the last homes to '$homes'." >&2
  exit 1
fi

rm "$work/silent"
touch "$work/stalled"
# The writer waits a second here rather than its minute's share, so the check
# stays quick.
started=$SECONDS
if ! said=$(CODE_SITE_WAITS=1 "$BASH" "$writer" "$work/site"); then
  echo "With repos stalled, the writer failed, so its unit would fail." >&2
  exit 1
fi
if ((SECONDS - started > 10)); then
  echo "With repos stalled, the writer waited $((SECONDS - started)) s, not its second." >&2
  exit 1
fi
homes=$(< "$work/site/homes.json")
if [ "$homes" != '{"kept":true}' ]; then
  echo "With repos stalled, the writer changed the last homes to '$homes'." >&2
  exit 1
fi
if [[ $said != *"the last homes stay"* ]] || [[ $said == *$'\n'* ]]; then
  echo "With repos stalled, the writer said '$said' rather than one line." >&2
  exit 1
fi
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/homes.json" ]; then
  echo "With repos stalled, the writer left ${left[*]} in the site's folder." >&2
  exit 1
fi

rm "$work/stalled"
"$BASH" "$writer" "$work/site"
homes=$(jq -c '[.things[] | [.kind, .key, .names[0], .address]]' "$work/site/homes.json")
want='[["repository","github:owl/core","core","code/#repo=owl%2Fcore"],'
want+='["repository","local:~/src/notes","notes","code/#repo=notes"],'
want+='["tree","/home/ada/trees/fresh","/home/ada/trees/fresh","code/#tree=%2Fhome%2Fada%2Ftrees%2Ffresh"]]'
if [ "$homes" != "$want" ]; then
  echo "Once repos answered, the homes read $homes rather than $want." >&2
  exit 1
fi
mode=$(stat -c %a "$work/site/homes.json")
if [ "$mode" != 644 ]; then
  echo "The homes are mode $mode, which the server may not read." >&2
  exit 1
fi
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/homes.json" ]; then
  echo "Once repos answered, the writer left ${left[*]} in the site's folder." >&2
  exit 1
fi
