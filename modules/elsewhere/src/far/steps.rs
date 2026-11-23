//! The far line's steps around the job: watching its archives cross,
//! making its tree of them, answering a stop, and leaving what its kind
//! brings back to cross home.
//!
//! The two steps that find files write what each finds to a file of its
//! own beside the job's copy, and check it, rather than piping one
//! command into the next: a
//! pipe's status is only its last command's in an sh with no pipefail, so
//! a step failing would read as one that found nothing. The files lie
//! beside the copy, not in the handover, since the handover crosses home.

use super::{
    BACK, BROUGHT, CROSSES_AT, CROSSES_WITHIN, CUT, END, LARGEST_FILE, LEFT, MOST_BYTES,
    MOST_FILES, TREE, quoted,
};
use crate::settings::Kind;

/// The far line's watch on the job's archives as they cross: `arrived`,
/// once the list is whole and each archive on it is its listed size, with
/// `waiting` naming what it still waits on; `grown`, how much has crossed
/// so far, as `got`; `ceiling`, the longest they may take in all, as
/// `most`; and `tick`, the time in seconds as `now`.
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
# The longest the files may take to cross in all, in seconds, as `most`,
# however steadily a trickle of them keeps the wait above from ending:
# that wait, and a second for each {CROSSES_AT} bytes they are listed at,
# or the most a job may be until the list has crossed. A size no number
# can be is left out, as unpacking refuses its list anyway.
ceiling() {{
  [ -n "$listed" ] && return 0
  bytes={MOST_BYTES}
  if [ "$(tail -n 1 "$job/{LEFT}" 2>/dev/null)" = {END} ]; then
    bytes=0 listed=yes
    while IFS="$tab" read -r size path; do
      case $size in {END}) break ;; ''|*[!0-9]*|0?*) continue ;; esac
      if [ "${{#size}}" -gt 12 ] || [ "$((bytes + size))" -gt {MOST_BYTES} ]; then
        bytes={MOST_BYTES}
        break
      fi
      bytes=$((bytes + size))
    done < "$job/{LEFT}"
  fi
  most=$(({CROSSES_WITHIN} + bytes / {CROSSES_AT}))
}}
# How much has crossed, list or not, as its names and kilobytes.
grown() {{
  kb=$(du -sk "$job" 2>/dev/null) names=$(find "$job" 2>/dev/null | wc -l)
  kb=${{kb%%[!0-9]*}}
  got=$((${{kb:-0}} + ${{names:-0}}))
}}
# The time in seconds, or where a date can't say it, one more each pass;
# what was counted before it first could, with this pass, carries over,
# to the last time more crossed and to when the wait began.
tick() {{
  t=$(date +%s 2>/dev/null)
  case $t in ''|*[!0-9]*|0?*) now=$((now + 1)) ;; *)
    [ -n "$counting" ] && began=$((t - now - 1 + began)) started=$((t - now - 1 + started)) counting=
    now=$t ;; esac
}}"#
    )
}

/// The far line's answer to being stopped, `stopped`, and the traps that
/// call it. Mycelium stops a run whose asker let go with a signal to its
/// whole group, so the far line takes its copy, the files written beside
/// it, the job's archives and whatever it had begun to bring back away,
/// since no one waits for them, then ends as the signal would have ended
/// it. A closed pipe is a let go too, as when the asker's end goes before
/// its signal comes. A second signal waits for the first one's clearing.
pub(super) fn stopping() -> String {
    format!(
        r#"stopped() {{
  trap '' HUP INT PIPE TERM
  cd "$HOME" && rm -rf "$work" "$work.names" "$work.back" "$work.back".* \
    "$job/{TREE}" "$job/{BACK}" "$job/{BROUGHT}"
  exit "$1"
}}
trap 'stopped 129' HUP
trap 'stopped 130' INT
trap 'stopped 141' PIPE
trap 'stopped 143' TERM"#
    )
}

/// The far line's step making the job's tree of its files, `copied`: 0
/// once made, 1 for the tree's own fault, which every try meets, and 2
/// for this machine's, which another may not. Each name the list holds
/// must be one of the archives the near side writes, `0.tar` and on, all
/// checked before any is unpacked. A job with no file that travels sends
/// none, and its tree is made empty.
///
/// The near side writes only files and folders, each named within the
/// tree, so an archive naming a member from the root or through `..`
/// came from no near side and is refused before it's unpacked. So is a
/// tree holding anything else, a link of either kind, a pipe, a socket
/// or a device, checked as each archive is unpacked, before the next,
/// so no archive writes through what one before it made, and none takes
/// away what one before it made before it's seen. Past that, the fence
/// around the far line writes only the quest's scratch. A file under two
/// names is found as one that isn't a folder yet has more than one link,
/// not by its type, so this find reads apart from the bring-back step's.
/// Each folder is made the far line's own to read, write and enter as it
/// is unpacked, since an archive's folder modes are kept and one closed
/// to its owner would leave its contents unchecked and its copy behind;
/// a file's mode stays as the near side sent it.
pub(super) fn copying() -> String {
    format!(
        r#"copied() {{
  while IFS="$tab" read -r size part; do
    [ "$size" = {END} ] && break
    case $part in *.tar) number=${{part%.tar}} ;; *) number= ;; esac
    case $number in
      ''|*[!0-9]*) printf "elsewhere: the job's list names %s, which isn't one of its archives\n" "$part" >&2; return 1 ;;
    esac
  done < "$job/{LEFT}"
  while IFS="$tab" read -r size part; do
    [ "$size" = {END} ] && break
    tar -tf "$job/{TREE}/$part" > "$work.names" || return 2
    while IFS= read -r member; do
      case /$member/ in
        //*|*/../*)
          printf "elsewhere: the job's archive %s names %s, outside its tree\n" "$part" "$member" >&2
          return 1 ;;
      esac
    done < "$work.names"
    tar -xf "$job/{TREE}/$part" -C "$work" || return 2
    find "$work" -type d ! -perm -700 -exec chmod u+rwx {{}} \; || return 2
    odd=$(find "$work" -type l -o -type p -o -type s -o -type b -o -type c \
      -o ! -type d -links +1) || return 2
    [ -z "$odd" ] || {{
      printf "elsewhere: the job's tree holds %s, which is neither a file nor a folder of its own\n" "${{odd%%
*}}" >&2
      return 1
    }}
  done < "$job/{LEFT}"
}}"#
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
/// the near side waits it out and says it didn't come.
pub(super) fn bringing(kind: &Kind) -> String {
    if kind.brings_back.is_empty() {
        return format!("{}echo {END} > \"$job/{BROUGHT}\"\n", freed());
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
        r#"names="$work.back" wrote=0
(
lose() {{
  for path in {paths}; do
    [ -e "$path" ] || continue
    printf '%s\t%s\n' - "$path" || return 1
  done
}}
if true > "$names" && true > "$names.lost"; then
for path in {paths}; do
  [ -e "$path" ] || continue
  find "$path" -type l ! -path '*/.*' ! -name '.*' \
    -exec printf "elsewhere: %s is a link, so it isn't brought back\n" {{}} \; >&2
  find "$path" -type f ! -path '*/.*' ! -name '.*' >> "$names" ||
    printf '%s\t%s\n' - "$path" >> "$names.lost" || exit 1
done
# A file two of the paths hold crosses once, and counts once. One that
# can't be sorted loses every path, each named once.
LC_ALL=C sort -u "$names" > "$names.in" ||
  {{ true > "$names.in" && lose > "$names.lost" || exit 1; }}
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
# Freed before the list ends, the job's copy and its tree with them, as
# on a disk that filled, these are the room its end needs.
rm -f "$names" "$names".*
{freed}# Cut on a line of its own, so a line the full disk cut in half can't
# hide it.
if [ $wrote -ne 0 ]; then
  printf "elsewhere: couldn't write down all it brings back\n" >&2
  printf '\n%s\n' {CUT} >> "$job/{BROUGHT}"
elif ! echo {END} >> "$job/{BROUGHT}"; then
  printf "elsewhere: couldn't end the list of what it brings back\n" >&2
  printf '\n%s\n' {CUT} >> "$job/{BROUGHT}"
fi
"#,
        freed = freed(),
    )
}

/// Frees the job's copy, with what it made, and the tree it was made
/// from, once nothing is brought back out of them.
fn freed() -> String {
    format!("cd \"$HOME\" && rm -rf \"$work\" \"$job/{TREE}\"\n")
}
