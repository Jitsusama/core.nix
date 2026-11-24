#!/usr/bin/env bash
# The dev shell's cargo: an agent's goes through `nix-offload cargo`, which
# sends a whole suite of a committed tree to a lent builder and runs the
# rest here in night-shift's turn, one heavy job at a time on the machine
# (offload/src/cargo.rs): a build, check or doc, and a suite of a tree
# with changes or with no builder standing. A person's, one already
# running elsewhere, and one where the nix-offload installed is too old to
# know `cargo` is cargo itself. It passes cargo's own words and answers
# on, so it has no help of its own.
#
# An agent is a pi session's command: pi sets PI_SESSION_ID in every shell
# it starts (as pineapple reads it), and in a terminal PI_CODING_AGENT too,
# which a session in the shared pi host doesn't, so either counts. pi
# leaves both out of a terminal it opens for the person (the harness's
# SESSION_ONLY_VARIABLES), though a server an agent started, such as
# tmux, hands its own on; such a person's cargo then goes too, saying so,
# and NIX_OFFLOAD_CARGO=here runs it here.
set -euo pipefail

# Asked with no words, a nix-offload that knows `cargo` runs what it's
# named, here true; an older one refuses, as it would the real run, so
# a machine not yet switched to it keeps its cargo working.
if [ -n "${PI_CODING_AGENT:-}${PI_SESSION_ID:-}" ] && [ -z "${WORK_RUNS_ELSEWHERE:-}" ] &&
	command -v nix-offload >/dev/null 2>&1 &&
	nix-offload cargo --cargo @true@ -- >/dev/null 2>&1; then
	exec nix-offload cargo --cargo @cargo@ -- "$@"
fi
exec @cargo@ "$@"
