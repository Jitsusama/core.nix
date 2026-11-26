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
//! commit descends from each, or from main's newest when it needs none
//! (`fence.rs` runs each git). Only then does
//! `there --changed` run, through `outside`, on the exemption's terms,
//! kept to the lent builders, so its pass is kept where a merge reads it.
//!
//! It runs as the person, outside the agent fence, so it trusts nothing
//! in the quest's scratch by its path: the gate's folder is held open
//! through no link (`folder.rs`), and it sweeps nothing there, since the
//! harness clears a quest's scratch when the quest ends. Its own state is
//! refused when any folder of it is a link or another account's, and it
//! gates only the repos `jitsusama.nix-offload.gateRepos` names, so no
//! agent can have it build a stranger's flake on the exemption's terms
//! (security review, 16 November).
//!
//! Two gates run at once at most, and a commit already being gated isn't
//! gated beside itself; either waits for the Mac to ask again. Each run is
//! journalled, and its own work goes a week after.

use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use terminal_kit::{Exit, Failure};

use super::fence::{Fence, Net};
use super::folder::GateFolder;
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
/// What it exits with when its own state can't be trusted: sysexits.h's
/// permission denied.
const UNTRUSTED: u8 = 77;
/// The repos a gate takes, one `github:OWNER/NAME` a line, as the home
/// module writes them under XDG's config folder.
const GATE_REPOS: &str = "gate-repos";

/// What a call asked wrongly is told to type, for how it's asked.
const HELP: &str = "nix-offload answer --help";

/// Where a gate's answer keeps its own: this machine's state, which no
/// other machine's files reach.
#[derive(Debug, Clone)]
pub struct Places {
    /// `nix-offload/gates` under XDG's state folder.
    pub state: PathBuf,
    /// Where repos are fetched from ([`super::forge`]).
    pub forge: String,
    /// The repos it gates; any other is refused before it's fetched.
    pub repos: Vec<Repo>,
}

/// Makes `folder` with only its account able to enter it, when it isn't
/// there, and refuses it when it's a link, not a folder, or another
/// account's, so nothing planted turns a write or a removal elsewhere.
fn owned(folder: &Path) -> Result<(), Refusal> {
    let untrusted = |why: String| (UNTRUSTED, format!("{}: {why}", folder.display()));
    match fs::DirBuilder::new().mode(0o700).create(folder) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(untrusted(error.to_string())),
    }
    let meta = fs::symlink_metadata(folder).map_err(|error| untrusted(error.to_string()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(untrusted("it's a link or not a folder".to_owned()));
    }
    // SAFETY: geteuid can't fail.
    if meta.uid() != unsafe { libc::geteuid() } {
        return Err(untrusted("another account holds it".to_owned()));
    }
    Ok(())
}

/// Opens `path` to write, following no link.
fn unlinked() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    options.custom_flags(libc::O_NOFOLLOW);
    options
}

impl Places {
    /// Its folders, each made or refused by [`owned`]: the state, and its
    /// work, mirrors and locks.
    fn owned(&self) -> Result<(), Refusal> {
        if let Some(parent) = self.state.parent() {
            owned(parent)?;
        }
        owned(&self.state)?;
        for folder in ["work", "mirrors", "locks"] {
            owned(&self.state.join(folder))?;
        }
        Ok(())
    }

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
    let file = unlinked()
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
    let file = unlinked()
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

/// Takes away what's in `folder`, a folder of its own state, last changed
/// over [`KEPT`] ago; a link there goes itself, never what it names.
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

/// The gate's folder under `scratches` once its request and bundle are
/// there, waited for at most [`CROSSING`].
fn arrived(scratches: &Path, quest: &str, commit: &str) -> Option<GateFolder> {
    let since = std::time::Instant::now();
    loop {
        if let Ok(gate) = GateFolder::open(scratches, quest, commit)
            && gate.holds(REQUEST)
            && gate.holds(BUNDLE)
        {
            return Some(gate);
        }
        if since.elapsed() >= CROSSING {
            return None;
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

/// The locks a gate of `commit` runs under: its own, so no two gate it
/// at once, and one of [`AT_ONCE`] slots; or why not now.
fn held(places: &Places, commit: &str) -> Result<(Held, Held), Refusal> {
    let locks = places.locks();
    let Some(alone) = locked_now(&locks.join(format!("{commit}.lock")))
        .map_err(|error| unreached(error.to_string()))?
    else {
        return Err((NOT_NOW, format!("{commit} is being gated already")));
    };
    for at in 0..AT_ONCE {
        if let Some(slot) = locked_now(&locks.join(format!("slot-{at}.lock")))
            .map_err(|error| unreached(error.to_string()))?
        {
            return Ok((alone, slot));
        }
    }
    Err((
        NOT_NOW,
        format!("{AT_ONCE} gates are running; ask again once one ends"),
    ))
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
    gate: &GateFolder,
    request: &Request,
    (quest, commit): (&str, &str),
    run: &mut dyn FnMut(&Path, &Path) -> Result<u8, String>,
) -> Result<u8, Refusal> {
    if request.commit != commit || request.quest != quest {
        return Err(refused(
            "the request asks for another commit or quest than it was started for",
        ));
    }
    if !places.repos.contains(&request.repo) {
        return Err(refused(format!(
            "{} isn't a repo this machine gates; jitsusama.nix-offload.gateRepos names those it does",
            request.repo.key()
        )));
    }
    places.owned()?;
    let _held = held(places, commit)?;
    let work = places.work(commit);
    if fs::symlink_metadata(&work).is_ok() {
        // A link goes itself, never what it names.
        fs::remove_dir_all(&work)
            .or_else(|_| fs::remove_file(&work))
            .map_err(|error| unreached(format!("{}: {error}", work.display())))?;
    }
    owned(&work)?;
    // Read once, into a folder nothing else writes, and judged from those
    // bytes: the quest's scratch is the quest's agents' to write.
    let bytes = gate.read(BUNDLE, LARGEST_BUNDLE).map_err(refused)?;
    let header = super::header(&bytes).map_err(refused)?;
    if let Some(why) = super::unlike_asked(&header, commit) {
        return Err(refused(why));
    }
    let bundle = work.join(BUNDLE);
    unlinked()
        .write(true)
        .create_new(true)
        .open(&bundle)
        .and_then(|mut file| file.write_all(&bytes))
        .map_err(|error| unreached(format!("{}: {error}", bundle.display())))?;
    let tree = work.join("tree");
    let _cleared = Cleared([tree.clone(), bundle.clone()]);
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
    // The commit has to stand on main's history: on each commit of main
    // the bundle says it needs, or on main's newest when it needs none.
    // Main's newest isn't asked of a bundle that needs some, since main
    // moves while an ask waits its turn, and a change behind main is
    // gated as it stands, as `there --changed` gates one on this machine.
    let stands_on = if header.prerequisites.is_empty() {
        vec!["refs/remotes/origin/main".to_owned()]
    } else {
        header.prerequisites.clone()
    };
    for base in &stands_on {
        git(Net::None, &["merge-base", "--is-ancestor", base, commit])
            .map_err(|_| refused("the commit doesn't descend from the forge's main"))?;
    }
    git(Net::None, &["checkout", "--quiet", "--detach", commit]).map_err(refused)?;
    run(&tree, &work.join(LOG)).map_err(unreached)
}

/// A gate's clone and bundle, taken away however its run ends: a clone
/// is about 100 MB, and only the log is read afterwards.
struct Cleared([PathBuf; 2]);

impl Drop for Cleared {
    fn drop(&mut self) {
        let [tree, bundle] = &self.0;
        // Either may never have been made; what's left is swept a week on.
        let _ = fs::remove_dir_all(tree);
        let _ = fs::remove_file(bundle);
    }
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
    owned(&parent)?;
    let _filling =
        locked(&mirror.with_extension("lock")).map_err(|error| unreached(error.to_string()))?;
    let url = repo.url(&places.forge);
    if fs::symlink_metadata(&mirror).is_ok() {
        owned(&mirror)?;
    }
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
            // Through git's transport rather than a copy of its files, so
            // each object is checked against its hash as it's taken.
            &[
                "-c",
                "transfer.fsckObjects=true",
                "clone",
                "--quiet",
                "--no-local",
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
    let said = unlinked()
        .write(true)
        .create_new(true)
        .open(log)
        .map_err(|error| format!("{}: {error}", log.display()))?;
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
fn carried_log(log: &Path, gate: &GateFolder) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = unlinked().read(true).open(log) else {
        return;
    };
    let length = file.metadata().map_or(0, |meta| meta.len());
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(LARGEST_LOG)));
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_ok() {
        // The log is a convenience: the result says the run's end.
        let _ = gate.write_whole(LOG, &bytes);
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
    if let Ok(mut file) = unlinked().create(true).append(true).open(&journal) {
        // A journal that can't be written loses a line, never the gate.
        let _ = writeln!(file, "{line}");
    }
}

/// Whether the gate's own work folder is one to read a log from: its
/// state trusted, and the folder there, neither a link nor another's.
fn trusted_work(places: &Places, commit: &str) -> bool {
    let work = places.work(commit);
    places.owned().is_ok()
        && fs::symlink_metadata(&work).is_ok_and(|meta| meta.is_dir())
        && owned(&work).is_ok()
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
pub fn answer(arguments: &[String], state: &Path) -> Result<(), Failure> {
    let [quest, commit] = arguments else {
        return Err(crate::ended::usage("answer takes a quest and a commit").try_this(HELP));
    };
    if !super::plain_quest(quest) || !super::plain_commit(commit) {
        return Err(
            crate::ended::usage("answer takes a quest's id and a full commit hash").try_this(HELP),
        );
    }
    let Some(scratches) = super::scratches() else {
        return Err(
            Failure::new(Exit::Config, "answer has no gate to find").because(super::NO_SCRATCH)
        );
    };
    let Some(gate) = arrived(&scratches, quest, commit) else {
        return Err(Failure::new(
            Exit::NotFound,
            format!(
                "answer found no gate at {}, or found a link on the way to it",
                scratches
                    .join(quest)
                    .join(super::GATES)
                    .join(commit)
                    .display()
            ),
        )
        .because("`nix-offload ask` on the machine asking leaves it there and starts this"));
    };
    let places = Places {
        state: state.join("nix-offload/gates"),
        forge: super::forge(),
        repos: super::configured(GATE_REPOS)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| Repo::parse(line.trim()))
            .collect(),
    };
    if places.owned().is_ok() {
        sweep(&places.state.join("work"));
    }
    let fence = match Fence::found() {
        Ok(fence) => fence,
        Err(why) => {
            return Err(Failure::new(Exit::Unavailable, "answer can't fence its git").because(why));
        }
    };
    let request = super::read_request(&gate);
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
    if trusted_work(&places, commit) {
        carried_log(&places.work(commit).join(LOG), &gate);
    }
    let found = Found {
        commit: commit.clone(),
        exit,
        said,
    };
    if places.owned().is_ok() {
        journalled(&places, quest, request.as_ref(), &found);
    }
    if let Err(error) = gate.write_whole(RESULT, found.json().as_bytes()) {
        eprintln!("nix-offload: answer couldn't leave its result: {error}");
    }
    println!("{}", found.said);
    crate::ended::coded(exit, || {
        format!("the gate of {commit} didn't pass, as the line above and its log say")
    })
}

// A gate is answered on Linux alone, so its folder refuses to open
// anywhere else, and so would every test here.
#[cfg(all(test, target_os = "linux"))]
mod tests;
