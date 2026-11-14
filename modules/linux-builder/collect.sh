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
#
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand, asked as its first word or over ssh.
set -euo pipefail

case "${1:-${SSH_ORIGINAL_COMMAND-}}" in
-h | --help)
  cat << 'EOF'
collect  collect the Linux builder's store garbage

  What a key the builder takes only for collecting runs,
  in place of what its holder asked: all that's dead, or
  at most GIB GiB, or nothing for free. It then says how
  many bytes the store has free.

EXAMPLES
  ssh linux-builder collect 20
  ssh linux-builder free

EXIT
  0   collected, saying the bytes free
  64  asked wrongly
  else nix-store's own
EOF
  exit 0
  ;;
*) ;;
esac

asked=${SSH_ORIGINAL_COMMAND-collect}
if [[ ${asked} == collect ]]; then
  nix-store --gc >&2
elif [[ ${asked} =~ ^collect\ ([1-9][0-9]{0,5})$ ]]; then
  nix-store --gc --max-freed "$((BASH_REMATCH[1] * 1024 * 1024 * 1024))" >&2
elif [[ ${asked} != free ]]; then
  printf '%s\n' "collect takes collect, collect GIB or free" \
    "  try  collect --help" >&2
  exit 64
fi
df -B1 --output=avail /nix/store | tail -n 1
