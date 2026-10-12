# Keeps the global file git writes a plain file, so a program can change its
# own settings, and ends it with an include of the declared settings, which
# git then reads after everything a program wrote. Run on every switch, which
# puts the include back at the end after something added a section below it.
#
# Usage: declared-last <declared file> <global file>
declared=$1
global=$2

if [ -L "$global" ]; then
  echo "$global is a link, so git can't write to it; leaving it as it is" >&2
  exit 0
fi

# Quoted as git quotes a value, so any path reads back as itself.
escaped=$(printf '%s' "$declared" | sed 's/[\\"]/\\&/g')
include=$(printf '[include]\n\tpath = "%s"' "$escaped")

if [ -f "$global" ] && [ "$(tail -n 2 "$global")" = "$include" ]; then
  exit 0
fi

touch "$global"
status=0
git config --file "$global" --fixed-value --unset-all include.path "$declared" || status=$?
# 5 means it wasn't there to remove.
if [ "$status" -ne 0 ] && [ "$status" -ne 5 ]; then
  exit "$status"
fi
# Appended rather than added with git config, which would put it in the last
# [include] section wherever that is, and a section a program added after
# that one would win over it.
if [ -s "$global" ] && [ -n "$(tail -c 1 "$global")" ]; then
  echo >> "$global"
fi
printf '%s\n' "$include" >> "$global"
