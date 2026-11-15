# shellcheck shell=bash
# Checks the mycelium site's writer against a stand-in for mycelium: while
# no node answers, it ends well, says so in a line and leaves the last map as
# it was, with nothing of its own left beside it; once one answers, it writes
# a new map holding mycelium's networks, session cells and receptor cells as
# it drew them, each machine, and no message's words.
# usage: mycelium-map-keeps-its-last-map.sh <mycelium-map.sh> <filter> <jq> <coreutils bin>
set -euo pipefail

writer=$1
filter=$2
jq=$3
coreutils=$4
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/site"

# The stand-in answers as mycelium does, or, while $work/down exists, as it
# does when no node is there.
cat > "$work/bin/mycelium" << EOF
#!$BASH
if [ -e "$work/down" ]; then
  echo "mycelium: no node answers at /run/user/1000/mycelium/work.sock" >&2
  exit 1
fi
case "\$1" in
  networks)
    echo '{"kind":"network","name":"orchestration.y","shape":"orchestration","cells":["AAAAAA"]}'
    echo '{"kind":"network","name":"outside.slack","shape":"outside","cells":["slack@m"]}'
    echo '{"kind":"cell","type":"session","name":"AAAAAA","networks":["orchestration.y"]}'
    echo '{"kind":"cell","type":"machine","name":"m","networks":["machines"]}'
    echo '{"kind":"cell","type":"receptor","name":"slack@m","state":"reading","counts":{"hour":1,"today":4,"addressed":2},"networks":["outside.slack"]}' ;;
  machines) echo '{"Ok":{"Machines":[{"name":"m","state":"ready","cpu":{"cores":4},"room":{}}]}}' ;;
  messages) echo '{"Ok":{"Messages":[{"at":"now","from":"AAAAAA","intent":"fyi","groups":[],"text":"words"}]}}' ;;
esac
EOF
chmod +x "$work/bin/mycelium"
PATH="$work/bin:${jq%/*}:$coreutils"
export PATH

echo '{"kept":true}' > "$work/site/now.json"
touch "$work/down"
if ! said=$("$BASH" "$writer" "$filter" "$work/site"); then
  echo "With no node answering, the writer failed, so its unit would fail every 20 seconds." >&2
  exit 1
fi
map=$(< "$work/site/now.json")
if [ "$map" != '{"kept":true}' ]; then
  echo "With no node answering, the writer changed the last map." >&2
  exit 1
fi
if [ -z "$said" ] || [[ $said == *$'\n'* ]]; then
  echo "With no node answering, the writer said '$said' rather than one line." >&2
  exit 1
fi
shopt -s nullglob dotglob
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/now.json" ]; then
  echo "The writer left ${left[*]} in the site's folder." >&2
  exit 1
fi

rm "$work/down"
"$BASH" "$writer" "$filter" "$work/site"
drawn=$("$jq" -c '[.cells[].name, .networks[].name, .machines[].name]' "$work/site/now.json")
if [ "$drawn" != '["AAAAAA","slack@m","orchestration.y","outside.slack","m"]' ]; then
  echo "Once the node answered, the map drew $drawn." >&2
  exit 1
fi
if ! "$jq" -e '[.. | strings] | index("words") == null' "$work/site/now.json" > /dev/null; then
  echo "The map holds a message's words." >&2
  exit 1
fi
