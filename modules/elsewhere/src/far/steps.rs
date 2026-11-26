//! The far line's steps around the job: clearing what earlier runs left,
//! watching its archives cross, waiting on them, making its tree of them,
//! answering a stop, and leaving what its kind brings back to cross home.
//!
//! The two steps that find files write what each finds to a file of its
//! own beside the job's copy, and check it, rather than piping one
//! command into the next: a
//! pipe's status is only its last command's in an sh with no pipefail, so
//! a step failing would read as one that found nothing. The files lie
//! beside the copy, not in the handover, since the handover crosses home.

use super::{
    BACK, BROUGHT, CROSSES_AT, CROSSES_WITHIN, CUT, END, HANDOVER, LARGEST_FILE, LEFT, MOST_BYTES,
    MOST_FILES, TREE, UNTAKEN, quoted,
};
use crate::settings::Kind;

/// The far line's watch on the job's archives as they cross: `arrived`,
/// once the list is whole and each archive on it is its listed size, with
/// `waiting` naming what it still waits on; `grown`, how much has crossed
/// so far, as `got`; and `tick`, the time in seconds as `now`.
pub(super) fn crossing() -> String {
    format!(
        r#"arrived() {{
  waiting='its list of them'
  [ "$(tail -n 1 "$job/{LEFT}" 2>/dev/null)" = {END} ] || return 1
  while IFS="$tab" read -r size path; do
    [ "$size" = {END} ] && return 0
    case "/$path/" in //*|*/../*|*/./*) continue ;; esac
    waiting=$path
    [ -f "$job/{TREE}/$path" ] && [ "$(($(wc -c < "$job/{TREE}/$path")))" -eq "$size" ] || return 1
  done < "$job/{LEFT}"
  return 1
}}
# How much has crossed, list or not, as its names and kilobytes.
grown() {{
  kb=$(du -sk "$job" 2>/dev/null) names=$(find "$job" 2>/dev/null | wc -l)
  kb=${{kb%%[!0-9]*}}
  got=$((${{kb:-0}} + ${{names:-0}}))
}}
# The time in seconds, or where a date can't say it, one more each pass;
# what was counted before it first could, with this pass, carries over
# to the last pass's time.
tick() {{
  t=$(date +%s 2>/dev/null)
  case $t in ''|*[!0-9]*|0?*) now=$((now + 1)) ;; *)
    [ -n "$counting" ] && was=$((t - now - 1 + was)) counting=
    now=$t ;; esac
}}"#
    )
}

/// The most of the far wait one pass may spend, in seconds, however far
/// the clock moved over it: a pass takes a second and a look, so one
/// longer is a clock stepping on or a machine waking from sleep, which
/// spends nothing of a crossing. A pass that truly took longer, on a far
/// machine too slow to look over a few archives in ten seconds, makes
/// both waits longer, never shorter.
pub(super) const A_PASS_SPENDS_AT_MOST: u64 = 10;

/// The far line's wait for the job's archives, `bytes` in all, to cross,
/// saying `untaken`, already quoted for the shell, and ending untaken if
/// they stop crossing or cross too slowly; `crossing`'s steps, `say` and
/// `unlinked` come before it. A link at the job's handover or above it,
/// as a run killed with no stop leaves for the quest's next job, goes
/// first, so nothing is read through it and that job isn't refused for
/// it, and is said, since what crossed through it is then out of reach
/// and the wait may run out for it; one that can't be cleared ends the
/// line untaken, as this machine's to answer.
pub(super) fn waiting(bytes: u64, untaken: &str) -> String {
    // No more than a share carries, which keeps the far sum in range.
    let bytes = bytes.min(MOST_BYTES);
    format!(
        r#"relinked=
unlinked || {{
  printf "elsewhere: a link at the job's handover couldn't be cleared\n" >&2
  say {untaken}
  exit {UNTAKEN}
}}
[ -z "$relinked" ] ||
  printf "elsewhere: what crossed through it is out of reach, so the wait may run out\n" >&2
# By the clock, since a pass over a large tree takes long itself: each
# pass spends what the clock moved over it, never less than the second
# it slept, as when the clock steps back or stands still, nor more than
# {A_PASS_SPENDS_AT_MOST}s, as when it steps on or the machine slept. The wait
# counts what was spent since more last crossed, as the near side waits;
# a pass counts after its second, so passes are never short. However
# steadily they cross, all that is spent is held to a ceiling: the wait,
# and a second for each {CROSSES_AT} bytes left.
bytes={bytes}
most=$(({CROSSES_WITHIN} + bytes / {CROSSES_AT}))
now=0 was=0 counting=yes; tick; grown
was=$now had=$got quiet=0 spent=0
until arrived; do
  sleep 1
  tick
  grown
  step=$((now - was)) was=$now
  [ "$step" -lt 1 ] && step=1
  [ "$step" -gt {A_PASS_SPENDS_AT_MOST} ] && step={A_PASS_SPENDS_AT_MOST}
  quiet=$((quiet + step)) spent=$((spent + step))
  [ "$got" -gt "$had" ] && quiet=0 had=$got
  if [ "$quiet" -ge {CROSSES_WITHIN} ]; then
    printf "elsewhere: the job's files didn't reach %s, none crossing for {CROSSES_WITHIN}s, waiting on %s\n" "$(uname -n)" "$waiting" >&2
    say {untaken}
    exit {UNTAKEN}
  fi
  if [ "$spent" -ge "$most" ] && ! arrived; then
    printf "elsewhere: the job's files, %s bytes, didn't reach %s within %ss, still crossing too slowly, waiting on %s\n" "$bytes" "$(uname -n)" "$most" "$waiting" >&2
    say {untaken}
    exit {UNTAKEN}
  fi
done"#
    )
}

/// The far line's answer to being stopped, `stopped`, and the traps that
/// call it. Mycelium stops a run whose asker let go with a signal to its
/// whole group, so the far line takes its copy, the files written beside
/// it, the job's archives and whatever it had begun to bring back away,
/// since no one waits for them, then dies by the same signal, so whatever
/// waits on it sees it stopped rather than ended. The exit with the code a
/// shell gives such a death, after it, is only a fallback: dash and bash 5
/// die at the signal it sent itself, and bash 3.2, the Mac's sh, reaches
/// the exit but still dies by that signal. A closed pipe is a let go
/// too, as when the asker's end goes before its signal comes. A signal
/// that comes while it clears is dropped, so the clearing runs once, and
/// one ignored when the far line began is never trapped at all. The copy
/// goes after the rest, and its run's file only once all of that went:
/// mycelium kills a group still running a short grace after its signal,
/// and a job's build can hold more files than go in that time, so a
/// removal cut short or failed leaves the file that lets the next run
/// here clear it. The copy, what lies beside it and its run's file go
/// only once this line wrote that file, `mine`: before then, any there
/// are another run's, of this job or of another job of the quest, and
/// only where no link stands at the copies' folder, as `copies` makes
/// sure, since a job of the quest may put one there. What
/// lies in the handover is cleared only once no link the job left stands
/// at it or above it, as `unlinked` makes sure. Every copy, and what lies
/// beside it, is taken away by `CLEARING`'s `cleared`.
pub(super) fn stopping() -> String {
    format!(
        r#"# True once neither the job's handover folder nor either folder above
# it in the quest's scratch is a link, any one there cleared first: a job
# may leave one, and whatever the far line writes or clears below goes
# through it. Each is opened, top down from the scratch, once it's known
# to be a folder of its own, since a job may close one so that nothing
# below it, a link included, can be seen or cleared. False only when a
# link stays, never for a folder that couldn't be opened, which fails what
# follows in its own words. One cleared is said, and noted in `relinked`.
unlinked() {{
  [ -L "${{job%/*/*/*}}" ] || chmod u+rwx "${{job%/*/*/*}}" 2>/dev/null
  for at in "${{job%/*/*}}" "${{job%/*}}" "$job"; do
    [ ! -L "$at" ] || {{ rm -f "$at" 2>/dev/null
      [ -L "$at" ] || {{ relinked=1
        printf "elsewhere: a link at the job's handover was cleared\n" >&2; }}; }}
    [ ! -L "$at" ] || return 1
    [ ! -d "$at" ] || chmod u+rwx "$at" 2>/dev/null
  done
  return 0
}}
stopped() {{
  trap '' HUP INT PIPE TERM
  cd "$HOME" && {{ ! unlinked || cleared "$job/{TREE}" "$job/{BACK}" "$job/{BROUGHT}"; }} &&
    {{ [ -z "$mine" ] || {{ {{ copies || stays; }} &&
      cleared "$work.names" "$work.tar" "$work.back" "$work.back".* "$work" &&
        rm -f "$work{RUN}"
    }}; }}
  trap - "$1"
  kill -s "$1" "$$"
  exit "$2"
}}
trap 'stopped HUP 129' HUP
trap 'stopped INT 130' INT
trap 'stopped PIPE 141' PIPE
trap 'stopped TERM 143' TERM"#
    )
}

/// The far line's step clearing what earlier runs here left beside its
/// copy, before it waits on its files, which cross meanwhile, and `here`,
/// this machine and its process space, which each run writes down with
/// its own number in its run's file before it makes its copy. A copy is
/// cleared, with what lies beside it, only when its run's file names
/// `here` and that run is known gone, as when a kill cut its stop short
/// or came with no stop at all: never by its age, since a copy's folder
/// stops changing once it's made however long its job runs, and never
/// one whose run's file is missing or names another machine or space.
/// A run is gone only when signalling it finds no such process; one it
/// may not signal, as from inside a fence, is kept. Numbers are judged
/// only where they name one process: on a Mac, which has one space of
/// them, or on Linux once this run's space is known. Only a copy named
/// as a job is, by digits and dashes, is taken, so no run's file can
/// name what lies beside another's copy. It never removes through a
/// link: it reads only a folder of copies that is no link, and only run
/// files that are plain files, and find and rm take a link at a copy's
/// name as itself alone. A folder a job closed to its owner, in its copy
/// or beside it, is opened first, as the copy step opens one, so no such
/// copy stays for good;
/// one that still can't go is named once, never file by file, since
/// each far line after it tries again, as an earlier run's. A run's file
/// goes only once its copy did, and only its first bytes are read, with
/// any NUL dropped byte by byte, so one a job filled can't hold the sweep
/// or have a shell or a Mac's tr complain of it in the job's output.
pub(super) fn sweeping() -> String {
    format!(
        r#"runs=${{work%/*}}
ours=$(readlink /proc/self/ns/pid 2>/dev/null)
here="$(uname -n) $ours"
case $(uname -s) in
  Darwin) judged=yes ;;
  Linux) judged=${{ours:+yes}} ;;
  *) judged='' ;;
esac
if [ -n "$judged" ] && [ -d "$runs" ] && [ ! -L "$runs" ]; then
  for run in "$runs"/*{RUN}; do
    [ -f "$run" ] && [ ! -L "$run" ] || continue
    copy=${{run%{RUN}}}
    case ${{copy##*/}} in ''|*[!0-9-]*) continue ;; esac
    line=$(head -c 512 "$run" | LC_ALL=C tr -d '\000' | head -n 1)
    set -f
    set -- $line
    set +f
    [ $# -le 3 ] || continue
    pid=${{1-}} machine=${{2-}} space=${{3-}}
    [ "$machine $space" = "$here" ] || continue
    case $pid in ''|*[!0-9]*) continue ;; esac
    case $(LC_ALL=C kill -0 "$pid" 2>&1) in
      *'No such process'*) ;;
      *) continue ;;
    esac
    copies || {{
      printf "elsewhere: a link at its copies' folder couldn't be cleared, so the sweep stops\n" >&2
      break
    }}
    {{ cleared "$copy.names" "$copy.tar" "$copy.back" "$copy.back".* "$copy" &&
      rm -f "$run"; }} 2>/dev/null ||
      printf "elsewhere: couldn't clear %s, an earlier run's copy\n" "$copy" >&2
  done
fi"#
    )
}

/// What follows a copy's name in its run's file, which says what machine
/// and process space the run was in and its number there.
pub(super) const RUN: &str = ".run";

/// The far line's step defining `cleared`, which takes each path it is
/// given away whole, its folders made the far line's own to enter and
/// write first, since a job may close one so that nothing under it can be
/// taken, `copies`, true once no link stands at the copies' folder,
/// `placed`, true once none stands at the copy either, and `stays`,
/// saying where one can't be cleared that the copy stays. It comes first,
/// before any step that clears, so each finds all four whichever runs
/// first. `cleared` opens what its walk meets only below
/// each path, and a link met there as itself alone; a folder above a path
/// is passed through as `rm` passes it.
pub(super) const CLEARING: &str = r#"# Takes each path away whole,
# its folders made the far line's own to enter and write first: a job
# may close one, so that what lies under it can't be taken. find takes a
# link it meets as itself alone, as -P says, so nothing below one is
# opened. Its answer is the removal's.
cleared() {
  find -P "$@" -type d ! -perm -700 -exec chmod u+rwx {} \; 2>/dev/null
  rm -rf "$@"
}
# True once the copies' folder in the quest's scratch is no link, any one
# there cleared first: a job of the quest may put one there while this
# line runs, and the copy, what lies beside it and its run's file are
# each written, unpacked or cleared by name through that folder. The
# sweep before each copy it clears, the start's clearing, each archive's
# rename and names list, the unpacking, each clearing after, and the
# bring-back before its lists are made and again before they're sorted
# ask first, which narrows a link put there after to the span of a step
# rather than closing it, and the job's cd asks once in. Not asked
# again: the reads of an archive and its names once written, the
# unpacking's chmod and the odd files' find, which is why a refusal is
# this machine's where the copy's place no longer holds after, and the
# bring-back's finds and
# reads of its lists, which follow a link put meanwhile. Only
# a link is caught: a folder put in its place is taken as the copies'
# own. Under a strict fence either gains a job of the quest nothing it
# couldn't write itself, so what this guards is which machine is blamed
# and what is kept. One cleared is said.
copies() {
  [ ! -L "${work%/*}" ] || { rm -f "${work%/*}" 2>/dev/null
    [ -L "${work%/*}" ] || printf "elsewhere: a link at its copies' folder was cleared\n" >&2; }
  [ ! -L "${work%/*}" ]
}
# True once the copy is a folder of its own at its name, no link
# standing at it or at the copies' folder, as `copies` says: a job of
# the quest may put one at either, which tar's -C and the job's cd follow.
placed() {
  copies && [ -d "$work" ] && [ ! -L "$work" ]
}
# Says the copy isn't the folder it was where `placed` no longer holds.
moved() {
  printf "elsewhere: the job's copy was moved or linked from its place\n" >&2
}
# Says the copy stays, as a link at the copies' folder couldn't be
# cleared, and fails, so what would clear it is passed over.
stays() {
  printf "elsewhere: a link at its copies' folder couldn't be cleared, so its copy stays\n" >&2
  return 1
}"#;

/// The far line's step making the job's tree of its files, `copied`: 0
/// once made, 1 for the tree's own fault, which every try meets, and 2
/// for this machine's, which another may not. Each name the list holds
/// must be one of the archives the near side writes, `0.tar` and on, all
/// checked before any is unpacked. The list is read once, so the names
/// unpacked are the very names checked, whatever is at its name after,
/// and each pass is fed it through a pipe, not a here-document, which
/// bash 3.2, the Mac's sh, writes to a file where a fence may let none
/// be made. A job with no file that travels sends none, and its tree is
/// made empty.
///
/// Archives are taken from the job's tree held as the pass's working
/// folder, once that folder is the very one the quest's scratch holds
/// at the tree's name, since through a link put at the tree or above it
/// a rename would take a file from wherever the link points: a link at
/// the tree's name, or anything but a folder of its own there, is the
/// job's fault, and a link above it, put there since the wait cleared
/// them, or a tree gone, is this machine's, as a rename failing was,
/// each said. Held so, a link put there later redirects no rename's
/// source, which is by its name in that folder; where a rename puts it,
/// beside the copy, is through the copies' folder, which `copies` makes
/// sure is no link just before, though a folder put in its place is
/// taken as its own. Each archive
/// is taken from the handover by renaming it to the copy's own `.tar`,
/// once, so the archive checked is the very one unpacked:
/// a share writes a file by renaming a new one over its name, which
/// leaves the one taken as it was, and where the two lie on different
/// disks the rename copies it, and what's read after is the copy. The
/// near side writes only ustar archives, each once, at a size in plain
/// digits, so one listed twice or at a size that isn't, one that isn't a
/// plain file of at least one header and of the size its list gives, or
/// whose first header's magic isn't ustar's, byte for byte, is refused
/// before tar reads it, as a compressed one is: tar unpacks those too,
/// to far more than a share carries. The magic catches an archive no
/// near side of ours wrote, not one built to pass: a compressed archive
/// can carry those bytes where a header's magic sits and still unpack.
/// Such a near side gains nothing by it, since the job it sends runs
/// here anyway. A size is read as digits, never as arithmetic, which
/// would run what a list rewritten after its arrival put there. Each
/// pass runs in a subshell of its own, so its `exit` ends the pass alone
/// even in a shell that runs a pipe's last part itself.
/// The near side writes only files and folders, each named within the
/// tree, so an archive naming a member from the root or through `..`
/// came from no near side of ours and is refused before it's unpacked.
/// So is a tree holding anything else, a link of either kind, a pipe, a
/// socket or a device, checked as each archive is unpacked, before the
/// next, so no archive writes through what one before it made, and none
/// takes away what one before it made before it's seen. Past that, the
/// fence around the far line writes only the quest's scratch. A file
/// under two names is found as one that isn't a folder yet has more
/// than one link, not by its type, so this find reads apart from the
/// bring-back step's. Each folder is made the
/// far line's own to read, write and enter as it is unpacked, an
/// unpacking that failed partway included, since an archive's folder
/// modes are kept and one closed to its owner would leave its contents
/// unchecked and its copy behind; one that still won't open fails the
/// search after it, which can't read it. A file's mode stays as the near
/// side sent it. Each archive's list of names is made new where nothing
/// is, as the run's file is.
pub(super) fn copying() -> String {
    format!(
        r#"copied() {{
  left=$(cat "$job/{LEFT}") || return 2
  # Each pass runs in a subshell of its own, so it ends by exit, and its
  # answer is the pipe's.
  printf '%s\n' "$left" | (seen=; while IFS="$tab" read -r size part; do
    [ "$size" = {END} ] && break
    case $part in *.tar) number=${{part%.tar}} ;; *) number= ;; esac
    case $number in
      ''|*[!0-9]*) printf "elsewhere: the job's list names %s, which isn't one of its archives\n" "$part" >&2; exit 1 ;;
    esac
    case " $seen " in
      *" $part "*) printf "elsewhere: the job's list names %s twice\n" "$part" >&2; exit 1 ;;
    esac
    case $size in
      ''|*[!0-9]*) printf "elsewhere: the job's list gives %s a size that isn't a number\n" "$part" >&2; exit 1 ;;
    esac
    seen="$seen $part"
  done) || return
  printf '%s\n' "$left" | (held=; while IFS="$tab" read -r size part; do
    [ "$size" = {END} ] && break
    # Taken from the tree held as the pass's working folder, once it is
    # the very folder the quest's scratch holds at its name, so no link
    # put at the tree or above it, then or later, redirects where a
    # rename takes from. Where it puts the archive, beside the copy, goes
    # through the copies' folder, which is checked for a link before
    # each rename and on each side of each unpacking, the copy too, as
    # `copies` says, narrowing one to that span.
    [ -n "$held" ] || {{
      # The copy lies in the same scratch, so it is a whole path too.
      case $job in /*) ;; *) {{
        printf "elsewhere: this machine's cache folder isn't a whole path\n" >&2
        exit 2
      }} ;; esac
      top=$(cd -P "${{job%/*/*/*}}" && pwd -P) || exit 2
      [ ! -L "$job/{TREE}" ] && cd -P "$job/{TREE}" 2>/dev/null &&
        [ "$(pwd -P)" = "$top/{HANDOVER}/${{job##*/}}/{TREE}" ] || {{
        # Judged once the tree couldn't be held, so one taken away
        # meanwhile is gone too. A link at a folder above the tree that
        # `unlinked` walks was put there since the wait cleared them, by
        # anything but this job, which hasn't run, so it is this
        # machine's. A link at the tree, even one pointing nowhere, is the
        # job's doing, as a file or a folder closed to this account at the
        # tree is. Above the tree, one pointing nowhere, a file or a closed
        # folder fails the list's read before this, as this machine's.
        for at in "$job" "${{job%/*}}" "${{job%/*/*}}"; do
          [ ! -L "$at" ] || {{
            printf "elsewhere: a link was put above the job's tree after its wait\n" >&2
            exit 2
          }}
        done
        [ -L "$job/{TREE}" ] || [ -e "$job/{TREE}" ] || {{
          printf "elsewhere: the job's tree is gone from its handover\n" >&2
          exit 2
        }}
        printf '%s %s\n' "elsewhere: the job's tree in its handover is reached through a link," \
          "or isn't a folder of its own" >&2
        exit 1
      }}
      held=1
    }}
    # Taken once, so what a share puts at its name after is never read.
    copies && cleared "$work.tar" && mv -f "./$part" "$work.tar" || exit 2
    # A plain file of the size the list gives, holding a whole header,
    # with ustar's magic in it, "ustar" and a NUL, then its version,
    # "00". One that can't be read is this machine's fault.
    plain=
    if [ -f "$work.tar" ] && [ ! -L "$work.tar" ]; then
      length=$(wc -c < "$work.tar") || exit 2
      if [ $(($length)) -ge 512 ] && [ $(($length)) -eq "$size" ]; then
        magic=$(od -An -v -tx1 -j257 -N8 "$work.tar") || exit 2
        [ "$(echo $magic)" = '75 73 74 61 72 00 30 30' ] && plain=1
      fi
    fi
    [ -n "$plain" ] || {{
      printf "elsewhere: the job's archive %s isn't a plain one of the size its list gives, as the near side writes\n" "$part" >&2
      exit 1
    }}
    copies && rm -f "$work.names" && (set -C && tar -tf "$work.tar" > "$work.names") || exit 2
    while IFS= read -r member; do
      case /$member/ in
        //*|*/../*)
          printf "elsewhere: the job's archive %s names %s, outside its tree\n" "$part" "$member" >&2
          exit 1 ;;
      esac
    done < "$work.names"
    placed || {{ moved; exit 2; }}
    tar -xf "$work.tar" -C "$work"; unpacked=$?
    placed || {{ moved; exit 2; }}
    rm -f "$work.tar" || exit 2
    find "$work" -type d ! -perm -700 -exec chmod u+rwx {{}} \; || exit 2
    [ "$unpacked" -eq 0 ] || exit 2
    odd=$(find "$work" -type l -o -type p -o -type s -o -type b -o -type c \
      -o ! -type d -links +1) || exit 2
    [ -z "$odd" ] || {{
      printf "elsewhere: the job's tree holds %s, which is neither a file nor a folder of its own\n" "${{odd%%
*}}" >&2
      exit 1
    }}
  done)
}}"#
    )
}

/// The far line's step once the job ends, making the folder its copies
/// cross in new. A link the job left at its handover folder, or at either
/// folder above it in the quest's scratch, goes first, so neither the
/// copies, their list nor the tree's removal is carried through it. Where
/// one can't be cleared, nothing is written or cleared in the handover:
/// only the copy goes. Otherwise the handover, the folders above it and
/// those under its copies' folder are opened, so a job that closed them
/// changes nothing, and whatever lies at its list's name goes, a folder
/// the job made there included, so the list's own write never fails on it.
/// `cleared` takes a link it meets as itself alone, so nothing past the
/// handover is opened.
/// Where the folder still can't be made new, as on a disk gone wrong, the
/// tree goes too, to free its room, and the list is only its cut, so the
/// near side says at once that nothing came back, whatever list the job
/// left there. Each refusal says which of the two it was.
pub(super) fn handing() -> String {
    format!(
        r#"# Made fresh, so no copy is written through whatever sat at its number,
# its folders opened first, as a job may close them: only once no link
# stands, so nothing past the handover is opened.
stuck="a link it left at its handover couldn't be cleared"
if unlinked; then
  stuck="its folder for copies or its list couldn't be made new"
  cleared "$job/{BACK}" "$job/{BROUGHT}" && mkdir -p "$job/{BACK}" && stuck=
fi
if [ -n "$stuck" ]; then
  printf 'elsewhere: %s, so nothing is brought back\n' "$stuck" >&2
  cd "$HOME" && {{ copies || stays; }} &&
    cleared "$work.names" "$work.tar" "$work.back" "$work.back".* "$work" && rm -f "$work{RUN}"
  ! unlinked || {{
    cleared "$job/{TREE}" "$job/{BACK}" "$job/{BROUGHT}"
    (set -C && printf '\n%s\n' {CUT} > "$job/{BROUGHT}") ||
      printf "elsewhere: nor could it cut its list, so the near side waits it out\n" >&2
  }}
  exit "$code"
fi
"#
    )
}

/// The far line's step leaving what `kind` brings back in the handover.
/// What comes back crosses under its number, since a kind may bring back
/// what lies under a folder a share keeps home. A hidden file is left, as
/// the near side would pass it over. A path whose files couldn't all be
/// found or sorted is listed as lost, so the near side says so and the
/// run doesn't pass as whole. The list gets its end only once every line
/// of it was written, so a list cut short, as by a disk that filled, never
/// reads as whole: it ends cut where that can be written, and otherwise
/// the near side waits it out and says it didn't come. The list, and the
/// lists it's made from, are made new where nothing is, as the run's file
/// is, so none is written through a link the job left at its name.
pub(super) fn bringing(kind: &Kind) -> String {
    if kind.brings_back.is_empty() {
        return format!(
            "{}[ -z \"$handed\" ] || {{ rm -f \"$job/{BROUGHT}\" && \
             (set -C && echo {END} > \"$job/{BROUGHT}\"); }}\n",
            freed()
        );
    }
    // Each once, so a path a kind names twice is never lost twice.
    let mut paths = Vec::new();
    for path in &kind.brings_back {
        let path = quoted(path.trim_matches('/'));
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    let paths = paths.join(" ");
    format!(
        // A subshell of its own, which every write that fails ends, since
        // a shell that isn't interactive may itself exit when a redirection
        // fails, on a compound command or a function as on a special
        // builtin: then only this step ends, never the far line, and the
        // list gets no end. `true` rather than `:` for the same reason.
        // What can't be cleared, as a folder the job made at a list's
        // name, is left for the list's own write to fail on, so each path
        // is lost as when nowhere is left to list them. Its lists lie
        // beside the copy, so they are begun only once no link stands at
        // the copies' folder, where one does each path being lost, and
        // sorted only once none stands after the finds, where one does
        // the list being cut.
        r#"names="$work.back" wrote=0 beside=
(
rm -f "$job/{BROUGHT}" 2>/dev/null
copies && beside=1 && rm -f "$names" "$names".* 2>/dev/null
set -C
lose() {{
  for path in {paths}; do
    [ -e "$path" ] || continue
    printf '%s\t%s\n' - "$path" || return 1
  done
}}
# The sorted list's name is tried with the others, then left free for
# sort's own write.
if [ -n "$beside" ] && true > "$names" && true > "$names.lost" && true > "$names.in" && rm -f "$names.in"; then
for path in {paths}; do
  [ -e "$path" ] || continue
  find "$path" -type l ! -path '*/.*' ! -name '.*' \
    -exec printf "elsewhere: %s is a link, so it isn't brought back\n" {{}} \; >&2
  find "$path" -type f ! -path '*/.*' ! -name '.*' >> "$names" ||
    printf '%s\t%s\n' - "$path" >> "$names.lost" || exit 1
done
# A file two of the paths hold crosses once, and counts once. One that
# can't be sorted loses every path, each named once.
copies || exit 1
LC_ALL=C sort -u "$names" > "$names.in" ||
  {{ rm -f "$names.in" "$names.lost" && true > "$names.in" && lose > "$names.lost" ||
    exit 1; }}
{{
  n=0 files=0 bytes=0 most_files={MOST_FILES} most_bytes={MOST_BYTES}
  # One that can't be read or copied keeps its number, listed with no
  # size, so the near side lands the rest and says it's missing.
  missing() {{
    rm -f "$job/{BACK}/$n"
    printf "elsewhere: %s didn't come back\n" "$file" >&2
    printf '%s\t%s\n' - "$file" || exit 1
    n=$((n + 1))
  }}
  # One past what a share holds, alone or with what crossed before it,
  # is listed marked with its size and never crosses, so the near side
  # lands the rest and says why without waiting for it.
  past() {{
    [ $1 -gt {LARGEST_FILE} ] || [ $((files + 1)) -gt $most_files ] ||
      [ $(($bytes + $1)) -gt $most_bytes ]
  }}
  kept() {{
    rm -f "$job/{BACK}/$n"
    printf '>%s\t%s\n' $(($1)) "$file" || exit 1
    n=$((n + 1))
  }}
  while IFS= read -r file; do
    size=$(wc -c 2>/dev/null < "$file") || {{ missing; continue; }}
    past $size && {{ kept $size; continue; }}
    cp "$file" "$job/{BACK}/$n" && copied=$(wc -c < "$job/{BACK}/$n") ||
      {{ missing; continue; }}
    # Only what crossed counts toward the totals, at the size it crossed.
    past $copied && {{ kept $copied; continue; }}
    files=$((files + 1)) bytes=$(($bytes + $copied))
    printf '%s\t%s\n' $(($copied)) "$file" || exit 1
    n=$((n + 1))
  done < "$names.in" || exit 1
  # Last, so no file's number moves.
  while IFS= read -r lost; do printf '%s\n' "$lost" || exit 1; done < "$names.lost" ||
    exit 1
}} > "$job/{BROUGHT}"
else
  printf "elsewhere: couldn't list what to bring back beside %s\n" "$work" >&2
  lose > "$job/{BROUGHT}"
fi
) || wrote=$?
{ended}"#,
        ended = ended(),
    )
}

/// The end of `bringing`'s list of what's brought back, once the list's
/// lines were written, or `wrote` says they couldn't all be.
fn ended() -> String {
    format!(
        r#"# Freed before the list ends, the job's copy and its tree with them, as
# on a disk that filled, these are the room its end needs. Where a link
# at the handover couldn't be cleared, as `freed` says, nothing more is
# written through it, and the near side waits out a list with no end.
! copies || rm -f "$names" "$names".*
{freed}if [ -z "$handed" ]; then wrote=1
elif [ $wrote -ne 0 ]; then
  printf "elsewhere: couldn't write down all it brings back\n" >&2
elif ! echo {END} >> "$job/{BROUGHT}"; then
  printf "elsewhere: couldn't end the list of what it brings back\n" >&2
  wrote=1
fi
# Cut on a line of its own, so a line the full disk cut in half can't
# hide it, and never through a link that couldn't be cleared at its name
# or at the handover.
[ $wrote -eq 0 ] || [ -z "$handed" ] || [ -L "$job/{BROUGHT}" ] ||
  printf '\n%s\n' {CUT} >> "$job/{BROUGHT}"
"#,
        freed = freed(),
    )
}

/// Frees the job's copy, with what it made, and the tree it was made
/// from, once nothing is brought back out of them or the job was
/// refused, and then its run's file, once they went. The tree goes only
/// where no link stood at the job's handover or above it as `unlinked`
/// looked, and the copy and its run's file only where none stood at the
/// copies' folder as `copies` looked, which narrows a link put at either
/// after to the span from that look to the clearing's `rm`, past its walk
/// of what it clears, rather than closing it. Each kept
/// is said, and each goes whether or not the other could.
pub(super) fn freed() -> String {
    format!(
        r#"handed=
cd "$HOME" && {{
  if unlinked; then handed=1; else
    printf "elsewhere: a link at the job's handover couldn't be cleared, so its tree stays\n" >&2
  fi
  if ! copies; then
    stays
    [ -z "$handed" ] || cleared "$job/{TREE}"
  elif [ -n "$handed" ]; then cleared "$job/{TREE}" "$work" && rm -f "$work{RUN}"
  else cleared "$work" && rm -f "$work{RUN}"; fi
}}
"#
    )
}
