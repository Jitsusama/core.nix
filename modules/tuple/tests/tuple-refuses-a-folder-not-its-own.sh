#!/usr/bin/env bash
# Two of the scripts that run the real tuple refuse, ending 64 before any
# tuple runs, folders that would let it reach a person's own. Given portal,
# tuple-shares-through-the-portal.sh, given second, with the folder library
# given third, refuses a data folder to find that isn't a whole path or
# ends in a slash, and moves no step for one that is only the tail of the
# steps' own. Given editors, tuple-completion.zsh, given second,
# refuses any folders but those of the folder of its own its wrapper makes,
# a person's home holding its own runtime and data folders among them, and
# lets that folder's own through. Each is run from a person's home, with a
# stand-in tuple first on PATH that marks being reached, so a refusal that
# comes too late fails here rather than on the person's Tuple. The checks
# that run them for real show what they let through.
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
# Steps that write nothing, and a completion that only runs tuple, as
# Tuple's own does on each Tab, so a script that got past its refusal
# would only go on to reach tuple, even one that refused only after its
# completion had run.
echo 'run true' > "$here/activate"
printf '#compdef tuple\ntuple\n' > "$here/_tuple"

# Every case is run, and each that fails says so, so a red names all the
# cases a change breaks rather than only the first.
failed=0

# Runs the script from the home with the environment and command given,
# held to a time in case it went on to wait on tuple, keeping its status
# in status and what it said in said.
run_from_home() {
  status=0
  rm -f "$here/reached"
  (cd "$home" && env -i PATH="$here/bin:$PATH" "$@") > "$here/said" 2>&1 < /dev/null ||
    status=$?
}

# Fails unless the script, run from the home with the environment and
# command given, ends 64 without reaching tuple.
turned_away() {
  local why=$1
  shift
  run_from_home "$@"
  if [[ $status -ne 64 || -e $here/reached ]]; then
    local reached="didn't reach tuple"
    if [[ -e $here/reached ]]; then reached="reached tuple"; fi
    echo "${script##*/} with $why ended $status and $reached, not refusing;" \
      "it said \"$(< "$here/said")\"" >&2
    failed=1
  fi
}

# Fails unless the script, run from the home with the environment and
# command given, gets past its refusal and reaches tuple, so a script that
# refused everything couldn't pass for one refusing only what it should.
let_through() {
  local why=$1
  shift
  run_from_home "$@"
  if [[ $status -eq 64 || ! -e $here/reached ]]; then
    echo "${script##*/} with $why ended $status and didn't reach tuple, refusing it;" \
      "it said \"$(< "$here/said")\"" >&2
    failed=1
  fi
}

case $what in
  portal)
    # Could one ending in a slash, were it let through, match part of a
    # step's folder? No: the script finds the data folder only where a word
    # starts and with a slash after it, so one ending in a slash is found
    # only where a step writes two, and then as the whole of a folder. It's
    # refused all the same, as a sign that what was given was a mistake.
    for data in "" share share/ /x/ /; do
      turned_away "the data folder \"$data\"" HOME="$home" \
        timeout -k 5 60 "$BASH" "$script" "$library" "$here/activate" "$data"
    done
    # A whole path that is only the tail of a step's real folder can't be
    # told apart before the steps are read, so it's held to finding no step
    # it may move: matched inside the step's folder, it would write into the
    # home, at the home's path with the folder of its own's put after it.
    # Its library's own folder and stop are its own, so only the home and
    # what it said are read.
    printf 'run mkdir -p %q\n' "$home/.local/share/tuple/0/settings" > "$here/activate"
    status=0
    (cd "$home" && env -i PATH="$here/bin:$PATH" HOME="$home" TMPDIR="$here" \
      timeout -k 5 60 "$BASH" "$script" "$library" "$here/activate" /.local/share) \
      > "$here/said" 2>&1 < /dev/null || status=$?
    (cd "$home" && find . -mindepth 1 ! -path ./run ! -path ./data) > "$here/written"
    if [[ $status -ne 1 || -s $here/written ]] ||
      ! grep -q 'has no step writing into /.local/share/tuple/' "$here/said"; then
      echo "${script##*/} with a tail of the data folder ended $status and wrote" \
        "\"$(< "$here/written")\" into the home; it said \"$(< "$here/said")\"" >&2
      failed=1
    fi
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
    # One case to a folder, so a script checking only one of them can't
    # pass on the other's refusal.
    editors "a runtime folder below its own" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$home/run/x" \
      XDG_DATA_HOME="$home/data"
    editors "a data folder below its own" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$home/run" \
      XDG_DATA_HOME="$home/data/x"
    # Last, since the completion it lets through writes into the home.
    let_through "the folder of its own's name, home and folders, run from it" \
      TUPLE_FOLDER_OF_ITS_OWN="$home" HOME="$home" XDG_RUNTIME_DIR="$home/run" \
      XDG_DATA_HOME="$home/data" timeout -k 5 60 zsh -f "$script" "$here/_tuple"
    ;;
  *)
    echo "tuple-refuses-a-folder-not-its-own.sh takes portal or editors, not \"$what\"" >&2
    exit 64
    ;;
esac
exit "$failed"
