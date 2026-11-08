# shellcheck shell=bash
# Runs the disk guard's GitHub predicate (modules/disk-guard/merged-on-github.sh)
# over a stand-in gh (stand-in-gh.sh). A checkout is finished only when a
# merged pull request's head is exactly its own, whichever form of remote
# GitHub gave it. One with more work on top isn't, nor one on no branch,
# which gh is never asked about, and a gh that fails gives no answer.
# usage: disk-guard-asks-github-what-merged.sh <predicate>
set -euo pipefail

predicate=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export ASKED=$work/asked
finished='{"finished": true}'
unfinished='{"finished": false}'

fail() {
  echo "$1"
  exit 1
}

# Asks about a checkout of a remote on a branch at a head, with the heads
# merged from that branch, and gives the answer.
ask() {
  : > "$ASKED"
  DISK_GUARD_REMOTE=$1 DISK_GUARD_BRANCH=$2 DISK_GUARD_HEAD=$3 MERGED=$4 "$predicate"
}

for remote in \
  https://github.com/owner/repo.git \
  https://github.com/owner/repo \
  git@github.com:owner/repo.git \
  ssh://git@github.com/owner/repo.git; do
  said=$(ask "$remote" tidy 2b1f "$(printf '9c0e\n2b1f\n')")
  [ "$said" = "$finished" ] || fail "a merged head from $remote was answered $said"
  asked=$(cat "$ASKED")
  [ "$asked" = "pr list --repo owner/repo --head tidy --state merged --json headRefOid --jq .[].headRefOid" ] ||
    fail "gh was asked $asked for $remote"
done

said=$(ask git@github.com:owner/repo.git tidy 7d3a "$(printf '9c0e\n2b1f\n')")
[ "$said" = "$unfinished" ] || fail "a head with more work on top was answered $said"
said=$(ask git@github.com:owner/repo.git tidy 2b1f 2b1f7d3a)
[ "$said" = "$unfinished" ] || fail "a head only part of a merged one was answered $said"
said=$(ask git@github.com:owner/repo.git tidy 2b1f "")
[ "$said" = "$unfinished" ] || fail "a branch with nothing merged was answered $said"

said=$(ask git@github.com:owner/repo.git "" 2b1f 2b1f)
[ "$said" = "$unfinished" ] || fail "a checkout on no branch was answered $said"
[ ! -s "$ASKED" ] || fail "gh was asked about a checkout on no branch: $(cat "$ASKED")"

if said=$(FAILS=1 ask git@github.com:owner/repo.git tidy 2b1f 2b1f 2> /dev/null); then
  fail "a gh that failed was answered $said"
fi
