# shellcheck shell=bash
# What rust-tests-are-read-as-declared.nix holds each crate it plants to be
# refused to: the reader must refuse it rather than read it, saying what it
# expects, so a reader that guesses at a crate can't pass.
# usage: rust-tests-are-read-as-declared.sh <python> <reader> <crate> <says>
set -euo pipefail
python=$1 reader=$2 crate=$3 says=$4

if "${python}" "${reader}" --own "${crate}/src" > /dev/null 2> said; then
  echo "the reader took the ${crate} crate" >&2
  exit 1
fi
if ! grep -qF -- "${says}" said; then
  echo "the reader refused the ${crate} crate saying otherwise than '${says}':" >&2
  cat said >&2
  exit 1
fi
