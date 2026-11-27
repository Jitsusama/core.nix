#!/usr/bin/env bash
# tuple-ends-its-flags.sh stops Tuple's daemon as it ends, so it must refuse
# to run anywhere but its check's build. Here it's run as the shells a
# person would run it from: their own, from their home or a checkout; the
# repository's dev shell, which sets NIX_BUILD_TOP; and a bare environment.
# Then it's run as the build is, which it must let through, and as the build
# with each of its four signs alone moved, which it must refuse, so each half
# of its guard is shown to refuse on its own. Each has a stand-in tuple
# first on PATH that marks being reached, so a guard that lets any of them
# through fails here rather than on the person's Tuple.
set -euo pipefail

probe=$1
# Its own scratch folder, so run anywhere it leaves nothing behind. Taken
# as the probe will see it: a shell started bare takes its folder from the
# kernel, past any link.
here=$(mktemp -d)
here=$(cd "$here" && pwd -P)
trap 'rm -rf "$here"' EXIT
mkdir -p "$here/bin" "$here/home/checkout" "$here/run-user" "$here/build"
printf '#!%s\ntouch %q\n' "$BASH" "$here/reached" > "$here/bin/tuple"
chmod +x "$here/bin/tuple"

# Runs the probe from a folder, with the environment given, and says how it
# ended in status and what it said.
run() {
  local folder=$1
  shift
  status=0
  rm -f "$here/reached"
  (cd "$folder" && env -i PATH="$here/bin:$PATH" "$@" "$BASH" "$probe") \
    > "$here/said" 2>&1 || status=$?
}

# Fails unless the probe, run so, refuses before reaching tuple.
refused() {
  local what=$1
  shift
  run "$@"
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

build=$here/build
refused "a person's shell in a checkout" "$here/home/checkout" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user"
refused "a person's shell in their home" "$here/home" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user"
refused "the repository's dev shell" "$here/home/checkout" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/run-user" NIX_BUILD_TOP="$here" IN_NIX_SHELL=impure
refused "a bare environment" "$here/home/checkout"

# As the check's build runs it, it goes on to the stand-in, which takes
# every probe and so fails the first, saying it ran.
run "$build" NIX_BUILD_TOP="$build" HOME="$build" XDG_RUNTIME_DIR="$build/run" \
  XDG_DATA_HOME="$build/data"
if [[ ! -e $here/reached || $(< "$here/said") != *"should have refused its names"* ]]; then
  echo "tuple-ends-its-flags.sh run as its build is didn't reach tuple; it ended $status," \
    "saying \"$(< "$here/said")\"" >&2
  exit 1
fi

# Each of the build's signs moved alone. A person's home holding its
# runtime and data folders, at their usual places, is the build's but for
# NIX_BUILD_TOP, which alone tells them apart.
refused "a person's home holding its runtime and data folders" "$here/home" \
  HOME="$here/home" XDG_RUNTIME_DIR="$here/home/run" XDG_DATA_HOME="$here/home/.local/share"
refused "a build whose home is elsewhere" "$build" \
  NIX_BUILD_TOP="$build" HOME="$here/home" XDG_RUNTIME_DIR="$build/run" \
  XDG_DATA_HOME="$build/data"
refused "a build whose runtime folder is the person's" "$build" \
  NIX_BUILD_TOP="$build" HOME="$build" XDG_RUNTIME_DIR="$here/run-user" \
  XDG_DATA_HOME="$build/data"
refused "a build whose data folder is the person's" "$build" \
  NIX_BUILD_TOP="$build" HOME="$build" XDG_RUNTIME_DIR="$build/run" \
  XDG_DATA_HOME="$here/home/.local/share"
# Unset, Tuple keeps its data under HOME, so it must be set.
refused "a build with no data folder" "$build" \
  NIX_BUILD_TOP="$build" HOME="$build" XDG_RUNTIME_DIR="$build/run"
# At the root, the folder's paths would begin with two slashes. The data
# folder is given so, and so are the other signs, which leaves only the
# runtime folder, as a person's would be, to refuse it.
refused "a shell at the root" / \
  NIX_BUILD_TOP=/ HOME=/ XDG_RUNTIME_DIR=/run XDG_DATA_HOME=//data
