#!/usr/bin/env bash
# Two of the scripts that run the real tuple refuse, ending 64 before any
# tuple runs, folders that would let it reach a person's own. Given portal,
# tuple-shares-through-the-portal.sh, given second, with the folder library
# given third, refuses a data folder to find that isn't a whole path or
# ends in a slash. Given editors, tuple-completion.zsh, given second,
# refuses any folders but those of the folder of its own its wrapper makes,
# a person's home holding its own runtime and data folders among them. Each
# is run from a person's home, with a stand-in tuple first on PATH that
# marks being reached, so a refusal that comes too late fails here rather
# than on the person's Tuple. The checks that run them for real show what
# they let through.
set -euo pipefail

what=$1
script=$(realpath -- "$2")
library=${3:-}
# Taken as zsh will see it: started bare, a shell takes its folder from the
# kernel, past any link.
here=$(mktemp -d)
here=$(cd "$here" && pwd -P)
trap 'rm -rf "$here"' EXIT
home=$here/home
mkdir -p "$here/bin" "$home/run" "$home/data"
printf '#!%s\ntouch %q\n' "$BASH" "$here/reached" > "$here/bin/tuple"
chmod +x "$here/bin/tuple"
# A folder mounted noexec would let the stand-in be passed over for the
# real tuple further along PATH, so it must run before anything is tried.
if ! "$here/bin/tuple" || [[ ! -e $here/reached ]]; then
  echo "the stand-in tuple in $here doesn't run; is TMPDIR mounted noexec?" >&2
  exit 1
fi
# Steps that write nothing, and a completion that completes nothing, so a
# script that got past its refusal would only go on to reach tuple.
echo 'run true' > "$here/activate"
: > "$here/_tuple"

# Fails unless the script, run from the home with the environment and
# command given, ends 64 without reaching tuple, held to a time in case it
# went on to wait on one.
turned_away() {
  local why=$1
  shift
  local status=0
  rm -f "$here/reached"
  (cd "$home" && env -i PATH="$here/bin:$PATH" "$@") > "$here/said" 2>&1 < /dev/null ||
    status=$?
  if [[ $status -ne 64 || -e $here/reached ]]; then
    local reached="didn't reach tuple"
    if [[ -e $here/reached ]]; then reached="reached tuple"; fi
    echo "${script##*/} with $why ended $status and $reached, not refusing;" \
      "it said \"$(< "$here/said")\"" >&2
    exit 1
  fi
}

case $what in
  portal)
    for data in "" share share/ /x/ /; do
      turned_away "the data folder \"$data\"" HOME="$home" \
        timeout -k 5 60 "$BASH" "$script" "$library" "$here/activate" "$data"
    done
    ;;
  editors)
    editors() { turned_away "$1" "${@:2}" timeout -k 5 60 zsh -f "$script" "$here/_tuple"; }
    editors "a home holding its own runtime and data folders" \
      HOME="$home" XDG_RUNTIME_DIR="$home/run" XDG_DATA_HOME="$home/data"
    editors "the folder's name, but the person's runtime folder" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$here/run-user" \
      XDG_DATA_HOME="$home/data"
    editors "the folder's name, but the person's data folder" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$home/run" \
      XDG_DATA_HOME="$here/data-user"
    editors "the folder's name, but the person's home" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$here" XDG_RUNTIME_DIR="$home/run" \
      XDG_DATA_HOME="$home/data"
    editors "another folder's name" \
      TUPLE_FOLDER_OF_ITS_OWN="$here" HOME="$home" XDG_RUNTIME_DIR="$home/run" \
      XDG_DATA_HOME="$home/data"
    editors "runtime and data folders below their own" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$home/run/x" \
      XDG_DATA_HOME="$home/data/x"
    ;;
  *)
    echo "tuple-refuses-a-folder-not-its-own.sh takes portal or editors, not \"$what\"" >&2
    exit 64
    ;;
esac
