#!/usr/bin/env bash
# Signs what a Mac's Nix just built with the Mac's own key, as Nix's post-build
# hook, which Nix runs after each build, one a builder did included, with the
# paths in OUT_PATHS. Nix signs only what it builds itself, so what the Mac's
# Linux builder built came back unsigned, and another machine that takes the
# Mac's work by its key refused it (15 November). Signing a path the Mac
# already signed changes nothing.
#
# A signing that fails is said and let be: the path stays unsigned, which a
# machine taking it refuses, rather than failing a build that passed.
#
# Usage: sign-what-came-back.sh NIX, as darwin.nix's hook runs it.
set -uo pipefail

nix=$1
read -r -a paths <<<"${OUT_PATHS:-}"
if [ "${#paths[@]}" -gt 0 ] &&
  ! "$nix" store sign --key-file /etc/nix/signing-key "${paths[@]}"; then
  echo "sign-what-came-back: ${paths[*]} stays unsigned" >&2
fi
exit 0
