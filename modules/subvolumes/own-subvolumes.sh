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
  if [ $# -ne 1 ]; then
    echo "✕ own-subvolumes --after-quit needs a unit, then one folder" >&2
    echo "  try  own-subvolumes --help" >&2
    exit 64
  fi
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

# Says whether the process `$1` still holds anything under `$2`, a real
# path, as holding would find it: one process's links and map, against
# holding's walk of every process.
holds() {
  local found
  found=$(find "/proc/$1/fd" "/proc/$1/cwd" -maxdepth 1 \
    \( -lname "$2" -o -lname "$2/*" \) ! -lname '* (deleted)' \
    -print -quit 2> /dev/null) || true
  if [ -z "$found" ]; then
    found=$(awk -v under="$2/" '
      index($0, "/") {
        path = substr($0, index($0, "/"))
        if (index(path, under) == 1 && path !~ / \(deleted\)$/) {
          print "maps"
          exit
        }
      }' "/proc/$1/maps" 2> /dev/null) || true
  fi
  echo "$found"
}

# Where a folder's move keeps what it counted since the machine started,
# a name for its `.copies` and its `.waits`, or nothing where the account
# has no runtime folder.
counted() {
  if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -n "$waiter" ]; then
    echo "$XDG_RUNTIME_DIR/own-subvolumes/$waiter"
  fi
}

# Says how many copies of the folder its move began since the machine
# started.
copies_made() {
  local base copies=0
  base=$(counted)
  if [ -n "$base" ] && [ -f "$base.copies" ]; then
    copies=$(cat -- "$base.copies")
  fi
  case $copies in
  '' | *[!0-9]*) echo 0 ;;
  *) echo "$copies" ;;
  esac
}

# The copy being made, and the folder as it was once moved aside.
making() { echo "$1.subvolume-new"; }
kept() {
  local stamp
  # To the nanosecond, so two kept within a second don't share a name.
  stamp=$(date +%Y%m%d%H%M%S-%N)
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
  # A mark with no copy is one a run stopped before it could clear.
  if [ -e "$new.keep" ] && [ ! -e "$new" ]; then
    rm -f -- "$new.keep"
  fi
  # A copy half made is a subvolume's root; anything else there is the
  # folder as it was, left by a run stopped just after its exchange, and is
  # kept as any other.
  if [ ! -e "$new.keep" ]; then
    if [ -d "$new" ]; then
      root=$(stat -c %i -- "$new") || return 0
      if [ "$root" != 256 ]; then
        old=$(kept "$folder")
        mv -T -- "$new" "$old" || return 0
      fi
    fi
    rm -rf -- "$new"
  fi
  find "$beside" -maxdepth 1 -name "$name.before-subvolume-*" \
    -ctime +2 -print0 2> /dev/null | while IFS= read -r -d '' old; do
    rm -rf -- "$old" && echo "removed $old, kept three days since the move"
  done
}

# Says which file under `$1` changed since the stamp `$2`, or nothing. A
# file's change time can't be set back, as its modified time can; the
# folder's own is left out, as its exchange sets it, and its modified time
# stands for what was made or removed in it.
changed() {
  local found
  # A walk that couldn't read all of it says so, since what it couldn't
  # read may be what changed, and every caller keeps a folder it's told
  # changed.
  if ! found=$(find "$1" -mindepth 1 -cnewer "$2" -print -quit 2> /dev/null); then
    [ -n "$found" ] || found="$1, as part of it couldn't be read"
  fi
  [ -n "$found" ] || found=$(find "$1" -maxdepth 0 -newer "$2" -print 2> /dev/null) || true
  echo "$found"
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
# holds nothing open is left to the next start, since nothing says when it
# next writes, unless a waiter already started since the machine did:
# Chrome's metrics writer runs again as Chrome quits, and the move its
# waiter starts may find it. A copy that ends in no move is a whole copy
# spent, so after three the folder waits for the manager's next start.
left() {
  local folder=$1 why=$2 real=$3 holder base copies writer='' wait=''
  case $why in
  /proc/*) holder=$why ;;
  *) holder=$(holding "$real") ;;
  esac
  base=$(counted)
  copies=$(copies_made)
  [ -z "$holder" ] || wait=yes
  # The writers are asked again rather than read from the reason, which may
  # be the name of a file in the folder.
  if [ -z "$holder" ] && [ -n "$base" ] && [ -e "$base.waits" ]; then
    writer=$(running)
    [ -z "$writer" ] || wait=yes
  fi
  if [ -n "$wait" ] && [ -n "$waiter" ] && [ "$copies" -ge 3 ]; then
    echo "$why, so $folder stays in the home's tree until the account's manager next" \
      "starts, as $copies copies of it since the machine started ended in no move"
    return 0
  fi
  if [ -n "$wait" ] && [ -n "$waiter" ] &&
    systemctl --user restart --no-block -- "$waiter"; then
    if [ -n "$base" ]; then
      mkdir -p -- "${base%/*}" && touch -- "$base.waits"
    fi
    case $why in
    /proc/*) echo "$why, so $folder stays in the home's tree until that quits" ;;
    *)
      if [ -n "$holder" ]; then
        echo "$why, and $holder, so $folder stays in the home's tree until that quits"
      elif [ -n "$writer" ] && [[ $why != *"$writer"* ]]; then
        echo "$why, and $writer, so $folder stays in the home's tree until that stops"
      else
        echo "$why, so $folder stays in the home's tree until that stops"
      fi
      ;;
    esac
    return 0
  fi
  echo "$why, so $folder stays in the home's tree until the account's manager next starts"
}

# Moves `$1` into a subvolume of its own, or says why it's left as it was.
# Returns 1 only when something failed.
own() {
  local folder=$1 real fs inode why late still='' new beside name stamp swapped aside
  local base copies marked=''
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
    # A move that couldn't put the folder back left it at the copy's path,
    # marked, and the subvolume in use may lack what was written in it, so
    # each run says so until Joel has looked.
    new=$(making "$real")
    [ -e "$new.keep" ] || return 0
    # The mark bears the time the copy began, so once nothing holds the
    # folder as it was, a walk says whether anything reached it since; a
    # run stopped as it looked is resolved here when nothing did.
    why=$(holding "$new")
    if [ -n "$why" ]; then
      echo "✕ $folder is a subvolume, but the folder as it was, at $new, is held ($why)," \
        "so it may yet be written in, and it's kept until a run finds it let go" >&2
      return 1
    fi
    why=$(changed "$new" "$new.keep")
    if [ -n "$why" ]; then
      echo "✕ $folder is a subvolume, but the folder as it was, at $new, changed as it" \
        "moved ($why); once what's wanted is out of it, removing it and $new.keep" \
        "ends this" >&2
      return 1
    fi
    aside=$(kept "$real")
    mv -T -- "$new" "$aside" || return 1
    rm -f -- "$new.keep"
    echo "$folder is a subvolume of its own, and the folder as it was, left by a run" \
      "stopped as it moved, held nothing more, so it's kept at $aside"
    return 0
  fi
  tidy "$real"
  # What a move kept for the writes in it is said before anything else, so
  # a folder held all day says so at each run, not only once it's free.
  new=$(making "$real")
  if [ -e "$new.keep" ]; then
    echo "✕ a copy with writes in it is kept at $new, so $folder stays as it is;" \
      "once what's wanted is out of it, removing it and $new.keep lets it move as the" \
      "account's manager next starts" >&2
    return 1
  fi
  beside=$(dirname -- "$real")
  name=$(basename -- "$real")
  aside=$(find "$beside" -maxdepth 1 -name "$name.split-*" -print -quit 2> /dev/null) || true
  if [ -n "$aside" ]; then
    echo "✕ what was written in $folder as it last moved is at $aside, so it stays as it" \
      "is; once what's wanted is out of it, removing it lets the folder move as the" \
      "account's manager next starts" >&2
    return 1
  fi
  why=$(running)
  [ -n "$why" ] || why=$(holding "$real")
  if [ -n "$why" ]; then
    left "$folder" "$why" "$real"
    return 0
  fi
  stamp=$(mktemp) || return 1
  if ! btrfs -q subvolume create "$new" > /dev/null; then
    rm -f -- "$stamp"
    echo "✕ couldn't make a subvolume beside $folder, so it's left as it is" >&2
    return 1
  fi
  base=$(counted)
  if [ -n "$base" ]; then
    copies=$(copies_made)
    mkdir -p -- "${base%/*}" && echo $((copies + 1)) > "$base.copies"
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
  swapped=$(mktemp) || {
    rm -rf -- "$new" "$stamp"
    echo "✕ couldn't make a stamp for $folder's exchange, so it's left as it is" >&2
    return 1
  }
  # Past the clock's coarse tick, at most a hundredth of a second, so a
  # write just after the exchange can't share the stamp's time and pass it.
  sleep 0.01
  # Two renames would leave no folder between them, for a run stopped there
  # or a writer to make afresh; an exchange either swaps both or neither.
  if ! mv --exchange -T -- "$new" "$real"; then
    rm -rf -- "$new" "$stamp" "$swapped"
    echo "✕ couldn't put the subvolume in $folder's place, so it's left as it is" >&2
    return 1
  fi
  # Until the looks below find what sits at the copy's path holds nothing
  # the folder lacks, it's marked, so a run stopped meanwhile, or an
  # exchange back that fails, leaves it to Joel, never to the next run to
  # clear. It bears the time the copy began, for the run that next finds it.
  if touch -r "$stamp" -- "$new.keep"; then
    marked=yes
  fi
  # The look above takes seconds, and a program started meanwhile, as Joel
  # opening Slack, isn't in it. The folder as it was now sits where the copy
  # was made, a path no program opens it by, so one more look at it there
  # finds whatever reached it before the exchange, and no later one can.
  # A writer named running would next open the subvolume, so it isn't asked.
  # Holders are asked first: once none holds it, nothing more can reach it,
  # so the walk after sees every write.
  why=$(holding "$new")
  [ -n "$why" ] || why=$(changed "$new" "$stamp")
  rm -f -- "$stamp"
  if [ -n "$why" ]; then
    if ! mv --exchange -T -- "$new" "$real"; then
      rm -f -- "$swapped"
      if [ -n "$marked" ]; then
        echo "✕ $folder was in use as it moved ($why), and couldn't be put back; the" \
          "folder as it was, with what was written in it, is kept at $new" >&2
      else
        echo "✕ $folder was in use as it moved ($why), and couldn't be put back; the" \
          "folder as it was, with what was written in it, is at $new, which couldn't" \
          "be marked, so the next run sets it aside for three days" >&2
      fi
      return 1
    fi
    # Between the two exchanges the copy had the folder's path, so a
    # program that wrote in it then, or holds it still, wrote what the
    # folder lacks. A writer named running says nothing of the copy, so
    # it isn't asked.
    late=$(holding "$new")
    if [ -n "$late" ]; then
      still=", which may go on writing there until it quits"
    else
      late=$(changed "$new" "$swapped")
    fi
    rm -f -- "$swapped"
    if [ -n "$late" ]; then
      # A name of its own, so no earlier split in the same second is in
      # the way.
      aside="$real.split-$(date +%Y%m%d%H%M%S)-$$"
      if ! mv -T -- "$new" "$aside"; then
        if [ -n "$marked" ]; then
          echo "✕ $folder was in use as it moved, so it's back, and what was written" \
            "in the copy meanwhile ($late) is at $new; once what's wanted is out of" \
            "it, removing it and $new.keep lets the folder move as the account's manager" \
            "next starts" >&2
        else
          echo "✕ $folder was in use as it moved, so it's back, and what was written" \
            "in the copy meanwhile ($late) is at $new, which couldn't be marked, so" \
            "the next run clears it unless it's moved aside first" >&2
        fi
        return 1
      fi
      rm -f -- "$new.keep"
      echo "✕ $folder was in use as it moved, so it's back, and what was written" \
        "in the copy meanwhile (${late//"$new"/"$aside"}) is kept at $aside$still" >&2
      return 1
    fi
    # Known to hold nothing, the copy is unmarked first, so a removal
    # stopped partway leaves a half copy the next run clears.
    rm -f -- "$new.keep"
    rm -rf -- "$new"
    # Said by the folder's own path, as the copy's is gone.
    left "$folder" "${why//"$new"/"$real"}, as it moved" "$real"
    return 0
  fi
  rm -f -- "$swapped"
  # The folder as it was now sits where the copy was made.
  aside=$(kept "$real")
  if ! mv -T -- "$new" "$aside"; then
    # Unmarked, so the next run keeps it as any other.
    rm -f -- "$new.keep"
    echo "✕ $folder is a subvolume now; the folder as it was, at $new, is kept by the next run" >&2
    return 1
  fi
  rm -f -- "$new.keep"
  base=$(counted)
  [ -z "$base" ] || rm -f -- "$base.copies" "$base.waits" "$base.idle"
  echo "$folder is a subvolume of its own now, the folder as it was kept at $aside"
}

# Waits until nothing holds the folder `$1` and no writer named runs, then
# starts its move again. Each holder is waited for until it quits or lets
# go, as its folder's move would find it, so Chrome's many processes are
# waited for one at a time, though they mostly quit together.
after_quit() {
  local folder=$1 real inode why pid held state last='' waited='' base idle=0
  local started=$SECONDS
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
      waited=yes
      # systemd has no wait for a unit to stop, and a look every half
      # minute is one call, against the minute a writer such as Chrome's
      # metrics move takes, and the days the node runs.
      while [ -n "$why" ]; do
        sleep 30
        why=$(running)
      done
      continue
    fi
    why=$(holding "$real")
    if [ -z "$why" ]; then
      # A move under way, as a switch's or the one that started this, is
      # let end rather than stopped partway, and the folder looked at again.
      state=$(systemctl --user is-active -- "$move" 2> /dev/null) || true
      [ "$state" = activating ] || break
      echo "nothing holds $folder now, and its move is under way, so this waits for it"
      while [ "$state" = activating ]; do
        # systemd has no wait for a job to end, and a look every two
        # seconds is one call, against a move's copy of seconds or minutes.
        sleep 2
        state=$(systemctl --user is-active -- "$move" 2> /dev/null) || true
      done
      continue
    fi
    pid=${why#/proc/}
    pid=${pid%%/*}
    # Said once for each holder, not each minute it's looked at again.
    if [ "$pid" != "$last" ]; then
      echo "$why, so this waits for it to quit"
      last=$pid
    fi
    waited=yes
    # tail looks for the process every two seconds, which is as soon after
    # a quit as the move needs to start. A process that only passed
    # through, as a shell cd'd there or a reader of Chrome's sessions, may
    # let go and live on, so each minute that one process is asked whether
    # it still holds the folder, and only once it doesn't are all of them
    # looked at again.
    while :; do
      timeout 60 tail --pid="$pid" -s 2 -f /dev/null || true
      held=$(holds "$pid" "$real")
      [ -n "$held" ] || break
    done
  done
  # A holder the move saw and this never finds, as a program that opens
  # a file now and then, would have the two start each other without end,
  # as would a stream of programs each holding it a moment, so a start of
  # the move is counted unless this waited a minute, as for Chrome or
  # Slack in use, and after three in a row, the move waits for the
  # manager. The count is kept under this unit's name, given with
  # --waiter, so a move that lands clears it with the copies it counts.
  base=$(counted)
  if [ -n "$waited" ] && [ $((SECONDS - started)) -ge 60 ] && [ -n "$base" ]; then
    rm -f -- "$base.idle"
  elif [ -n "$base" ]; then
    if [ -f "$base.idle" ]; then
      idle=$(cat -- "$base.idle")
    fi
    case $idle in
    '' | *[!0-9]*) idle=0 ;;
    *) ;;
    esac
    if [ "$idle" -ge 3 ]; then
      echo "nothing holds $folder as this looks, though its move found something" \
        "$idle times in a row with nothing held a minute, so it waits for the account's" \
        "manager next start"
      return 0
    fi
    mkdir -p -- "${base%/*}" && echo $((idle + 1)) > "$base.idle"
  fi
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
