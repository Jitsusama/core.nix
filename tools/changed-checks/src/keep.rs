//! `changed-checks keep-main`: keeps each head of origin's main where runs
//! are sent, once it has stood a while, so a run on a branch cut from it
//! evaluates its own tree alone.
//!
//! The first run against a new base paid for the base: on 5 November, 42
//! of the lent builder's 105 runs evaluated theirs before their own tree,
//! a median of 444 s each. Merges land in bursts, and a head that moves on
//! within two minutes is seldom anyone's base, so a head is kept once it
//! has stood that long: replaying that day's 101 heads, waiting two
//! minutes kept 74 and spared 29 of those 42 evaluations, where keeping
//! every head spared 38 for 101. By 12 November merges came faster, and
//! a keep took 3 to 8 minutes, so the wait cost more than it saved: of 23
//! runs that paid for their base from 09:00 to noon, most came 50 to 335 s
//! after it landed, before a keep begun two minutes on could finish, and 4
//! were on heads that moved on within the two minutes. A head is now kept
//! by the second look that finds it, 30 s on, which still passes over a
//! burst's middle heads, since a keep holds the loop and the next look
//! after it reads only the newest. A head is kept only where nix-offload
//! sends it, never on this machine, whose own runs are sent too, and only
//! to a builder already lent, never one asked for: a store file left
//! naming a builder whose tunnel had gone asked for one at 00:11 on 6
//! November, before nix-offload took `--never-ask`. A kept
//! head is sent again when another builder is lent, since one lent afresh
//! starts with nothing kept, as one lent that evening did; never on a
//! timer, since a builder is given back after 15 idle minutes and sending
//! on one would keep it lent all day.
//!
//! A switch leaves the keeper be, and it ends between keeps once another
//! build is installed where `--follow` names, for the service to start
//! that one: on 14 November each of the 5 switches that restarted it did
//! so mid-keep, and the one at 14:47 threw away 9 minutes of a keep and
//! the record it would have left, so the next keep evaluated whole.
//!
//! Once a head's record stands, the Mac's checks of it are kept beside
//! the loop, as its own `--keep-base` run, so a run comparing the Mac's
//! checks finds its base kept too. It never holds the loop: it's stopped
//! with all it started before the next keep, which shares its clone, and
//! when the keeper ends, and one stopped, failed or never started leaves
//! the record standing and is tried again at the next keep.

use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How often main's head is read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How long a head stands before it's kept.
pub const STOOD: Duration = LOOK_EVERY;

/// How long a head that couldn't be kept waits before it's sent again:
/// longer than a lent builder's 15 idle minutes, so a keep that fails
/// every time never holds one lent. A head that moves on is sent once it
/// stands, and one that couldn't be kept once another builder is lent.
pub const FAILED_AGAIN: Duration = Duration::from_mins(20);

/// The system whose checks are kept beside each head's: the Mac's.
const BESIDE: &str = "aarch64-darwin";

/// Follow origin's main, keeping each head once it has stood, until stopped,
/// or until `follow`, the installed program, is another than this one.
pub fn main(follow: Option<&Path>) -> ExitCode {
    let url =
        env::var("CHANGED_CHECKS_REPOSITORY").unwrap_or_else(|_| crate::REPOSITORY.to_owned());
    let clone = crate::state().join("changed-checks/main");
    let mut keeper = Keeper::default();
    let mut aside = None;
    eprintln!(
        "changed-checks: keeping each head of {url}'s main once it has stood {} s",
        STOOD.as_secs()
    );
    loop {
        if let Some(installed) = follow
            && replaced(std::fs::canonicalize(installed), env::current_exe())
        {
            eprintln!(
                "changed-checks: another changed-checks is installed, so this one ends \
                 between keeps, for the service to start that one"
            );
            return ExitCode::SUCCESS;
        }
        if let Some(ended) = look_aside(&mut aside) {
            eprintln!("changed-checks: the keep of {BESIDE}'s checks ended: {ended}");
        }
        // The store line names the lent builder by its host key, which is
        // new for each one lent, so it says when what's kept is gone.
        let builder = std::fs::read_to_string(crate::offload_dir().join("store")).ok();
        let builder = builder
            .as_deref()
            .map(str::trim)
            .filter(|line| !line.is_empty());
        match head(&url) {
            Ok(head) => {
                if let (Some(head), Some(builder)) =
                    (keeper.due(&head, builder, Instant::now()), builder)
                {
                    let short = head.get(..12).unwrap_or(&head);
                    let kept = keep_beside(
                        &mut aside,
                        || crate::keep_head(&url, &clone, &head),
                        || beside(&clone),
                    );
                    match kept {
                        Ok(true) => keeper.kept(&head, builder),
                        Ok(false) => {
                            eprintln!("changed-checks: nothing took {short}, so it waits");
                            keeper.failed(&head, builder, Instant::now());
                        }
                        Err(why) => {
                            eprintln!("changed-checks: couldn't keep {short}: {why}");
                            keeper.failed(&head, builder, Instant::now());
                        }
                    }
                }
            }
            Err(why) => eprintln!("changed-checks: couldn't read main's head: {why}"),
        }
        std::thread::sleep(LOOK_EVERY);
    }
}

/// The Mac's keep of the head checked out in `clone`, as this program's own
/// `--keep-base` run there, which sends it where the Mac's runs go.
fn beside(clone: &Path) -> io::Result<Aside> {
    let mut command = Command::new(env::current_exe()?);
    command
        .args(["--keep-base", "--system", BESIDE])
        .current_dir(clone);
    Aside::start(command)
}

/// What the keeper has seen of main, which says when a head is sent.
#[derive(Default)]
pub struct Keeper {
    /// The head last read, and when it was first read.
    seen: Option<(String, Instant)>,
    /// The head last kept, and the builder it was kept on.
    kept: Option<(String, String)>,
    /// The head last sent and not kept, the builder it was sent to, and when.
    failed: Option<(String, String, Instant)>,
}

impl Keeper {
    /// The head to send now, given main's head and the lent builder as read
    /// at `now`: one that has stood `STOOD`, with a builder lent that
    /// hasn't kept it, and that didn't just fail there.
    pub fn due(&mut self, head: &str, builder: Option<&str>, now: Instant) -> Option<String> {
        if self.seen.as_ref().is_none_or(|(seen, _)| seen != head) {
            self.seen = Some((head.to_owned(), now));
        }
        let builder = builder?;
        let (seen, since) = self.seen.as_ref()?;
        let stood = now.duration_since(*since) >= STOOD;
        let kept = self
            .kept
            .as_ref()
            .is_some_and(|(kept, on)| kept == head && on == builder);
        let failing = self.failed.as_ref().is_some_and(|(failed, on, at)| {
            failed == head && on == builder && now.duration_since(*at) < FAILED_AGAIN
        });
        (stood && !kept && !failing).then(|| seen.clone())
    }

    /// Note that `head` is kept on `builder`.
    pub fn kept(&mut self, head: &str, builder: &str) {
        self.kept = Some((head.to_owned(), builder.to_owned()));
        self.failed = None;
    }

    /// Note that `head` couldn't be kept on `builder` at `now`.
    pub fn failed(&mut self, head: &str, builder: &str, now: Instant) {
        self.failed = Some((head.to_owned(), builder.to_owned(), now));
    }
}

/// Main's head at `url`, as the remote says it now.
pub fn head(url: &str) -> Result<String, String> {
    let said = git(None, &["ls-remote", "--exit-code", url, "refs/heads/main"])?;
    said.split_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| format!("{url} said nothing of its main"))
}

/// Put the keeper's own clone of `url`, in `clone`, at `head` with nothing
/// changed, and origin's main where the remote has it, as a run reads them.
pub fn check_out(url: &str, clone: &Path, head: &str) -> Result<(), String> {
    if !clone.join(".git").exists() {
        if let Some(parent) = clone.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("making {}: {error}", parent.display()))?;
        }
        let into = clone.to_string_lossy();
        git(None, &["clone", "-q", "--no-checkout", url, &into])?;
    }
    git(
        Some(clone),
        &[
            "fetch",
            "-q",
            "origin",
            "+refs/heads/main:refs/remotes/origin/main",
        ],
    )?;
    git(
        Some(clone),
        &["checkout", "-q", "--force", "--detach", head],
    )?;
    Ok(())
}

/// Run git, in `folder` when one is named, and give what it printed.
fn git(folder: Option<&Path>, arguments: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    let ran = command
        .args(arguments)
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}

/// How long a stopped aside is given to end before it's killed.
const GRACE: Duration = Duration::from_secs(10);

/// Another system's keep, run beside the loop rather than in it. Dropping
/// it stops it and all it started, so it never outlives the keeper.
pub struct Aside {
    child: Child,
}

impl Aside {
    /// Start `command` as an aside, in a process group of its own, so what
    /// it starts is stopped with it.
    pub fn start(mut command: Command) -> io::Result<Self> {
        command.process_group(0).stdin(Stdio::null());
        Ok(Self {
            child: command.spawn()?,
        })
    }
}

impl Drop for Aside {
    /// Ask the aside's group to end, kill it once the aside has or the
    /// grace is up, and reap the aside.
    fn drop(&mut self) {
        let group = format!("-{}", self.child.id());
        let signal = |name: &str| {
            Command::new("kill")
                .args(["-s", name, "--", &group])
                .stderr(Stdio::null())
                .status()
        };
        let _ = signal("TERM");
        let deadline = Instant::now() + GRACE;
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = signal("KILL");
        let _ = self.child.wait();
    }
}

/// Keep a head with `keep`, first stopping the aside left from the last,
/// since both read the keeper's one clone, and once it's kept start the
/// next with `beside`. The aside never changes what the keep says: one
/// that can't start, fails or is stopped leaves the record standing, and
/// is tried again at the next keep.
pub fn keep_beside(
    aside: &mut Option<Aside>,
    keep: impl FnOnce() -> Result<bool, String>,
    beside: impl FnOnce() -> io::Result<Aside>,
) -> Result<bool, String> {
    *aside = None;
    let kept = keep()?;
    if kept {
        match beside() {
            Ok(started) => *aside = Some(started),
            Err(why) => eprintln!("changed-checks: couldn't start the keep beside it: {why}"),
        }
    }
    Ok(kept)
}

/// How the aside ended, once it has, letting it go.
pub fn look_aside(aside: &mut Option<Aside>) -> Option<ExitStatus> {
    let ended = aside.as_mut()?.child.try_wait().ok().flatten()?;
    *aside = None;
    Some(ended)
}

/// Whether the build installed is another than the one running, both as
/// their paths resolve. A running build no longer on disk has been
/// replaced; an installed one that can't be read says nothing.
pub fn replaced(installed: io::Result<PathBuf>, running: io::Result<PathBuf>) -> bool {
    let Ok(installed) = installed else {
        return false;
    };
    !running
        .and_then(std::fs::canonicalize)
        .is_ok_and(|running| running == installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: Option<&str> = Some("one");

    /// A shell running `script`, as an aside.
    fn shell(script: &str) -> io::Result<Aside> {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        Aside::start(command)
    }

    /// Whether `pid` runs: a zombie only waits to be reaped. Linux says
    /// so in /proc, which the build's sandbox always has.
    #[cfg(target_os = "linux")]
    fn runs(pid: &str) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
            !stat
                .rsplit(')')
                .next()
                .unwrap_or("")
                .trim_start()
                .starts_with('Z')
        })
    }

    /// Whether `pid` runs: a zombie only waits to be reaped. A Mac has no
    /// /proc, so its own ps says, by the path macOS keeps it at.
    #[cfg(not(target_os = "linux"))]
    fn runs(pid: &str) -> bool {
        Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", pid])
            .output()
            .is_ok_and(|told| {
                let stat = String::from_utf8_lossy(&told.stdout);
                told.status.success() && !stat.trim_start().starts_with('Z')
            })
    }

    /// What `file` holds once something has written it, within 10 s.
    fn written(file: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match std::fs::read_to_string(file) {
                Ok(held) if held.ends_with('\n') => return held,
                _ if Instant::now() > deadline => panic!("nothing wrote {}", file.display()),
                _ => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }

    #[test]
    fn the_aside_starts_only_once_a_head_is_kept() {
        for said in [Ok(false), Err("the far side's run failed".to_owned())] {
            let mut aside = None;
            let mut started = false;
            let kept = keep_beside(
                &mut aside,
                || said.clone(),
                || {
                    started = true;
                    shell("exit 0")
                },
            );
            assert_eq!(kept, said, "the keep says what it said");
            assert!(
                !started,
                "no record was written, so nothing starts beside it"
            );
        }
        let mut aside = None;
        assert_eq!(
            keep_beside(&mut aside, || Ok(true), || shell("exit 0")),
            Ok(true)
        );
        assert!(aside.is_some(), "a kept head starts the aside");
    }

    #[test]
    fn the_aside_and_what_it_started_end_before_the_next_keep() {
        let dir = std::env::temp_dir().join(format!("keep-aside-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pids = dir.join("pids");
        let script = format!("sleep 600 & echo $$ $! > {}; wait", pids.display());
        let mut aside = None;
        keep_beside(&mut aside, || Ok(true), || shell(&script)).unwrap();
        let held = written(&pids);
        let (leader, started) = held.trim().split_once(' ').unwrap();
        assert!(runs(leader) && runs(started));
        let kept = keep_beside(
            &mut aside,
            || {
                assert!(
                    !runs(leader) && !runs(started),
                    "the keep and the aside would share the one clone"
                );
                Ok(false)
            },
            || panic!("nothing was kept"),
        );
        assert_eq!(kept, Ok(false));
        assert!(aside.is_none(), "nothing kept, so nothing beside it");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_keep_stands_when_its_aside_cannot_start() {
        let mut aside = None;
        let kept = keep_beside(&mut aside, || Ok(true), || Err(io::Error::other("gone")));
        assert_eq!(
            kept,
            Ok(true),
            "the record stands, and the aside is tried at the next keep"
        );
        assert!(aside.is_none(), "an aside that couldn't start isn't held");
    }

    #[test]
    fn an_asides_end_is_heard_and_it_is_let_go() {
        let mut aside = Some(shell("exit 3").unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        let ended = loop {
            if let Some(ended) = look_aside(&mut aside) {
                break ended;
            }
            assert!(Instant::now() < deadline, "the aside's end went unheard");
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(ended.code(), Some(3));
        assert!(aside.is_none(), "an aside that ended is let go");
    }

    #[test]
    fn the_keeper_ends_once_another_build_is_installed() {
        let dir = std::env::temp_dir().join(format!("keep-follow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (old, new) = (dir.join("old"), dir.join("new"));
        std::fs::write(&old, "").unwrap();
        std::fs::write(&new, "").unwrap();
        let installed = || std::fs::canonicalize(&new);
        assert!(
            !replaced(installed(), Ok(new.clone())),
            "the build installed runs"
        );
        assert!(
            replaced(installed(), Ok(old.clone())),
            "an older build runs"
        );
        assert!(
            replaced(installed(), Ok(dir.join("collected"))),
            "the running build is gone from disk"
        );
        assert!(
            !replaced(std::fs::canonicalize(dir.join("unread")), Ok(old)),
            "nothing can be read of what's installed"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_head_waits_until_it_has_stood() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        assert_eq!(keeper.due("a", ONE, start), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD / 2), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD), Some("a".to_owned()));
    }

    #[test]
    fn a_head_is_kept_by_the_second_look_at_it() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        assert_eq!(
            keeper.due("a", ONE, start + LOOK_EVERY),
            Some("a".to_owned()),
            "a head still there a look later waited longer, though most runs that paid for \
             their base came within five minutes of it landing"
        );
    }

    #[test]
    fn a_head_that_moves_on_starts_its_wait_again() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        assert_eq!(
            keeper.due("b", ONE, start + STOOD),
            None,
            "b was only just seen"
        );
        assert_eq!(
            keeper.due("b", ONE, start + STOOD * 2),
            Some("b".to_owned())
        );
    }

    #[test]
    fn nothing_is_sent_with_no_builder_lent() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", None, start);
        assert_eq!(keeper.due("a", None, start + STOOD), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD), Some("a".to_owned()));
    }

    #[test]
    fn a_kept_head_is_sent_again_only_to_another_builder() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        keeper.kept("a", "one");
        assert_eq!(keeper.due("a", ONE, start + STOOD * 30), None);
        assert_eq!(
            keeper.due("a", Some("two"), start + STOOD * 30),
            Some("a".to_owned())
        );
    }

    #[test]
    fn a_head_that_failed_waits_past_a_builders_idle_spell_unless_another_is_lent() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        let tried = start + STOOD;
        keeper.failed("a", "one", tried);
        assert!(
            FAILED_AGAIN > Duration::from_mins(15),
            "a failed head waits past a builder's idle spell"
        );
        assert_eq!(keeper.due("a", ONE, tried + LOOK_EVERY), None);
        assert_eq!(
            keeper.due("a", Some("two"), tried + LOOK_EVERY),
            Some("a".to_owned())
        );
        assert_eq!(
            keeper.due("a", ONE, tried + FAILED_AGAIN),
            Some("a".to_owned())
        );
    }

    #[test]
    fn a_head_that_moves_on_is_sent_once_it_stands_whatever_failed_before() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        keeper.failed("a", "one", start + STOOD);
        let moved = start + STOOD + LOOK_EVERY;
        keeper.due("b", ONE, moved);
        assert_eq!(keeper.due("b", ONE, moved + STOOD), Some("b".to_owned()));
    }
}
