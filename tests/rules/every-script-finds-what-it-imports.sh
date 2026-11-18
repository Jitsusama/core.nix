# shellcheck shell=bash
# shellcheck disable=SC2310 # The rule is run in a condition, as it must fail.
# The check every-script-finds-what-it-imports.nix builds: the rule must fail
# a planted tree where one folder's script imports a module from another
# folder, naming that folder alone and passing over an excluded one that
# imports what's nowhere, and pass it once that module is on mypy's path. It
# must fail an untyped function only when handed the argument that forbids
# one, so mypy is given its arguments, say how a run mypy can't make ended,
# by its exit or the signal that killed it, naming the tree's root as such,
# and fail a tree with no script. Folders run in name order, so kept, whose
# run caches the module, runs before lost, which must fail all the same:
# what one run caches finds nothing the next run couldn't.
# A note beside kept that isn't Python must be left out by --include, and a
# script at the root named like an option must reach mypy as a script. Only
# then does it judge the repository's own tree, so a rule that passes
# everything can't pass. The planted tree is read by a bare mypy, the
# repository's by the mypy its treefmt runs.
# usage: every-script-finds-what-it-imports.sh <python> <rule> <bare mypy>
#   <tree> [--include <glob> ...] [--exclude <glob> ...] -- <mypy>
#   [<argument> ...]
set -euo pipefail
python=$1 rule=$2 bare=$3 src=$4
shift 4

mkdir -p planted/lost planted/kept planted/skipped planted/untyped empty
printf 'from helper import SHARED\n\nprint(SHARED)\n' > planted/lost/uses.py
printf 'SHARED = 1\n' > planted/kept/helper.py
printf 'from nowhere import NOTHING\n\nprint(NOTHING)\n' > planted/skipped/uses.py
printf 'def same(given):\n    return given\n' > planted/untyped/same.py
printf 'not Python at all\n' > planted/kept/notes.txt
printf 'X = 1\n' > planted/-x.py
selected=(--include '*.py' --exclude 'skipped/*')

if "${python}" "${rule}" planted "${selected[@]}" -- "${bare}" 2> planted-said; then
  echo "the rule passed a script importing from another folder, which it must fail"
  exit 1
fi
if ! grep -q '^lost: mypy, handed only its scripts, exited 1:' planted-said \
  || ! grep -q 'failed those of lost\. ' planted-said \
  || grep -q '^\(kept\|skipped\|untyped\): ' planted-said; then
  echo "the rule failed the planted tree without naming the lost folder alone:"
  cat planted-said
  exit 1
fi

if ! MYPYPATH=${PWD}/planted/kept "${python}" "${rule}" planted "${selected[@]}" \
  -- "${bare}" 2> kept-said; then
  echo "the rule failed the planted tree with the module it imports on mypy's path:"
  cat kept-said
  exit 1
fi

if MYPYPATH=${PWD}/planted/kept "${python}" "${rule}" planted "${selected[@]}" \
  -- "${bare}" --disallow-untyped-defs 2> strict-said \
  || ! grep -q '^untyped: mypy, handed only its scripts, exited 1:' strict-said; then
  echo "the rule didn't hand mypy the argument forbidding an untyped function:"
  cat strict-said
  exit 1
fi

if "${python}" "${rule}" planted --include 'kept/*.py' --include=-x.py -- "${bare}" \
  --not-an-option 2> unread-said \
  || ! grep -q "^the tree's root: mypy, handed only its scripts, exited 2:" unread-said \
  || ! grep -q '^kept: mypy, handed only its scripts, exited 2:' unread-said \
  || ! grep -q "failed those of the tree's root, kept\. " unread-said; then
  echo "the rule didn't say how mypy ended on folders, given an option it can't read:"
  cat unread-said
  exit 1
fi

# A mypy killed by a signal must be said to be, by the signal's number.
if "${python}" "${rule}" planted --include 'kept/*.py' -- "${BASH}" -c 'kill -TERM $$' \
  2> killed-said \
  || ! grep -q '^kept: mypy, handed only its scripts, was killed by signal 15:' killed-said; then
  echo "the rule didn't say mypy was killed, given one a signal kills:"
  cat killed-said
  exit 1
fi

if "${python}" "${rule}" empty --include '*.py' -- "${bare}" 2> empty-said \
  || ! grep -q 'holds no file mypy is handed' empty-said; then
  echo "the rule didn't fail a tree holding no script for holding none:"
  cat empty-said
  exit 1
fi

"${python}" "${rule}" "${src}" "$@"
