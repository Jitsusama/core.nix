# shellcheck shell=bash
# Checks that iwlmld's block-ack session data, reorder buffers and all, is
# allocated so it may come from vmalloc, and that no file given frees it
# with kfree(), which takes only kmalloc's memory and corrupts the slab on
# vmalloc's. kfree_rcu() is kvfree_rcu() in 7.3, so it may stay. A newer
# kernel's new free path, carried onto the patch, would otherwise turn a
# refused session into a crash.
# usage: every-reorder-buffer-may-be-vmalloc.sh <mld/agg.c> [<mld/*.c>...]
set -euo pipefail

if ! grep -q 'baid_data = kvzalloc(' "$1"; then
  echo "A Wi-Fi block-ack session needs a contiguous order-5 block, which fragmentation refuses."
  exit 1
fi
if grep -nE '\bkfree\(baid_data\b' "$@"; then
  echo "A free above takes only kmalloc's memory, though the session data may be vmalloc's."
  exit 1
fi
