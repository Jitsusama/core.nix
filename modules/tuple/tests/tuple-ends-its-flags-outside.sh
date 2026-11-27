#!/usr/bin/env bash
# tuple-ends-its-flags.sh stops Tuple's daemon as it ends, so it must refuse
# to run anywhere but its check's build. Here it's run as the shells a
# person would run it from: their own, from their home or a checkout; the
# repository's dev shell, which sets NIX_BUILD_TOP; a bare environment; and
# shells that carry only some of the build's folders. Each has a stand-in
# tuple first on PATH that marks being reached, so a guard that lets any of
# them through fails here rather than on the person's Tuple.
set -euo pipefail

probe=$1
# Its own scratch folder, so run anywhere it leaves nothing behind. Taken
# as the probe will see it: a shell started bare takes its folder from the
# kernel, past any link.
here=$(mktemp -d)
here=$(cd "$here" && pwd -P)
trap 'rm -rf "$here"' EXIT
mkdir -p "$here/bin" "$here/home/checkout" "$here/run-user"
printf '#!%s\ntouch %q\n' "$BASH" "$here/reached" > "$here/bin/tuple"
chmod +x "$here/bin/tuple"

# Runs the probe from a folder, with the environment given, and fails unless
# it refuses before reaching tuple.
refused() {
  local what=$1 folder=$2
  shift 2
  local status=0
  (cd "$folder" && env -i PATH="$here/bin:$PATH" "$@" "$BASH" "$probe") \
    > "$here/said" 2>&1 || status=$?
  if [[ -e $here/reached ]]; then
    echo "tuple-ends-its-flags.sh reached tuple from $what; it said \"$(< "$here/said")\"" >&2
    exit 1
  fi
  if [[ $status -ne 1 || $(< "$here/said") != *"runs only in its check's build"* ]]; then
    echo "tuple-ends-its-flags.sh from $what ended $status, not refusing;" \
      "it said \"$(< "$here/said")\"" >&2
    exit 1
  fi
}

refused "a person's shell in a checkout" "$here/home/checkout" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user"
# Its home is the folder it runs in, as the build's is, but not its
# runtime folder.
refused "a person's shell in their home" "$here/home" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user"
refused "the repository's dev shell" "$here/home/checkout" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user" NIX_BUILD_TOP="$here" IN_NIX_SHELL=impure
refused "a bare environment" "$here/home/checkout"
# Nor is the runtime folder alone enough: the daemon is found by both, and
# Tuple's account and log are kept in its data folder.
refused "a shell with only its runtime folder in the folder" "$here/home" \
  HOME="$here" XDG_RUNTIME_DIR="$here/home/run"
refused "a shell whose data folder is the person's" "$here/home" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/home/run" XDG_DATA_HOME="$here/data"
# With no data folder set, Tuple keeps it under the person's home, so a
# home holding its own runtime folder is still the person's.
refused "a person's home holding its runtime folder" "$here/home" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/home/run"
# At the root, the folder's paths would begin with two slashes, which no
# runtime folder does, so that half alone refuses it.
refused "a shell at the root" / HOME=/ XDG_RUNTIME_DIR=/run
