#!/usr/bin/env bash
# What a key the Linux builder's VM takes only for collecting may do. It
# runs as the builder in place of whatever its holder asked: collect the
# store's garbage, all that's dead or up to a number of GiB, then print
# how many bytes the store has free. darwin.nix says why the door is this
# narrow.
#
#   collect      collect all that's dead (what a bare ssh asks)
#   collect GIB  collect at most GIB GiB
#   free         collect nothing
set -euo pipefail

asked=${SSH_ORIGINAL_COMMAND-collect}
if [[ ${asked} == collect ]]; then
  nix-store --gc >&2
elif [[ ${asked} =~ ^collect\ ([1-9][0-9]{0,5})$ ]]; then
  nix-store --gc --max-freed "$((BASH_REMATCH[1] * 1024 * 1024 * 1024))" >&2
elif [[ ${asked} != free ]]; then
  echo "usage: collect [GIB] | free" >&2
  exit 64
fi
df -B1 --output=avail /nix/store | tail -n 1
