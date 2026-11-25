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
# as it was, and the run says why and goes on to the next. One exchange
# swaps the subvolume into the folder's place, so its path always names the
# folder or the subvolume, never nothing, wherever a run stops; a half-made
# copy is cleared by the next run. The folder as it was is kept beside it,
# dated, for three days, then removed by a later run, since it holds the
# data a second time once the two part ways.
#
# A folder left because a process held it is moved once that process
# quits: the run starts its folder's waiter, named with --waiter, which
# runs this with --after-quit to wait for each holder in turn, and for
# the writers named, then starts the move again. So a switch that finds
# Chrome or Slack running moves its folder the first time Joel quits it,
# rather than at the next boot.
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
  own-subvolumes --waiter waits.service 60 ~/.config/Slack
      the same, starting waits.service when a process holds
      the folder, to move it once that one quits
  own-subvolumes --after-quit move.service ~/.config/Slack
      wait until nothing holds the folder and no writer
      named runs, then start move.service again

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
waiter=''
move=''
while :; do
  case ${1:-} in
  --writer)
    [ $# -ge 2 ] || asked_wrongly
    writers+=("$2")
    shift 2
    ;;
  --waiter)
    [ $# -ge 2 ] || asked_wrongly
    waiter=$2
    shift 2
    ;;
  --after-quit)
    [ $# -ge 2 ] || asked_wrongly
    move=$2
    shift 2
    ;;
  *) break ;;
  esac
done
# Waiting takes one folder and no seconds, since it copies nothing.
if [ -n "$move" ]; then
  [ $# -eq 1 ] || asked_wrongly
else
  [ $# -ge 2 ] || asked_wrongly
  case $1 in
  '' | *[!0-9]*) asked_wrongly ;;
  *) ;;
  esac
  # timeout takes no time at all as no limit, so none is refused.
  [ "$1" -gt 0 ] || asked_wrongly
  seconds=$1
  shift
fi
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
    # Another account's maps, init's first among them, can't be opened, and
    # gawk ends at a file it can't open unless told to pass it by.
    found=$(awk -v under="$1/" '
      BEGINFILE { if (ERRNO != "") nextfile }
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
  local folder=$1 new beside name old root
  new=$(making "$folder")
  beside=$(dirname "$folder")
  name=$(basename "$folder")
  # A copy a run kept for the writes in it, but couldn't set aside, stays
  # until Joel has looked; each run until then fails at making its copy.
  if [ -e "$new.keep" ]; then
    return 0
  fi
  # A copy half made is a subvolume's root; anything else there is the
  # folder as it was, left by a run stopped just after its exchange, and is
  # kept as any other.
  if [ -d "$new" ]; then
    root=$(stat -c %i -- "$new") || return 0
    if [ "$root" != 256 ]; then
      old=$(kept "$folder")
      mv -T -- "$new" "$old" || return 0
    fi
  fi
  rm -rf -- "$new"
  find "$beside" -maxdepth 1 -name "$name.before-subvolume-*" \
    -ctime +2 -print0 2> /dev/null | while IFS= read -r -d '' old; do
    rm -rf -- "$old" && echo "removed $old, kept three days since the move"
  done
}

# Says which file under `$1` changed since the stamp `$2`, or nothing.
changed() {
  find "$1" -newer "$2" -print -quit 2> /dev/null
}

# Says what of `$1`, a real path, is in use: a file in it changed since
# the stamp `$2`, a writer named running, or a process holding it.
in_use() {
  local why
  why=$(changed "$1" "$2")
  [ -n "$why" ] || why=$(running)
  [ -n "$why" ] || why=$(holding "$1")
  echo "$why"
}

# Says why the folder `$1`, at the real path `$3`, stays where it is, `$2`,
# and when a process holds it, starts the waiter that moves it once that
# process lets go. The holder is looked for whatever the reason, since a
# program writes as it opens its folder, and a writer's run may be what
# was seen first while the program holds it all the same. A writer that
# holds nothing open is left to the next start: nothing says when it next
# writes.
left() {
  local folder=$1 why=$2 real=$3 holder
  case $why in
  /proc/*) holder=$why ;;
  *) holder=$(holding "$real") ;;
  esac
  if [ -n "$holder" ] && [ -n "$waiter" ] &&
    systemctl --user restart --no-block -- "$waiter"; then
    case $why in
    /proc/*) echo "$why, so $folder stays in the home's tree until that quits" ;;
    *) echo "$why, and $holder, so $folder stays in the home's tree until that quits" ;;
    esac
    return 0
  fi
  echo "$why, so $folder stays in the home's tree until the account's manager next starts"
}

# Moves `$1` into a subvolume of its own, or says why it's left as it was.
# Returns 1 only when something failed.
own() {
  local folder=$1 real fs inode why late new stamp swapped aside
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
    left "$folder" "$why" "$real"
    return 0
  fi
  new=$(making "$real")
  if [ -e "$new.keep" ]; then
    echo "✕ a copy with writes in it is kept at $new, so $folder stays as it is" \
      "until $new.keep is removed" >&2
    return 1
  fi
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
  why=$(in_use "$real" "$stamp")
  if [ -n "$why" ]; then
    rm -rf -- "$new" "$stamp"
    left "$folder" "$why, as it was copied" "$real"
    return 0
  fi
  swapped=$(mktemp) || return 1
  # Two renames would leave no folder between them, for a run stopped there
  # or a writer to make afresh; an exchange either swaps both or neither.
  if ! mv --exchange -T -- "$new" "$real"; then
    rm -rf -- "$new" "$stamp" "$swapped"
    echo "✕ couldn't put the subvolume in $folder's place, so it's left as it is" >&2
    return 1
  fi
  # The look above takes seconds, and a program started meanwhile, as Joel
  # opening Slack, isn't in it. The folder as it was now sits where the copy
  # was made, a path no program opens it by, so one more look at it there
  # finds whatever reached it before the exchange, and no later one can.
  why=$(in_use "$new" "$stamp")
  rm -f -- "$stamp"
  if [ -n "$why" ]; then
    if ! mv --exchange -T -- "$new" "$real"; then
      rm -f -- "$swapped"
      echo "✕ $folder was in use as it moved ($why), and couldn't be put back" >&2
      return 1
    fi
    # Between the two exchanges the copy had the folder's path, so a
    # program that wrote in it then, or holds it still, wrote what the
    # folder lacks. A writer named running says nothing of the copy, so
    # it isn't asked.
    late=$(changed "$new" "$swapped")
    [ -n "$late" ] || late=$(holding "$new")
    rm -f -- "$swapped"
    if [ -n "$late" ]; then
      # A name of its own, so no earlier split in the same second is in
      # the way.
      aside="$real.split-$(date +%Y%m%d%H%M%S)-$$"
      if ! mv -T -- "$new" "$aside"; then
        # The next run would take the copy for a half-made one and clear
        # it, so it's marked to be kept.
        touch -- "$new.keep"
        echo "✕ $folder was in use as it moved, so it's back, and what was written" \
          "in the copy meanwhile ($late) is at $new, kept until $new.keep is removed" >&2
        return 1
      fi
      echo "✕ $folder was in use as it moved, so it's back, and what was written" \
        "in the copy meanwhile ($late) is kept at $aside" >&2
      return 1
    fi
    rm -rf -- "$new"
    left "$folder" "$why, as it moved" "$real"
    return 0
  fi
  rm -f -- "$swapped"
  # The folder as it was now sits where the copy was made.
  aside=$(kept "$real")
  if ! mv -T -- "$new" "$aside"; then
    echo "✕ $folder is a subvolume now; the folder as it was, at $new, is kept by the next run" >&2
    return 1
  fi
  echo "$folder is a subvolume of its own now, the folder as it was kept at $aside"
}

# Waits until nothing holds the folder `$1` and no writer named runs, then
# starts its move again. Each holder is waited for until it quits or lets
# go, as its folder's move would find it, so Chrome's many processes are
# waited for one at a time, though they mostly quit together.
after_quit() {
  local folder=$1 real inode why pid last=''
  while :; do
    if [ ! -d "$folder" ] || [ -L "$folder" ]; then
      echo "$folder isn't a folder here, so there's nothing to wait for"
      return 0
    fi
    real=$(realpath -- "$folder") || return 1
    inode=$(stat -c %i -- "$real") || return 1
    if [ "$inode" = 256 ]; then
      echo "$folder is a subvolume of its own already, so there's nothing to wait for"
      return 0
    fi
    why=$(running)
    if [ -n "$why" ]; then
      echo "$why, so this waits for it to stop"
      # systemd has no wait for a unit to stop, and a look every five
      # seconds is one call, against the minute a writer such as Chrome's
      # metrics move takes.
      while [ -n "$why" ]; do
        sleep 5
        why=$(running)
      done
      continue
    fi
    why=$(holding "$real")
    [ -n "$why" ] || break
    pid=${why#/proc/}
    pid=${pid%%/*}
    # Said once for each holder, not each minute it's looked at again.
    if [ "$pid" != "$last" ]; then
      echo "$why, so this waits for it to quit"
      last=$pid
    fi
    # tail looks for the process every two seconds, which is as soon after
    # a quit as the move needs to start. A process that only passed
    # through, as a shell cd'd there or a reader of Chrome's sessions, may
    # let go and live on, and its number may pass to another, so the
    # holders are looked at again each minute.
    timeout 60 tail --pid="$pid" -s 2 -f /dev/null || true
  done
  echo "nothing holds $folder now, so its move starts again"
  systemctl --user restart --no-block -- "$move"
}

if [ -n "$move" ]; then
  after_quit "$1"
  exit
fi
status=0
for folder in "$@"; do
  own "$folder" || status=1
done
exit "$status"
