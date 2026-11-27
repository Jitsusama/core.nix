# shellcheck shell=bash
# Checks that iwlmld's block-ack session data, reorder buffers and all, is
# allocated so it may come from vmalloc, and that no file given frees it
# with kfree(), which takes only kmalloc's memory and corrupts the slab on
# vmalloc's. kfree_rcu() is kvfree_rcu() in 7.3, so it may stay. A newer
# kernel's new free path, carried onto the patch, would otherwise turn a
# refused session into a crash. The driver names the session data
# baid_data, ba_data or data from one function to the next, so each
# function's own names for it are read, not one name.
# usage: every-reorder-buffer-may-be-vmalloc.sh <mld/agg.c> [<mld/*.c>...]
set -euo pipefail

if ! grep -q 'baid_data = kvzalloc(' "$1"; then
  echo "A Wi-Fi block-ack session needs a contiguous order-5 block, which fragmentation refuses."
  exit 1
fi
# Within each function, from its opening line to the brace closing it at the
# line's start, a name declared as the session data's pointer, as a
# parameter or a local, the first on its declaration or one after, is held;
# a kfree() of a name held is reported. So are a kfree() of what the
# session table holds, whatever reads it out, and the session data's pointer
# declared to be kfree()'d as it leaves its scope.
freed=$(awk '
  /^[A-Za-z].*\(/ { delete names }
  {
    line = $0
    while (match(line, /struct iwl_mld_baid_data \*[A-Za-z_][A-Za-z0-9_]*/)) {
      name = substr(line, RSTART, RLENGTH)
      sub(/.*\*/, "", name)
      names[name] = 1
      line = substr(line, RSTART + RLENGTH)
      while (match(line, /^[^;)(]*, *\*[A-Za-z_][A-Za-z0-9_]*/)) {
        name = substr(line, RSTART, RLENGTH)
        sub(/.*\*/, "", name)
        names[name] = 1
        line = substr(line, RSTART + RLENGTH)
      }
    }
    hit = $0 ~ /kfree\([^;]*fw_id_to_ba/
    hit = hit || $0 ~ /struct iwl_mld_baid_data .*__free\(kfree\)/
    for (name in names) {
      hit = hit || $0 ~ ("(^|[^A-Za-z0-9_])kfree\\(" name "([^A-Za-z0-9_]|$)")
    }
    if (hit) {
      print FILENAME ":" FNR ":" $0
    }
  }
  /^}/ { delete names }
' "$@")
if [ -n "$freed" ]; then
  echo "$freed"
  echo "A free above takes only kmalloc's memory, though the session data may be vmalloc's."
  exit 1
fi
