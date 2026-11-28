#!/usr/bin/env bash
# The test of lock-follows (lock-follows.sh), against stand-ins: origin is
# a bare repository whose main locks a mycelium from GitHub and a nixpkgs,
# upstream is a repository git reaches in GitHub's place, and gh and nix
# are stand-ins. gh keeps its pull requests in a JSON file, opens one only
# for a branch origin has, and closes one whose branch is deleted, as
# GitHub does; nix's `flake update INPUT` locks INPUT at what upstream's
# HEAD is, touching nothing else. Fourteen cases: up to date does
# nothing; behind opens one, each upstream subject in a code span, so a
# "Fixes #12" closes nothing; behind with one open does nothing; newer,
# after main moved on, opens the newer and replaces the older draft,
# naming it, commenting on it, then closing it, then deleting its
# branch; newest replaces none of an older whose lock commit someone
# amended (another input's rev in its lock, or a line added to its
# message), one marked ready, another input's or a fork's (whose name
# origin's branch of the tool's commit shares), naming none and keeping
# each branch; declined, its draft closed by hand, isn't opened again or
# pushed over; declined with its branch deleted isn't opened again
# either; taken, a branch of its name holding the tool's commit with
# someone's file amended in, is left untouched with none opened; a
# failed create exits non-zero naming the step, and its next run opens
# on the branch already pushed, a fork's closed draft of that name
# declining nothing; a failed close leaves the older open, its branch
# kept; a refused delete leaves the olders closed, their branches kept,
# and says so; a failed listing of pull requests exits 75; a failed
# fetch exits 75 and writes its line.
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
  # A listing fails once while its flag stands, as GitHub's can.
  if [ -e $here/list-fails ]; then
    rm $here/list-fails
    echo "gh stand-in: couldn't list the pull requests" >&2
    exit 1
  fi
  if [ "\$*" = "pr list --state open --limit 100 --json number,headRefName,url,isCrossRepository,isDraft" ]; then
    jq -c 'map(select(.state == "open")
      | {number, headRefName, url, isCrossRepository: (.isCrossRepository // false),
        isDraft: .draft})' "\$prs"
  elif [ "\$#" = 8 ] && [ "\$3 \$4 \$5 \$7 \$8" = "--state closed --head --json number,isCrossRepository" ]; then
    jq -c --arg head "\$6" 'map(select(.state == "closed" and .headRefName == \$head)
      | {number, isCrossRepository: (.isCrossRepository // false)})' "\$prs"
  else
    echo "gh stand-in: listed by \$*" >&2
    exit 1
  fi
  ;;
create)
  [ "\$3 \$4 \$5 \$6 \$8" = "--draft --base main --head --title" ] ||
    { echo "gh stand-in: opened by \$*" >&2; exit 1; }
  # A create fails once while its flag stands, as GitHub's can.
  if [ -e $here/create-fails ]; then
    rm $here/create-fails
    echo "gh stand-in: couldn't open the pull request" >&2
    exit 1
  fi
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
comment)
  [ "\$#" = 5 ] && [ "\$4" = --body ] || { echo "gh stand-in: commented by \$*" >&2; exit 1; }
  jq --argjson number "\$3" --arg comment "\$5" \
    'map(if .number == \$number then .comments += [\$comment] else . end)' \
    "\$prs" >"\$prs.new"
  mv "\$prs.new" "\$prs"
  ;;
close)
  [ "\$#" = 3 ] || { echo "gh stand-in: closed by \$*" >&2; exit 1; }
  # A close fails once while its flag stands, as GitHub's can.
  if [ -e $here/close-fails ]; then
    rm $here/close-fails
    echo "gh stand-in: couldn't close the pull request" >&2
    exit 1
  fi
  state=\$(jq -r --argjson number "\$3" '.[] | select(.number == \$number) | .state' "\$prs")
  if [ "\$state" = closed ]; then
    echo "! Pull request #\$3 is already closed" >&2
    exit 0
  fi
  jq --argjson number "\$3" \
    'map(if .number == \$number then .state = "closed" | .closedBy = "close" else . end)' \
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
# Deleting a branch closes the open pull requests from it, as GitHub does.
cat >"$here/origin.git/hooks/post-receive" <<EOF
#!$BASH
set -euo pipefail
while read -r _ new ref; do
  if [ -z "\${new//0/}" ] && [ -e $prs ]; then
    jq --arg head "\${ref#refs/heads/}" 'map(if .state == "open" and .headRefName == \$head
        and ((.isCrossRepository // false) | not)
      then .state = "closed" | .closedBy = "its branch deleted" else . end)' \
      $prs >$prs.new
    mv $prs.new $prs
  fi
done
EOF
chmod +x "$here/origin.git/hooks/post-receive"

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

# Where origin's BRANCH points, or nothing once it's gone.
head_of() {
  at=$(git -C "$here/origin.git" rev-parse --verify --quiet "refs/heads/$1" || true)
}

# Says what the run's line says it closed, kept the branch of, and left
# open, as "CLOSED KEPT LEFT".
outcome() {
  done_with=$(jq -c '"\(.closed) \(.closedBranchKept) \(.leftOpen)"' <<<"$line")
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

# Upstream takes a commit whose subject would close an issue, and merges
# a change.
git -C "$upstream" commit --quiet --allow-empty --message "Fixes #12 in \`x\`"
fixes=$(git -C "$upstream" rev-parse HEAD)
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
if [[ $body != *"- ${second:0:7} \`Merge pull request Jitsusama/mycelium#7 from someone/change\`"* ]]; then
  fail "behind: its body doesn't list upstream's merge, naming #7 as upstream's, in a code span: $body"
fi
if [[ $body != *"- ${fixes:0:7} \`Fixes Jitsusama/mycelium#12 in 'x'\`"* ]]; then
  fail "behind: its body doesn't hold the subject saying Fixes in a code span it can't end: $body"
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

# Origin's main moves on, and upstream merges a newer change.
echo "what core is" >"$seed/README.md"
git -C "$seed" add README.md
git -C "$seed" commit --quiet --message "a readme"
git -C "$seed" push --quiet "$here/origin.git" HEAD:main
git -C "$upstream" commit --quiet --allow-empty --message "a newer change"
third=$(git -C "$upstream" rev-parse HEAD)

run "newer" 0 opened
counted
older=$(jq -c '.[0] // {}' "$prs")
newer=$(jq -c '.[1] // {}' "$prs")
state=$(jq -r '"\(.state) \(.closedBy)"' <<<"$older")
head=$(jq -r .headRefName <<<"$newer")
if [ "$count" != "2 1" ] || [ "$state" != "closed close" ] ||
  [ "$head" != "lock-follows/mycelium-${third:0:8}" ]; then
  fail "newer: it didn't open one for the newer rev and close the older itself: $state"
fi
comment=$(jq -r '.comments // [] | join(" ")' <<<"$older")
if [[ $comment != "Replaced by https://example.com/pull/2, which locks mycelium at ${third:0:8}." ]]; then
  fail "newer: the older's one comment doesn't name the newer: $comment"
fi
body=$(jq -r .body <<<"$newer")
if [[ $body != *"Replaces #1"* ]]; then
  fail "newer: the newer's body doesn't name the older it replaces: $body"
fi
outcome
if [ "$done_with" != '"[1] [] []"' ]; then
  fail "newer: its line doesn't name the one it closed alone: $line"
fi
head_of "$branch"
if [ -n "$at" ]; then
  fail "newer: the older's branch is still on origin"
fi

# Pushes BRANCH to origin holding what lock-follows would for REV: one
# commit on main locking mycelium alone, with the tool's two lines, and
# MORE after them.
work=$here/work
git clone --quiet "$here/origin.git" "$work"
lock_commit() {
  local branch=$1 rev=$2 more=${3:-}
  git -C "$work" fetch --quiet origin
  git -C "$work" checkout --quiet -B made origin/main
  jq --arg rev "$rev" '.nodes.mycelium.locked.rev = $rev' "$work/flake.lock" >"$work/flake.lock.new"
  mv "$work/flake.lock.new" "$work/flake.lock"
  git -C "$work" commit --quiet --message "build(mycelium): lock mycelium ${rev:0:8}

Moves mycelium from ${first:0:8} to ${rev:0:8}, its upstream's newest.$more" -- flake.lock
  git -C "$work" push --quiet origin "HEAD:refs/heads/$branch"
}

# Someone amends the newer's lock commit to lock another nixpkgs too,
# keeping its message. Another input whose name extends mycelium's has a
# draft open; a fork has one on a branch named as origin's holding the
# tool's commit for that rev, an earlier run's that never opened; one
# holds the tool's commit with a line added to its message; and one
# holding the tool's commit was marked ready. Upstream merges again.
newer_branch=lock-follows/mycelium-${third:0:8}
git -C "$work" fetch --quiet origin
git -C "$work" checkout --quiet -B newer "origin/$newer_branch"
jq '.nodes.nixpkgs.locked.rev = "0000000000000000000000000000000000000002"' \
  "$work/flake.lock" >"$work/flake.lock.new"
mv "$work/flake.lock.new" "$work/flake.lock"
git -C "$work" commit --quiet --amend --no-edit -- flake.lock
git -C "$work" push --quiet --force origin "HEAD:refs/heads/$newer_branch"
amended=$(git -C "$work" rev-parse HEAD)
forked=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
reworded=cccccccccccccccccccccccccccccccccccccccc
ready=dddddddddddddddddddddddddddddddddddddddd
lock_commit "lock-follows/mycelium-${forked:0:8}" "$forked"
lock_commit "lock-follows/mycelium-${reworded:0:8}" "$reworded" "

Also pins it for the release."
lock_commit "lock-follows/mycelium-${ready:0:8}" "$ready"
jq --arg forked "lock-follows/mycelium-${forked:0:8}" \
  --arg reworded "lock-follows/mycelium-${reworded:0:8}" \
  --arg ready "lock-follows/mycelium-${ready:0:8}" '. + [
  {number: 3, headRefName: "lock-follows/mycelium-relay-abcdef12", state: "open",
    draft: true, url: "https://example.com/pull/3"},
  {number: 4, headRefName: $forked, state: "open", draft: true,
    isCrossRepository: true, url: "https://example.com/pull/4"},
  {number: 5, headRefName: $reworded, state: "open", draft: true,
    url: "https://example.com/pull/5"},
  {number: 6, headRefName: $ready, state: "open", draft: false,
    url: "https://example.com/pull/6"}
]' "$prs" >"$prs.new"
mv "$prs.new" "$prs"
standing=$(git -C "$here/origin.git" for-each-ref --format='%(refname) %(objectname)' 'refs/heads/lock-follows/')
git -C "$upstream" commit --quiet --allow-empty --message "the newest change"
newest=$(git -C "$upstream" rev-parse HEAD)

run "newest" 0 opened
states=$(jq -r 'map(.state) | join(" ")' "$prs")
if [ "$states" != "closed open open open open open open" ]; then
  fail "newest: it closed an amended one, another input's, a fork's or a ready one: $states"
fi
outcome
if [ "$done_with" != '"[] [] []"' ]; then
  fail "newest: its line names one closed or left: $line"
fi
body=$(jq -r '.[6].body // ""' "$prs")
if [[ $body == *Replaces* ]]; then
  fail "newest: its body names one it doesn't replace: $body"
fi
now=$(git -C "$here/origin.git" for-each-ref --format='%(refname) %(objectname)' 'refs/heads/lock-follows/' |
  grep -v "mycelium-${newest:0:8}" || true)
if [ "$now" != "$standing" ]; then
  fail "newest: it deleted or moved a branch it doesn't replace: $now"
fi
head_of "$newer_branch"
if [ "$at" != "$amended" ]; then
  fail "newest: it deleted or moved the branch whose lock someone amended: $at"
fi

# The newest's draft is closed by hand, to decline that rev; its branch
# stays on origin, as GitHub leaves it.
newest_branch=$(jq -r '.[6].headRefName' "$prs")
jq '.[6].state = "closed"' "$prs" >"$prs.new"
mv "$prs.new" "$prs"
head_of "$newest_branch"
before=$at

run "declined" 0 declined
counted
if [ "$count" != "7 5" ]; then
  fail "declined: it opened one again: $count"
fi
head_of "$newest_branch"
if [ "$at" != "$before" ]; then
  fail "declined: it pushed over the declined rev's branch"
fi
said=$(jq -r '"\(.branch) \(.number)"' <<<"$line")
if [ "$said" != "$newest_branch 7" ]; then
  fail "declined: its line doesn't name the branch and the closed one: $line"
fi

# The declined draft's branch is deleted, as GitHub's button does.
git -C "$here/origin.git" branch --quiet --delete --force "$newest_branch"

run "declined, its branch deleted" 0 declined
counted
if [ "$count" != "7 5" ]; then
  fail "declined, its branch deleted: it opened one again: $count"
fi
head_of "$newest_branch"
if [ -n "$at" ]; then
  fail "declined, its branch deleted: it pushed the declined rev's branch again"
fi
said=$(jq -r '"\(.branch) \(.number)"' <<<"$line")
if [ "$said" != "$newest_branch 7" ]; then
  fail "declined, its branch deleted: its line doesn't name the branch and the closed one: $line"
fi

# Upstream merges again, and someone pushes to the branch that rev's
# would have, with no pull request, the tool's commit with a file of
# their own amended in.
git -C "$upstream" commit --quiet --allow-empty --message "a taken change"
taken=$(git -C "$upstream" rev-parse HEAD)
taken_branch=lock-follows/mycelium-${taken:0:8}
lock_commit "$taken_branch" "$taken"
echo "someone's work" >"$work/work.txt"
git -C "$work" add work.txt
git -C "$work" commit --quiet --amend --no-edit
git -C "$work" push --quiet --force origin "HEAD:refs/heads/$taken_branch"
head_of "$taken_branch"
before=$at

run "taken" 0 taken
counted
if [ "$count" != "7 5" ]; then
  fail "taken: it opened or closed one: $count"
fi
head_of "$taken_branch"
if [ "$at" != "$before" ]; then
  fail "taken: it moved or deleted someone else's branch: $at"
fi
said=$(jq -r .branch <<<"$line")
if [ "$said" != "$taken_branch" ]; then
  fail "taken: its line doesn't name the branch: $line"
fi

# Upstream merges again; a fork's closed draft has a branch named like
# that rev's, and opening the tool's own fails once.
git -C "$upstream" commit --quiet --allow-empty --message "a change whose opening fails"
retried=$(git -C "$upstream" rev-parse HEAD)
retried_branch=lock-follows/mycelium-${retried:0:8}
jq --arg head "$retried_branch" '. + [
  {number: 8, headRefName: $head, state: "closed", isCrossRepository: true,
    draft: true, url: "https://example.com/pull/8"}
]' "$prs" >"$prs.new"
mv "$prs.new" "$prs"
touch "$here/create-fails"

run "a failed create" 1 failed
step=$(jq -r .step <<<"$line")
if [ "$step" != "opening the pull request" ]; then
  fail "a failed create: its line doesn't say it failed opening: $line"
fi
counted
if [ "$count" != "8 5" ]; then
  fail "a failed create: it opened or closed one: $count"
fi
head_of "$retried_branch"
pushed_head=$at
if [ -z "$pushed_head" ]; then
  fail "a failed create: it didn't push its branch before opening"
fi

run "a failed create, run again" 0 opened
counted
shape=$(jq -r '.[8] // {} | "\(.headRefName) \(.state)"' "$prs")
if [ "$count" != "9 6" ] || [ "$shape" != "$retried_branch open" ]; then
  fail "a failed create, run again: it didn't open one on the branch pushed: $count $shape"
fi
head_of "$retried_branch"
ahead=$(git -C "$here/origin.git" rev-list --count "main..$retried_branch")
if [ "$at" != "$pushed_head" ] || [ "$ahead" != 1 ]; then
  fail "a failed create, run again: it pushed the branch again, $ahead ahead of main"
fi
said=$(jq -r .pr <<<"$line")
if [ "$said" != https://example.com/pull/9 ]; then
  fail "a failed create, run again: its line doesn't name the one opened: $line"
fi

# Upstream merges again, and closing the older fails once.
git -C "$upstream" commit --quiet --allow-empty --message "a change whose older won't close"
unclosed=$(git -C "$upstream" rev-parse HEAD)
unclosed_branch=lock-follows/mycelium-${unclosed:0:8}
touch "$here/close-fails"

run "a failed close" 0 opened
state=$(jq -r '.[8].state' "$prs")
if [ "$state" != open ]; then
  fail "a failed close: the older it couldn't close isn't open: $state"
fi
outcome
if [ "$done_with" != '"[] [] [9]"' ]; then
  fail "a failed close: its line doesn't say it left the older open: $line"
fi
head_of "$retried_branch"
if [ "$at" != "$pushed_head" ]; then
  fail "a failed close: the older's branch moved or went: $at"
fi

# Upstream merges again, and origin refuses every delete, as a rule or a
# lost connection can.
git -C "$upstream" commit --quiet --allow-empty --message "a change whose older's branch stays"
cat >"$here/origin.git/hooks/pre-receive" <<EOF
#!$BASH
while read -r _ new _; do
  if [ -z "\${new//0/}" ]; then
    echo "origin refuses deletes" >&2
    exit 1
  fi
done
EOF
chmod +x "$here/origin.git/hooks/pre-receive"
head_of "$unclosed_branch"
unclosed_head=$at

run "a refused delete" 0 opened
states=$(jq -r '.[8:] | map("\(.state) \(.closedBy)") | join(", ")' "$prs")
if [ "$states" != "closed close, closed close, open null" ]; then
  fail "a refused delete: it didn't close the olders whose branches it couldn't delete: $states"
fi
outcome
if [ "$done_with" != '"[9,10] [9,10] []"' ]; then
  fail "a refused delete: its line doesn't say it closed them with their branches kept: $line"
fi
head_of "$retried_branch"
kept=$at
head_of "$unclosed_branch"
if [ "$kept" != "$pushed_head" ] || [ "$at" != "$unclosed_head" ]; then
  fail "a refused delete: an older's branch moved or went"
fi
rm "$here/origin.git/hooks/pre-receive"

# Listing the pull requests fails.
git -C "$upstream" commit --quiet --allow-empty --message "a change listed later"
touch "$here/list-fails"
run "a failed listing" 75 failed
step=$(jq -r .step <<<"$line")
if [ "$step" != "listing pull requests" ]; then
  fail "a failed listing: its line doesn't say it failed listing: $line"
fi

# Origin can't be reached.
mv "$here/origin.git" "$here/gone.git"
run "a failed fetch" 75 failed
step=$(jq -r .step <<<"$line")
if [ "$step" != "fetching origin" ]; then
  fail "a failed fetch: its line doesn't say it failed fetching: $line"
fi
mv "$here/gone.git" "$here/origin.git"

asked=$(grep -cv '^gh pr \(list\|create\|comment\|close\) ' "$here/gh-said" || true)
if [ "$asked" != 0 ]; then
  fail "it asked gh for more than listing, opening, commenting on and closing pull requests"
fi

if [ "$failures" != 0 ]; then
  echo "$failures failed" >&2
  exit 1
fi
echo "every case held"
