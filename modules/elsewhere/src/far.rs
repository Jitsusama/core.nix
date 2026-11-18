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
use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use terminal_kit::Exit;

/// Where a job's folder lies within a quest's scratch.
pub const HANDOVER: &str = "handover/elsewhere";

/// The list of what's left, a `SIZE<TAB>PATH` line a file, and the list of
/// what's brought back, the same; each ends with [`END`].
const LEFT: &str = "elsewhere-files";
const BROUGHT: &str = "elsewhere-back";
const END: &str = "end";
/// The last line of a list of what's brought back that the far side knows
/// it couldn't write whole, in place of [`END`], so the near side says so
/// at once rather than waiting out what will never come.
const CUT: &str = "cut";

/// The folders the job's files lie in, under their paths as crossed, and
/// what it brings back lies in, each named by its line in the list.
const TREE: &str = "tree";
const BACK: &str = "back";

/// The names a shared quest keeps home wherever they lie, as a build
/// fills them, and what such a name crosses with after it.
const BUILT: [&str; 2] = ["target", "node_modules"];
const MARK: &str = ".elsewhere";

/// What a shared folder may hold at most, as mycelium refuses to name
/// one holding more and holds each file whole as it lands.
const MOST_FILES: usize = 10_000;
const MOST_BYTES: u64 = 1 << 30;
const LARGEST_FILE: u64 = 64 << 20;

/// How long each side waits for the other's files to cross, in seconds:
/// a minute's worth of a busy link's looks, five times over.
pub const CROSSES_WITHIN: u64 = 300;

/// How many seconds the near side waits for what comes home before it
/// says it is waiting: past a few looks, so a share keeping pace is quiet.
const SAYS_IT_WAITS: u64 = 10;

/// What marks a name in the far list that isn't UTF-8, in place of its
/// size.
const UNREADABLE: &str = "?";

/// How many of a kind of miss are named, before the rest are counted.
const NAMED: usize = 10;

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
    let copying = copying();
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
{copying}
# Each word for the near side on a line of its own, whatever came before.
say() {{ printf '\n%s\n' "$1"; }}
waited=0
until arrived; do
  if [ "$waited" -ge {CROSSES_WITHIN} ]; then
    printf "elsewhere: the job's files didn't reach %s within {CROSSES_WITHIN}s, waiting on %s\n" "$(uname -n)" "$waiting" >&2
    say {untaken}
    exit {UNTAKEN}
  fi
  sleep 1
  waited=$((waited + 1))
done
if rm -rf "$work" && mkdir -p "$work"; then copied; why=$?; else why=2; fi
if [ "$why" -ne 0 ]; then
  cd "$HOME" && rm -rf "$work" "$job/{TREE}"
  [ "$why" -eq 1 ] && {{ say {refused}; exit {REFUSES}; }}
  say {untaken}
  exit {UNTAKEN}
fi
cd {folder} || {{
  printf "elsewhere: the job's folder holds nothing that travels\n" >&2
  cd "$HOME" && rm -rf "$work" "$job/{TREE}"
  say {refused}
  exit {REFUSES}
}}
say {started}
(
  {{ {setup}
  }} && exec {job_line}
)
code=$?
# Made fresh, so no copy is written through whatever sat at its number.
rm -rf "$job/{BACK}" && mkdir -p "$job/{BACK}"
{bringing}cd "$HOME" && rm -rf "$work" "$job/{TREE}"
exit "$code"
"#,
        started = quoted(&Heard::Started.line(id)),
        refused = quoted(&Heard::Refused.line(id)),
        untaken = quoted(&Heard::Untaken.line(id)),
        setup = if setup.is_empty() { ":" } else { setup },
    )
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
    let lines: Vec<&str> = lines(text).collect();
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

/// What the far side's list `text` says it kept back as past what a share
/// holds, each its size and path, of what the kind brings back.
fn kept<'a>(text: &'a str, brings_back: &'a [String]) -> impl Iterator<Item = (u64, &'a str)> {
    lines(text).filter_map(move |line| {
        let (size, path) = line.strip_prefix('>')?.split_once('\t')?;
        let size = size.parse().ok()?;
        brought_back(path, brings_back).then_some((size, path))
    })
}

/// Why what a job made didn't all come home.
#[derive(Debug, PartialEq, Eq)]
pub enum Missed {
    /// It would land somewhere it mustn't, or is more than a share
    /// carries, which another try won't change.
    Refused(String),
    /// Some of it crossed but couldn't land in the folder, which another
    /// try meets the same way until the folder changes.
    Unlanded(String),
    /// Some of it crossed but found no room here, a disk full or one that
    /// writes nothing, which another try may not meet once it is freed.
    Roomless(String),
    /// Some of it didn't cross, which another try may not meet.
    Lost(String),
}

impl Missed {
    /// Why, in words.
    pub fn why(&self) -> &str {
        match self {
            Missed::Refused(why)
            | Missed::Unlanded(why)
            | Missed::Roomless(why)
            | Missed::Lost(why) => why,
        }
    }

    /// What a passing job that missed so ends with: never 1, which a
    /// job's own failure passes through as, and 75 only where another try
    /// may not meet it.
    pub fn exit(&self) -> Exit {
        match self {
            Missed::Refused(_) | Missed::Unlanded(_) => Exit::Refused,
            Missed::Roomless(_) | Missed::Lost(_) => Exit::TryAgain,
        }
    }
}

/// Whether `files`, all said to have crossed, are more than a share
/// carries, which only a far side counting otherwise than this one's
/// would say, so nobody waits for what never crosses.
fn too_many(files: &[Brought]) -> Option<String> {
    let bytes = files
        .iter()
        .fold(0_u64, |bytes, brought| bytes.saturating_add(brought.size));
    if files.len() > MOST_FILES || bytes > MOST_BYTES {
        return Some(format!(
            "its {} files of {bytes} bytes are past the {MOST_FILES} files or {MOST_BYTES} bytes a share carries",
            files.len()
        ));
    }
    None
}

/// Copies what the far machine brought back for the job in the handover
/// folder `job` into `folder`, once it has all crossed, or says why not.
/// It waits up to `within` seconds past the last time more crossed, and
/// then lands what did. Everything that can land does, whatever else
/// didn't.
pub fn bring_back(
    job: &Path,
    folder: &Path,
    brings_back: &[String],
    within: u64,
) -> Result<usize, Missed> {
    if brings_back.is_empty() {
        return Ok(0);
    }
    let (mut waited, mut most, mut seen, mut said) = (0, 0, None, false);
    loop {
        let text = readable(&fs::read(job.join(BROUGHT)).unwrap_or_default());
        if lines(&text).last() == Some(CUT) {
            return Err(Missed::Lost(
                "the far side couldn't write down all it would bring back".to_owned(),
            ));
        }
        if let Some(files) = listed(&text, brings_back) {
            if let Some(why) = too_many(&files) {
                return Err(Missed::Refused(why));
            }
            let (whole, short): (Vec<_>, Vec<_>) = files
                .into_iter()
                .partition(|brought| crossed(job, brought) == Some(brought.size));
            if short.is_empty() {
                return landed(job, folder, brings_back, &text, &whole, &[]);
            }
            seen = Some((text, whole, short));
        }
        // A share still carrying it is no share that lost it, whether or
        // not its list has crossed yet.
        let arrived = arrived(job);
        if arrived > most {
            (most, waited) = (arrived, 0);
        }
        if waited >= within {
            break;
        }
        if waited == SAYS_IT_WAITS && !said {
            eprintln!(
                "waiting for what it made to come home, until {within}s pass with none of it crossing"
            );
            said = true;
        }
        // Each end of a shared quest looks once a second, so looking more
        // often finds nothing new.
        std::thread::sleep(std::time::Duration::from_secs(1));
        waited += 1;
    }
    match seen {
        Some((text, whole, short)) => landed(job, folder, brings_back, &text, &whole, &short),
        None => Err(Missed::Lost(format!(
            "it didn't cross home within {within}s"
        ))),
    }
}

/// The far side's list in `bytes`, a line each, with a name that isn't
/// UTF-8 marked `?` in place of its size, so the line keeps its number
/// and the rest still reads.
fn readable(bytes: &[u8]) -> String {
    let mut text = String::new();
    for line in bytes.split_inclusive(|&byte| byte == b'\n') {
        if let Ok(line) = std::str::from_utf8(line) {
            text.push_str(line);
            continue;
        }
        let line = String::from_utf8_lossy(line);
        text.push_str(UNREADABLE);
        text.push('\t');
        text.push_str(line.split_once('\t').map_or(&*line, |(_, path)| path));
    }
    text
}

/// The lines of `text`, each just as the far side wrote it, a carriage
/// return at its end included, since a name may end in one.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split_terminator('\n')
}

/// How much of `brought` has crossed into the handover folder `job`, or
/// nothing while none of it has: a plain file only, since a link crossing
/// would be followed. A copy longer than its list says isn't the file
/// listed, so only its size exactly is whole.
fn crossed(job: &Path, brought: &Brought) -> Option<u64> {
    fs::symlink_metadata(job.join(BACK).join(brought.number.to_string()))
        .ok()
        .filter(fs::Metadata::is_file)
        .map(|meta| meta.len())
}

/// How much has reached the handover folder `job`: every plain file in it
/// and in its back folder, under whatever name a share carries one, the
/// list's included, each counting one more than its bytes, so a share
/// still carrying, even empty files, is told from one that stopped.
fn arrived(job: &Path) -> u64 {
    [job.to_owned(), job.join(BACK)]
        .iter()
        .filter_map(|folder| fs::read_dir(folder).ok())
        .flatten()
        .flatten()
        // An entry's metadata is its own, a link's never what it leads to.
        .filter_map(|entry| entry.metadata().ok())
        .filter(fs::Metadata::is_file)
        .fold(0, |arrived: u64, meta| {
            arrived.saturating_add(meta.len()).saturating_add(1)
        })
}

/// At most `NAMED` of `said`, and how many more, so one line can't name
/// thousands.
fn some_of(mut said: Vec<String>, more: &str) -> Vec<String> {
    if said.len() > NAMED {
        let rest = said.len() - NAMED;
        said.truncate(NAMED);
        said.push(format!("{rest} more{more}"));
    }
    said
}

/// Lands `files`, all crossed whole into the handover folder `job`, in
/// `folder`, all but those that would land through a link they mustn't,
/// and says what didn't land, what the far side kept or couldn't bring
/// back, by `text`, its list, and what never crossed `short`.
fn landed(
    job: &Path,
    folder: &Path,
    brings_back: &[String],
    text: &str,
    files: &[Brought],
    short: &[Brought],
) -> Result<usize, Missed> {
    let (mut refused, mut unlanded, mut landed) = (Vec::new(), Vec::new(), 0);
    let mut lasting = false;
    let mut swept = HashSet::new();
    for brought in files {
        if let Some((link, leads)) = landing::linked(folder, &brought.path) {
            refused.push(format!(
                "{} would land through the link {}, which {leads}",
                brought.path,
                link.display()
            ));
            continue;
        }
        let from = job.join(BACK).join(brought.number.to_string());
        match landing::land(&from, &folder.join(&brought.path), &mut swept) {
            Ok(()) => landed += 1,
            Err(error) => {
                lasting |= lasts(&error);
                unlanded.push(format!("{} didn't land: {error}", brought.path));
            }
        }
    }
    // Each kind named apart, so one kind's many can't hide another's.
    let mut refused = some_of(refused, " like them");
    refused.extend(some_of(kept(text, brings_back).map(|(size, path)| {
        if size > LARGEST_FILE {
            format!("{path} is {size} bytes, past the {LARGEST_FILE} a share holds in one file")
        } else {
            format!("{path} is past the {MOST_FILES} files or {MOST_BYTES} bytes a share carries")
        }
    }).collect(), " like them"));
    refused.extend(some_of(
        marked(text, UNREADABLE, brings_back)
            .map(|path| format!("{path} isn't named in UTF-8, so it can't land here"))
            .collect(),
        " like them",
    ));
    let unlanded = some_of(unlanded, " like them");
    let mut missed: Vec<String> = refused.iter().chain(&unlanded).cloned().collect();
    let lost: Vec<String> = marked(text, "-", brings_back).map(str::to_owned).collect();
    if !lost.is_empty() {
        missed.push(format!(
            "{} didn't come back from the far side",
            some_of(lost, "").join(", ")
        ));
    }
    if !short.is_empty() {
        let short = short.iter().map(|brought| brought.path.clone()).collect();
        missed.push(format!(
            "{} didn't cross home",
            some_of(short, "").join(", ")
        ));
    }
    if missed.is_empty() {
        return Ok(landed);
    }
    // So nobody takes the folder for untouched, or for whole.
    if landed > 0 {
        missed.push(format!("the other {landed} landed"));
    }
    let missed = missed.join("; ");
    Err(missing(
        !refused.is_empty(),
        lasting,
        !unlanded.is_empty(),
        missed,
    ))
}

/// What a bring-back that fell short `missed`, given whether any of it
/// was `refused`, whether any couldn't land for a reason that `lasts`, and
/// whether any was `unlanded` at all. What is refused is refused every
/// time, and what couldn't land meets the same folder; only what didn't
/// cross, or met no room, may not on another try.
fn missing(refused: bool, lasts: bool, unlanded: bool, missed: String) -> Missed {
    if refused {
        Missed::Refused(missed)
    } else if lasts {
        Missed::Unlanded(missed)
    } else if unlanded {
        Missed::Roomless(missed)
    } else {
        Missed::Lost(missed)
    }
}

/// Whether what kept a file from landing meets another try too: anything
/// but a disk that filled or that writes nothing, which may be freed.
fn lasts(error: &io::Error) -> bool {
    !matches!(
        error.kind(),
        io::ErrorKind::StorageFull
            | io::ErrorKind::QuotaExceeded
            | io::ErrorKind::ReadOnlyFilesystem
    )
}

/// The paths the far side's list `text` marks with `mark` in place of a
/// size, of what the kind brings back.
fn marked<'a>(
    text: &'a str,
    mark: &'a str,
    brings_back: &'a [String],
) -> impl Iterator<Item = &'a str> {
    lines(text).filter_map(move |line| {
        let (marks, path) = line.split_once('\t')?;
        (marks == mark && brought_back(path, brings_back)).then_some(path)
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

mod heard;
mod landing;
mod steps;

use steps::{bringing, copying};

pub use heard::{Heard, REFUSES, UNTAKEN, relay};

#[cfg(test)]
mod tests;
