# shellcheck shell=bash
# Checks that iwlmld's block-ack session data, reorder buffers and all, is
# allocated so it may come from vmalloc, and that no file given frees it
# with kfree() or kfree_sensitive(), which take only kmalloc's memory and
# corrupt the slab on vmalloc's. kfree_rcu() is kvfree_rcu() in 7.3, so it
# may stay, as kvfree() and kvfree_sensitive() may. A newer
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
# a kfree() or kfree_sensitive() of a name held is reported. So is such a
# free of what the session table holds, whatever reads it out, and the
# session data's pointer declared to be freed so as it leaves its scope.
# The lines of a statement are read as one, up to the semicolon or brace
# ending it, each line after its first with its indent taken off, so a call
# or a declaration broken over lines is read whole, with any space, cast or
# parenthesis in it, and a pointer or a declarator on a line of its own.
freed=$(awk '
  # Reads one statement: its names for the session data first, then its frees.
  function judge(statement, from,    line, name, hit) {
    line = statement
    while (match(line, /struct iwl_mld_baid_data +\* *[A-Za-z_][A-Za-z0-9_]*/)) {
      name = substr(line, RSTART, RLENGTH)
      sub(/.*\* */, "", name)
      names[name] = 1
      line = substr(line, RSTART + RLENGTH)
      while (match(line, /^[^;)(]*, *\* *[A-Za-z_][A-Za-z0-9_]*/)) {
        name = substr(line, RSTART, RLENGTH)
        sub(/.*\* */, "", name)
        names[name] = 1
        line = substr(line, RSTART + RLENGTH)
      }
    }
    hit = statement ~ (kfree "[^;]*fw_id_to_ba")
    hit = hit || statement ~ /struct iwl_mld_baid_data .*__free\( *kfree(_sensitive)? *\)/
    for (name in names) {
      hit = hit || statement ~ (kfree cast name "([^A-Za-z0-9_]|$)")
    }
    if (hit) {
      print FILENAME ":" from ":" statement
    }
  }
  BEGIN {
    # A call of kfree() or kfree_sensitive(), which zeroes the memory and then
    # calls kfree(), with any space between its name and its parenthesis, and
    # any cast, parenthesis or space before its argument after them.
    kfree = "(^|[^A-Za-z0-9_])kfree(_sensitive)?[ \t]*\\("
    cast = "(\\(([^()]*\\))?|[ \t])*"
  }
  /^[A-Za-z].*\(/ { delete names; statement = "" }
  {
    line = $0
    if (statement == "") {
      from = FNR
      statement = line
    } else {
      sub(/^[ \t]+/, "", line)
      statement = statement " " line
    }
  }
  /[;{}]/ {
    judge(statement, from)
    statement = ""
  }
  /^}/ { delete names }
' "$@")
if [ -n "$freed" ]; then
  echo "$freed"
  echo "A free above takes only kmalloc's memory, though the session data may be vmalloc's."
  exit 1
fi
