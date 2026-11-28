# shellcheck shell=bash
# Follows each flake input named to its upstream's newest commit, through
# a draft pull request on the repository whose lock holds it, so a change
# merged upstream reaches the lock without waiting on a hand bump. It reads
# the rev origin's main locks the input at, and upstream's newest with git
# ls-remote, from the source flake.lock says the input came from. When they
# differ and no pull request has the branch for that rev, it cuts
# PREFIX/INPUT-REV8 from origin's main, runs `nix flake update INPUT`, and
# commits the lock alone, as the account's git config signs. It pushes that
# branch, never main, and opens a draft whose body lists upstream's merges
# between the two revs, each subject in a code span, so no closing word in
# one closes anything. A newer rev opens a new pull request and replaces
# each older draft whose branch still holds the tool's own commit, exactly
# as the tool would write it: the new one names it, and it is commented
# on, closed, and its branch deleted. An older marked ready, or one whose
# branch someone pushed or amended anything onto, is left open, its
# branch kept. A rev whose branch a closed pull request names was
# declined, and isn't opened again, whether its branch stands or was
# deleted. A branch standing with no pull request is opened on when it
# holds the tool's own commit, an earlier run having failed before
# opening it, and is left as taken when it holds anything else. It never
# pushes over a branch, and never merges.
#
# Each input's run adds one JSON line to the record, and prints it: when,
# the input, the rev locked, upstream's, and what it did. A failure writes
# its line too, naming the step it failed at, and ends the run, so the
# inputs after it wait for the next.
#
# The pull request command and nix are the account's, from the PATH, and
# git reads the account's config, so its commits are signed and its pull
# requests opened as the account's own are. Its help takes the shape
# every command of ours does (docs/commands.md), by hand.

case ${1:-} in
-h | --help)
  cat <<'EOF'
lock-follows  flake inputs moved to upstream's newest by PR

EXAMPLES
  lock-follows --checkout ~/.local/state/lock-follows/core \
      --repository https://github.com/Jitsusama/core.nix \
      --prefix lock-follows \
      --record ~/.local/state/lock-follows/runs.jsonl \
      mycelium
      open a draft PR locking mycelium at its upstream's
      newest, unless main locks it there or one is open

FLAGS
  --checkout DIR     the clone it works in, made if missing
  --repository URL   where the clone comes from
  --prefix WORD      the start of each branch it pushes
  --record FILE      where each run's JSON line is added
  --pr COMMAND       the command PRs are listed, opened,
                     commented on and closed with, taking
                     gh's words (gh)

EXIT
  0   each input locked at upstream's newest, a PR open,
      its rev declined, or its branch taken by others
  1   a step failed, named in its line
  64  asked wrongly
  75  origin, its PRs or upstream couldn't be read, or
      upstream moved; tried next run
  78  an input can't be followed: it isn't in origin's
      flake.lock, comes from neither GitHub nor git, or
      upstream has no such branch
EOF
  exit 0
  ;;
*) ;;
esac

set -euo pipefail

asked_wrongly() {
  echo "✕ lock-follows $1" >&2
  echo "  try  lock-follows --help" >&2
  exit 64
}

checkout=''
repository=''
prefix=''
record=''
pr=gh
while [ "$#" -gt 0 ]; do
  case $1 in
  --checkout | --repository | --prefix | --record | --pr)
    [ "$#" -ge 2 ] || asked_wrongly "needs a value after $1"
    case $1 in
    --checkout) checkout=$2 ;;
    --repository) repository=$2 ;;
    --prefix) prefix=$2 ;;
    --record) record=$2 ;;
    *) pr=$2 ;;
    esac
    shift 2
    ;;
  --)
    shift
    break
    ;;
  -*) asked_wrongly "takes no flag $1" ;;
  *) break ;;
  esac
done
[ -n "$checkout" ] || asked_wrongly "needs --checkout"
[ -n "$repository" ] || asked_wrongly "needs --repository"
[ -n "$prefix" ] || asked_wrongly "needs --prefix"
[ -n "$record" ] || asked_wrongly "needs --record"
[ "$#" -gt 0 ] || asked_wrongly "needs an input to follow"

# Adds the input's line to the record and prints it: what it did, and an
# object of anything more to say.
said() {
  local when line
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  line=$(jq -cn --arg when "$when" --arg input "$input" --arg locked "$locked" \
    --arg upstream "$upstream" --arg did "$1" --argjson more "$2" \
    '{when: $when, input: $input,
      locked: (if $locked == "" then null else $locked end),
      upstream: (if $upstream == "" then null else $upstream end),
      did: $did} + $more')
  echo "$line" >>"$record"
  echo "$line"
  written=1
}

# A run ending on a failure writes the input's line, naming the step it
# failed at, and ends with a code from the table: a git or gh failure's
# own code means nothing to a reader of this one.
ended() {
  local status=$? where
  rm -f -- "$body"
  if [ "$status" = 0 ]; then
    return
  fi
  if [ "$written" = 0 ] && [ -n "$input" ]; then
    where=$(jq -cn --arg step "$step" '{step: $step}')
    said failed "$where"
  fi
  case $status in
  64 | 75 | 78) exit "$status" ;;
  *) exit 1 ;;
  esac
}

# The message of the tool's lock commit moving INPUT from FROM to REV,
# each said by its first eight hex digits.
message() {
  printf 'build(%s): lock %s %s\n\nMoves %s from %s to %s, its upstream'"'"'s newest.\n' \
    "$1" "$1" "${3:0:8}" "$1" "${2:0:8}" "${3:0:8}"
}

# Whether origin's BRANCH holds the tool's own lock commit for INPUT at
# the rev starting REV, printing its head when it does and nothing when
# it holds anything else, and failing when origin can't be read. What
# the tool would have written is rebuilt from the commit's parent, as
# main may have moved on since: one commit, on one of main's, its tree
# the parent's but for flake.lock, that lock the parent's with only
# INPUT's node moved to a rev starting REV, as the branch's name and
# subject say, and its message the tool's two lines. Anything else is
# someone's work, an amend of the lock, the tree or the message too, and
# is never closed, deleted or pushed over.
ours() {
  local branch=$1 input=$2 rev=$3 theirs parents changed was now node from wanted words
  git fetch --quiet origin "refs/heads/$branch" || return 75
  theirs=$(git rev-parse FETCH_HEAD) || return 75
  parents=$(git show --no-patch --format=%P "$theirs") || return 0
  if ! [[ $parents =~ ^[0-9a-f]+$ ]] ||
    ! git merge-base --is-ancestor "$parents" origin/main; then
    return 0
  fi
  changed=$(git diff --name-only "$parents" "$theirs") || return 0
  [ "$changed" = flake.lock ] || return 0
  was=$(git show "$parents:flake.lock") || return 0
  now=$(git show "$theirs:flake.lock") || return 0
  node=$(jq -r --arg input "$input" '.nodes[.root].inputs[$input] // empty | strings' <<<"$was") ||
    return 0
  [ -n "$node" ] || return 0
  from=$(jq -r --arg node "$node" '.nodes[$node].locked.rev // empty' <<<"$was") || return 0
  # nix moves the rev and what it reads from it, and nothing else.
  printf '%s\n%s\n' "$was" "$now" | jq -se --arg node "$node" --arg rev "$rev" '
    .[0] as $was | .[1] as $now
    | ($now.nodes[$node].locked // {}) as $locked
    | ($locked.rev // "" | strings) as $moved
    | ($moved | test("^[0-9a-f]{40}$")) and ($moved | startswith($rev))
      and ($was | .nodes[$node].locked += ($locked | with_entries(select(.key
        | IN("rev", "narHash", "lastModified", "revCount"))))) == $now' \
    >/dev/null || return 0
  wanted=$(message "$input" "$from" "$rev")
  words=$(git show --no-patch --format=%B "$theirs") || return 0
  if [ "$words" = "$wanted" ]; then
    echo "$theirs"
  fi
}

# Appends NUMBER to the JSON list LIST names.
listed_with() {
  local -n list=$1
  list=$(jq -c --argjson number "$2" '. + [$number]' <<<"$list")
}

record_folder=$(dirname -- "$record")
mkdir -p -- "$record_folder"
body=$(mktemp)
input=''
locked=''
upstream=''
step=''
written=0
trap ended EXIT

for input in "$@"; do
  locked=''
  upstream=''
  written=0

  step='cloning the repository'
  if [ ! -d "$checkout/.git" ]; then
    git clone --quiet -- "$repository" "$checkout" || exit 75
  fi
  cd -- "$checkout"
  step='fetching origin'
  git fetch --quiet origin main || exit 75

  step='reading the lock'
  lock=$(git show origin/main:flake.lock)
  node=$(jq -r --arg input "$input" '.nodes[.root].inputs[$input] // empty | strings' <<<"$lock")
  if [ -z "$node" ]; then
    echo "✕ lock-follows can't follow $input: origin's flake.lock has no input so named" >&2
    exit 78
  fi
  locked=$(jq -r --arg node "$node" '.nodes[$node].locked.rev // empty' <<<"$lock")
  # Where the input came from and which of its branches, as flake.lock
  # keeps them, so the rev is the one `nix flake update` would lock.
  source=$(jq -r --arg node "$node" '.nodes[$node].original
    | if .type == "github" then "https://github.com/\(.owner)/\(.repo)"
      elif .type == "git" then .url
      else empty end' <<<"$lock")
  ref=$(jq -r --arg node "$node" \
    '.nodes[$node].original.ref // empty | "refs/heads/" + .' <<<"$lock")
  ref=${ref:-HEAD}
  if [ -z "$source" ]; then
    echo "✕ lock-follows can't follow $input: it comes from neither GitHub nor git" >&2
    exit 78
  fi

  step='reading upstream'
  heads=$(git ls-remote -- "$source" "$ref") || exit 75
  upstream=$(awk -v ref="$ref" '$2 == ref { print $1 }' <<<"$heads")
  if [ -z "$upstream" ]; then
    echo "✕ lock-follows can't follow $input: $source has no $ref" >&2
    exit 78
  fi

  if [ "$upstream" = "$locked" ]; then
    said up-to-date '{}'
    continue
  fi

  short=${upstream:0:8}
  branch=$prefix/$input-$short
  step='listing pull requests'
  listed=$("$pr" pr list --state open --limit 100 \
    --json number,headRefName,url,isCrossRepository,isDraft) || exit 75
  # Only branches on this repository are the tool's: a fork's pull
  # request may carry the same branch name.
  open=$(jq -c 'map(select(.isCrossRepository | not))' <<<"$listed")
  already=$(jq -r --arg branch "$branch" \
    '.[] | select(.headRefName == $branch) | .url' <<<"$open")
  if [ -n "$already" ]; then
    already=$(jq -cn --arg pr "$already" '{pr: $pr}')
    said open-already "$already"
    continue
  fi
  # A closed pull request naming its branch means this rev was declined,
  # by hand or merged and reverted, whether its branch stands or was
  # deleted with it, so it isn't opened again; a newer rev opens anew.
  step='listing closed pull requests'
  listed=$("$pr" pr list --state closed --head "$branch" \
    --json number,isCrossRepository) || exit 75
  declined=$(jq -r 'map(select(.isCrossRepository | not) | .number) | first // empty' <<<"$listed")
  if [ -n "$declined" ]; then
    declined=$(jq -cn --arg branch "$branch" --argjson number "$declined" \
      '{branch: $branch, number: $number}')
    said declined "$declined"
    continue
  fi
  # Its branch standing with no pull request is an earlier run's that
  # failed before opening one, when it holds the tool's own commit, and
  # is opened on rather than pushed again; holding anything else, it is
  # someone's work, and is left as taken.
  step='reading the branch'
  standing=$(git ls-remote --quiet origin "refs/heads/$branch") || exit 75
  pushed=''
  if [ -n "$standing" ]; then
    # A failed read ends the run with ours' 75, as origin couldn't be read.
    pushed=$(ours "$branch" "$input" "$upstream")
    if [ -z "$pushed" ]; then
      taken=$(jq -cn --arg branch "$branch" '{branch: $branch}')
      said taken "$taken"
      continue
    fi
  fi
  # Any other open draft for this input locks an older rev, since
  # upstream's newest is this one. Its branch is the stem and eight hex
  # digits, so an input whose name extends this one with a dash isn't
  # caught. One marked ready is someone's word that it's to land, and is
  # left open: merge-net's train boards no draft, and marks what it
  # boards with no label. Each is replaced only while its branch holds
  # the tool's own commit, read here, before the body names it.
  step='reading the older branches'
  drafts=$(jq -r --arg stem "$prefix/$input-" --arg branch "$branch" \
    '.[] | select(.isDraft
        and (.headRefName | startswith($stem))
        and (.headRefName | ltrimstr($stem) | test("^[0-9a-f]{8}$"))
        and .headRefName != $branch)
      | "\(.number) \(.headRefName)"' <<<"$open")
  replaced=''
  while read -r number older_branch; do
    if [ -n "$number" ]; then
      theirs=$(ours "$older_branch" "$input" "${older_branch#"$prefix/$input-"}")
      if [ -n "$theirs" ]; then
        replaced+="$number $older_branch $theirs"$'\n'
      fi
    fi
  done <<<"$drafts"

  if [ -z "$pushed" ]; then
    step='updating the lock'
    git checkout --quiet --force -B "$branch" origin/main
    nix flake update "$input"
    now=$(jq -r --arg node "$node" '.nodes[$node].locked.rev // empty' flake.lock)
    if [ "$now" != "$upstream" ]; then
      echo "✕ lock-follows locked $input at ${now:-nothing}, not $upstream: upstream moved" >&2
      exit 75
    fi

    step='committing'
    words=$(message "$input" "$locked" "$upstream")
    git commit --quiet --message "$words" -- flake.lock
    step='pushing'
    # Only where origin has no such branch, leased so, as one standing is
    # never pushed over.
    git push --quiet --force-with-lease="refs/heads/$branch:" origin "HEAD:refs/heads/$branch"
  fi

  step='listing upstream merges'
  git fetch --quiet -- "$source" "+$ref:refs/lock-follows/$input" || exit 75
  # On GitHub a merge's #N is upstream's, not this repository's. Each
  # subject goes in a code span, its backticks made quotes so none ends
  # it early, as the body is posted as the account and a closing word in
  # it, as in "Fixes #12", would close what it names once merged.
  slug=''
  if [[ $source == https://github.com/* ]]; then
    slug=${source#https://github.com/}
  fi
  if [ -n "$locked" ] &&
    merges=$(git log --first-parent --format='%h %s' "$locked..$upstream"); then
    merges=$(head -n 100 <<<"$merges" | sed -E \
      -e "s/\`/'/g" \
      -e "s@(^|[^[:alnum:]/])#([0-9]+)@\\1$slug#\\2@g" \
      -e "s/^([^ ]+) (.*)$/- \\1 \`\\2\`/")
  else
    merges="- none listed: $short isn't after ${locked:0:8} in upstream's history"
  fi
  {
    echo "Moves \`$input\` from \`${locked:0:8}\` to \`$short\`, its upstream's newest."
    echo
    echo "Upstream's merges between them, newest first:"
    echo
    echo "$merges"
    while read -r number _ _; do
      if [ -n "$number" ]; then
        echo
        echo "Replaces #$number, which locks an older $input."
      fi
    done <<<"$replaced"
    echo
    echo "Opened by lock-follows, which never merges."
  } >"$body"

  step='opening the pull request'
  created=$("$pr" pr create --draft --base main --head "$branch" \
    --title "Lock ${input^} $short" --body-file "$body")
  opened=${created##*$'\n'}

  step='closing the older pull requests'
  closed='[]'
  kept='[]'
  left='[]'
  while read -r number older_branch theirs; do
    if [ -z "$number" ]; then
      continue
    fi
    # Commented on first, then closed, then its branch deleted, leased at
    # the head read: deleting it first would let GitHub close it with no
    # word of what replaced it. One whose branch moved since stays open,
    # as does one whose comment or close fails, its branch kept; one
    # whose delete fails is closed with its branch kept.
    again=$(git ls-remote --quiet origin "refs/heads/$older_branch") || again=''
    if [ "${again%%[[:space:]]*}" = "$theirs" ] &&
      "$pr" pr comment "$number" --body "Replaced by $opened, which locks $input at $short." &&
      "$pr" pr close "$number"; then
      listed_with closed "$number"
      if ! git push --quiet --force-with-lease="refs/heads/$older_branch:$theirs" \
        origin --delete "$older_branch"; then
        listed_with kept "$number"
      fi
    else
      listed_with left "$number"
    fi
  done <<<"$replaced"

  outcome=$(jq -cn --arg pr "$opened" --argjson closed "$closed" --argjson kept "$kept" \
    --argjson left "$left" '{pr: $pr, closed: $closed, closedBranchKept: $kept, leftOpen: $left}')
  said opened "$outcome"
done
