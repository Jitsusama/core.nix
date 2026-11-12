//! What passed at a commit, kept so it's never built again: each check
//! `there` built there, each `there --changed` run, with what it named,
//! and each VM test's subtests run-subtests sent, with where their
//! pictures went.
//!
//! A check built on a lent builder leaves its output there, and each
//! builder lent starts with an empty store: the first, the second and the
//! VM tests' each have their own, and each is given back after hours. So
//! Nix's store answered a check built again only on the builder that built
//! it, and one passed on another was built in full again (716 s, then
//! 790 s, for one at one commit on 3 November), while a changed-checks run
//! on a builder lent since evaluated both sides again, 400 to 900 s.
//!
//! A commit names every input of a flake at it, its lock included, so a
//! check that passed at it passes there again. Each commit has a file of
//! its own, a line of JSON for each pass, and its folder in the repository
//! where it's not the top; nothing that failed is kept. Subtests run again
//! copy the pictures from where the last run put them, while those are
//! there. The files are the keeper's where the machine runs it
//! (`keep.rs`), which no session can remove or rewrite, and otherwise in
//! `nix-offload/passed` under XDG's state folder.
//! `nix-offload passed forget COMMIT` has a commit's checks built again.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::keep;

/// Where a flake is in a repository: the commit, and the folder within it
/// when not its top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct At {
    pub commit: String,
    pub dir: Option<String>,
}

impl At {
    /// Where a commit's flake is, when `commit` names one in full: a short
    /// one could name another commit later.
    pub fn new(commit: &str, dir: Option<String>) -> Option<Self> {
        let whole = matches!(commit.len(), 40 | 64)
            && commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        whole.then(|| Self {
            commit: commit.to_owned(),
            dir,
        })
    }

    /// Where a `github:OWNER/NAME/COMMIT` flake is, with `?dir=` or not.
    pub fn of_github(flake: &str) -> Option<Self> {
        let rest = flake.strip_prefix("github:")?;
        let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
        let mut parts = path.split('/');
        let (_owner, _name, commit) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let mut dir = None;
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            match pair.split_once('=') {
                Some(("dir", folder)) => dir = Some(folder.to_owned()),
                // Anything else may change what's built.
                _ => return None,
            }
        }
        Self::new(commit, dir)
    }

    /// The commit, short, as a person reads it.
    pub fn short(&self) -> &str {
        self.commit.get(..12).unwrap_or(&self.commit)
    }
}

/// The record of what passed, in its folder, and the keeper that writes
/// it, where one does.
pub struct Passed {
    folder: PathBuf,
    keeper: Option<PathBuf>,
}

impl Passed {
    /// The record this machine keeps: its keeper's where it runs one, else
    /// the one under the state folder `state`.
    pub fn here(state: &Path) -> Self {
        if keep::stands() {
            Self::kept(Path::new(keep::FOLDER), Path::new(keep::SOCKET))
        } else {
            Self::under(state)
        }
    }

    /// The record under the state folder `state`, written here.
    pub fn under(state: &Path) -> Self {
        Self {
            folder: state.join("nix-offload/passed"),
            keeper: None,
        }
    }

    /// The record in `folder`, which the keeper at `socket` writes.
    pub fn kept(folder: &Path, socket: &Path) -> Self {
        Self {
            folder: folder.to_owned(),
            keeper: Some(socket.to_owned()),
        }
    }

    /// Remove what passed at `commit`, so its checks build again.
    pub fn forget(&self, at: &At) -> Result<(), String> {
        match &self.keeper {
            Some(keeper) => keep::ask(keeper, &serde_json::json!({ "forget": at.commit })),
            None => match fs::remove_file(self.file(at)) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    Err(error.to_string())
                }
                _ => Ok(()),
            },
        }
    }

    /// The file a commit's passes are kept in.
    pub fn file(&self, at: &At) -> PathBuf {
        self.folder.join(&at.commit)
    }

    /// Each line kept for the flake at `at`.
    fn lines(&self, at: &At) -> Vec<serde_json::Value> {
        fs::read_to_string(self.file(at))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["dir"].as_str() == at.dir.as_deref())
            .collect()
    }

    /// The checks, as their attributes, that passed at `at`.
    pub fn checks(&self, at: &At) -> Vec<String> {
        self.lines(at)
            .iter()
            .filter_map(|line| line["check"].as_str().map(str::to_owned))
            .collect()
    }

    /// What a changed-checks run with `arguments` printed when it passed
    /// at `at`, if one did.
    pub fn run(&self, at: &At, arguments: &[String]) -> Option<String> {
        self.lines(at).iter().find_map(|line| {
            (line["run"] == serde_json::json!(arguments))
                .then(|| line["printed"].as_str().map(str::to_owned))
                .flatten()
        })
    }

    /// Where the pictures of subtests run as `run` went when they passed at
    /// `at`, the newest that's still there, if any is.
    pub fn subtests(&self, at: &At, run: &str) -> Option<PathBuf> {
        self.lines(at)
            .iter()
            .rev()
            .filter(|line| line["subtests"].as_str() == Some(run))
            .filter_map(|line| line["out"].as_str().map(PathBuf::from))
            .find(|out| out.is_dir())
    }

    /// Keep that subtests run as `run` passed at `at`, their pictures in
    /// `out`.
    pub fn keep_subtests(&self, at: &At, run: &str, out: &Path) {
        let line = serde_json::json!({
            "dir": at.dir,
            "subtests": run,
            "out": out.to_string_lossy(),
        });
        self.keep(at, &[line]);
    }

    /// Keep that each of `checks` passed at `at`, those not kept already.
    pub fn keep_checks(&self, at: &At, checks: &[String]) {
        let kept = self.checks(at);
        let lines: Vec<serde_json::Value> = checks
            .iter()
            .filter(|check| !kept.contains(check))
            .map(|check| serde_json::json!({ "dir": at.dir, "check": check }))
            .collect();
        self.keep(at, &lines);
    }

    /// Keep that a changed-checks run with `arguments`, started at
    /// `started` in seconds since 1970, passed at `at`, having printed
    /// `printed`. When it started orders merges whose gates share files
    /// (`running.rs`), and when it ended, now, says with it how long the
    /// gate took, which cadence's day reads. Only the run's line says so,
    /// since a check passed later at the commit joins the same file.
    pub fn keep_run(&self, at: &At, arguments: &[String], printed: &str, started: u64) {
        if self.run(at, arguments).is_none() {
            let ended = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs());
            let line = serde_json::json!({
                "dir": at.dir,
                "run": arguments,
                "printed": printed,
                "started": started,
                "ended": ended,
            });
            self.keep(at, &[line]);
        }
    }

    /// Add `lines` to the commit's file, each written whole in one go, so
    /// two runs keeping theirs at once never mix them.
    fn keep(&self, at: &At, lines: &[serde_json::Value]) {
        if lines.is_empty() {
            return;
        }
        if let Some(keeper) = &self.keeper {
            let asked = serde_json::json!({ "keep": at.commit, "lines": lines });
            if let Err(why) = keep::ask(keeper, &asked) {
                eprintln!(
                    "nix-offload: couldn't keep what passed at {}: {why}",
                    at.short()
                );
            }
            return;
        }
        let mut text = String::new();
        for line in lines {
            text.push_str(&line.to_string());
            text.push('\n');
        }
        let file = self.file(at);
        let written = fs::create_dir_all(&self.folder)
            .and_then(|()| fs::OpenOptions::new().create(true).append(true).open(&file))
            .and_then(|mut opened| opened.write_all(text.as_bytes()));
        if let Err(error) = written {
            eprintln!(
                "nix-offload: couldn't keep what passed in {}: {error}",
                file.display()
            );
        }
    }
}

/// `nix-offload passed forget COMMIT`: have a commit's checks built again.
pub fn forgetting(state: &Path, commit: &str) -> ExitCode {
    let Some(at) = At::new(commit, None) else {
        eprintln!("nix-offload: passed forget needs a whole commit, not {commit}");
        return ExitCode::from(64);
    };
    match Passed::here(state).forget(&at) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!(
                "nix-offload: couldn't forget what passed at {}: {why}",
                at.short()
            );
            ExitCode::FAILURE
        }
    }
}

/// `nix-offload passed import`: hand the keeper, once it stands, the
/// records kept under the state folder, by a nix-offload from before it
/// did or one still running from then.
pub fn importing(state: &Path) -> ExitCode {
    let kept = Passed::here(state);
    if kept.keeper.is_none() {
        eprintln!("nix-offload: this machine runs no keeper to hand records to");
        return ExitCode::from(69);
    }
    let (handed, failed) = hand_over(&Passed::under(state), &kept);
    println!("nix-offload: handed the keeper {handed} commits' records");
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Hand `kept`'s keeper each line of `own` its record lacks, a commit at a
/// time, even one it holds already, since a run from before the keeper
/// may add to a commit after it stands. Each file handed over whole goes,
/// so nothing handed over comes back after it's forgotten. Says how many
/// commits had lines handed over, and whether any couldn't be.
fn hand_over(own: &Passed, kept: &Passed) -> (usize, bool) {
    let Some(keeper) = &kept.keeper else {
        return (0, true);
    };
    let read = |file: &Path| -> Vec<serde_json::Value> {
        fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    };
    let (mut handed, mut failed) = (0, false);
    for entry in fs::read_dir(&own.folder).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(at) = At::new(&name, None) else {
            continue;
        };
        let held = read(&kept.file(&at));
        let lines: Vec<serde_json::Value> = read(&entry.path())
            .into_iter()
            .filter(|line| !held.contains(line))
            .collect();
        if !lines.is_empty() {
            if let Err(why) =
                keep::ask(keeper, &serde_json::json!({ "keep": name, "lines": lines }))
            {
                eprintln!("nix-offload: couldn't hand over {}: {why}", at.short());
                failed = true;
                continue;
            }
            handed += 1;
        }
        if let Err(error) = fs::remove_file(entry.path()) {
            eprintln!(
                "nix-offload: handed over {} but kept its file: {error}",
                at.short()
            );
        }
    }
    (handed, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn only_a_whole_commit_names_where_a_flake_is() {
        assert!(At::new(COMMIT, None).is_some());
        assert!(At::new(&"a".repeat(64), None).is_some());
        for short in ["abc", &COMMIT[..39], &COMMIT.to_uppercase(), "main"] {
            assert!(At::new(short, None).is_none(), "{short}");
        }
    }

    #[test]
    fn a_github_flake_at_a_commit_is_read_with_its_folder() {
        assert_eq!(
            At::of_github(&format!("github:o/r/{COMMIT}")),
            At::new(COMMIT, None)
        );
        assert_eq!(
            At::of_github(&format!("github:o/r/{COMMIT}?dir=sub")),
            At::new(COMMIT, Some("sub".to_owned()))
        );
        for unknown in [
            "github:o/r".to_owned(),
            "github:o/r/main".to_owned(),
            format!("github:o/r/{COMMIT}?narHash=x"),
            format!("github:o/r/{COMMIT}/more"),
            format!("git+https://x/r?rev={COMMIT}"),
        ] {
            assert_eq!(At::of_github(&unknown), None, "{unknown}");
        }
    }

    #[test]
    fn passes_are_kept_by_commit_and_folder() {
        let state = tempfile::tempdir().unwrap();
        let passed = Passed::under(state.path());
        let top = At::new(COMMIT, None).unwrap();
        let sub = At::new(COMMIT, Some("sub".to_owned())).unwrap();
        let checks = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| (*name).to_owned()).collect()
        };
        passed.keep_checks(&top, &checks(&["checks.x.a", "checks.x.b"]));
        passed.keep_checks(&top, &checks(&["checks.x.b", "checks.x.c"]));
        assert_eq!(
            passed.checks(&top),
            ["checks.x.a", "checks.x.b", "checks.x.c"]
        );
        assert_eq!(passed.checks(&sub), [] as [std::string::String; 0]);

        let arguments = checks(&["--build"]);
        assert_eq!(passed.run(&top, &arguments), None);
        passed.keep_run(&top, &arguments, "changed  a\n", 1_700_000_000);
        passed.keep_run(&top, &arguments, "changed  a\n", 1_700_000_001);
        assert_eq!(
            passed.run(&top, &arguments).as_deref(),
            Some("changed  a\n")
        );
        assert_eq!(passed.run(&top, &checks(&["--build", "--no-vm"])), None);
        assert_eq!(passed.run(&sub, &arguments), None);
        let kept = fs::read_to_string(passed.file(&top)).unwrap();
        assert_eq!(kept.lines().count(), 4, "{kept}");
        assert!(
            kept.contains("\"started\":1700000000"),
            "the run's start wasn't kept: {kept}"
        );
    }

    #[test]
    fn a_gates_end_is_kept_on_its_run_alone() {
        let state = tempfile::tempdir().unwrap();
        let passed = Passed::under(state.path());
        let top = At::new(COMMIT, None).unwrap();
        let now = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        };
        let before = now();
        passed.keep_run(&top, &["--build".to_owned()], "changed  a\n", 1_700_000_000);
        let after = now();
        passed.keep_checks(&top, &["checks.x.a".to_owned()]);
        let kept = fs::read_to_string(passed.file(&top)).unwrap();
        let lines: Vec<serde_json::Value> = kept
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let ended = lines[0]["ended"].as_u64();
        assert!(
            ended.is_some_and(|ended| (before..=after).contains(&ended)),
            "the gate's end wasn't kept as it passed: {kept}"
        );
        assert!(
            lines[1].get("ended").is_none(),
            "a check passed later was kept as a gate's end: {kept}"
        );
    }

    #[test]
    fn subtests_that_passed_are_found_by_their_pictures_while_those_are_there() {
        let state = tempfile::tempdir().unwrap();
        let passed = Passed::under(state.path());
        let at = At::new(COMMIT, None).unwrap();
        let (first, second) = (state.path().join("first"), state.path().join("second"));
        fs::create_dir(&first).unwrap();
        passed.keep_subtests(&at, "run a", &first);
        assert_eq!(passed.subtests(&at, "run a"), Some(first.clone()));
        assert_eq!(passed.subtests(&at, "run b"), None);
        passed.keep_subtests(&at, "run a", &second);
        assert_eq!(
            passed.subtests(&at, "run a"),
            Some(first.clone()),
            "the newest is gone, so the one still there"
        );
        fs::remove_dir(&first).unwrap();
        assert_eq!(passed.subtests(&at, "run a"), None, "none is left");
    }

    #[test]
    fn a_kept_record_is_written_and_forgotten_by_its_keeper_alone() {
        let place = tempfile::tempdir().unwrap();
        let (socket, folder) = (place.path().join("keep.sock"), place.path().join("kept"));
        fs::create_dir(&folder).unwrap();
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let answers = folder.clone();
        let keeper = std::thread::spawn(move || {
            keep::serve(&listener, &answers, std::time::Duration::from_millis(500));
        });
        let passed = Passed::kept(&folder, &socket);
        let at = At::new(COMMIT, None).unwrap();
        passed.keep_checks(&at, &["checks.x.a".to_owned()]);
        assert_eq!(
            passed.checks(&at),
            ["checks.x.a"],
            "the keeper didn't keep it"
        );
        assert_eq!(passed.forget(&at), Ok(()), "the keeper didn't forget it");
        assert_eq!(
            passed.checks(&at),
            [] as [String; 0],
            "a forgotten commit still had its checks"
        );
        keeper.join().unwrap();
    }

    #[test]
    fn import_hands_over_what_a_held_record_lacks_and_lets_its_file_go() {
        let place = tempfile::tempdir().unwrap();
        let (socket, folder) = (place.path().join("keep.sock"), place.path().join("kept"));
        fs::create_dir(&folder).unwrap();
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let answers = folder.clone();
        let keeper = std::thread::spawn(move || {
            keep::serve(&listener, &answers, std::time::Duration::from_millis(500));
        });
        let (kept, own) = (Passed::kept(&folder, &socket), Passed::under(place.path()));
        let at = At::new(COMMIT, None).unwrap();
        kept.keep_checks(&at, &["checks.x.a".to_owned()]);
        own.keep_checks(&at, &["checks.x.a".to_owned(), "checks.x.b".to_owned()]);
        assert_eq!(
            hand_over(&own, &kept),
            (1, false),
            "its lines weren't handed over"
        );
        assert_eq!(
            kept.checks(&at),
            ["checks.x.a", "checks.x.b"],
            "the keeper lacks a line, or holds one twice"
        );
        assert!(!own.file(&at).exists(), "the handed-over file stayed");
        assert_eq!(
            hand_over(&own, &kept),
            (0, false),
            "a second import handed some again"
        );
        keeper.join().unwrap();
    }

    #[test]
    fn a_forgotten_record_here_is_built_again() {
        let state = tempfile::tempdir().unwrap();
        let passed = Passed::under(state.path());
        let at = At::new(COMMIT, None).unwrap();
        passed.keep_checks(&at, &["checks.x.a".to_owned()]);
        assert_eq!(passed.forget(&at), Ok(()), "it wasn't forgotten");
        assert!(!passed.file(&at).exists(), "its file stayed");
    }
}
