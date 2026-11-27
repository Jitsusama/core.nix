#!/usr/bin/env bash
# The test of lock-follows (lock-follows.sh), against stand-ins: origin is
# a bare repository whose main locks a mycelium from GitHub and a nixpkgs,
# upstream is a repository git reaches in GitHub's place, and gh and nix
# are stand-ins. gh keeps its pull requests in a JSON file and opens one
# only for a branch origin has; nix's `flake update INPUT` locks INPUT at
# what upstream's HEAD is, touching nothing else. Five cases: up to date
# does nothing; behind opens one; behind with one open does nothing; newer
# opens the newer and closes the older; a failed fetch exits non-zero and
# writes its line.
#
#   lock-follows-test.sh LOCK-FOLLOWS
set -euo pipefail

program=$1
here=$PWD
bin=$here/bin
checkout=$here/state/checkout
record=$here/state/runs.jsonl
prs=$here/prs.json
mkdir -p "$bin"

export HOME=$here/home
export GIT_CONFIG_GLOBAL=$here/gitconfig
export GIT_CONFIG_NOSYSTEM=1
git config --global user.name "lock-follows test"
git config --global user.email "test@example.com"
git config --global init.defaultBranch main
git config --global commit.gpgsign false
# GitHub, as flake.lock names upstream, is a folder here.
git config --global url."$here/github/".insteadOf "https://github.com/"

cat >"$bin/gh" <<EOF
#!$BASH
set -euo pipefail
prs=$prs
[ -e "\$prs" ] || echo '[]' >"\$prs"
echo "gh \$*" >>$here/gh-said
[ "\$1" = pr ] || { echo "gh stand-in: asked \$*" >&2; exit 1; }
case \$2 in
list)
  [ "\$*" = "pr list --state open --limit 100 --json number,headRefName,url" ] ||
    { echo "gh stand-in: listed by \$*" >&2; exit 1; }
  jq -c 'map(select(.state == "open") | {number, headRefName, url})' "\$prs"
  ;;
create)
  [ "\$3 \$4 \$5 \$6 \$8" = "--draft --base main --head --title" ] ||
    { echo "gh stand-in: opened by \$*" >&2; exit 1; }
  [ "\${10}" = --body-file ] || { echo "gh stand-in: no body file" >&2; exit 1; }
  git ls-remote --exit-code $here/origin.git "refs/heads/\$7" >/dev/null ||
    { echo "gh stand-in: origin has no branch \$7" >&2; exit 1; }
  number=\$((\$(jq length "\$prs") + 1))
  jq --argjson number "\$number" --arg head "\$7" --arg title "\$9" \
    --rawfile body "\${11}" \
    '. + [{number: \$number, headRefName: \$head, title: \$title, body: \$body,
      draft: true, state: "open", url: "https://example.com/pull/\(\$number)"}]' \
    "\$prs" >"\$prs.new"
  mv "\$prs.new" "\$prs"
  echo "Creating draft pull request for \$7 into main"
  echo "https://example.com/pull/\$number"
  ;;
close)
  [ "\$4" = --comment ] || { echo "gh stand-in: closed by \$*" >&2; exit 1; }
  jq --argjson number "\$3" --arg comment "\$5" \
    'map(if .number == \$number then .state = "closed" | .comment = \$comment else . end)' \
    "\$prs" >"\$prs.new"
  mv "\$prs.new" "\$prs"
  ;;
*)
  echo "gh stand-in: asked \$*" >&2
  exit 1
  ;;
esac
EOF
cat >"$bin/nix" <<EOF
#!$BASH
set -euo pipefail
[ "\$#" = 3 ] && [ "\$1 \$2" = "flake update" ] ||
  { echo "nix stand-in: asked \$*" >&2; exit 1; }
node=\$(jq -r --arg input "\$3" '.nodes.root.inputs[\$input]' flake.lock)
original=\$(jq -r --arg node "\$node" \\
  '.nodes[\$node].original | "https://github.com/\(.owner)/\(.repo)"' flake.lock)
rev=\$(git ls-remote "\$original" HEAD | cut -f1)
jq --arg node "\$node" --arg rev "\$rev" '.nodes[\$node].locked.rev = \$rev' \\
  flake.lock >flake.lock.new
mv flake.lock.new flake.lock
EOF
chmod +x "$bin/gh" "$bin/nix"
export PATH=$bin:$PATH

# Upstream, with a first commit main locks.
upstream=$here/github/Jitsusama/mycelium
git init --quiet "$upstream"
git -C "$upstream" commit --quiet --allow-empty --message "the first"
first=$(git -C "$upstream" rev-parse HEAD)

# Origin, whose main locks upstream's first commit.
seed=$here/seed
git init --quiet "$seed"
jq -n --arg rev "$first" '{
  nodes: {
    mycelium: {
      locked: {owner: "Jitsusama", repo: "mycelium", rev: $rev, type: "github"},
      original: {owner: "Jitsusama", repo: "mycelium", type: "github"}
    },
    nixpkgs: {
      locked: {
        owner: "NixOS", repo: "nixpkgs", type: "github",
        rev: "0000000000000000000000000000000000000001"
      },
      original: {owner: "NixOS", ref: "nixos-unstable", repo: "nixpkgs", type: "github"}
    },
    root: {inputs: {mycelium: "mycelium", nixpkgs: "nixpkgs"}}
  },
  root: "root",
  version: 7
}' >"$seed/flake.lock"
git -C "$seed" add flake.lock
git -C "$seed" commit --quiet --message "the lock"
git clone --quiet --bare "$seed" "$here/origin.git"
main=$(git -C "$here/origin.git" rev-parse main)

failures=0
fail() {
  echo "FAIL $1" >&2
  failures=$((failures + 1))
}

# Each check below reads what it checks into a variable first, so a
# failing git or jq ends the test rather than passing unseen.

# Runs lock-follows once for mycelium, and checks it ended as wanted and
# its run added one line, which it printed, saying what it did.
run() {
  local case=$1 want=$2 did=$3 ended=0 before=0 after=0 told said
  if [ -e "$record" ]; then
    before=$(wc -l <"$record")
  fi
  "$program" --checkout "$checkout" --repository "$here/origin.git" \
    --prefix lock-follows --record "$record" mycelium >"$here/said" 2>"$here/told" || ended=$?
  told=$(<"$here/told")
  if [ "$ended" != "$want" ]; then
    fail "$case: ended $ended, wanted $want: $told"
  fi
  if [ -e "$record" ]; then
    after=$(wc -l <"$record")
  fi
  if [ "$after" != "$((before + 1))" ]; then
    fail "$case: the run added $((after - before)) lines to its record, not one"
    return 0
  fi
  line=$(tail -n 1 "$record")
  shape=$(jq -r '"\(.did) \(.input) \(.when | type)"' <<<"$line")
  if [ "$shape" != "$did mycelium string" ]; then
    fail "$case: its line said $line, not that it did $did for mycelium, and when"
  fi
  said=$(<"$here/said")
  if [ "$said" != "$line" ]; then
    fail "$case: it printed $said rather than its line"
  fi
}

# How many pull requests there are, and how many are open, as "ALL OPEN".
counted() {
  if [ -e "$prs" ]; then
    count=$(jq -r '"\(length) \(map(select(.state == "open")) | length)"' "$prs")
  else
    count="0 0"
  fi
}

# The branches lock-follows pushed to origin.
pushed() {
  branches=$(git -C "$here/origin.git" branch --list 'lock-follows/*')
}

run "up to date" 0 up-to-date
counted
pushed
if [ "$count" != "0 0" ] || [ -n "$branches" ]; then
  fail "up to date: it opened a pull request or pushed a branch"
fi
revs=$(jq -r '"\(.locked) \(.upstream)"' <<<"$line")
if [ "$revs" != "$first $first" ]; then
  fail "up to date: its line doesn't say both revs are upstream's first: $line"
fi

# Upstream merges a change.
git -C "$upstream" checkout --quiet -b change
git -C "$upstream" commit --quiet --allow-empty --message "a change"
git -C "$upstream" checkout --quiet main
git -C "$upstream" merge --quiet --no-ff change \
  --message "Merge pull request #7 from someone/change"
second=$(git -C "$upstream" rev-parse HEAD)
branch=lock-follows/mycelium-${second:0:8}

run "behind" 0 opened
counted
pr=$(jq -c '.[0] // {}' "$prs")
shape=$(jq -r '"\(.headRefName) \(.draft) \(.title)"' <<<"$pr")
if [ "$count" != "1 1" ] || [ "$shape" != "$branch true Lock Mycelium ${second:0:8}" ]; then
  fail "behind: it didn't open one draft pull request, titled for the rev, from $branch: $pr"
fi
body=$(jq -r .body <<<"$pr")
if [[ $body != *"- ${second:0:7}"* ]]; then
  fail "behind: its body doesn't list upstream's merge: $body"
fi
if [[ $body != *"Jitsusama/mycelium#7 from someone/change"* ]]; then
  fail "behind: its body's merge names #7 as this repository's, not upstream's: $body"
fi
if [[ $body == *"the first"* ]]; then
  fail "behind: its body lists the commit main already locked: $body"
fi
subject=$(git -C "$here/origin.git" log --format=%s -1 "$branch")
parent=$(git -C "$here/origin.git" rev-parse "$branch^")
if [ "$subject" != "build(mycelium): lock mycelium ${second:0:8}" ] ||
  [ "$parent" != "$main" ]; then
  fail "behind: the branch's commit is '$subject' on $parent, not a lock commit on main"
fi
locked=$(git -C "$here/origin.git" show "$branch:flake.lock")
rev=$(jq -r .nodes.mycelium.locked.rev <<<"$locked")
nixpkgs=$(jq -c .nodes.nixpkgs <<<"$locked")
was=$(jq -c .nodes.nixpkgs "$seed/flake.lock")
changed=$(git -C "$here/origin.git" diff --name-only main "$branch")
if [ "$rev" != "$second" ] || [ "$nixpkgs" != "$was" ] || [ "$changed" != flake.lock ]; then
  fail "behind: the branch doesn't lock mycelium alone at upstream's newest"
fi
now=$(git -C "$here/origin.git" rev-parse main)
if [ "$now" != "$main" ]; then
  fail "behind: origin's main moved"
fi

run "behind with one open" 0 open-already
counted
if [ "$count" != "1 1" ]; then
  fail "behind with one open: it opened or closed one"
fi
said=$(jq -r .pr <<<"$line")
if [ "$said" != https://example.com/pull/1 ]; then
  fail "behind with one open: its line doesn't name the one open: $line"
fi

# Upstream merges a newer change.
git -C "$upstream" commit --quiet --allow-empty --message "a newer change"
third=$(git -C "$upstream" rev-parse HEAD)

run "newer" 0 opened
counted
older=$(jq -c '.[0] // {}' "$prs")
newer=$(jq -c '.[1] // {}' "$prs")
state=$(jq -r .state <<<"$older")
head=$(jq -r .headRefName <<<"$newer")
if [ "$count" != "2 1" ] || [ "$state" != closed ] ||
  [ "$head" != "lock-follows/mycelium-${third:0:8}" ]; then
  fail "newer: it didn't open one for the newer rev and close the older"
fi
comment=$(jq -r .comment <<<"$older")
if [[ $comment != *https://example.com/pull/2* ]]; then
  fail "newer: the older's closing doesn't name the newer: $comment"
fi
body=$(jq -r .body <<<"$newer")
if [[ $body != *"Replaces #1"* ]]; then
  fail "newer: the newer's body doesn't name the older it replaces: $body"
fi
closed=$(jq -c .closed <<<"$line")
if [ "$closed" != "[1]" ]; then
  fail "newer: its line doesn't name the one it closed: $line"
fi
gone=$(git -C "$here/origin.git" branch --list "$branch")
if [ -n "$gone" ]; then
  fail "newer: the older's branch is still on origin"
fi

# Origin can't be reached.
mv "$here/origin.git" "$here/gone.git"
run "a failed fetch" 75 failed
step=$(jq -r .step <<<"$line")
if [ "$step" != "fetching origin" ]; then
  fail "a failed fetch: its line doesn't say it failed fetching: $line"
fi
mv "$here/gone.git" "$here/origin.git"

asked=$(grep -cv '^gh pr \(list\|create\|close\) ' "$here/gh-said" || true)
if [ "$asked" != 0 ]; then
  fail "it asked gh for more than listing, opening and closing pull requests"
fi

if [ "$failures" != 0 ]; then
  echo "$failures failed" >&2
  exit 1
fi
echo "every case held"
