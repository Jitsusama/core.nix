//! `nix-offload answer QUEST COMMIT`: runs the gate a Mac asked for in
//! its quest's scratch, on the machine mycelium started it on.
//!
//! What the Mac left is a stranger's until checked, since whoever writes
//! the quest's scratch could have written it. So the request is read once
//! and held to its grammar, the bundle is read once into a folder of this
//! machine's state that nothing else writes, and its header is judged
//! before git sees it: one ref, `HEAD`, at the commit asked for. A clone
//! of the forge's main is made fresh, from a mirror only the forge
//! fills, and the bundle's commits are fetched into it object by object
//! checked, refused unless each commit it needs is on that main and the
//! commit descends from it (`fence.rs` runs each git). Only then does
//! `there --changed` run, through `outside`, on the exemption's terms,
//! kept to the lent builders, so its pass is kept where a merge reads it.
//!
//! Two gates run at once at most, and a commit already being gated isn't
//! gated beside itself; either waits for the Mac to ask again. Each run is
//! journalled, and a gate's folders go a week after.

use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime};

use super::fence::{Fence, Net};
use super::{BUNDLE, Found, LARGEST_BUNDLE, LOG, NOT_NOW, REFUSED, REQUEST, RESULT, Repo, Request};

/// How many gates run at once.
const AT_ONCE: usize = 2;
/// How long a gate's folders are kept.
const KEPT: Duration = Duration::from_hours(7 * 24);
/// The most of a run's log a gate's folder carries back: the end.
const LARGEST_LOG: u64 = 8 << 20;
/// What a gate exits with when the forge or git couldn't be reached:
/// sysexits.h's unavailable service.
const UNREACHED: u8 = 69;

const USED: &str = "Usage: nix-offload answer QUEST COMMIT";

const HELP: &str = "nix-offload answer: run the gate a Mac asked for, from its quest's scratch.

Mycelium starts it, as `nix-offload ask` on the Mac asks. It reads the request and bundle in \
the quest's handover/gates/COMMIT once, checks the bundle against a fresh clone of the \
forge's main, runs `nix-offload there --changed` for the commit on the lent builders, and \
leaves result.json and log beside the bundle. result.json is a convenience: a merge reads the \
pass the build keeper kept.

Example:
  nix-offload answer QEST-20261115-ABC123 0123456789abcdef0123456789abcdef01234567";

/// Where a gate's answer keeps its own: this machine's state, which no
/// other machine's files reach.
#[derive(Debug, Clone)]
pub struct Places {
    /// `nix-offload/gates` under XDG's state folder.
    pub state: PathBuf,
    /// Where repos are fetched from ([`super::forge`]).
    pub forge: String,
}

impl Places {
    fn work(&self, commit: &str) -> PathBuf {
        self.state.join("work").join(commit)
    }

    fn mirror(&self, repo: &Repo) -> PathBuf {
        self.state
            .join("mirrors")
            .join(&repo.owner)
            .join(format!("{}.git", repo.name))
    }

    fn locks(&self) -> PathBuf {
        self.state.join("locks")
    }
}

/// A lock held while the file stays open.
struct Held {
    _file: fs::File,
}

/// The lock on `path`, taken now or not at all.
fn locked_now(path: &Path) -> io::Result<Option<Held>> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    // SAFETY: flock on a descriptor this owns.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        Ok(Some(Held { _file: file }))
    } else {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

/// The lock on `path`, waited for.
fn locked(path: &Path) -> io::Result<Held> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    // SAFETY: flock on a descriptor this owns.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
        Ok(Held { _file: file })
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Takes away what's in `folder` last changed over [`KEPT`] ago.
fn sweep(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|changed| now.duration_since(changed).is_ok_and(|age| age > KEPT));
        if old {
            // A folder that can't go now goes at the next gate's sweep.
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// How long `answer` waits for a gate to cross, where the asking end's
/// mycelium couldn't wait for it to before starting this.
const CROSSING: Duration = Duration::from_mins(2);

/// Whether the gate's request and bundle are here, waited for at most
/// [`CROSSING`].
fn arrived(gate: &Path) -> bool {
    let since = std::time::Instant::now();
    loop {
        if gate.join(REQUEST).is_file() && gate.join(BUNDLE).is_file() {
            return true;
        }
        if since.elapsed() >= CROSSING {
            return false;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Why a gate couldn't be run: the code it exits with, and the sentence.
type Refusal = (u8, String);

fn refused(why: impl Into<String>) -> Refusal {
    (REFUSED, why.into())
}

fn unreached(why: impl Into<String>) -> Refusal {
    (UNREACHED, why.into())
}

/// The gate of `request`, read once from `gate`, to the run's exit, the
/// run given the checked clone and its log; or why it ran none.
///
/// # Errors
///
/// When it ran none, with the code and the reason.
pub fn gated(
    fence: &Fence,
    places: &Places,
    gate: &Path,
    request: &Request,
    (quest, commit): (&str, &str),
    run: &mut dyn FnMut(&Path, &Path) -> Result<u8, String>,
) -> Result<u8, Refusal> {
    if request.commit != commit || request.quest != quest {
        return Err(refused(
            "the request asks for another commit or quest than it was started for",
        ));
    }
    let locks = places.locks();
    fs::create_dir_all(&locks)
        .map_err(|error| unreached(format!("{}: {error}", locks.display())))?;
    let Some(_alone) = locked_now(&locks.join(format!("{commit}.lock")))
        .map_err(|error| unreached(error.to_string()))?
    else {
        return Err((NOT_NOW, format!("{commit} is being gated already")));
    };
    let mut slot = None;
    for at in 0..AT_ONCE {
        slot = locked_now(&locks.join(format!("slot-{at}.lock")))
            .map_err(|error| unreached(error.to_string()))?;
        if slot.is_some() {
            break;
        }
    }
    let Some(_slot) = slot else {
        return Err((
            NOT_NOW,
            format!("{AT_ONCE} gates are running; ask again once one ends"),
        ));
    };
    let work = places.work(commit);
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| unreached(format!("{}: {error}", work.display())))?;
    }
    fs::create_dir_all(&work).map_err(|error| unreached(format!("{}: {error}", work.display())))?;
    // Read once, into a folder nothing else writes, and judged from those
    // bytes: the quest's scratch is the quest's agents' to write.
    let bytes = super::read_once(&gate.join(BUNDLE), LARGEST_BUNDLE).map_err(refused)?;
    let header = super::header(&bytes).map_err(refused)?;
    if let Some(why) = super::unlike_asked(&header, commit) {
        return Err(refused(why));
    }
    let bundle = work.join(BUNDLE);
    fs::write(&bundle, &bytes)
        .map_err(|error| unreached(format!("{}: {error}", bundle.display())))?;
    let tree = work.join("tree");
    cloned(fence, places, &request.repo, &work, &tree)?;
    let git = |net, arguments: &[&str]| fence.git(net, &tree, &[&work], &[], arguments);
    for needed in &header.prerequisites {
        git(
            Net::None,
            &[
                "merge-base",
                "--is-ancestor",
                needed,
                "refs/remotes/origin/main",
            ],
        )
        .map_err(|_| {
            refused(format!(
                "the bundle needs {needed}, which isn't on the forge's main"
            ))
        })?;
    }
    let named = bundle.to_string_lossy();
    git(Net::None, &["bundle", "verify", "--quiet", &named]).map_err(refused)?;
    git(
        Net::None,
        &[
            "-c",
            "transfer.fsckObjects=true",
            "-c",
            "fetch.fsckObjects=true",
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            &named,
            "HEAD:refs/gate/head",
        ],
    )
    .map_err(refused)?;
    let head = git(
        Net::None,
        &["rev-parse", "--verify", "refs/gate/head^{commit}"],
    )
    .map_err(refused)?;
    if head.trim() != commit {
        return Err(refused(format!(
            "the bundle's HEAD is {}, not the commit asked for",
            head.trim()
        )));
    }
    git(
        Net::None,
        &[
            "merge-base",
            "--is-ancestor",
            "refs/remotes/origin/main",
            commit,
        ],
    )
    .map_err(|_| refused("the commit doesn't descend from the forge's main"))?;
    git(Net::None, &["checkout", "--quiet", "--detach", commit]).map_err(refused)?;
    run(&tree, &work.join(LOG)).map_err(unreached)
}

/// A fresh clone of the repo's main in `tree`, from a mirror that only
/// the forge fills, its origin then named as the forge.
fn cloned(
    fence: &Fence,
    places: &Places,
    repo: &Repo,
    work: &Path,
    tree: &Path,
) -> Result<(), Refusal> {
    let mirror = places.mirror(repo);
    let parent = mirror.parent().unwrap_or(&places.state).to_path_buf();
    fs::create_dir_all(&parent)
        .map_err(|error| unreached(format!("{}: {error}", parent.display())))?;
    let _filling =
        locked(&mirror.with_extension("lock")).map_err(|error| unreached(error.to_string()))?;
    let url = repo.url(&places.forge);
    if !mirror.join("HEAD").exists() {
        let named = mirror.to_string_lossy();
        fence
            .git(
                Net::None,
                &parent,
                &[&parent],
                &[],
                &["init", "--quiet", "--bare", &named],
            )
            .map_err(unreached)?;
    }
    let forge: Vec<&Path> = url
        .strip_prefix('/')
        .map(|_| vec![Path::new(&url)])
        .unwrap_or_default();
    fence
        .git(
            Net::Shared,
            &mirror,
            &[&mirror],
            &forge,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--prune",
                &url,
                "+refs/heads/main:refs/heads/main",
            ],
        )
        .map_err(|why| {
            unreached(format!(
                "the forge's main couldn't be fetched without a credential: {why}"
            ))
        })?;
    let (from, into) = (mirror.to_string_lossy(), tree.to_string_lossy());
    fence
        .git(
            Net::None,
            work,
            &[work],
            &[&mirror],
            &[
                "clone",
                "--quiet",
                "--no-hardlinks",
                "--no-checkout",
                "--single-branch",
                "--branch",
                "main",
                &from,
                &into,
            ],
        )
        .map_err(unreached)?;
    fence
        .git(
            Net::None,
            tree,
            &[work],
            &[],
            &["remote", "set-url", "origin", &url],
        )
        .map_err(unreached)?;
    Ok(())
}

/// The run of `there --changed` in the checked clone, through `outside`,
/// what it says going to `log`.
fn changed_checks(tree: &Path, log: &Path) -> Result<u8, String> {
    let program =
        std::env::current_exe().map_err(|error| format!("this program's path: {error}"))?;
    let said = fs::File::create(log).map_err(|error| format!("{}: {error}", log.display()))?;
    let also = said.try_clone().map_err(|error| error.to_string())?;
    let (git_dir, work_tree) = (tree.join(".git"), tree.to_path_buf());
    let status = Command::new(program)
        .arg("outside")
        .arg("--git-dir")
        .arg(&git_dir)
        .arg("--work-tree")
        .arg(&work_tree)
        .args(["--lent-only", "--", "there", "--never-ask", "--changed"])
        .stdin(Stdio::null())
        .stdout(said)
        .stderr(also)
        .status()
        .map_err(|error| format!("the run couldn't start: {error}"))?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}

/// Copies the end of `log` into the gate's folder, at most
/// [`LARGEST_LOG`], since its whole could pass what a shared folder takes.
fn carried_log(log: &Path, gate: &Path) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = fs::File::open(log) else {
        return;
    };
    let length = file.metadata().map_or(0, |meta| meta.len());
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(LARGEST_LOG)));
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_ok() {
        // The log is a convenience: the result says the run's end.
        let _ = super::write_whole(gate, LOG, &bytes);
    }
}

/// Adds the gate's line to the journal of gates.
fn journalled(places: &Places, quest: &str, request: Option<&Request>, found: &Found) {
    let line = serde_json::json!({
        "at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_secs()),
        "quest": quest,
        "session": request.and_then(|request| request.session.clone()),
        "repo": request.map(|request| request.repo.key()),
        "commit": found.commit,
        "exit": found.exit,
        "said": found.said,
    });
    let journal = places.state.join("journal.jsonl");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&journal)
    {
        // A journal that can't be written loses a line, never the gate.
        let _ = writeln!(file, "{line}");
    }
}

/// What a run's exit says, in a sentence.
fn said_of(exit: u8) -> String {
    match exit {
        0 => "the commit's changed checks passed, and the pass is kept for a merge".to_owned(),
        code => format!("the commit's changed checks failed, exiting {code}; the log says which"),
    }
}

/// `answer`: the gate for `QUEST COMMIT`, its result left beside the
/// request.
pub fn answer(arguments: &[String], state: &Path) -> ExitCode {
    if arguments
        .first()
        .is_some_and(|word| word == "--help" || word == "-h")
    {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let [quest, commit] = arguments else {
        eprintln!("nix-offload: answer takes a quest and a commit.\n{USED}");
        return ExitCode::from(64);
    };
    if !super::plain_quest(quest) || !super::plain_commit(commit) {
        eprintln!("nix-offload: answer takes a quest's id and a full commit hash.\n{USED}");
        return ExitCode::from(64);
    }
    let gate = super::folder(quest, commit);
    if !arrived(&gate) {
        eprintln!(
            "nix-offload: answer found no gate at {}. Run `nix-offload ask` on the machine \
             asking, which leaves it there and starts this.",
            gate.display()
        );
        return ExitCode::from(66);
    }
    let places = Places {
        state: state.join("nix-offload/gates"),
        forge: super::forge(),
    };
    sweep(&places.state.join("work"));
    if let Some(gates) = gate.parent() {
        sweep(gates);
    }
    let fence = match Fence::found() {
        Ok(fence) => fence,
        Err(why) => {
            eprintln!("nix-offload: answer can't fence its git: {why}.");
            return ExitCode::from(69);
        }
    };
    let request = super::read_request(&gate.join(REQUEST));
    let mut ran = |tree: &Path, log: &Path| changed_checks(tree, log);
    let gone = request
        .as_ref()
        .map_err(|why| refused(why.clone()))
        .and_then(|request| gated(&fence, &places, &gate, request, (quest, commit), &mut ran));
    let (exit, said) = match gone {
        Ok(exit) => (exit, said_of(exit)),
        Err((exit, why)) => (exit, why),
    };
    let request = request.ok();
    carried_log(&places.work(commit).join(LOG), &gate);
    let found = Found {
        commit: commit.clone(),
        exit,
        said,
    };
    journalled(&places, quest, request.as_ref(), &found);
    if let Err(error) = super::write_whole(&gate, RESULT, found.json().as_bytes()) {
        eprintln!("nix-offload: answer couldn't leave its result: {error}");
    }
    println!("{}", found.said);
    ExitCode::from(exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// A forge of one repo, `kestrel/osprey`, whose main has two commits,
    /// and a tree cloned from it with commits of its own past main.
    struct Forge {
        root: tempfile::TempDir,
        main: String,
    }

    fn git(folder: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(arguments)
            .current_dir(folder)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn committed(tree: &Path, name: &str) -> String {
        fs::write(tree.join(name), name).expect("a file is written");
        git(tree, &["add", name]);
        git(tree, &["commit", "--quiet", "-m", name]);
        git(tree, &["rev-parse", "HEAD"])
    }

    impl Forge {
        fn new() -> Self {
            let root = tempfile::tempdir().expect("a folder");
            let bare = root.path().join("forge/kestrel/osprey.git");
            fs::create_dir_all(&bare).expect("the forge's folder");
            git(
                &bare,
                &["init", "--quiet", "--bare", "--initial-branch", "main"],
            );
            let tree = root.path().join("tree");
            git(
                root.path(),
                &["clone", "--quiet", &bare.to_string_lossy(), "tree"],
            );
            git(&tree, &["checkout", "--quiet", "-b", "main"]);
            committed(&tree, "one");
            let main = committed(&tree, "two");
            git(&tree, &["push", "--quiet", "origin", "main"]);
            Self { root, main }
        }

        fn tree(&self) -> PathBuf {
            self.root.path().join("tree")
        }

        /// A gate's folder for `commit`, with `bundle` and a request.
        fn gate(&self, commit: &str, bundle: &[u8]) -> PathBuf {
            let gate = self.root.path().join("gate");
            fs::create_dir_all(&gate).expect("the gate's folder");
            fs::write(gate.join(BUNDLE), bundle).expect("the bundle");
            let request = Request {
                repo: Repo::parse("github:kestrel/osprey").expect("a repo"),
                commit: commit.to_owned(),
                quest: QUEST.to_owned(),
                session: None,
            };
            fs::write(gate.join(REQUEST), request.json()).expect("the request");
            gate
        }

        fn bundle(&self, range: &str) -> Vec<u8> {
            let path = self.root.path().join("made.bundle");
            git(
                &self.tree(),
                &[
                    "bundle",
                    "create",
                    "--quiet",
                    &path.to_string_lossy(),
                    range,
                ],
            );
            fs::read(path).expect("the bundle reads")
        }

        fn places(&self) -> Places {
            Places {
                state: self.root.path().join("state"),
                forge: self
                    .root
                    .path()
                    .join("forge")
                    .to_string_lossy()
                    .into_owned(),
            }
        }

        /// The gate run with a run that says what it ran and passes.
        fn gated(&self, gate: &Path, commit: &str) -> Result<String, Refusal> {
            let git_program = super::super::fence::on_path("git").expect("git is on the PATH");
            let fence = Fence::Bare { git: git_program };
            let request = super::super::read_request(&gate.join(REQUEST)).map_err(refused)?;
            let mut checked_out = String::new();
            let mut run = |tree: &Path, _: &Path| {
                checked_out = git(tree, &["rev-parse", "HEAD"]);
                Ok(0)
            };
            gated(
                &fence,
                &self.places(),
                gate,
                &request,
                (QUEST, commit),
                &mut run,
            )
            .map(|_| checked_out)
        }
    }

    const QUEST: &str = "QEST-20261115-ABC123";

    #[test]
    fn a_bundle_of_commits_past_main_is_checked_out_and_run() {
        let forge = Forge::new();
        let head = committed(&forge.tree(), "three");
        let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
        assert_eq!(
            forge.gated(&gate, &head),
            Ok(head),
            "the commit asked for should be the one run"
        );
    }

    #[test]
    fn a_bundle_of_another_commit_is_refused() {
        let forge = Forge::new();
        committed(&forge.tree(), "three");
        let gate = forge.gate(&forge.main, &forge.bundle("origin/main..HEAD"));
        let refusal = forge
            .gated(&gate, &forge.main)
            .expect_err("another commit should be refused");
        assert_eq!(
            refusal.0, REFUSED,
            "a wrong hash should be refused as bad data: {}",
            refusal.1
        );
    }

    #[test]
    fn a_bundle_with_a_corrupted_object_is_refused() {
        let forge = Forge::new();
        let head = committed(&forge.tree(), "three");
        let mut bundle = forge.bundle("origin/main..HEAD");
        let last = bundle.len() - 30;
        bundle[last] ^= 0xff;
        let gate = forge.gate(&head, &bundle);
        let refusal = forge
            .gated(&gate, &head)
            .expect_err("a corrupted bundle should be refused");
        assert_eq!(
            refusal.0, REFUSED,
            "a corrupted object should be refused as bad data: {}",
            refusal.1
        );
    }

    #[test]
    fn a_commit_not_descending_from_main_is_refused() {
        let forge = Forge::new();
        let tree = forge.tree();
        git(&tree, &["checkout", "--quiet", "--orphan", "apart"]);
        let apart = committed(&tree, "apart");
        let mut bundle = forge.bundle("HEAD");
        // A header naming main's commit as needed, over a pack that doesn't
        // descend from it, as a bundle made to pass the header's check is.
        let needed = format!("-{} two\n", forge.main);
        let line = bundle
            .iter()
            .position(|byte| *byte == b'\n')
            .expect("a version line")
            + 1;
        bundle.splice(line..line, needed.bytes());
        let gate = forge.gate(&apart, &bundle);
        let refusal = forge
            .gated(&gate, &apart)
            .expect_err("a commit apart from main should be refused");
        assert_eq!(
            refusal.0, REFUSED,
            "a commit off main should be refused as bad data: {}",
            refusal.1
        );
    }

    #[test]
    fn a_request_for_another_commit_than_started_for_is_refused() {
        let forge = Forge::new();
        let head = committed(&forge.tree(), "three");
        let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
        let refusal = forge
            .gated(&gate, &forge.main)
            .expect_err("a mismatched start should be refused");
        assert_eq!(refusal.0, REFUSED, "{}", refusal.1);
    }

    #[test]
    fn a_commit_being_gated_is_not_gated_beside_itself() {
        let forge = Forge::new();
        let head = committed(&forge.tree(), "three");
        let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
        let locks = forge.places().locks();
        fs::create_dir_all(&locks).expect("the locks' folder");
        let _running = locked_now(&locks.join(format!("{head}.lock")))
            .expect("a lock")
            .expect("the lock is free");
        let refusal = forge
            .gated(&gate, &head)
            .expect_err("a second gate of a commit should wait");
        assert_eq!(refusal.0, NOT_NOW, "{}", refusal.1);
    }

    #[test]
    fn no_more_than_two_gates_run_at_once() {
        let forge = Forge::new();
        let head = committed(&forge.tree(), "three");
        let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
        let locks = forge.places().locks();
        fs::create_dir_all(&locks).expect("the locks' folder");
        let _running: Vec<Held> = (0..AT_ONCE)
            .map(|at| {
                locked_now(&locks.join(format!("slot-{at}.lock")))
                    .expect("a lock")
                    .expect("the lock is free")
            })
            .collect();
        let refusal = forge
            .gated(&gate, &head)
            .expect_err("a third gate should wait");
        assert_eq!(refusal.0, NOT_NOW, "{}", refusal.1);
    }
}
