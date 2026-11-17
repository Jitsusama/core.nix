//! A job sent to a far machine by mycelium alone. Its tracked files, and
//! nothing else, are left in the quest's handover, which crosses to every
//! machine the quest is shared with; `mycelium run` then runs a shell line
//! there that waits for them to arrive, copies them out to a folder of its
//! own, runs the kind's setup and the job, and leaves what the kind brings
//! back in the handover to cross home. No ssh and no copy of its own,
//! since mycelium already says which machines may be reached, by whom, and
//! fences an agent's run where it lands.
//!
//! Each end of a shared quest looks once a second and files arrive in no
//! set order, so each side lists what it left, with sizes and an end line
//! written last, and the other waits until all of it is there at the size
//! listed. A share writes what it receives whole, by a hidden file renamed
//! over its path (mycelium's `share/disk.rs`), but it may have read a file
//! here while it was being written, so the size is what says the file
//! arrived as it was left.
//!
//! A shared quest keeps home any folder a build fills, `target` and
//! `node_modules`, wherever it lies, which a tracked file can sit in as
//! source. So such a name crosses with [`MARK`] after it, as does a name
//! already ending in it, and the far side takes one off each name ending
//! in it. What comes back is few files, and crosses under its numbers in
//! the list.

use crate::settings::{Copies, Kind};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Where a job's folder lies within a quest's scratch.
pub const HANDOVER: &str = "handover/elsewhere";

/// The list of what's left, a `SIZE<TAB>PATH` line a file, and the list of
/// what's brought back, the same; each ends with [`END`].
const LEFT: &str = "elsewhere-files";
const BROUGHT: &str = "elsewhere-back";
const END: &str = "end";

/// The folders the job's files lie in, under their paths as crossed, and
/// what it brings back lies in, each named by its line in the list.
const TREE: &str = "tree";
const BACK: &str = "back";

/// The names a shared quest keeps home wherever they lie, as a build
/// fills them, and what such a name crosses with after it.
const BUILT: [&str; 2] = ["target", "node_modules"];
const MARK: &str = ".elsewhere";

/// What a shared folder carries at most, as mycelium refuses past it.
const MOST_FILES: usize = 10_000;
const MOST_BYTES: u64 = 1 << 30;
const LARGEST_FILE: u64 = 64 << 20;

/// How long each side waits for the other's files to cross, in seconds:
/// a minute's worth of a busy link's looks, five times over.
pub const CROSSES_WITHIN: u64 = 300;

/// Whether `quest` is a quest's id as the harness mints it, so it names a
/// folder and nothing more.
pub fn plain_quest(quest: &str) -> bool {
    let parts: Vec<&str> = quest.split('-').collect();
    matches!(
        parts.as_slice(),
        ["QEST", date, id]
            if date.len() == 8
                && date.bytes().all(|byte| byte.is_ascii_digit())
                && id.len() == 6
                && id.bytes().all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
    )
}

/// The folder holding each quest's scratch, as the harness and mycelium
/// both find it.
pub fn scratches() -> PathBuf {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|it| !it.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache"),
            PathBuf::from,
        );
    cache.join("pi/agentic-harness.pi/quest-workspace")
}

/// The tree a kind copies for a job in `folder`: the folder, or the
/// repository holding it, as git says, or what's wrong.
pub fn root(kind: &Kind, folder: &Path) -> Result<PathBuf, String> {
    if kind.copies == Copies::Folder {
        return Ok(folder.to_path_buf());
    }
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(folder)
        .output()
        .map_err(|error| format!("git didn't start: {error}"))?;
    if !output.status.success() {
        return Err(format!("{} isn't in a repository", folder.display()));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

/// Whether a tracked `path` may travel: never a hidden file or one in a
/// hidden folder, such as `.env` or `.git/config`, never a key or a
/// certificate, and nothing the kind excludes. A tracked file in a folder
/// named as a build's is source, and travels; a kind excludes the build
/// output it tracks.
pub fn travels(path: &str, excludes: &[String]) -> bool {
    let hidden = path.split('/').any(|name| name.starts_with('.'));
    let secret = Path::new(path).extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("pem") || extension.eq_ignore_ascii_case("key")
    });
    !hidden && !secret && !excludes.iter().any(|exclude| excluded(path, exclude))
}

/// Whether `pattern` excludes `path`: one starting with `/` names a path
/// from the tree's top, and any other a name, or names, at any depth; each
/// excludes what's under it too.
fn excluded(path: &str, pattern: &str) -> bool {
    let anchored = pattern.starts_with('/');
    let pattern = pattern.trim_matches('/');
    if pattern.is_empty() {
        return false;
    }
    let at = |from: &str| from == pattern || from.starts_with(&format!("{pattern}/"));
    if anchored {
        return at(path);
    }
    at(path)
        || path
            .match_indices('/')
            .any(|(slash, _)| at(&path[slash + 1..]))
}

/// The files of the tree at `root` that travel, with their sizes: git's
/// tracked files, read where they lie, not links or folders, or why the
/// job can't travel.
pub fn gathered(root: &Path, kind: &Kind) -> Result<Vec<(String, u64)>, String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("git didn't start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{} isn't in a repository, and only tracked files travel",
            root.display()
        ));
    }
    let mut files = Vec::new();
    let mut bytes = 0u64;
    for path in output.stdout.split(|&byte| byte == 0) {
        let Ok(path) = std::str::from_utf8(path) else {
            return Err("a tracked file's name isn't UTF-8".to_owned());
        };
        if path.is_empty() || !travels(path, &kind.excludes) {
            continue;
        }
        if path.contains(['\t', '\n']) {
            return Err(format!(
                "the tracked name {path:?} holds a tab or a newline"
            ));
        }
        // A file tracked but gone, a link or a submodule doesn't travel.
        let Ok(meta) = fs::symlink_metadata(root.join(path)) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        if meta.len() > LARGEST_FILE {
            return Err(format!("{path} is past the 64 MiB a shared file may be"));
        }
        bytes += meta.len();
        files.push((path.to_owned(), meta.len()));
    }
    if files.len() > MOST_FILES || bytes > MOST_BYTES {
        return Err(format!(
            "its {} files and {bytes} bytes are past what a shared folder carries",
            files.len()
        ));
    }
    Ok(files)
}

/// `path` as it crosses: each name a share keeps home, and each already
/// ending in [`MARK`], with the mark after it, which the far side takes
/// off again.
pub fn crossing(path: &str) -> String {
    path.split('/')
        .map(|name| {
            if BUILT.contains(&name) || name.ends_with(MARK) {
                format!("{name}{MARK}")
            } else {
                name.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Leaves `files`, from the tree at `root`, in the job's folder of the
/// handover, each under the name it crosses by, and lists them last so
/// the list is never ahead of its files. Each is listed at the size its
/// copy came to, which a file rewritten since it was gathered changes.
pub fn left<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
    job: &Path,
) -> io::Result<()> {
    let tree = job.join(TREE);
    fs::create_dir_all(&tree)?;
    let mut list = String::new();
    for path in paths {
        let crossed = crossing(path);
        let into = tree.join(&crossed);
        if let Some(parent) = into.parent() {
            fs::create_dir_all(parent)?;
        }
        let size = fs::copy(root.join(path), into)?;
        // Writing to a String can't fail.
        let _ = writeln!(list, "{size}\t{crossed}");
    }
    list.push_str(END);
    list.push('\n');
    fs::write(job.join(LEFT), list)
}

/// The line a job's far shell writes once its files are there and before
/// anything of the job's, telling a job that ran from one that never did.
pub fn started(id: &str) -> String {
    format!("elsewhere: running {id}")
}

/// What the far machine runs for the job `id`, through `sh -c`: it waits
/// for the job's files to cross, copies them to the far `scratch` and
/// takes the mark off each name a share would have kept home, runs
/// the kind's setup and then the job, its program found on the far path by
/// its name, in the job's folder `within` the tree, then leaves what the
/// kind brings back in the handover and takes its copy away.
pub fn script(
    quest: &str,
    id: &str,
    scratch: &str,
    within: &Path,
    kind: &Kind,
    command: &[String],
) -> String {
    let job = format!(
        "\"${{XDG_CACHE_HOME:-$HOME/.cache}}\"/pi/agentic-harness.pi/quest-workspace/{quest}/{HANDOVER}/{id}"
    );
    let work = format!(
        "\"$HOME\"/{}",
        quoted(&format!("{}/{id}", scratch.trim_matches('/')))
    );
    let folder = match within.to_string_lossy() {
        within if within.is_empty() => "\"$work\"".to_owned(),
        within => format!("\"$work\"/{}", quoted(&within)),
    };
    let program = Path::new(&command[0]).file_name().map_or_else(
        || command[0].clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    let job_line = std::iter::once(&program)
        .chain(&command[1..])
        .map(|word| quoted(word))
        .collect::<Vec<_>>()
        .join(" ");
    let bringing = bringing(kind);
    let setup = kind.setup.trim();
    format!(
        r#"job={job}
work={work}
tab=$(printf '\t')
arrived() {{
  waiting='its list of them'
  [ "$(tail -n 1 "$job/{LEFT}" 2>/dev/null)" = {END} ] || return 1
  while IFS="$tab" read -r size path; do
    [ "$size" = {END} ] && return 0
    case "/$path/" in //*|*/../*|*/./*) continue ;; esac
    waiting=$path
    [ -f "$job/{TREE}/$path" ] || return 1
    [ "$(($(wc -c < "$job/{TREE}/$path")))" -eq "$size" ] || return 1
  done < "$job/{LEFT}"
  return 1
}}
copied() {{
  while IFS="$tab" read -r size path; do
    [ "$size" = {END} ] && break
    case "/$path/" in
      //*|*/../*|*/./*) printf "elsewhere: the job's list names %s, outside its folder\n" "$path" >&2; return 1 ;;
    esac
  done < "$job/{LEFT}"
  cp -R "$job/{TREE}/." "$work/" || return 1
  # Deepest first, so a folder's contents are named before it is, and
  # shortest first within a folder, so target.elsewhere is target before
  # target.elsewhere.elsewhere takes its name.
  find "$work" -name '*{MARK}' |
    awk -F/ '{{ print NF "\t" length($NF) "\t" $0 }}' |
    sort -t "$tab" -k1,1nr -k2,2n | cut -f3- | (
    # A subshell of its own, so its exit ends only the loop in every sh.
    while IFS= read -r name; do
      to=${{name%{MARK}}}
      case "${{to##*/}}" in
        ''|.|..) printf "elsewhere: the job's tree names %s, outside its folder\n" "$name" >&2; exit 1 ;;
      esac
      [ -e "$to" ] && {{ printf "elsewhere: the job's tree names %s twice\n" "$to" >&2; exit 1; }}
      mv "$name" "$to" || exit 1
    done
  )
}}
waited=0
until arrived; do
  if [ "$waited" -ge {CROSSES_WITHIN} ]; then
    printf "elsewhere: the job's files didn't reach %s within {CROSSES_WITHIN}s, waiting on %s\n" "$(uname -n)" "$waiting" >&2
    exit 75
  fi
  sleep 1
  waited=$((waited + 1))
done
rm -rf "$work" && mkdir -p "$work" && copied || {{
  cd "$HOME" && rm -rf "$work" "$job/{TREE}"
  exit 70
}}
cd {folder} || exit 70
echo {started}
(
  {{ {setup}
  }} && exec {job_line}
)
code=$?
mkdir -p "$job/{BACK}"
{bringing}echo {END} >> "$job/{BROUGHT}"
cd "$HOME" && rm -rf "$work" "$job/{TREE}"
exit "$code"
"#,
        started = quoted(&started(id)),
        setup = if setup.is_empty() { ":" } else { setup },
    )
}

/// The far line's step leaving what `kind` brings back in the handover.
/// What comes back crosses under its number, since a kind may bring back
/// what lies under a folder a share keeps home. A hidden file is left, as
/// the near side would pass it over.
fn bringing(kind: &Kind) -> String {
    if kind.brings_back.is_empty() {
        return format!(": > \"$job/{BROUGHT}\"\n");
    }
    let paths = kind
        .brings_back
        .iter()
        .map(|path| quoted(path.trim_matches('/')))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"for path in {paths}; do
  [ -e "$path" ] && find "$path" -type f ! -path '*/.*' ! -name '.*'
done | {{
  n=0
  while IFS= read -r file; do
    cp "$file" "$job/{BACK}/$n" || {{ printf "elsewhere: %s didn't come back\n" "$file" >&2; continue; }}
    printf '%s\t%s\n' "$(($(wc -c < "$job/{BACK}/$n")))" "$file"
    n=$((n + 1))
  done
}} > "$job/{BROUGHT}"
"#
    )
}

/// Hands on what the far run says on `out`: what comes before the line
/// saying the job started is the far shell's own, and goes to stderr; what
/// comes after is the job's, and goes to stdout as it comes. Says whether
/// the job started, which a stream lost after it did never unsays, so a
/// job that ran is never run again here.
pub fn relay(out: impl Read, id: &str) -> bool {
    let mut out = BufReader::new(out);
    let started = format!("{}\n", self::started(id));
    let mut line = Vec::new();
    loop {
        line.clear();
        match out.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return false,
            Ok(_) if line == started.as_bytes() => break,
            // The far shell's own words are worth no failure of the job's.
            Ok(_) => drop(io::stderr().write_all(&line)),
        }
    }
    let mut stdout = io::stdout();
    // A reader of ours that has gone is the job's caller's business, and
    // the job's code still says how it ended.
    drop(io::copy(&mut out, &mut stdout).and_then(|_| stdout.flush()));
    true
}

/// A file brought back: its number in the far list, which names it in
/// the handover, its path in the job's folder, and its size.
#[derive(Debug, PartialEq, Eq)]
pub struct Brought {
    pub number: usize,
    pub path: String,
    pub size: u64,
}

/// What the far machine says it brought back, once its list has crossed
/// whole. A path outside what the kind brings back, or one climbing out
/// of it, is passed over, since the far side doesn't choose what lands
/// here.
pub fn listed(text: &str, brings_back: &[String]) -> Option<Vec<Brought>> {
    let lines: Vec<&str> = text.lines().collect();
    let (last, files) = lines.split_last()?;
    if *last != END {
        return None;
    }
    Some(
        files
            .iter()
            .enumerate()
            .filter_map(|(number, line)| {
                let (size, path) = line.split_once('\t')?;
                let size = size.parse().ok()?;
                brought_back(path, brings_back).then(|| Brought {
                    number,
                    path: path.to_owned(),
                    size,
                })
            })
            .collect(),
    )
}

/// Whether `path` lies within one of the paths a kind brings back.
fn brought_back(path: &str, brings_back: &[String]) -> bool {
    // A hidden file never crosses, so waiting for one would never end.
    let plain = Path::new(path).components().all(
        |part| matches!(part, Component::Normal(name) if !name.to_string_lossy().starts_with('.')),
    );
    plain
        && brings_back.iter().any(|back| {
            let back = back.trim_matches('/');
            !back.is_empty() && (path == back || path.starts_with(&format!("{back}/")))
        })
}

/// Copies what the far machine brought back for the job in the handover
/// folder `job` into `folder`, once it has all crossed `within` seconds,
/// or says why not.
pub fn bring_back(
    job: &Path,
    folder: &Path,
    brings_back: &[String],
    within: u64,
) -> Result<usize, String> {
    if brings_back.is_empty() {
        return Ok(0);
    }
    let numbered = |brought: &Brought| job.join(BACK).join(brought.number.to_string());
    let arrived = |files: &[Brought]| {
        // Only a plain file, since a link crossing would be followed.
        files.iter().all(|brought| {
            fs::symlink_metadata(numbered(brought))
                .is_ok_and(|meta| meta.is_file() && meta.len() == brought.size)
        })
    };
    for _ in 0..=within {
        let heard = fs::read_to_string(job.join(BROUGHT))
            .ok()
            .and_then(|text| listed(&text, brings_back));
        if let Some(files) = heard.filter(|files| arrived(files)) {
            let through = files
                .iter()
                .find_map(|brought| linked(folder, &brought.path).map(|link| (brought, link)));
            if let Some((brought, (link, leads))) = through {
                return Err(format!(
                    "{} would land through the link {}, which {leads}",
                    brought.path,
                    link.display()
                ));
            }
            for brought in &files {
                land(&numbered(brought), &folder.join(&brought.path))
                    .map_err(|error| format!("{} didn't land: {error}", brought.path))?;
            }
            return Ok(files.len());
        }
        // Each end of a shared quest looks once a second, so looking more
        // often finds nothing new.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err(format!("it didn't cross home within {within}s"))
}

/// Puts `from` at `into` under a name of its own first and then moves it
/// there, so a file already at `into`, a hard link to one elsewhere
/// perhaps, is replaced rather than written through.
fn land(from: &Path, into: &Path) -> io::Result<()> {
    let parent = into.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let name = into.file_name().unwrap_or_default().to_string_lossy();
    let landing = parent.join(format!(".{name}{MARK}-landing"));
    fs::copy(from, &landing)?;
    fs::rename(&landing, into).inspect_err(|_| {
        // What's left would be in the person's folder; it's spent anyway.
        let _ = fs::remove_file(&landing);
    })
}

/// The first part of `path` under `folder` that is a symbolic link a file
/// brought back mustn't be written through, and where it leads: out of
/// `folder`, nowhere, which would make what it names, or into a hidden
/// folder, where nothing brought back may go. Any other link within the
/// folder is the person's own.
fn linked(folder: &Path, path: &str) -> Option<(PathBuf, &'static str)> {
    let home = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let mut at = folder.to_path_buf();
    Path::new(path).components().find_map(|part| {
        at.push(part);
        if !fs::symlink_metadata(&at).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return None;
        }
        let Ok(to) = fs::canonicalize(&at) else {
            return Some((at.clone(), "leads nowhere"));
        };
        let Ok(within) = to.strip_prefix(&home) else {
            return Some((at.clone(), "leads out of its folder"));
        };
        within
            .components()
            .any(|part| part.as_os_str().to_string_lossy().starts_with('.'))
            .then(|| (at.clone(), "leads into a hidden folder"))
    })
}

/// A word as a POSIX shell reads it back whole.
pub fn quoted(word: &str) -> String {
    if !word.is_empty()
        && word
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:@%+,".contains(&b))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests;
