# shellcheck shell=bash
# Checks videobuf2's scattered allocator, as patched, never asks again for
# a block size that failed and asks for blocks above a page without
# retrying, since each failed size otherwise pays compaction again and a
# camera took 40 s to open. The patch's lines are read in the allocator's
# own order, since each is right only where it sits: the failed size kept
# right after stepping down from it, by a variable outside the loop over
# pieces so the next piece remembers it, and every piece's first size held
# to it before anything is asked for.
# usage: camera-buffers-never-ask-twice.sh <videobuf2-dma-sg.c>
set -euo pipefail

allocator=$1
fail() {
  echo "The camera's buffers $1, so opening it can compact for seconds."
  exit 1
}

# The allocator's function alone, from its name to its closing brace.
body=$(awk '/^static int vb2_dma_sg_alloc_compacted\(/ { on = 1 }
  on { print }
  on && /^}/ { exit }' "$allocator")
[ -n "$body" ] || fail "have no vb2_dma_sg_alloc_compacted to read"

# The line number in the function of the `$2`th line holding `$1` (the
# first by default), or 0 when there's none.
at() {
  local found
  found=$(grep -nF -- "$1" <<<"$body" | sed -n "${2:-1}p" || true)
  found=${found%%:*}
  echo "${found:-0}"
}

declared=$(at 'int max_order = MAX_PAGE_ORDER;')
pieces=$(at 'while (size > 0) {')
shrunk=$(at 'order--;' 1)
held=$(at 'order = min(order, max_order);')
tries=$(at 'while (!pages) {')
asked=$(at 'alloc_pages(order ? gfp | __GFP_NORETRY : gfp,')
stepped=$(at 'order--;' 2)
kept=$(at 'max_order = order;')
split=$(at 'split_page(pages, order);')

[ "$asked" -gt "$tries" ] || fail "retry for blocks above a page"
if [ "$held" -le "$shrunk" ] || [ "$held" -ge "$tries" ]; then
  fail "ask again for a failed size"
fi
if [ "$declared" = 0 ] || [ "$declared" -ge "$pieces" ]; then
  fail "forget between pieces which size failed"
fi
if [ "$stepped" -le "$asked" ] || [ "$kept" != $((stepped + 1)) ] || [ "$kept" -ge "$split" ]; then
  fail "forget which size failed"
fi
