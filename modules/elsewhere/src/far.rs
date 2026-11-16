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
//! written last, and the other waits until all of it is there.

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

/// The folders the job's files and what it brings back lie in.
const TREE: &str = "tree";
const BACK: &str = "back";

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
/// certificate, never build output, and nothing the kind excludes.
/// Mycelium keeps the hidden ones and build output home anyway; leaving
/// them out here keeps the list of what's left to what can arrive.
pub fn travels(path: &str, excludes: &[String]) -> bool {
    let names: Vec<&str> = path.split('/').collect();
    let kept_home = names
        .iter()
        .any(|name| name.starts_with('.') || matches!(*name, "target" | "node_modules"));
    let secret = Path::new(path).extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("pem") || extension.eq_ignore_ascii_case("key")
    });
    !kept_home && !secret && !excludes.iter().any(|exclude| excluded(path, exclude))
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

/// Leaves `files`, from the tree at `root`, in the job's folder of the
/// handover, listed last so the list is never ahead of its files.
pub fn left(root: &Path, files: &[(String, u64)], job: &Path) -> io::Result<()> {
    let tree = job.join(TREE);
    fs::create_dir_all(&tree)?;
    let mut list = String::new();
    for (path, size) in files {
        let into = tree.join(path);
        if let Some(parent) = into.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(root.join(path), into)?;
        // Writing to a String can't fail.
        let _ = writeln!(list, "{size}\t{path}");
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
/// for the job's files to cross, copies them to the far `scratch`, runs
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
    let bringing = if kind.brings_back.is_empty() {
        String::new()
    } else {
        let paths = kind
            .brings_back
            .iter()
            .map(|path| quoted(path.trim_matches('/')))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            r#"for path in {paths}; do
  [ -e "$path" ] || continue
  into="$job/{BACK}/$(dirname "$path")"
  mkdir -p "$into" && cp -R "$path" "$into/"
done
"#
        )
    };
    let setup = kind.setup.trim();
    format!(
        r#"job={job}
work={work}
tab=$(printf '\t')
arrived() {{
  [ "$(tail -n 1 "$job/{LEFT}" 2>/dev/null)" = {END} ] || return 1
  while IFS="$tab" read -r size path; do
    [ "$size" = {END} ] && return 0
    [ -f "$job/{TREE}/$path" ] || return 1
    [ "$(($(wc -c < "$job/{TREE}/$path")))" -eq "$size" ] || return 1
  done < "$job/{LEFT}"
  return 1
}}
waited=0
until arrived; do
  if [ "$waited" -ge {CROSSES_WITHIN} ]; then
    echo "the job's files didn't reach $(uname -n) within {CROSSES_WITHIN}s" >&2
    exit 75
  fi
  sleep 1
  waited=$((waited + 1))
done
rm -rf "$work" && mkdir -p "$work" && cp -R "$job/{TREE}/." "$work/" || exit 70
cd {folder} || exit 70
echo {started}
(
  {{ {setup}
  }} && exec {job_line}
)
code=$?
mkdir -p "$job/{BACK}"
{bringing}(cd "$job/{BACK}" && find . -type f) | while IFS= read -r file; do
  printf '%s\t%s\n' "$(($(wc -c < "$job/{BACK}/$file")))" "${{file#./}}"
done > "$job/{BROUGHT}"
echo {END} >> "$job/{BROUGHT}"
cd "$HOME" && rm -rf "$work" "$job/{TREE}"
exit "$code"
"#,
        started = quoted(&started(id)),
        setup = if setup.is_empty() { ":" } else { setup },
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

/// What the far machine says it brought back, once its list has crossed
/// whole: the paths a kind brings back, and their sizes. A path outside
/// what the kind brings back, or one climbing out of it, is passed over,
/// since the far side doesn't choose what lands here.
pub fn listed(text: &str, brings_back: &[String]) -> Option<Vec<(String, u64)>> {
    let lines: Vec<&str> = text.lines().collect();
    let (last, files) = lines.split_last()?;
    if *last != END {
        return None;
    }
    Some(
        files
            .iter()
            .filter_map(|line| {
                let (size, path) = line.split_once('\t')?;
                let size = size.parse().ok()?;
                brought_back(path, brings_back).then(|| (path.to_owned(), size))
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
/// folder `job` into `folder`, once it has all crossed, or says why not.
pub fn bring_back(job: &Path, folder: &Path, brings_back: &[String]) -> Result<usize, String> {
    if brings_back.is_empty() {
        return Ok(0);
    }
    let arrived = |files: &[(String, u64)]| {
        files.iter().all(|(path, size)| {
            fs::metadata(job.join(BACK).join(path)).is_ok_and(|meta| meta.len() == *size)
        })
    };
    for _ in 0..=CROSSES_WITHIN {
        let heard = fs::read_to_string(job.join(BROUGHT))
            .ok()
            .and_then(|text| listed(&text, brings_back));
        if let Some(files) = heard.filter(|files| arrived(files)) {
            for (path, _) in &files {
                let into = folder.join(path);
                if let Some(parent) = into.parent() {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                fs::copy(job.join(BACK).join(path), &into)
                    .map_err(|error| format!("{path} didn't land: {error}"))?;
            }
            return Ok(files.len());
        }
        // Each end of a shared quest looks once a second, so looking more
        // often finds nothing new.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err(format!("it didn't cross home within {CROSSES_WITHIN}s"))
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
mod tests {
    use super::{listed, plain_quest, quoted, script, started, travels};
    use crate::settings::{Copies, Kind};
    use std::path::Path;

    fn kind(setup: &str) -> Kind {
        Kind {
            description: String::new(),
            programs: vec!["tla-rs".into()],
            setup: setup.into(),
            copies: Copies::Folder,
            excludes: vec!["/target".into()],
            brings_back: vec!["out".into()],
            needs: Vec::new(),
            lasts: None,
        }
    }

    #[test]
    fn neither_secrets_nor_hidden_files_nor_what_s_excluded_travel() {
        let excludes = ["/states".to_owned(), "cache".to_owned()];
        for path in [
            ".env",
            ".env.local",
            "deploy/.env",
            ".git/config",
            "keys/server.key",
            "cert.pem",
            "target/debug/x",
            "web/node_modules/a.js",
            "states/1.bin",
            "a/cache/b",
            "cache",
        ] {
            assert!(!travels(path, &excludes), "{path} travelled");
        }
        for path in ["Spec.tla", "a/states/1.bin", "src/cached.rs", "keys.md"] {
            assert!(travels(path, &excludes), "{path} was kept home");
        }
    }

    #[test]
    fn the_far_line_waits_for_the_files_then_runs_the_setup_and_the_job_by_name() {
        let command: Vec<String> = ["/nix/store/x-tla-rs/bin/tla-rs", "-config", "it's.cfg"]
            .map(str::to_owned)
            .into();
        let line = script(
            "QEST-20261115-ABC123",
            "7-9",
            ".cache/e/",
            Path::new("a b"),
            &kind("export PATH=/far/bin:$PATH"),
            &command,
        );
        for says in [
            "/quest-workspace/QEST-20261115-ABC123/handover/elsewhere/7-9\n",
            "work=\"$HOME\"/.cache/e/7-9\n",
            "until arrived; do",
            "cd \"$work\"/'a b' || exit 70\necho 'elsewhere: running 7-9'\n",
            "{ export PATH=/far/bin:$PATH\n  } && exec tla-rs -config 'it'\\''s.cfg'\n",
            "for path in out; do",
            "exit \"$code\"",
        ] {
            assert!(line.contains(says), "the far line lacks {says:?}:\n{line}");
        }
        assert!(
            line.find("until arrived").unwrap() < line.find("exec tla-rs").unwrap(),
            "the job ran before its files arrived"
        );
        assert!(
            script(
                "QEST-20261115-ABC123",
                "1",
                "s",
                Path::new(""),
                &kind(""),
                &command
            )
            .contains("cd \"$work\" || exit 70\necho 'elsewhere: running 1'\n(\n  { :\n"),
            "a job at the tree's top or with no setup wasn't run there plainly"
        );
    }

    #[test]
    fn only_what_the_kind_brings_back_comes_home_once_listed_whole() {
        let back = ["out".to_owned()];
        assert_eq!(
            listed("2\tout/n\n", &back),
            None,
            "a list not yet whole was taken"
        );
        assert_eq!(
            listed(
                "2\tout/n\n3\tout/../../.bashrc\n4\tsrc/x\n5\toutside\n6\tout/.hidden\nend\n",
                &back
            ),
            Some(vec![("out/n".to_owned(), 2)]),
            "something outside what the kind brings back was taken"
        );
    }

    #[test]
    fn a_quest_names_a_folder_and_nothing_more() {
        assert!(
            plain_quest("QEST-20261115-ABC123"),
            "a quest's id was refused"
        );
        for odd in ["QEST-20261115-ABC12/", "../QEST", "QEST-2026111-ABC123", ""] {
            assert!(!plain_quest(odd), "{odd:?} was taken for a quest");
        }
        assert_eq!(
            started("1-2"),
            "elsewhere: running 1-2",
            "the start line moved"
        );
    }

    #[test]
    fn a_word_is_quoted_only_when_a_shell_would_split_it() {
        assert_eq!(quoted("-workers"), "-workers", "a plain word was quoted");
        assert_eq!(quoted(""), "''", "an empty word was lost");
        assert_eq!(
            quoted("$HOME"),
            "'$HOME'",
            "a variable was left for the far shell"
        );
    }
}
