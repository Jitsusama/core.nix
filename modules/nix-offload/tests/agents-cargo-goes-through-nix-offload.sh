#!/usr/bin/env bash
# Proves the dev shell's cargo (cargo.sh): an agent's goes through
# `nix-offload cargo`, naming cargo itself and keeping each word whole,
# while a person's, one already running on a lent builder, one where
# nix-offload isn't installed and one where it's too old to know `cargo`
# run cargo itself. Each stand-in says the words it was given, one a line.
set -euo pipefail

shim=$1
cargo=$2

# A nix-offload that knows cargo runs what it's named when given no words,
# as offload/src/cargo.rs does, and an older one refuses the subcommand.
knowing=$(mktemp -d)
cat >"$knowing/nix-offload" <<EOF
#!$BASH
if [ "\$#" = 4 ] && [ "\$1" = cargo ]; then exec "\$3"; fi
printf '%s\n' nix-offload "\$@"
EOF
older=$(mktemp -d)
printf '#!%s\necho "nix-offload: --rate takes a price" >&2\nexit 64\n' "$BASH" \
	>"$older/nix-offload"
chmod +x "$knowing/nix-offload" "$older/nix-offload"

expect() {
	local what=$1 wanted=$2
	shift 2
	local said
	said=$("$@")
	if [ "$said" != "$wanted" ]; then
		printf '%s, but it said:\n%s\nwhere it should have said:\n%s\n' \
			"$what" "$said" "$wanted" >&2
		exit 1
	fi
}

expect "an agent's cargo didn't go through nix-offload" \
	"$(printf '%s\n' nix-offload cargo --cargo "$cargo" -- test 'a b')" \
	env -i PATH="$knowing" PI_CODING_AGENT=1 "$shim" test 'a b'

expect "a person's cargo didn't run cargo itself" \
	"$(printf '%s\n' cargo test 'a b')" \
	env -i PATH="$knowing" "$shim" test 'a b'

expect "an agent's cargo on a lent builder didn't run cargo itself" \
	"$(printf '%s\n' cargo test)" \
	env -i PATH="$knowing" PI_CODING_AGENT=1 WORK_RUNS_ELSEWHERE=1 "$shim" test

expect "an agent's cargo without nix-offload didn't run cargo itself" \
	"$(printf '%s\n' cargo test)" \
	env -i PATH=/var/empty PI_CODING_AGENT=1 "$shim" test

expect "an agent's cargo beside a nix-offload too old for it didn't run cargo itself" \
	"$(printf '%s\n' cargo test)" \
	env -i PATH="$older" PI_CODING_AGENT=1 "$shim" test
