#!/usr/bin/env bash
# tuple-in-a-folder-of-its-own.sh, given first, promises the checks that
# source it that their tuple stop reaches only a daemon started in its
# folder, never a person's. The checks themselves run in a build, where no
# person's daemon is, so they can't show it; here it's sourced by scripts
# run as a person's shell would, with a stand-in tuple first on PATH that
# writes down where its stop was sent. It must see only the folder's own
# home and every folder XDG names, the runtime folder already made, and no
# session bus or display of the person's, the folder made under the TMPDIR
# the script is given. The folder must go once a script ends, passing or
# failing, its status kept, and stay, saying how to stop its daemon, only
# when the stop is cut off, its status kept even when there's no stderr to
# say it to; and a runtime folder that can't be made must end the script
# before any tuple runs.
set -euo pipefail

library=$1
here=$(mktemp -d)
trap 'rm -rf "$here"' EXIT
mkdir -p "$here/bin" "$here/home" "$here/run-user" "$here/tmp"
# The stand-in does what the file `does` says as it's stopped: nothing,
# fail, or hang until it's cut off.
{
  printf '#!%s\n' "$BASH"
  printf 'seen=%q does=%q\n' "$here/seen" "$here/does"
  cat << 'EOF'
printf '%s\n' "$*" "$HOME" "$XDG_RUNTIME_DIR" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" \
  "$XDG_CACHE_HOME" "$XDG_STATE_HOME" "$PWD" > "$seen"
if [[ -d $XDG_RUNTIME_DIR ]]; then echo made >> "$seen"; fi
echo "desktop:${DBUS_SESSION_BUS_ADDRESS-}${DISPLAY-}${WAYLAND_DISPLAY-}" >> "$seen"
case $(< "$does") in
  fail) exit 1 ;;
  # Longer than the library's ten seconds for a stop, so it's cut off.
  hang) exec sleep 60 ;;
esac
EOF
} > "$here/bin/tuple"
chmod +x "$here/bin/tuple"
# A script that sources the library given first, writes down the folder it
# was moved into where the second says, and ends with the third's status.
cat > "$here/script" << 'EOF'
set -euo pipefail
source "$1"
pwd > "$2"
exit "$3"
EOF
# A folder mounted noexec would let the stand-in be passed over for the
# real tuple further along PATH, so it must run before anything is tried.
echo > "$here/does"
if ! "$here/bin/tuple" || [[ ! -e $here/seen ]]; then
  echo "the stand-in tuple in $here doesn't run; is TMPDIR mounted noexec?" >&2
  exit 1
fi

# A mkdir that can't make a runtime folder, writing down which it was
# asked for, and making every other folder as the real one does.
mkdir -p "$here/no-runtime"
real_mkdir=$(command -v mkdir)
{
  printf '#!%s\n' "$BASH"
  printf 'asked=%q mkdir=%q\n' "$here/asked" "$real_mkdir"
  cat << 'EOF'
for last; do :; done
if [[ $last == */run ]]; then
  printf '%s\n' "$last" > "$asked"
  exit 1
fi
exec "$mkdir" "$@"
EOF
} > "$here/no-runtime/mkdir"
chmod +x "$here/no-runtime/mkdir"

# Sources the library as a script would, from the person's home with their
# own folders, its stand-in stop doing what's given, the script ending with
# the status given, any folder given third first on PATH, and its stderr
# closed when the fourth says so; it writes down the folder it was moved
# into, once it's moved.
run() {
  local does=$1 ends=$2 first=${3:+$3:} stderr=${4:-}
  echo "$does" > "$here/does"
  rm -f "$here/seen" "$here/folder"
  local -a sourcing=(env -i PATH="$first$here/bin:$PATH" TMPDIR="$here/tmp"
    HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user"
    XDG_DATA_HOME="$here/home/.local/share" XDG_CONFIG_HOME="$here/home/.config"
    XDG_CACHE_HOME="$here/home/.cache" XDG_STATE_HOME="$here/home/.local/state"
    DBUS_SESSION_BUS_ADDRESS=unix:path=/nowhere DISPLAY=:9 WAYLAND_DISPLAY=wayland-9
    "$BASH" "$here/script" "$library" "$here/folder" "$ends")
  status=0
  if [[ $stderr == closed ]]; then
    (cd "$here/home" && "${sourcing[@]}") > "$here/said" 2>&- || status=$?
  else
    (cd "$here/home" && "${sourcing[@]}") > "$here/said" 2>&1 || status=$?
  fi
  folder=
  if [[ -e $here/folder ]]; then folder=$(< "$here/folder"); fi
}

# Fails unless the stand-in's stop was sent to the folder alone, its
# runtime folder made, and the script ended as it would have without it.
stopped_within() {
  local what=$1 ended=$2
  local -a seen
  if [[ ! -e $here/seen ]]; then
    echo "a script that $what never sent tuple its stop; it said \"$(< "$here/said")\"" >&2
    exit 1
  fi
  mapfile -t seen < "$here/seen"
  local wanted=("stop" "$folder" "$folder/run" "$folder/data" "$folder/.config"
    "$folder/.cache" "$folder/.local/state" "$folder" "made" "desktop:")
  if [[ ${seen[*]} != "${wanted[*]}" || $folder != "$here/tmp/"* ]]; then
    echo "a script that $what sent tuple ${seen[*]}, not ${wanted[*]}" >&2
    exit 1
  fi
  if [[ $status -ne $ended ]]; then
    echo "a script that $what ended $status, not $ended; it said \"$(< "$here/said")\"" >&2
    exit 1
  fi
}

run "" 0
stopped_within "passed" 0
if [[ -e $folder ]]; then
  echo "a script that passed left $folder behind" >&2
  exit 1
fi
run "" 3
stopped_within "failed" 3
if [[ -e $folder ]]; then
  echo "a script that failed left $folder behind" >&2
  exit 1
fi
# A stop that fails is no sign a daemon is left, as one that never started
# fails too, so the folder goes.
run fail 0
stopped_within "passed with a stop that failed" 0
if [[ -e $folder ]]; then
  echo "a script whose stop failed left $folder behind" >&2
  exit 1
fi
# A runtime folder that can't be made ends the script before any tuple
# runs, the stop among them, which might otherwise fall back to another.
run "" 0 "$here/no-runtime"
if [[ ! -e $here/asked ]]; then
  echo "a script whose runtime folder couldn't be made never asked for one;" \
    "it ended $status, saying \"$(< "$here/said")\"" >&2
  exit 1
fi
asked=$(< "$here/asked")
if [[ $status -ne 1 || -e $here/seen || -e ${asked%/run} ]]; then
  sent="not sent" kept=removed
  if [[ -e $here/seen ]]; then sent=sent; fi
  if [[ -e ${asked%/run} ]]; then kept=kept; fi
  echo "a script whose runtime folder couldn't be made ended $status, its stop" \
    "$sent, ${asked%/run} $kept" >&2
  exit 1
fi
# Cut off, the stop may have left a daemon that finds its folder there.
run hang 0
stopped_within "passed with a stop that hung" 0
if [[ ! -d $folder || $(< "$here/said") != *"HOME=$(printf %q "$folder") "* ]]; then
  echo "a script whose stop hung didn't keep $folder and say how to stop it;" \
    "it said \"$(< "$here/said")\"" >&2
  exit 1
fi
rm -rf "$folder"
# Saying so to a closed stderr fails, which mustn't end a passing script
# as failed.
run hang 0 "" closed
stopped_within "passed with a stop that hung and no stderr" 0
if [[ ! -d $folder ]]; then
  echo "a script whose stop hung with no stderr didn't keep $folder" >&2
  exit 1
fi
rm -rf "$folder"
