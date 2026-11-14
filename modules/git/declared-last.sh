# shellcheck shell=bash
# Keeps the global file git writes a plain file, so a program can change its
# own settings, and ends it with an include of the declared settings, which
# git then reads after everything a program wrote. Run on every switch, which
# puts the include back at the end after something added a section below it.
#
# A program that includes its own file from the global one adds the include
# when it doesn't find it there, below the declared settings, so the files
# named after these two are included first, ahead of them.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
declared-last  have git read the declared settings last

  Keeps git's global file a plain file programs can
  write to, includes each program's own file in it, and
  ends it with an include of the declared settings.
  Every switch runs it.

EXAMPLES
  declared-last ~/.config/git/declared.config ~/.gitconfig

EXIT
  0   done, or the global file is a link, left alone
  64  asked wrongly
  else git config's own
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -lt 2 ]; then
  printf '%s\n' "declared-last takes the declared file and git's global one" \
    "  try  declared-last --help" >&2
  exit 64
fi

declared=$1
global=$2
shift 2

if [ -L "$global" ]; then
  echo "$global is a link, so git can't write to it; leaving it as it is" >&2
  exit 0
fi

# Quoted as git quotes a value, so any path reads back as itself.
escaped=$(printf '%s' "$declared" | sed 's/[\\"]/\\&/g')
include=$(printf '[include]\n\tpath = "%s"' "$escaped")

touch "$global"
for program in "$@"; do
  # A program may have written its path from ~, which git expands.
  if ! git config --file "$global" --no-includes --get-all include.path |
    sed "s|^~/|$HOME/|" | grep -qxF "$program"; then
    git config --file "$global" --add include.path "$program"
  fi
done

if [ "$(tail -n 2 "$global")" = "$include" ]; then
  exit 0
fi

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
