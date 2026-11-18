//! The far line's two steps around the job: making its tree of the files
//! that crossed, and leaving what its kind brings back to cross home.
//!
//! Each step writes what it finds to a file of its own beside the job's
//! copy, and checks it, rather than piping one command into the next: a
//! pipe's status is only its last command's in an sh with no pipefail, so
//! a step failing would read as one that found nothing. The files lie
//! beside the copy, not in the handover, since the handover crosses home.

use super::{
    BACK, BROUGHT, CUT, END, LARGEST_FILE, LEFT, MARK, MOST_BYTES, MOST_FILES, TREE, quoted,
};
use crate::settings::Kind;

/// The far line's step making the job's tree of its files, `copied`: 0
/// once made, 1 for the tree's own fault, which every try meets, and 2
/// for this machine's, which another may not. A tree with no file in it
/// may not cross at all, and is made empty.
pub(super) fn copying() -> String {
    format!(
        r#"copied() {{
  while IFS="$tab" read -r size path; do
    [ "$size" = {END} ] && break
    case "/$path/" in
      //*|*/../*|*/./*) printf "elsewhere: the job's list names %s, outside its folder\n" "$path" >&2; return 1 ;;
    esac
  done < "$job/{LEFT}"
  [ ! -d "$job/{TREE}" ] || cp -R "$job/{TREE}/." "$work/" || return 2
  # Deepest first, so a folder's contents are named before it is, and
  # shortest first within a folder, so target.elsewhere is target before
  # target.elsewhere.elsewhere takes its name.
  marks="$work.marks"
  find "$work" -name '*{MARK}' > "$marks" &&
    awk -F/ '{{ print NF "\t" length($NF) "\t" $0 }}' "$marks" > "$marks.by" &&
    sort -t "$tab" -k1,1nr -k2,2n "$marks.by" > "$marks.in" &&
    cut -f3- "$marks.in" > "$marks" || {{ rm -f "$marks" "$marks".*; return 2; }}
  (
    # A subshell of its own, so its exit ends only the loop in every sh.
    while IFS= read -r name; do
      to=${{name%{MARK}}}
      case "${{to##*/}}" in
        ''|.|..) printf "elsewhere: the job's tree names %s, outside its folder\n" "$name" >&2; exit 1 ;;
      esac
      [ -e "$to" ] && {{ printf "elsewhere: the job's tree names %s twice\n" "$to" >&2; exit 1; }}
      mv "$name" "$to" || exit 2
    done
  ) < "$marks"
  stripped=$?
  rm -f "$marks" "$marks".*
  return "$stripped"
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
