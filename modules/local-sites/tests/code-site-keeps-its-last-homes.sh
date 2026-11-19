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
# ending with the status $work/down holds, 1 if none,
# or, while $work/silent exists, ends well having said nothing at all, or,
# while $work/stalled exists, half a record out, stalls as a reading starved
# on a machine under pressure does, deaf to the first signal while
# $work/stubborn exists too, or, while $work/killed exists, is killed
# outright, as the kernel kills a program when memory runs out.
cat > "$work/bin/repos" << EOF
#!$BASH
if [ "\$*" != "--json" ]; then
  echo "repos: asked '\$*' rather than --json" >&2
  exit 64
fi
if [ -e "$work/down" ]; then
  printf '{"schema":"repos.repositories/1",'
  echo "repos: couldn't write its answer: broken pipe" >&2
  status=\$(< "$work/down")
  exit "\${status:-1}"
fi
if [ -e "$work/silent" ]; then
  exit 0
fi
if [ -e "$work/killed" ]; then
  echo "repos: reading /home/ada/src" >&2
  kill -KILL \$\$
fi
if [ -e "$work/stalled" ]; then
  printf '{"schema":"repos.repositories/1",'
  if [ -e "$work/stubborn" ]; then
    trap '' TERM
  fi
  # A minute is far past the writer's wait here, so only the writer's own bound ends it.
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
failed="repos failed (repos: couldn't write its answer: broken pipe); the last homes stay"
if [ "$said" != "$failed" ]; then
  echo "With repos failing, the writer said '$said' rather than what repos said." >&2
  exit 1
fi
shopt -s nullglob dotglob
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/homes.json" ]; then
  echo "The writer left ${left[*]} in the site's folder." >&2
  exit 1
fi

kept="the last homes stay"
# The writer says what happened in its one line alone, so the journal holds no
# other, such as bash's own word on a program a signal ended.
quiet() {
  if [ -s "$work/err" ]; then
    echo "With $1, the writer also said '$(< "$work/err")'." >&2
    exit 1
  fi
}

# A status no named signal explains is no kill, and is said as it is: 200 is past
# every signal, and 160 is 32's, which Linux keeps for its threads and names nothing.
ends() {
  local status=$1 said
  echo "$status" > "$work/down"
  if ! said=$("$BASH" "$writer" "$work/site" 2> "$work/err"); then
    echo "With repos ending $status, the writer failed, so its unit would fail." >&2
    exit 1
  fi
  quiet "repos ending $status"
  local failed="repos failed (ended $status: repos: couldn't write its answer: broken pipe)"
  if [ "$said" != "$failed; $kept" ]; then
    echo "With repos ending $status, the writer said '$said' rather than how it ended." >&2
    exit 1
  fi
}
ends 200
ends 160

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
stalls() {
  local how=$1 started=$SECONDS said homes
  if ! said=$(CODE_SITE_WAITS=1 "$BASH" "$writer" "$work/site" 2> "$work/err"); then
    echo "With repos $how, the writer failed, so its unit would fail." >&2
    exit 1
  fi
  quiet "repos $how"
  # Its second's wait, and the five it gives a read deaf to the first signal.
  if ((SECONDS - started > 10)); then
    echo "With repos $how, the writer waited $((SECONDS - started)) s, not its second." >&2
    exit 1
  fi
  homes=$(< "$work/site/homes.json")
  if [ "$homes" != '{"kept":true}' ]; then
    echo "With repos $how, the writer changed the last homes to '$homes'." >&2
    exit 1
  fi
  if [ "$said" != "repos read for over 1 s, as a starved machine makes it; $kept" ]; then
    echo "With repos $how, the writer said '$said' rather than that it read too long." >&2
    exit 1
  fi
  left=("$work/site"/*)
  if [ "${left[*]}" != "$work/site/homes.json" ]; then
    echo "With repos $how, the writer left ${left[*]} in the site's folder." >&2
    exit 1
  fi
}
stalls stalled
touch "$work/stubborn"
stalls "stalled, deaf to the first signal"
rm "$work/stalled" "$work/stubborn"

# A read killed by something else is a failure, not a stall, however it ended, and
# says what killed it. Its wait is long, so a slow builder can't make the kill a stall.
touch "$work/killed"
if ! said=$(CODE_SITE_WAITS=30 "$BASH" "$writer" "$work/site" 2> "$work/err"); then
  echo "With repos killed outright, the writer failed, so its unit would fail." >&2
  exit 1
fi
quiet "repos killed outright"
homes=$(< "$work/site/homes.json")
if [ "$homes" != '{"kept":true}' ]; then
  echo "With repos killed outright, the writer changed the last homes to '$homes'." >&2
  exit 1
fi
killed="repos failed (killed by SIGKILL: repos: reading /home/ada/src); the last homes stay"
if [ "$said" != "$killed" ]; then
  echo "With repos killed outright, the writer said '$said' rather than what killed it." >&2
  exit 1
fi
left=("$work/site"/*)
if [ "${left[*]}" != "$work/site/homes.json" ]; then
  echo "With repos killed outright, the writer left ${left[*]} in the site's folder." >&2
  exit 1
fi
rm "$work/killed"
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
