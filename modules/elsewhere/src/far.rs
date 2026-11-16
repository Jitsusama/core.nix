//! A job on a far machine: its tree copied to the same place under the far
//! home's scratch, the job run there over ssh in a shell that first runs
//! its kind's setup and finds its program on the far path, its output
//! streamed back as it comes, and what its kind brings back copied home.
//!
//! The copy goes over rsync rather than a shared filesystem, since the
//! jobs sent read a few megabytes and run for minutes: Delta's spec
//! folder, 2.9 MB, copied in 1.6 s, and a model check of it ran in 10.4 s
//! on the Mac against 64.0 s here while this machine was crowded.

use crate::settings::{Copies, Kind};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// ssh's own status when it couldn't reach a machine or log in.
const UNREACHED: i32 = 255;

/// Where a job's tree is, here and on the far machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copy {
    /// The tree copied, here.
    pub root: PathBuf,
    /// The job's folder, from the tree's top.
    pub within: PathBuf,
    /// The tree on the far machine, from its home.
    pub there: String,
}

/// The copy of the tree at `root` holding the job's folder `folder`, as it
/// lies under the far home's `scratch`: at the same path from home where
/// it's under this home, else at its whole path under `root`.
pub fn copy(root: &Path, folder: &Path, home: &Path, scratch: &str) -> Copy {
    let within = folder
        .strip_prefix(root)
        .unwrap_or(Path::new(""))
        .to_path_buf();
    let from = root.strip_prefix(home).map_or_else(
        |_| Path::new("root").join(root.strip_prefix("/").unwrap_or(root)),
        Path::to_path_buf,
    );
    let there = Path::new(scratch.trim_end_matches('/')).join(from);
    Copy {
        root: root.to_path_buf(),
        within,
        there: there.to_string_lossy().into_owned(),
    }
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

/// The arguments rsync takes to copy the tree there, making its folder
/// first, since a far rsync makes only the last folder of a path.
pub fn sending(copy: &Copy, kind: &Kind, machine: &str) -> Vec<String> {
    let mut args = vec![
        "--archive".to_owned(),
        "--delete".to_owned(),
        "--rsync-path".to_owned(),
        format!("mkdir -p {} && rsync", quoted(&copy.there)),
    ];
    for exclude in &kind.excludes {
        args.push(format!("--exclude={exclude}"));
    }
    args.push(format!("{}/", copy.root.display()));
    args.push(format!("{machine}:{}/", copy.there));
    args
}

/// The arguments rsync takes to bring `path`, from the job's folder,
/// home beside where it lies here.
pub fn bringing(copy: &Copy, path: &str, machine: &str) -> (Vec<String>, PathBuf) {
    let here = copy.root.join(&copy.within).join(path);
    let there = Path::new(&copy.there).join(&copy.within).join(path);
    let into = here
        .parent()
        .map_or_else(|| copy.root.clone(), Path::to_path_buf);
    let args = vec![
        "--archive".to_owned(),
        format!("{machine}:{}", there.display()),
        format!("{}/", into.display()),
    ];
    (args, into)
}

/// The shell line the far machine runs: into the job's folder, the kind's
/// setup, then the job, its program found on the far path by its name.
pub fn line(copy: &Copy, kind: &Kind, command: &[String]) -> String {
    let folder = Path::new(&copy.there).join(&copy.within);
    let mut line = format!("cd {} && ", quoted(&folder.to_string_lossy()));
    if !kind.setup.trim().is_empty() {
        line.push_str("{ ");
        line.push_str(kind.setup.trim());
        line.push_str("\n} && ");
    }
    line.push_str("exec");
    let program = Path::new(&command[0]).file_name().map_or_else(
        || command[0].clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    for word in std::iter::once(&program).chain(&command[1..]) {
        line.push(' ');
        line.push_str(&quoted(word));
    }
    line
}

/// How a step on the far machine ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    Done,
    /// ssh couldn't reach the machine.
    Unreached,
    /// It ended with this code, or 128 and the signal that ended it.
    Code(u8),
}

/// How a status a far step ended with reads.
pub fn ended(status: ExitStatus) -> Ended {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(0), _) => Ended::Done,
        (Some(UNREACHED), _) => Ended::Unreached,
        (Some(code), _) => Ended::Code(u8::try_from(code).unwrap_or(1)),
        (None, signal) => {
            let signal = signal
                .and_then(|signal| u8::try_from(signal).ok())
                .unwrap_or(0);
            Ended::Code(128u8.saturating_add(signal))
        }
    }
}

/// Runs `program` with `args`, its streams the caller's, saying how it
/// ended, or why it didn't start.
pub fn step(program: &str, args: &[String]) -> Result<Ended, String> {
    Command::new(program)
        .args(args)
        .status()
        .map(ended)
        .map_err(|error| format!("{program} didn't start: {error}"))
}

/// The arguments ssh takes to run `line` on `machine`, never asking for a
/// password, since nobody is there to type one.
pub fn running(machine: &str, line: &str) -> Vec<String> {
    vec![
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
        machine.to_owned(),
        line.to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::{bringing, copy, line, quoted, sending};
    use crate::settings::{Copies, Kind};
    use std::path::{Path, PathBuf};

    fn kind(setup: &str) -> Kind {
        Kind {
            description: String::new(),
            programs: vec!["tla-rs".into()],
            setup: setup.into(),
            copies: Copies::Folder,
            excludes: vec!["/target".into()],
            brings_back: Vec::new(),
            needs: Vec::new(),
            lasts: None,
        }
    }

    #[test]
    fn a_tree_under_home_lies_at_the_same_path_under_the_far_scratch() {
        let copied = copy(
            Path::new("/h/src/spec"),
            Path::new("/h/src/spec/tlc"),
            Path::new("/h"),
            ".cache/e/",
        );
        assert_eq!(
            copied.there, ".cache/e/src/spec",
            "the tree moved from its path under home"
        );
        assert_eq!(
            copied.within,
            PathBuf::from("tlc"),
            "the job's folder was lost"
        );
        let outside = copy(
            Path::new("/srv/spec"),
            Path::new("/srv/spec"),
            Path::new("/h"),
            ".cache/e",
        );
        assert_eq!(
            outside.there, ".cache/e/root/srv/spec",
            "a tree outside home wasn't kept apart"
        );
    }

    #[test]
    fn the_far_line_runs_the_setup_then_the_program_by_its_name() {
        let copied = copy(
            Path::new("/h/spec"),
            Path::new("/h/spec/a b"),
            Path::new("/h"),
            ".cache/e",
        );
        let command: Vec<String> = ["/nix/store/x-tla-rs/bin/tla-rs", "-config", "it's.cfg"]
            .map(str::to_owned)
            .into();
        assert_eq!(
            line(&copied, &kind("export PATH=/far/bin:$PATH"), &command),
            "cd '.cache/e/spec/a b' && { export PATH=/far/bin:$PATH\n} && \
             exec tla-rs -config 'it'\\''s.cfg'",
            "the far line wasn't the setup then the job"
        );
        assert!(
            !line(&copied, &kind(""), &command).contains('{'),
            "an empty setup was still run"
        );
    }

    #[test]
    fn the_copy_makes_its_far_folder_and_keeps_out_what_the_kind_excludes() {
        let copied = copy(
            Path::new("/h/spec"),
            Path::new("/h/spec"),
            Path::new("/h"),
            ".cache/e",
        );
        let args = sending(&copied, &kind(""), "far");
        assert_eq!(
            args,
            [
                "--archive",
                "--delete",
                "--rsync-path",
                "mkdir -p .cache/e/spec && rsync",
                "--exclude=/target",
                "/h/spec/",
                "far:.cache/e/spec/",
            ],
            "the copy's arguments weren't these"
        );
        let (back, into) = bringing(&copied, "out/states.json", "far");
        assert_eq!(
            back[1], "far:.cache/e/spec/out/states.json",
            "it brought back the wrong path"
        );
        assert_eq!(
            into,
            PathBuf::from("/h/spec/out"),
            "it brought it back to the wrong folder"
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
