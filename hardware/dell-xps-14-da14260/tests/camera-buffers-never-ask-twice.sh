# shellcheck shell=bash
# Checks videobuf2's scattered allocator, as patched, never asks again for
# a block size that failed and asks for blocks above a page without
# retrying, since each failed size otherwise pays compaction again and a
# camera took 40 s to open. The patch's lines are read in the allocator's
# own order, since each is right only where it sits: the allocator's flags
# and the largest size set before the loop over pieces, so the next piece
# remembers which size failed; every piece's first size held to it before
# anything is asked for; and the failed size kept right after stepping down
# from it. Each patch line must be there once, so a rebase that copies one
# can't pass by the copy, and where upstream's own lines are gone the check
# says the allocator moved rather than blaming the patch.
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

# How many lines of the function hold `$1`.
count() {
  grep -cF -- "$1" <<<"$body" || true
}

# The line number in the function of the `$2`th line holding `$1` (the
# first by default), or 0 when there's none.
at() {
  local found
  found=$(grep -nF -- "$1" <<<"$body" | sed -n "${2:-1}p" || true)
  found=${found%%:*}
  echo "${found:-0}"
}

# Upstream's own lines the patch is placed by. Without them the function
# changed upstream, and the patch needs reading against it again.
for line in 'order = get_order(size);' 'while (size > 0) {' 'while (!pages) {' \
  'split_page(pages, order);'; do
  if [ "$(count "$line")" != 1 ]; then
    echo "videobuf2's allocator no longer reads '$line' once: upstream changed it," \
      "so the camera patch needs reading against it again."
    exit 1
  fi
done

# Each of the patch's own lines, once.
for line in 'gfp_t gfp = GFP_KERNEL | __GFP_ZERO | __GFP_NOWARN | gfp_flags;' \
  'int max_order = MAX_PAGE_ORDER;' 'order = min(order, max_order);' \
  'alloc_pages(order ? gfp | __GFP_NORETRY : gfp,' 'max_order = order;'; do
  found=$(count "$line")
  if [ "$found" != 1 ]; then
    echo "The camera patch's line '$line' is in videobuf2's allocator $found times," \
      "where it belongs once, so its place can't be read."
    exit 1
  fi
done

flags=$(at 'gfp_t gfp = GFP_KERNEL | __GFP_ZERO | __GFP_NOWARN | gfp_flags;')
declared=$(at 'int max_order = MAX_PAGE_ORDER;')
pieces=$(at 'while (size > 0) {')
shrunk=$(at 'order--;' 1)
held=$(at 'order = min(order, max_order);')
tries=$(at 'while (!pages) {')
asked=$(at 'alloc_pages(order ? gfp | __GFP_NORETRY : gfp,')
stepped=$(at 'order--;' 2)
kept=$(at 'max_order = order;')
split=$(at 'split_page(pages, order);')

[ "$(count 'alloc_pages(')" = 1 ] || fail "are asked for in more than one place"
[ "$(count 'order--;')" = 2 ] || fail "step down from a size in a place the patch doesn't read"
[ "$(count 'max_order =')" = 2 ] || fail "keep which size failed in a second place"
[ "$flags" -lt "$pieces" ] || fail "lose the allocator's own flags before the first piece"
[ "$asked" -gt "$tries" ] || fail "retry for blocks above a page"
if [ "$held" -le "$shrunk" ] || [ "$held" -ge "$tries" ]; then
  fail "ask again for a failed size"
fi
[ "$declared" -lt "$pieces" ] || fail "forget between pieces which size failed"
if [ "$stepped" -le "$asked" ] || [ "$kept" != $((stepped + 1)) ] || [ "$kept" -ge "$split" ]; then
  fail "forget which size failed"
fi
