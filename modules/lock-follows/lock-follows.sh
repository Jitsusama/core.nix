# shellcheck shell=bash
# Follows each flake input named to its upstream's newest commit, through
# a draft pull request on the repository whose lock holds it, so a change
# merged upstream reaches the lock without waiting on a hand bump. It reads
# the rev origin's main locks the input at, and upstream's newest with git
# ls-remote, from the source flake.lock says the input came from. When they
# differ and no open pull request has the branch for that rev, it cuts
# PREFIX/INPUT-REV8 from origin's main, runs `nix flake update INPUT`, and
# commits the lock alone, as the account's git config signs. It pushes that
# branch, never main, and opens a draft whose body lists upstream's merges
# between the two revs. A newer rev opens a new pull request and closes the
# older, naming each in the other. It never merges.
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
  --pr COMMAND       the command PRs are listed, opened and
                     closed with, taking gh's words (gh)

EXIT
  0   each input locked at upstream's newest, or a PR open
  1   a step failed, named in its line
  64  asked wrongly
  75  origin or upstream couldn't be read; tried next run
  78  an input isn't in origin's flake.lock
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
    exit 1
  fi

  if [ "$upstream" = "$locked" ]; then
    said up-to-date '{}'
    continue
  fi

  short=${upstream:0:8}
  branch=$prefix/$input-$short
  step='listing pull requests'
  # Only branches on this repository are the tool's: a fork's pull
  # request may carry the same branch name.
  open=$("$pr" pr list --state open --limit 100 \
    --json number,headRefName,url,isCrossRepository |
    jq -c 'map(select(.isCrossRepository | not))')
  already=$(jq -r --arg branch "$branch" \
    '.[] | select(.headRefName == $branch) | .url' <<<"$open")
  if [ -n "$already" ]; then
    already=$(jq -cn --arg pr "$already" '{pr: $pr}')
    said open-already "$already"
    continue
  fi
  # Any other open for this input locks an older rev, since upstream's
  # newest is this one. Its branch is the stem and eight hex digits, so
  # an input whose name extends this one with a dash isn't caught.
  older=$(jq -r --arg stem "$prefix/$input-" --arg branch "$branch" \
    '.[] | select((.headRefName | startswith($stem))
        and (.headRefName | ltrimstr($stem) | test("^[0-9a-f]{8}$"))
        and .headRefName != $branch)
      | "\(.number) \(.headRefName)"' <<<"$open")

  step='updating the lock'
  git checkout --quiet --force -B "$branch" origin/main
  nix flake update "$input"
  now=$(jq -r --arg node "$node" '.nodes[$node].locked.rev // empty' flake.lock)
  if [ "$now" != "$upstream" ]; then
    echo "✕ lock-follows locked $input at ${now:-nothing}, not $upstream: upstream moved" >&2
    exit 75
  fi

  step='committing'
  git commit --quiet --message "build($input): lock $input $short" \
    --message "Moves $input from ${locked:0:8} to $short, its upstream's newest." \
    -- flake.lock
  step='pushing'
  git push --quiet --force origin "HEAD:refs/heads/$branch"

  step='listing upstream merges'
  git fetch --quiet -- "$source" "+$ref:refs/lock-follows/$input"
  # On GitHub a merge's #N is upstream's, not this repository's.
  slug=''
  if [[ $source == https://github.com/* ]]; then
    slug=${source#https://github.com/}
  fi
  if [ -n "$locked" ] &&
    merges=$(git log --first-parent --format='- %h %s' "$locked..$upstream"); then
    merges=$(head -n 100 <<<"$merges" | sed -E "s@(^|[^[:alnum:]/])#([0-9]+)@\\1$slug#\\2@g")
  else
    merges="- none listed: $short isn't after ${locked:0:8} in upstream's history"
  fi
  {
    echo "Moves \`$input\` from \`${locked:0:8}\` to \`$short\`, its upstream's newest."
    echo
    echo "Upstream's merges between them, newest first:"
    echo
    echo "$merges"
    while read -r number _; do
      if [ -n "$number" ]; then
        echo
        echo "Replaces #$number, which locks an older $input."
      fi
    done <<<"$older"
    echo
    echo "Opened by lock-follows, which never merges."
  } >"$body"

  step='opening the pull request'
  created=$("$pr" pr create --draft --base main --head "$branch" \
    --title "Lock ${input^} $short" --body-file "$body")
  opened=${created##*$'\n'}

  step='closing the older pull requests'
  closed='[]'
  while read -r number older_branch; do
    if [ -n "$number" ]; then
      "$pr" pr close "$number" --comment "Replaced by $opened, which locks $input at $short."
      # Its branch goes only while it holds the tool's one commit on
      # main, so work someone pushed onto it stays.
      if git fetch --quiet origin "refs/heads/$older_branch" &&
        theirs=$(git rev-parse FETCH_HEAD) &&
        [ "$(git rev-list --count origin/main.."$theirs")" = 1 ] &&
        [[ "$(git log -1 --format=%s "$theirs")" == "build($input): lock $input "* ]]; then
        git push --quiet --force-with-lease="refs/heads/$older_branch:$theirs" \
          origin --delete "$older_branch"
      fi
      closed=$(jq -c --argjson number "$number" '. + [$number]' <<<"$closed")
    fi
  done <<<"$older"

  outcome=$(jq -cn --arg pr "$opened" --argjson closed "$closed" '{pr: $pr, closed: $closed}')
  said opened "$outcome"
done
