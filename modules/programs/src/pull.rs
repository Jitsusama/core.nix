//! Pull on a repository's row (`code_rows.rs`): its checkout's branch
//! brought up to the branch it follows, fetched first, by fast-forward
//! alone. It never merges or rebases: a checkout with changes of its own
//! to its tracked files, or whose branch has commits the one it follows
//! hasn't, is left as it is, and the answer says why. It acts only on a
//! checkout's own folder, the one holding `.git` as a folder, never on a
//! tree cut from it, which is where an agent works.
//!
//! Git runs with Joel's config, since a fetch needs his keys and what his
//! ssh and credential helpers say, as his own git would. It runs in a
//! session of its own with no terminal, and with no askpass, hooks or
//! fsmonitor, so it asks at no terminal, and a program a hook names never
//! runs beside it. A fetch past [`FETCH`], or a read past [`READ`], is
//! stopped with all it started, and what git said when it failed goes to
//! the daemon's journal.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The longest a fetch may run before it's stopped, which leaves the
/// checkout as it was.
pub const FETCH: Duration = Duration::from_secs(120);

/// The longest reading the checkout may take before it's stopped: its
/// branch, the one it follows, its changes and how far apart they are.
pub const READ: Duration = Duration::from_secs(30);

/// How long git is given to let go of its locks once asked to stop,
/// before it's stopped outright.
const LETTING_GO: Duration = Duration::from_secs(5);

/// How a pull went when it changed nothing it shouldn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulled {
    /// Its branch was at the one it follows already, or ahead of it alone.
    Even,
    /// Its branch was brought forward by this many commits.
    Took(u64),
}

/// Why a pull left the checkout as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The folder isn't a checkout's own, as a tree cut from one isn't.
    Tree,
    /// A pull of the same checkout is under way.
    Busy,
    /// Its head is no branch, as while a rebase is under way.
    Detached,
    /// Its branch follows none.
    Unfollowed,
    /// Git couldn't read its branch, its changes or how far apart it is.
    Unread,
    /// Its tracked files have changes of its own.
    Changed,
    /// The fetch failed or ran past [`FETCH`].
    Unfetched,
    /// Its branch and the one it follows have each moved, so only a merge
    /// or a rebase would bring them together.
    Diverged,
    /// Git refused the fast-forward itself.
    Failed,
}

impl Refused {
    fn code(self) -> &'static str {
        match self {
            Refused::Tree => "tree",
            Refused::Busy => "busy",
            Refused::Detached => "detached",
            Refused::Unfollowed => "unfollowed",
            Refused::Unread => "unread",
            Refused::Changed => "changed",
            Refused::Unfetched => "unfetched",
            Refused::Diverged => "diverged",
            Refused::Failed => "failed",
        }
    }
}

/// What a pull's answer says, from Pull's own words in the vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    said: Value,
}

impl Words {
    /// Pull's words from the vocabulary, each it hasn't said by its name.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let said = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|thing| thing["id"] == "verb:code:pull")
            .map_or(Value::Null, |thing| thing["said"].clone());
        Words { said }
    }

    fn say(&self, word: &str) -> String {
        self.said[word].as_str().unwrap_or(word).to_owned()
    }

    /// The answer to a pull's ask: what it did, or why it didn't, refused
    /// as the interface codes it, with the words the row's foot says.
    #[must_use]
    pub fn answer(&self, id: &Value, pulled: Result<Pulled, Refused>) -> Value {
        match pulled {
            Ok(Pulled::Even) => json!({"id": id, "said": self.say("even")}),
            Ok(Pulled::Took(1)) => json!({"id": id, "said": self.say("took")}),
            Ok(Pulled::Took(count)) => json!({
                "id": id,
                "said": self.say("tookMany").replace("{count}", &count.to_string()),
            }),
            Err(why) => {
                let said = self.say(why.code());
                json!({"id": id, "error": {
                    "code": "refused",
                    "refused": why.code(),
                    "message": said,
                    "said": said,
                }})
            }
        }
    }
}

/// Brings the checkout at `folder` up to the branch it follows, by
/// fast-forward alone, with the git at `git`: it says why before
/// fetching where it can, so a refusal waits on no network.
///
/// # Errors
///
/// Says why it left the checkout as it was.
pub fn pull(git: &str, folder: &Path, fetch: Duration) -> Result<Pulled, Refused> {
    // A tree cut from a checkout holds `.git` as a file naming it, where
    // an agent works; only a checkout's own folder holds it as a folder,
    // with a HEAD in it. Git is told that folder outright (`ran`), so it
    // never looks past it for a repository further up.
    let own = folder.join(".git");
    if !own.is_dir() || !own.join("HEAD").is_file() {
        return Err(Refused::Tree);
    }
    let read = |args: &[&str]| ran(git, folder, args, Some(READ));
    // Its head being no branch is said by exiting 1; anything else is git
    // failing to read it.
    read(&["symbolic-ref", "--quiet", "HEAD"]).map_err(|exit| {
        if exit == Some(1) {
            Refused::Detached
        } else {
            Refused::Unread
        }
    })?;
    read(&["rev-parse", "--verify", "--quiet", "@{upstream}"]).map_err(|_| Refused::Unfollowed)?;
    let changed =
        read(&["status", "--porcelain", "--untracked-files=no"]).map_err(|_| Refused::Unread)?;
    if !changed.trim().is_empty() {
        return Err(Refused::Changed);
    }
    ran(git, folder, &["fetch", "--quiet"], Some(fetch)).map_err(|_| Refused::Unfetched)?;
    let counted = read(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .map_err(|_| Refused::Unread)?;
    match counts(&counted).ok_or(Refused::Unread)? {
        (_, 0) => Ok(Pulled::Even),
        (0, behind) => {
            // Never stopped part way, which would leave it half written.
            ran(
                git,
                folder,
                &["merge", "--ff-only", "--quiet", "@{upstream}"],
                None,
            )
            .map_err(|_| Refused::Failed)?;
            Ok(Pulled::Took(behind))
        }
        _ => Err(Refused::Diverged),
    }
}

/// The commits only the head has and only the branch it follows has, as
/// `rev-list --left-right --count` prints them.
fn counts(printed: &str) -> Option<(u64, u64)> {
    let mut counts = printed.split_whitespace().map(str::parse::<u64>);
    Some((counts.next()?.ok()?, counts.next()?.ok()?))
}

/// Runs git on the checkout at `folder`, named outright, with nothing to
/// answer it, answering what it printed, or else how it exited, none if
/// it was stopped or never started. It runs in a session of its own, with
/// no terminal for ssh to ask at and no askpass, hooks or fsmonitor. Past
/// `wait`, where one is given, it's stopped with all it started.
fn ran(
    git: &str,
    folder: &Path,
    args: &[&str],
    wait: Option<Duration>,
) -> Result<String, Option<i32>> {
    let mut command = Command::new(git);
    command
        .arg("-C")
        .arg(folder)
        .args(["-c", "core.askPass=", "-c", "core.fsmonitor=false"])
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_DIR", folder.join(".git"))
        .env("GIT_WORK_TREE", folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Nothing the daemon was started with points git at another repository.
    for elsewhere in [
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
    ] {
        command.env_remove(elsewhere);
    }
    // SAFETY: setsid is async-signal-safe and touches no memory of ours.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|_| None)?;
    // Read as it prints, so a long status never fills its pipe and waits.
    let (printed, said) = (drained(child.stdout.take()), drained(child.stderr.take()));
    if let Some(wait) = wait
        && !ended_within(&child, wait)
    {
        stop(&mut child);
        return Err(None);
    }
    let exit = child.wait().map_err(|_| None)?;
    // A helper it left running, as ssh's shared connection is, may hold
    // its pipes open, so what was printed is waited on only so long.
    let printed = printed.recv_timeout(LETTING_GO).unwrap_or_default();
    if !exit.success() {
        let said = said.recv_timeout(LETTING_GO).unwrap_or_default();
        let said = String::from_utf8_lossy(&said);
        eprintln!(
            "pull: git {} in {} failed: {}",
            args.first().unwrap_or(&""),
            folder.display(),
            said.trim()
        );
        return Err(exit.code());
    }
    String::from_utf8(printed).map_err(|_| exit.code())
}

/// What a pipe holds, read to its end on a thread of its own.
fn drained(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<Vec<u8>> {
    let (sent, read) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut all = Vec::new();
            let _ = pipe.read_to_end(&mut all);
            let _ = sent.send(all);
        });
    }
    read
}

/// Whether `child` ended within `wait`, left unreaped, so its id names its
/// group until it's waited on and no other group can take it meanwhile.
fn ended_within(child: &Child, wait: Duration) -> bool {
    let pid: libc::id_t = child.id();
    let started = Instant::now();
    loop {
        // SAFETY: waitid writes only the siginfo it's given; WNOWAIT
        // leaves the child to be reaped by `wait`.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let asked = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                &raw mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        // SAFETY: si_pid is set by waitid, and zero while it runs on.
        if asked != 0 || unsafe { info.si_pid() } != 0 {
            return true;
        }
        if started.elapsed() >= wait {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Stops `child` and everything it started, its session's whole group: a
/// fetch's ssh and remote helper as well as git. Asked first, so git
/// takes away its lock files, and stopped outright past [`LETTING_GO`],
/// all before git itself is reaped, while its id still names its group.
fn stop(child: &mut Child) {
    let Ok(group) = libc::pid_t::try_from(child.id()) else {
        let _ = child.kill();
        let _ = child.wait();
        return;
    };
    // SAFETY: signals the group the child leads, which setsid made.
    unsafe { libc::kill(-group, libc::SIGTERM) };
    ended_within(child, LETTING_GO);
    // What it started may outlive it, until stopped outright.
    // SAFETY: as above.
    unsafe { libc::kill(-group, libc::SIGKILL) };
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Words {
        Words::read(&json!({"things": [{"id": "verb:code:pull", "said": {
            "even": "up to date",
            "took": "took 1 commit",
            "tookMany": "took {count} commits",
            "changed": "not pulled: it has changes of its own",
        }}]}))
    }

    #[test]
    fn a_pull_says_how_many_commits_it_took_or_that_there_were_none() {
        let said = |pulled| words().answer(&json!(3), Ok(pulled))["said"].clone();
        assert_eq!(said(Pulled::Even), "up to date");
        assert_eq!(said(Pulled::Took(1)), "took 1 commit");
        assert_eq!(said(Pulled::Took(12)), "took 12 commits");
    }

    #[test]
    fn the_commits_each_side_has_alone_are_read_as_git_counts_them() {
        assert_eq!(counts("2\t5\n"), Some((2, 5)));
        assert_eq!(counts("2\n"), None);
    }

    /// The shell the path finds, as the build has no /bin/sh.
    fn shell() -> std::path::PathBuf {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(|folder| Path::new(folder).join("sh"))
            .find(|sh| sh.is_file())
            .expect("no sh on the path")
    }

    #[test]
    fn a_fetch_past_its_wait_is_stopped_with_all_it_started() {
        let folder = std::env::temp_dir().join(format!("pull-stopped-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join(".git")).unwrap();
        std::fs::write(folder.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        // A git whose fetch starts a helper, as ssh is, and then hangs.
        let git = folder.join("git");
        let started = folder.join("helper.pid");
        std::fs::write(
            &git,
            format!(
                "#!{}\ncase \"$*\" in *fetch*) sleep 60 & echo $! > {}; wait;; esac\n\
                 case \"$*\" in *status*) exit 0;; esac\n",
                shell().display(),
                started.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&git, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let asked = Instant::now();
        // Long enough for the helper to start on a busy builder.
        let pulled = pull(git.to_str().unwrap(), &folder, Duration::from_secs(2));
        assert_eq!(pulled, Err(Refused::Unfetched));
        assert!(
            asked.elapsed() < Duration::from_secs(10),
            "a fetch past its wait wasn't stopped"
        );
        let helper: libc::pid_t = std::fs::read_to_string(&started)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let gone = || {
            std::fs::read_to_string(format!("/proc/{helper}/stat"))
                .map_or(true, |stat| stat.split_whitespace().nth(2) == Some("Z"))
        };
        let waited = Instant::now();
        while !gone() && waited.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(gone(), "what the fetch started outlived it");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_refused_pull_says_why_in_its_own_words_and_by_its_code() {
        assert_eq!(
            words().answer(&json!(3), Err(Refused::Changed)),
            json!({"id": 3, "error": {
                "code": "refused",
                "refused": "changed",
                "message": "not pulled: it has changes of its own",
                "said": "not pulled: it has changes of its own",
            }})
        );
    }
}
