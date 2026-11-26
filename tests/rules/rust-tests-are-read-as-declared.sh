# shellcheck shell=bash
# How rust-tests-are-read-as-declared.nix holds the reader to each crate it
# plants to be refused: the reader must refuse the crate rather than read it,
# and its refusal, the last line it says, must hold the words the check
# gives, so a reader that guesses at a crate, or fails it for another reason,
# can't pass.
# usage: rust-tests-are-read-as-declared.sh <python> <reader> <crate> <says>
set -euo pipefail
python=$1 reader=$2 crate=$3 says=$4

if [[ -z ${says} ]]; then
  echo "the ${crate} crate is given no words its refusal must hold" >&2
  exit 1
fi
said=$(mktemp)
if "${python}" "${reader}" --own "${crate}/src" > "${said}.read" 2> "${said}"; then
  echo "the reader took the ${crate} crate, reading it as:" >&2
  cat "${said}.read" >&2
  exit 1
fi
refusal=$(tail -n 1 "${said}")
if [[ ${refusal} != *RefusedError:\ *"${says}"* ]]; then
  echo "the reader refused the ${crate} crate without the words '${says}':" >&2
  cat "${said}" >&2
  exit 1
fi
