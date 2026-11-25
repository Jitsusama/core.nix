# shellcheck shell=bash
# Moves each folder named into a Btrfs subvolume of its own, as the account's
# manager starts, before the units that write in it. A subvolume is a tree of its
# own, so a write in it no longer takes the lock on the root of the home's
# one tree, which every other writer there holds in turn (home.nix says
# what that cost). It needs no root: an account may make a subvolume, copy
# into it by reflink, which shares the data rather than writing it again,
# and rename it.
#
# It fails open: a folder that's missing, isn't on Btrfs, is already a
# subvolume's root, has a writer named running, has a file of the account's
# open in it, changes while it's copied, or can't be copied in time is left
# as it was, and the run
# says why and goes on to the next. The rename that puts the subvolume in
# the folder's place is the last step and atomic, so a run stopped at any
# point before it leaves the folder whole, and the half-made copy is cleared
# by the next run. The folder as it was is kept beside it, dated, for three
# days, then removed by a later run, since it holds the data a second time
# once the two part ways.
#
# Its help takes the shape every command of ours does (docs/commands.md), by
# hand.

case ${1:-} in
-h | --help)
  cat << 'EOF'
own-subvolumes  folders moved into subvolumes of their own

EXAMPLES
  own-subvolumes 60 ~/.config/google-chrome ~/.config/Slack
      move each into a subvolume of its own, with 60 seconds
      for each copy, unless something has a file open in it
  own-subvolumes --writer slack.service 60 ~/.config/Slack
      the same, but not while slack.service runs, since a
      writer between writes holds nothing open

EXIT
  0   each folder moved, or left as it was, saying why
  1   a folder couldn't be moved, and was left as it was
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

set -uo pipefail
umask 077

asked_wrongly() {
  echo "✕ own-subvolumes needs seconds for each copy, then folders" >&2
  echo "  try  own-subvolumes --help" >&2
  exit 64
}
writers=()
while [ "${1:-}" = --writer ]; do
  [ $# -ge 2 ] || asked_wrongly
  writers+=("$2")
  shift 2
done
[ $# -ge 2 ] || asked_wrongly
case $1 in
'' | *[!0-9]*) asked_wrongly ;;
*) ;;
esac
# timeout takes no time at all as no limit, so none is refused.
[ "$1" -gt 0 ] || asked_wrongly
seconds=$1
shift
for folder in "$@"; do
  case $folder in
  /*) ;;
  *)
    echo "✕ own-subvolumes needs whole paths, not '$folder'" >&2
    echo "  try  own-subvolumes --help" >&2
    exit 64
    ;;
  esac
done

# Says which process has a file under `$1`, a real path, open, as its
# folder or mapped, or nothing when none does. Only the account's own are
# readable, and only they could write in its home. One find reads every
# descriptor's link and one awk every map, since a fork for each of
# thousands of descriptors took minutes under agents' load. A file
# deleted since is in no folder, so its " (deleted)" doesn't count.
holding() {
  local found
  found=$(find /proc/[0-9]*/fd /proc/[0-9]*/cwd -maxdepth 1 \
    \( -lname "$1" -o -lname "$1/*" \) ! -lname '* (deleted)' \
    -printf '%p has %l open\n' -quit 2> /dev/null) || true
  if [ -z "$found" ]; then
    # A map's path starts at its line's first slash, and may hold spaces.
    found=$(awk -v under="$1/" '
      index($0, "/") {
        path = substr($0, index($0, "/"))
        if (index(path, under) == 1 && path !~ / \(deleted\)$/) {
          print FILENAME " maps " path
          exit
        }
      }' /proc/[0-9]*/maps 2> /dev/null) || true
  fi
  echo "$found"
}

# Says which writer named is running, or nothing when none is. One that
# runs may write in its folder by path at any moment, open or not, so a
# folder is moved only while its writers are stopped: as the account's
# manager starts, they wait for the move, but a switch that starts it
# finds them running.
running() {
  local unit state
  for unit in "${writers[@]}"; do
    state=$(systemctl --user is-active -- "$unit" 2> /dev/null) || true
    case $state in
    active | activating | deactivating | reloading | refreshing)
      echo "$unit is running"
      return 0
      ;;
    *) ;;
    esac
  done
}

# The copy being made, and the folder as it was once moved aside.
making() { echo "$1.subvolume-new"; }
kept() {
  local stamp
  stamp=$(date +%Y%m%d%H%M%S)
  echo "$1.before-subvolume-$stamp"
}

# Clears what a run stopped partway left, and the copies kept past three
# days. An account removes a subvolume's root by emptying it, as rm does.
tidy() {
  local folder=$1 new beside name old
  new=$(making "$folder")
  beside=$(dirname "$folder")
  name=$(basename "$folder")
  rm -rf -- "$new"
  find "$beside" -maxdepth 1 -name "$name.before-subvolume-*" \
    -ctime +3 -print0 2> /dev/null | while IFS= read -r -d '' old; do
    rm -rf -- "$old" && echo "removed $old, kept three days since the move"
  done
}

# Moves `$1` into a subvolume of its own, or says why it's left as it was.
# Returns 1 only when something failed.
own() {
  local folder=$1 real fs inode why new stamp aside
  if [ ! -d "$folder" ] || [ -L "$folder" ]; then
    echo "$folder isn't a folder here, so it's left as it is"
    return 0
  fi
  real=$(realpath -- "$folder") || return 1
  fs=$(stat -f -c %T -- "$real") || return 1
  if [ "$fs" != btrfs ]; then
    echo "$folder is on $fs, not Btrfs, so it's left as it is"
    return 0
  fi
  inode=$(stat -c %i -- "$real") || return 1
  # A subvolume's root is always inode 256.
  if [ "$inode" = 256 ]; then
    tidy "$real"
    return 0
  fi
  tidy "$real"
  why=$(running)
  [ -n "$why" ] || why=$(holding "$real")
  if [ -n "$why" ]; then
    echo "$why, so $folder stays in the home's tree until a login finds it free"
    return 0
  fi
  new=$(making "$real")
  stamp=$(mktemp) || return 1
  if ! btrfs -q subvolume create "$new" > /dev/null; then
    rm -f -- "$stamp"
    echo "✕ couldn't make a subvolume beside $folder, so it's left as it is" >&2
    return 1
  fi
  if ! timeout "$seconds" cp -a --reflink=always -- "$real/." "$new/"; then
    rm -rf -- "$new" "$stamp"
    echo "✕ couldn't copy $folder, or not within $seconds s, so it's left as it is" >&2
    return 1
  fi
  # Anything written, made or removed in it since the copy began, by a
  # process that opened and closed it meanwhile, would be lost.
  why=$(find "$real" -newer "$stamp" -print -quit 2> /dev/null)
  [ -n "$why" ] || why=$(running)
  [ -n "$why" ] || why=$(holding "$real")
  rm -f -- "$stamp"
  if [ -n "$why" ]; then
    rm -rf -- "$new"
    echo "$folder changed as it was copied ($why), so it's left as it is until the next login"
    return 0
  fi
  aside=$(kept "$real")
  if ! mv -T -- "$real" "$aside"; then
    rm -rf -- "$new"
    echo "✕ couldn't move $folder aside, so it's left as it is" >&2
    return 1
  fi
  if ! mv -T -- "$new" "$real"; then
    mv -T -- "$aside" "$real"
    rm -rf -- "$new"
    echo "✕ couldn't put the subvolume in $folder's place, so it's left as it was" >&2
    return 1
  fi
  echo "$folder is a subvolume of its own now, the folder as it was kept at $aside"
}

status=0
for folder in "$@"; do
  own "$folder" || status=1
done
exit "$status"
