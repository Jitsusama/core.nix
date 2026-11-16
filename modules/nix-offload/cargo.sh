#!/usr/bin/env bash
# The dev shell's cargo: an agent's goes through `nix-offload cargo`, which
# sends a whole suite to a lent builder and runs the rest here
# (offload/src/cargo.rs); a person's, one already running elsewhere, and
# one where the nix-offload installed is too old to know `cargo` is cargo
# itself. It passes cargo's own words and answers on, so it has no help of
# its own.
set -euo pipefail

# Asked with no words, a nix-offload that knows `cargo` runs what it's
# named, here true; an older one refuses, as it would the real run, so
# a machine not yet switched to it keeps its cargo working.
if [ -n "${PI_CODING_AGENT:-}" ] && [ -z "${WORK_RUNS_ELSEWHERE:-}" ] &&
	command -v nix-offload >/dev/null 2>&1 &&
	nix-offload cargo --cargo @true@ -- >/dev/null 2>&1; then
	exec nix-offload cargo --cargo @cargo@ -- "$@"
fi
exec @cargo@ "$@"
