# shellcheck shell=bash
# Checks videobuf2's scattered allocator, as patched, never asks again for
# a block size that failed and asks for blocks above a page without
# retrying, since each failed size otherwise pays compaction again and a
# camera took 40 s to open.
# usage: camera-buffers-never-ask-twice.sh <videobuf2-dma-sg.c>
set -euo pipefail

allocator=$1
fail() {
  echo "The camera's buffers $1, so opening it can compact for seconds."
  exit 1
}
grep -qF 'order = min(order, max_order);' "$allocator" || fail "ask again for a failed size"
grep -qF 'max_order = order;' "$allocator" || fail "forget which size failed"
grep -qF 'alloc_pages(order ? gfp | __GFP_NORETRY : gfp,' "$allocator" ||
  fail "retry for blocks above a page"
