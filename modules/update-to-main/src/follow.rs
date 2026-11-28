//! The asker that follows main: an account's service that asks for an
//! update as soon as main moves, so a merge doesn't wait for someone to
//! ask. It reads where each repository's branch is with `git ls-remote`,
//! which costs the host one small answer, and asks when one moved since its
//! last update ended. A hosted main's head moves far more often than the
//! flake's folder in it, so its head is the newest commit that changed the
//! folder, as the account's own `changed` command says, run on each look
//! like `git ls-remote` is. It also asks every so often, in case that
//! command can't be run, which the daemon answers without a fetch while the
//! folder is where it was.
//!
//! Everything that may stop a switch stays the daemon's to judge: a call,
//! a hold, a low battery, a daemon whose restart needs a hold, and asks
//! that come while an update runs join it. The asker only chooses when to
//! ask: again soon after a refusal for any reason but a hold, unless the
//! machine runs the heads already, not again at heads that failed until
//! they move or a while has passed, sooner when what failed was a builder
//! going away mid-build, and not at all after a refusal for a held daemon
//! until the machine's last ending changes, since only a person's ask,
//! with a hold sent, gets that one through.
//!
//! Long because half of it is the tests of when to ask, each a timeline
//! of asks and endings that reads only beside the `Follower` it drives.

use crate::config::{Config, Hosted};
use crate::follows::Followed;
use crate::git;
use crate::hosted;
use crate::line::Line;
use crate::tell::Telling;
use crate::update::NEEDS_HOLD;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

/// How often the branches are read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How soon an ask refused for being busy, such as during a call, is made
/// again.
const BUSY_AGAIN: Duration = Duration::from_secs(60);

/// The longest a refused ask at heads the machine runs waits before it's
/// made again, however seldom the follower is told to ask: under the half
/// hour that ends a stretch of refusals (`busy`), so a busy command that can
/// never tell still lets one ask through at its two hours.
const BUSY_AT_MOST: Duration = Duration::from_mins(20);

/// How long heads whose update failed wait before they're asked for
/// again, in case what failed was the builders and not the commit.
pub const FAILED_AGAIN: Duration = Duration::from_mins(30);

/// The least time between a switch, whoever asked for it, and the next
/// the follower asks for while someone is at the machine. A home switch
/// restarts the desktop's shell: on 9 November a launcher stopped taking
/// keys after five switches the follower asked for in half an hour, one at
/// each core merge, so main's newest head waits for the next. The first
/// opening after a restart is the slow one a person feels, too.
const SWITCH_EVERY: Duration = Duration::from_mins(30);

/// The least time between switches once nobody has been at the machine for
/// `AWAY_AFTER`, since a restart nobody's hands are under costs nobody that
/// slow first opening. On 11 November the launcher took keys after each of
/// five shell restarts in ten minutes (`modules/quickshell/tests/opening-on-niri-test.py`).
const SWITCH_EVERY_AWAY: Duration = Duration::from_mins(10);

/// How long the machine must have gone untouched, as the follower's looks
/// see it, before it counts as left, so a step away for a coffee keeps the
/// spacing for someone there.
const AWAY_AFTER: Duration = Duration::from_mins(10);

/// The mark, in the account's runtime folder, that someone touched a key
/// or the pointer in the last two minutes (`../../swayidle/home.nix`).
const SOMEONE_HERE: &str = "someone-at-the-machine";

/// Whether the mark says someone is at the machine now. A machine without
/// the marks has nobody at it, as the daemon's own wait takes it.
pub fn someone_here() -> bool {
    std::env::var_os("XDG_RUNTIME_DIR")
        .is_some_and(|runtime| Path::new(&runtime).join(SOMEONE_HERE).exists())
}

/// How soon heads whose build lost its builders are asked for again, and
/// how many times in a row before they wait `FAILED_AGAIN` like any other
/// failure. On 8 November a lent builder's tunnel dropped and its service
/// restarted in the middle of a switch Joel waited on, and nothing asked
/// again for 30 minutes, though a builder stood again within one.
pub const DROPPED_AGAIN: Duration = Duration::from_mins(2);
pub const DROPPED_TRIES: u32 = 5;

/// What a build that lost its builders, rather than one that failed on its
/// own, says: no builder could be reached, or the one it was on went away.
/// A piece that fails to compile says none of these.
const DROPPED: [&str; 5] = [
    "Failed to find a machine for remote build",
    "disconnected unexpectedly",
    "Connection reset by peer",
    "closed by remote host",
    "said nothing for",
];

/// Whether a failed update failed because its builders went away, which
/// asking again soon may get past, rather than for something in the
/// commit.
pub fn builders_dropped(why: &str, lines: &[String]) -> bool {
    why.contains("builders")
        && std::iter::once(why)
            .chain(lines.iter().map(String::as_str))
            .any(|line| DROPPED.iter().any(|said| line.contains(said)))
}

/// How long an update may fetch heads main has since moved past before
/// it's stopped, so the next ask fetches the new ones.
const STALE_AFTER: Duration = Duration::from_mins(30);

/// Whether an update asked for at `asked`, still fetching since `since`,
/// is fetching heads main has moved past for long enough to stop it.
///
/// A build is never stopped for main moving: it finishes and switches,
/// and the next ask follows main's newer heads. Main takes about twenty
/// merges an hour, so a build stopped whenever main had moved past it for
/// half an hour could never finish one past that: on 9 November the
/// 1,718 pieces of an inputs update were stopped 30 minutes in and
/// planned again. A build that stops working is the daemon's to stop,
/// by its quiet limits; one a revert made moot, as a kernel was on 8
/// November, costs a switch that the next one follows.
pub fn stale(asked: &Followed, now: &Followed, latest: Option<&Line>, since: Duration) -> bool {
    asked != now && since >= STALE_AFTER && matches!(latest, Some(Line::Fetching))
}

/// The key main's own head is kept under among the inputs', which no
/// flake input can be named.
const MAIN: &str = "";

/// What the machine runs, as the follower last learnt it: the heads, as
/// its own looks name them, and the system the machine ran then. It holds
/// only while the machine still runs that system, so a switch or rollback
/// someone else made ends it at once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runs {
    pub heads: Followed,
    pub system: PathBuf,
}

/// Where a follower keeps what the machine runs for the next one, which a
/// switch, a login or a boot starts: in the account's own state, since
/// root's is closed to it. None without a home to keep it in.
pub fn runs_file() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|state| !state.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|home| !home.is_empty())
                .map(|home| PathBuf::from(home).join(".local/state"))
        })?;
    Some(state.join("update-to-main/follower-runs.json"))
}

/// What the last follower kept of what the machine runs: none when
/// nothing was kept or it can't be read, which only costs a minute's asks.
pub fn kept_runs(file: &Path) -> Option<Runs> {
    serde_json::from_str(&fs::read_to_string(file).ok()?).ok()
}

/// Keeps what the machine runs for the next follower, whole: written
/// beside the file, then moved into place.
pub fn keep_runs(file: &Path, runs: &Runs) -> Result<(), String> {
    let named = |e: std::io::Error| format!("{}: {e}", file.display());
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(named)?;
    }
    let text = serde_json::to_string(runs).map_err(|e| e.to_string())?;
    let making = file.with_extension("new");
    fs::write(&making, text)
        .and_then(|()| fs::rename(&making, file))
        .map_err(named)
}

/// The system the machine runs, as its profile leads to it; none when the
/// profile can't be followed.
pub fn system(profile: &Path) -> Option<PathBuf> {
    fs::canonicalize(profile).ok()
}

/// What the asker knows from its last ask.
#[derive(Debug, Default)]
pub struct Follower {
    /// When it last asked.
    asked: Option<Instant>,
    /// The heads its last ask was made at.
    last: Option<Followed>,
    /// The heads whose update failed, when, and how long they wait.
    failed: Option<(Followed, Instant, Duration)>,
    /// How many updates in a row lost their builders at the heads that
    /// failed.
    dropped: u32,
    /// Whether the last ask was refused, for any reason but a hold: a
    /// load, a call, a pause, a battery, or a daemon it couldn't reach.
    busy: bool,
    /// What the machine runs, from the last ask that ended done or
    /// unchanged, its own or kept by the follower before it. A refusal
    /// there isn't asked again each minute, since there's nothing to
    /// build: on 21 November a Mac over its load refused 26 asks in half
    /// an hour for what it ran.
    current: Option<Runs>,
    /// The system the machine ran at its last look.
    system: Option<PathBuf>,
    /// The machine's last ending as its file held it when last read, so
    /// an ending someone else's update wrote is told from one already
    /// counted.
    ended: Option<String>,
    /// Whether main is a host's, whose endings name main's head while the
    /// follower's looks name where its folder last changed.
    hosted: bool,
    /// When the machine last switched, for an ask of its own or as its
    /// system profile says.
    switched: Option<Instant>,
    /// The machine's last ending when an ask was refused for a held
    /// daemon, so it waits until that changes.
    held: Option<String>,
    /// Since when its looks have found nobody at the machine; none while
    /// someone is, or before it has looked, since it can't know how long
    /// they've been gone.
    nobody_since: Option<Instant>,
    /// What the person is told of the machine not updating.
    telling: Telling,
}

impl Follower {
    /// A follower knowing what the machine ran when the last one stopped,
    /// as it kept it (`kept_runs`).
    pub fn knowing(runs: Option<Runs>) -> Self {
        Self {
            current: runs,
            ..Self::default()
        }
    }

    /// Counts whether main is a host's, as the config read last says.
    pub fn hosted(&mut self, hosted: bool) {
        self.hosted = hosted;
    }

    /// Counts the system the machine runs now, as its profile leads to it.
    pub fn saw_system(&mut self, system: Option<PathBuf>) {
        self.system = system;
    }

    /// Counts the machine's last ending as its file holds it at a look at
    /// `heads`. One that changed since the last read was written by an
    /// update someone else asked for, and when it ended done or unchanged
    /// the machine runs what it names, so a refusal there isn't asked
    /// again each minute. The first read may be from before a rollback,
    /// which writes no ending, so it's only kept to compare with.
    pub fn saw_ended(&mut self, heads: &Followed, ended: &str) {
        if self.ended.as_deref() == Some(ended) {
            return;
        }
        if self.ended.is_some()
            && let Ok(
                Line::Done {
                    commit, followed, ..
                }
                | Line::Unchanged {
                    commit, followed, ..
                },
            ) = serde_json::from_str(ended.trim())
        {
            self.current = self.ran(heads, &commit, &followed);
        }
        self.ended = Some(ended.to_owned());
    }

    /// What the machine runs, as the follower knows it, to keep for the
    /// next one.
    pub fn runs(&self) -> Option<&Runs> {
        self.current.as_ref()
    }

    /// What the machine runs after an ask at `heads` ended naming `commit`
    /// and `followed`, when the follower can say. A main not a host's is
    /// named by the ending, since the daemon fetches for itself and may
    /// have found heads the follower's look didn't, or joined a person's
    /// update at others. A hosted main's ending names main's head, which
    /// the follower's looks never do, so the heads it asked at stand in,
    /// when the ending's inputs agree with them.
    fn ran(&self, heads: &Followed, commit: &str, followed: &Followed) -> Option<Runs> {
        let system = self.system.clone()?;
        let mut ran = followed.clone();
        if self.hosted {
            let inputs: Followed = heads
                .iter()
                .filter(|(name, _)| name.as_str() != MAIN)
                .map(|(name, commit)| (name.clone(), commit.clone()))
                .collect();
            if inputs != *followed {
                return None;
            }
            if let Some(main) = heads.get(MAIN) {
                ran.insert(MAIN.to_owned(), main.clone());
            }
        } else {
            ran.insert(MAIN.to_owned(), commit.to_owned());
        }
        Some(Runs { heads: ran, system })
    }

    /// Counts a switch the machine made at `at`, whoever asked for it. On
    /// 9 November a person's switch at 18:03 restarted the follower, which
    /// asked again a minute later and built a switch by 18:08 that only
    /// Joel's hands held back, since what it remembered of switches had
    /// gone with it and that one wasn't its own.
    pub fn saw_switch(&mut self, at: Instant) {
        if self.switched.is_none_or(|switched| at > switched) {
            self.switched = Some(at);
        }
    }

    /// Counts whether someone is at the machine at a look made at `at`.
    pub fn saw(&mut self, someone: bool, at: Instant) {
        if someone {
            self.nobody_since = None;
        } else if self.nobody_since.is_none() {
            self.nobody_since = Some(at);
        }
    }

    /// The least time after a switch before it asks for the next, at `now`.
    fn spacing(&self, now: Instant) -> Duration {
        if self
            .nobody_since
            .is_some_and(|since| now.duration_since(since) >= AWAY_AFTER)
        {
            SWITCH_EVERY_AWAY
        } else {
            SWITCH_EVERY
        }
    }

    /// Whether to ask now, at these heads, with the machine's last ending
    /// as its file holds it, given how often a hosted main is asked for.
    pub fn should_ask(
        &mut self,
        heads: &Followed,
        ended: &str,
        now: Instant,
        every: Duration,
    ) -> bool {
        if let Some(when_held) = &self.held {
            if when_held == ended {
                return false;
            }
            self.held = None;
        }
        if self
            .switched
            .is_some_and(|switched| now.duration_since(switched) < self.spacing(now))
        {
            return false;
        }
        let Some(asked) = self.asked else {
            return true;
        };
        let since = now.duration_since(asked);
        if let Some((failed, when, wait)) = &self.failed
            && failed == heads
        {
            return now.duration_since(*when) >= *wait;
        }
        let behind = self
            .current
            .as_ref()
            .is_none_or(|runs| runs.heads != *heads || self.system.as_ref() != Some(&runs.system));
        self.last.as_ref() != Some(heads)
            || since >= every
            || (self.busy && (behind || since >= BUSY_AT_MOST) && since >= BUSY_AGAIN)
    }

    /// Why the machine can't update, once each time it stops being able
    /// to (`crate::tell`).
    pub fn to_tell(&mut self) -> Option<String> {
        self.telling.take()
    }

    /// Keeps how an ask made at `heads` ended, with the machine's last
    /// ending as its file holds it afterwards and the `system` it runs
    /// then, which a switch the ask made has changed. Says why it now
    /// waits for a person, when it does.
    pub fn asked(
        &mut self,
        heads: Followed,
        ending: &Line,
        ended: &str,
        system: Option<PathBuf>,
        now: Instant,
    ) -> Option<String> {
        self.system = system;
        self.ended = Some(ended.to_owned());
        self.asked = Some(now);
        self.busy = false;
        self.last = Some(heads.clone());
        if matches!(ending, Line::Done { .. }) {
            self.switched = Some(now);
        }
        match ending {
            Line::Done {
                commit, followed, ..
            }
            | Line::Unchanged {
                commit, followed, ..
            } => {
                self.current = self.ran(&heads, commit, followed);
                self.failed = None;
                self.dropped = 0;
                self.telling.went_through();
                None
            }
            Line::Failed { why, lines, .. } => {
                let same = self.failed.as_ref().is_some_and(|(h, _, _)| *h == heads);
                self.dropped = if builders_dropped(why, lines) {
                    if same { self.dropped + 1 } else { 1 }
                } else {
                    0
                };
                let wait = if (1..=DROPPED_TRIES).contains(&self.dropped) {
                    DROPPED_AGAIN
                } else {
                    self.telling.failed(&heads, why);
                    FAILED_AGAIN
                };
                self.failed = Some((heads, now, wait));
                None
            }
            Line::Refused { why, .. } if why.starts_with(NEEDS_HOLD) => {
                self.held = Some(ended.to_owned());
                self.telling.waits(why);
                Some(why.clone())
            }
            _ => {
                self.busy = true;
                None
            }
        }
    }
}

/// When the system profile last changed, which every switch does, as an
/// instant: none when it can't be read, or changed before this boot or in
/// what the clock says is the future.
pub fn profile_changed(profile: &Path, now: Instant) -> Option<Instant> {
    let made = fs::symlink_metadata(profile).ok()?.modified().ok()?;
    now.checked_sub(SystemTime::now().duration_since(made).ok()?)
}

/// The update-to-main a follower started as `started` should start in its
/// place: the config's `program` where it is one that exists and isn't
/// this one. A program that leads nowhere yet, as midway through a switch,
/// or a follower that can't name itself, starts nothing, since starting a
/// missing file would end the follower and starting itself again would
/// never pause. It's the program as resolved, so what was compared is what
/// runs.
pub fn newer(program: Option<&Path>, started: Option<&Path>) -> Option<PathBuf> {
    let program = fs::canonicalize(program?).ok()?;
    (program != started?).then_some(program)
}

/// Where main and each followed input's branch are, as their hosts say,
/// given where they were at the last look. A hosted main is where its
/// folder last changed instead, since its host wants a token only root
/// holds.
pub fn heads(config: &Config, was: Option<&Followed>) -> Result<Followed, String> {
    let mut heads = Followed::new();
    match &config.hosted {
        None => {
            heads.insert(
                MAIN.to_owned(),
                remote_head(config, &config.repository, &config.branch)?,
            );
        }
        Some(hosted) => {
            if let Some(folder) = folder_head(hosted, was) {
                heads.insert(MAIN.to_owned(), folder);
            }
        }
    }
    for (name, follow) in &config.follows {
        heads.insert(
            name.clone(),
            remote_head(config, &follow.repository, &follow.branch)?,
        );
    }
    Ok(heads)
}

/// The newest commit that changed a hosted main's folder, as its `changed`
/// command says. A read that fails, or says no commit, keeps the last
/// look's answer, so a host away a moment isn't taken for a move.
fn folder_head(hosted: &Hosted, was: Option<&Followed>) -> Option<String> {
    let read = || {
        let (program, args) = hosted.changed.as_ref()?.split_first()?;
        let mut command = Command::new(program);
        let output = asking_nobody(&mut command)
            .args(args)
            .env("PATH", std::env::join_paths(&hosted.path).ok()?)
            .current_dir(&hosted.checkout)
            .stderr(Stdio::null())
            .output()
            .ok()?;
        let said = String::from_utf8_lossy(&output.stdout);
        let commit = said.lines().last()?.trim();
        (output.status.success() && hosted::is_commit(commit)).then(|| commit.to_owned())
    };
    read().or_else(|| was?.get(MAIN).cloned())
}

fn remote_head(config: &Config, repository: &str, branch: &str) -> Result<String, String> {
    let mut command = Command::new(&config.tools.git);
    let output = asking_nobody(&mut command)
        .args([
            "ls-remote",
            "--exit-code",
            repository,
            &format!("refs/heads/{branch}"),
        ])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("{}: {e}", config.tools.git.display()))?;
    let said = String::from_utf8_lossy(&output.stdout);
    match said.split_whitespace().next() {
        Some(commit) if output.status.success() => Ok(commit.to_owned()),
        _ => Err(format!("{repository} names no {branch}")),
    }
}

/// Keeps a command the follower runs from asking anyone for a login. It
/// runs in the account's session from the moment the account signs in,
/// before the credential broker that hands out its host's tokens is up,
/// so a credential helper that gives nothing yet sent git to the session's
/// askpass, and the desktop showed a sign-in nobody could answer (Joel, 8
/// November: "everytime i boot it asks me for a delta git login"). Git
/// asks an askpass whether or not terminal prompts are off, so both go,
/// with git told not to ask at all; a look that gets no answer is taken
/// again in `LOOK_EVERY`, once the broker can give one. A look whose host
/// stops answering ends once it has stalled, and is taken again too.
fn asking_nobody(command: &mut Command) -> &mut Command {
    git::ending_stalls(command)
        .env_remove("SSH_ASKPASS")
        .env_remove("GIT_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "credential.interactive")
        .env("GIT_CONFIG_VALUE_0", "false")
        .stdin(Stdio::null())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn what_it_runs_can_ask_nobody_for_a_login() {
        let mut command = Command::new("git");
        command
            .env("SSH_ASKPASS", "/the/desktops/askpass")
            .env("GIT_ASKPASS", "/another/askpass");
        asking_nobody(&mut command);
        let envs: Vec<_> = command.get_envs().collect();
        for askpass in ["SSH_ASKPASS", "GIT_ASKPASS"] {
            assert!(
                envs.contains(&(OsStr::new(askpass), None)),
                "{askpass} reaches it: {envs:?}"
            );
        }
        for (key, value) in [
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_CONFIG_KEY_0", "credential.interactive"),
            ("GIT_CONFIG_VALUE_0", "false"),
            ("GIT_HTTP_LOW_SPEED_TIME", "60"),
        ] {
            assert!(
                envs.contains(&(OsStr::new(key), Some(OsStr::new(value)))),
                "{key} isn't {value}: {envs:?}"
            );
        }
    }

    #[test]
    fn a_hosted_mains_folder_is_read_without_a_login_prompt() {
        let hosted = hosted_saying(&format!(
            "[ -z \"${{SSH_ASKPASS-}}\" ] && [ \"$GIT_TERMINAL_PROMPT\" = 0 ] \
             && [ \"$GIT_CONFIG_VALUE_0\" = false ] && echo {COMMIT}"
        ));
        assert_eq!(folder_head(&hosted, None).as_deref(), Some(COMMIT));
    }

    #[test]
    fn an_update_is_stale_once_main_moves_past_a_long_fetch_but_a_build_runs_on() {
        let long = STALE_AFTER;
        assert!(stale(&at("a"), &at("b"), Some(&Line::Fetching), long));
        assert!(
            !stale(&at("a"), &at("a"), Some(&Line::Fetching), long),
            "main hasn't moved"
        );
        assert!(
            !stale(
                &at("a"),
                &at("b"),
                Some(&Line::Fetching),
                Duration::from_mins(29)
            ),
            "a short fetch finishes rather than start again at each merge"
        );
        let building = Line::Building {
            commit: "c".to_owned(),
        };
        assert!(
            !stale(&at("a"), &at("b"), Some(&building), Duration::from_hours(2)),
            "a build finishes and switches, however long main has moved past it"
        );
        let switching = Line::Switching {
            commit: "c".to_owned(),
        };
        assert!(
            !stale(&at("a"), &at("b"), Some(&switching), long),
            "a switch runs on"
        );
    }

    const EVERY: Duration = Duration::from_secs(600);

    pub(crate) fn at(commit: &str) -> Followed {
        Followed::from([("core".to_owned(), commit.to_owned())])
    }

    fn done() -> Line {
        Line::Unchanged {
            commit: "c".to_owned(),
            followed: Followed::new(),
            boots: None,
        }
    }

    pub(crate) fn refused(why: &str) -> Line {
        Line::Refused {
            commit: None,
            why: why.to_owned(),
        }
    }

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    fn hosted_saying(script: &str) -> Hosted {
        Hosted {
            user: "someone".to_owned(),
            checkout: std::env::temp_dir(),
            directory: "flake".to_owned(),
            token: Vec::new(),
            changed: Some(vec!["sh".to_owned(), "-c".to_owned(), script.to_owned()]),
            api: None,
            may_fetch: None,
            fetched: None,
            path: std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect(),
            inputs: Vec::new(),
        }
    }

    #[test]
    fn a_hosted_main_is_where_its_folder_last_changed() {
        let hosted = hosted_saying(&format!("echo a warning first; echo {COMMIT}"));
        assert_eq!(folder_head(&hosted, None).as_deref(), Some(COMMIT));
    }

    #[test]
    fn a_folder_that_cant_be_read_stays_where_it_was() {
        let was = Followed::from([(MAIN.to_owned(), COMMIT.to_owned())]);
        for script in ["echo not-a-commit", &format!("echo {COMMIT}; exit 1")] {
            let hosted = hosted_saying(script);
            assert_eq!(folder_head(&hosted, Some(&was)).as_deref(), Some(COMMIT));
            assert_eq!(folder_head(&hosted, None), None, "{script} names no commit");
        }
    }

    #[test]
    fn a_switch_it_asked_for_waits_half_an_hour_for_the_next() {
        let mut f = Follower::default();
        let t = Instant::now();
        let switched = Line::Done {
            commit: "c".to_owned(),
            applied: crate::line::Applied::Now,
            restart: None,
            restarted: Vec::new(),
            boots: None,
            followed: Followed::new(),
        };
        f.asked(at("a"), &switched, "", None, t);
        assert!(
            !f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY),
            "main moved a minute after a switch, which waits"
        );
        assert!(
            !f.should_ask(
                &at("b"),
                "",
                t + SWITCH_EVERY.saturating_sub(LOOK_EVERY),
                EVERY
            ),
            "it asked again within half an hour of a switch"
        );
        assert!(
            f.should_ask(&at("b"), "", t + SWITCH_EVERY, EVERY),
            "half an hour after a switch, it still didn't ask"
        );
        // An ask that changed nothing leaves the next switch free to come.
        let mut f = Follower::default();
        f.asked(at("a"), &done(), "", None, t);
        assert!(
            f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY),
            "an ask that switched nothing held the next"
        );
    }

    #[test]
    fn a_switch_anyone_made_waits_half_an_hour_for_the_next() {
        let mut f = Follower::default();
        let earlier = Instant::now();
        let t = earlier + LOOK_EVERY;
        f.saw_switch(t);
        f.saw_switch(earlier);
        assert!(
            !f.should_ask(
                &at("a"),
                "",
                t + SWITCH_EVERY.saturating_sub(LOOK_EVERY),
                EVERY
            ),
            "it asked within half an hour of a switch it didn't ask for"
        );
        assert!(
            f.should_ask(&at("a"), "", t + SWITCH_EVERY, EVERY),
            "half an hour after a switch, it still didn't ask"
        );
    }

    #[test]
    fn a_machine_left_alone_switches_every_ten_minutes_until_someone_is_back() {
        let mut f = Follower::default();
        let t = Instant::now();
        f.saw_switch(t);
        f.saw(false, t);
        let ten = t + SWITCH_EVERY_AWAY;
        assert!(
            f.should_ask(&at("a"), "", ten, EVERY),
            "ten minutes after a switch nobody was there for, it didn't ask"
        );
        f.saw(false, ten);
        assert!(
            f.should_ask(&at("a"), "", ten, EVERY),
            "a look finding nobody again started its count over"
        );
        f.saw(true, ten);
        assert!(
            !f.should_ask(&at("a"), "", ten, EVERY),
            "someone back at the machine didn't bring the half hour back"
        );
        let mut f = Follower::default();
        f.saw_switch(t);
        let stepped_away = t + Duration::from_mins(5);
        f.saw(false, stepped_away);
        assert!(
            !f.should_ask(&at("a"), "", stepped_away + LOOK_EVERY, EVERY),
            "a few minutes away counted as having left"
        );
    }

    #[test]
    fn the_profile_says_when_the_machine_last_switched() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("system");
        let now = Instant::now();
        assert_eq!(profile_changed(&profile, now), None);
        std::os::unix::fs::symlink("system-1-link", &profile).unwrap();
        let changed = profile_changed(&profile, now).expect("a profile just made");
        assert!(
            now.duration_since(changed) < Duration::from_secs(5),
            "a profile made now changed {:?} ago",
            now.duration_since(changed)
        );
    }

    #[test]
    fn it_asks_once_when_it_starts_and_again_only_when_a_head_moves() {
        let mut f = Follower::default();
        let t = Instant::now();
        assert!(f.should_ask(&at("a"), "", t, EVERY));
        f.asked(at("a"), &done(), "", None, t);
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY));
    }

    #[test]
    fn a_main_it_cant_read_is_asked_for_every_so_often() {
        let mut f = Follower::default();
        let t = Instant::now();
        f.asked(at("a"), &done(), "", None, t);
        assert!(!f.should_ask(&at("a"), "", t + EVERY / 2, EVERY));
        assert!(f.should_ask(&at("a"), "", t + EVERY, EVERY));
    }

    #[test]
    fn a_busy_machine_is_asked_again_soon() {
        let mut f = Follower::default();
        let t = Instant::now();
        assert_eq!(
            f.asked(at("a"), &refused("a call is on"), "", None, t),
            None
        );
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("a"), "", t + BUSY_AGAIN, EVERY));
    }

    /// Main, not a host's, at `commit`.
    fn main_at(commit: &str) -> Followed {
        Followed::from([(MAIN.to_owned(), commit.to_owned())])
    }

    /// An ending that switched nothing, the machine running `commit`.
    fn unchanged(commit: &str) -> Line {
        Line::Unchanged {
            commit: commit.to_owned(),
            followed: Followed::new(),
            boots: None,
        }
    }

    /// The system a profile leads to, the `n`th.
    fn system_n(n: u32) -> PathBuf {
        PathBuf::from(format!("/nix/store/system-{n}"))
    }

    const LOADED: &str = "the machine is over its load";

    #[test]
    fn a_busy_machine_running_the_heads_isnt_asked_again_soon() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        f.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        f.asked(
            main_at("a"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY,
        );
        assert!(
            !f.should_ask(&main_at("a"), "", t + EVERY + BUSY_AGAIN, EVERY),
            "a busy machine was asked again each minute for the heads it runs"
        );
        assert!(
            f.should_ask(&main_at("a"), "", t + 2 * EVERY, EVERY),
            "a busy machine running the heads wasn't asked every so often"
        );
        f.asked(
            main_at("b"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY + LOOK_EVERY,
        );
        assert!(
            f.should_ask(
                &main_at("b"),
                "",
                t + EVERY + LOOK_EVERY + BUSY_AGAIN,
                EVERY
            ),
            "a busy machine with a new head to build wasn't asked again soon"
        );
    }

    #[test]
    fn a_follower_started_again_knows_what_the_machine_runs() {
        let mut kept = Follower::default();
        kept.saw_system(Some(system_n(1)));
        let t = Instant::now();
        kept.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        let mut f = Follower::knowing(kept.runs().cloned());
        f.saw_system(Some(system_n(1)));
        assert!(
            f.should_ask(&main_at("a"), "", t, EVERY),
            "it didn't ask when it started"
        );
        f.asked(main_at("a"), &refused(LOADED), "", Some(system_n(1)), t);
        assert!(
            !f.should_ask(&main_at("a"), "", t + BUSY_AGAIN, EVERY),
            "a follower started again asked a busy machine each minute for what it runs"
        );
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        f.asked(main_at("a"), &refused(LOADED), "", Some(system_n(1)), t);
        assert!(
            f.should_ask(&main_at("a"), "", t + BUSY_AGAIN, EVERY),
            "a follower knowing nothing of what the machine runs wasn't asked again soon"
        );
    }

    #[test]
    fn a_switch_it_didnt_make_brings_the_soon_ask_back() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        f.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        f.asked(
            main_at("a"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY,
        );
        // Someone rolled the machine back, or switched it to other heads.
        f.saw_system(Some(system_n(2)));
        assert!(
            f.should_ask(&main_at("a"), "", t + EVERY + BUSY_AGAIN, EVERY),
            "a machine someone else switched still counted as running the heads"
        );
    }

    #[test]
    fn the_machine_runs_what_the_ending_names_not_what_the_follower_saw() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        // It looked at b, and joined a person's update that fetched a.
        f.asked(main_at("b"), &unchanged("a"), "", Some(system_n(1)), t);
        f.asked(
            main_at("b"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY,
        );
        assert!(
            f.should_ask(&main_at("b"), "", t + EVERY + BUSY_AGAIN, EVERY),
            "heads the machine doesn't run counted as running because the follower saw them"
        );
        f.asked(
            main_at("a"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY + BUSY_AGAIN,
        );
        assert!(
            !f.should_ask(&main_at("a"), "", t + EVERY + 2 * BUSY_AGAIN, EVERY),
            "the heads the ending named weren't counted as running"
        );
    }

    #[test]
    fn a_hosted_main_runs_the_heads_it_asked_at_when_the_inputs_agree() {
        let mut f = Follower::default();
        f.hosted(true);
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        let mut heads = main_at("folder");
        heads.insert("core".to_owned(), "c1".to_owned());
        let ending = |core: &str| Line::Unchanged {
            commit: "mains-head".to_owned(),
            followed: Followed::from([("core".to_owned(), core.to_owned())]),
            boots: None,
        };
        f.asked(heads.clone(), &ending("c1"), "", Some(system_n(1)), t);
        f.asked(
            heads.clone(),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + EVERY,
        );
        assert!(
            !f.should_ask(&heads, "", t + EVERY + BUSY_AGAIN, EVERY),
            "a hosted main whose inputs agree didn't count as running its folder's heads"
        );
        f.asked(
            heads.clone(),
            &ending("c0"),
            "",
            Some(system_n(1)),
            t + 2 * EVERY,
        );
        assert_eq!(
            f.runs(),
            None,
            "a hosted main whose ending named other inputs was said to run its folder's head"
        );
        f.asked(
            heads.clone(),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + 3 * EVERY,
        );
        assert!(
            f.should_ask(&heads, "", t + 3 * EVERY + BUSY_AGAIN, EVERY),
            "a hosted main whose ending named other inputs counted as running the heads"
        );
    }

    #[test]
    fn any_refusal_but_a_hold_at_heads_the_machine_runs_waits() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        f.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        f.asked(
            main_at("a"),
            &refused("a call is on"),
            "",
            Some(system_n(1)),
            t + EVERY,
        );
        assert!(
            !f.should_ask(&main_at("a"), "", t + EVERY + BUSY_AGAIN, EVERY),
            "a call's refusal at heads the machine runs was asked again each minute"
        );
    }

    /// An ending that switched the machine to `commit`.
    fn switched_to(commit: &str) -> Line {
        Line::Done {
            commit: commit.to_owned(),
            applied: crate::line::Applied::Now,
            restart: None,
            restarted: Vec::new(),
            boots: None,
            followed: Followed::new(),
        }
    }

    /// The ended file holding `ending`, as the daemon writes it.
    fn ended_file(ending: &Line) -> String {
        format!("{}\n", ending.json())
    }

    #[test]
    fn a_switch_its_own_ask_made_counts_as_what_the_machine_runs() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        f.asked(main_at("a"), &switched_to("a"), "", Some(system_n(2)), t);
        let later = t + SWITCH_EVERY;
        f.saw_system(Some(system_n(2)));
        f.asked(main_at("a"), &refused(LOADED), "", Some(system_n(2)), later);
        assert!(
            !f.should_ask(&main_at("a"), "", later + BUSY_AGAIN, EVERY),
            "a machine its own ask switched was asked again each minute for what it runs"
        );
    }

    #[test]
    fn an_update_someone_else_asked_for_counts_as_what_the_machine_runs() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        f.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        // A person's update switched the machine to b.
        let theirs = ended_file(&switched_to("b"));
        f.saw_system(Some(system_n(2)));
        f.saw_ended(&main_at("b"), &theirs);
        let refused_at = t + EVERY;
        f.asked(
            main_at("b"),
            &refused(LOADED),
            &theirs,
            Some(system_n(2)),
            refused_at,
        );
        f.saw_ended(&main_at("b"), &theirs);
        assert!(
            !f.should_ask(&main_at("b"), &theirs, refused_at + BUSY_AGAIN, EVERY),
            "a machine someone else's update switched was asked again each minute for what it runs"
        );
        // Someone rolled it back, which writes no ending.
        f.saw_system(Some(system_n(3)));
        f.saw_ended(&main_at("b"), &theirs);
        assert!(
            f.should_ask(&main_at("b"), &theirs, refused_at + 2 * BUSY_AGAIN, EVERY),
            "a rollback after someone else's update counted as running what that update named"
        );
    }

    #[test]
    fn the_first_ending_it_reads_isnt_counted_as_what_the_machine_runs() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        // A rollback since may have left the machine on another system.
        f.saw_ended(&main_at("a"), &ended_file(&unchanged("a")));
        assert_eq!(
            f.runs(),
            None,
            "an ending from before the follower started was counted as what the machine runs"
        );
    }

    #[test]
    fn a_hosted_mains_update_someone_else_asked_for_counts_when_the_inputs_agree() {
        let mut f = Follower::default();
        f.hosted(true);
        f.saw_system(Some(system_n(1)));
        let mut heads = main_at("folder");
        heads.insert("core".to_owned(), "c1".to_owned());
        f.saw_ended(&heads, "");
        let theirs = Line::Unchanged {
            commit: "mains-head".to_owned(),
            followed: Followed::from([("core".to_owned(), "c1".to_owned())]),
            boots: None,
        };
        f.saw_ended(&heads, &ended_file(&theirs));
        assert_eq!(
            f.runs().map(|runs| &runs.heads),
            Some(&heads),
            "a hosted main's update someone else asked for didn't count as running the folder's heads"
        );
    }

    #[test]
    fn a_refusal_at_heads_the_machine_runs_is_asked_again_before_its_stretch_ends() {
        let mut f = Follower::default();
        f.saw_system(Some(system_n(1)));
        let t = Instant::now();
        let seldom = Duration::from_hours(1);
        f.asked(main_at("a"), &unchanged("a"), "", Some(system_n(1)), t);
        f.asked(
            main_at("a"),
            &refused(LOADED),
            "",
            Some(system_n(1)),
            t + seldom,
        );
        let twenty = Duration::from_mins(20);
        let almost = Duration::from_secs(20 * 60 - 1);
        assert!(
            !f.should_ask(&main_at("a"), "", t + seldom + almost, seldom),
            "a refusal at heads the machine runs was asked again before twenty minutes"
        );
        assert!(
            f.should_ask(&main_at("a"), "", t + seldom + twenty, seldom),
            "a follower told to ask hourly left a refusal past the stretch that counts it"
        );
        assert!(
            BUSY_AT_MOST < Duration::from_secs(crate::busy::A_STRETCH_ENDS_AFTER),
            "the longest wait isn't under the half hour that ends a stretch of refusals"
        );
    }

    #[test]
    fn what_the_machine_runs_is_kept_for_the_next_follower() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("state/update-to-main/follower-runs.json");
        assert_eq!(kept_runs(&file), None, "nothing kept read as something");
        let runs = Runs {
            heads: main_at("a"),
            system: PathBuf::from("/nix/store/system-1"),
        };
        keep_runs(&file, &runs).unwrap();
        assert_eq!(
            kept_runs(&file),
            Some(runs),
            "what was kept didn't read back"
        );
        fs::write(&file, "not json").unwrap();
        assert_eq!(
            kept_runs(&file),
            None,
            "a file it can't read counted as something"
        );
    }

    #[test]
    fn heads_that_failed_wait_until_they_move_or_a_while_passes() {
        let mut f = Follower::default();
        let t = Instant::now();
        let failed = Line::Failed {
            commit: None,
            why: "it didn't evaluate".to_owned(),
            lines: Vec::new(),
        };
        f.asked(at("a"), &failed, "", None, t);
        assert!(!f.should_ask(&at("a"), "", t + EVERY, EVERY));
        assert!(f.should_ask(&at("a"), "", t + FAILED_AGAIN, EVERY));
        assert!(f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY));
    }

    fn dropped(line: &str) -> Line {
        Line::Failed {
            commit: None,
            why: "building the system on the machine's builders failed (exit status: 1), and \
                  CPU pressure here is 32.7%, not below 15%, so they wait"
                .to_owned(),
            lines: vec!["copying 1 paths...".to_owned(), line.to_owned()],
        }
    }

    #[test]
    fn heads_whose_builders_went_away_are_asked_for_again_soon_a_few_times() {
        let mut f = Follower::default();
        let mut t = Instant::now();
        let lost = dropped("error: Nix daemon disconnected unexpectedly (maybe it crashed?)");
        for _ in 0..DROPPED_TRIES {
            f.asked(at("a"), &lost, "", None, t);
            assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
            t += DROPPED_AGAIN;
            assert!(f.should_ask(&at("a"), "", t, EVERY));
        }
        // Past its tries, it waits like any failure.
        f.asked(at("a"), &lost, "", None, t);
        assert!(!f.should_ask(&at("a"), "", t + DROPPED_AGAIN, EVERY));
        assert!(f.should_ask(&at("a"), "", t + FAILED_AGAIN, EVERY));
    }

    #[test]
    fn a_piece_that_fails_on_a_builder_isnt_asked_for_again_soon() {
        let mut f = Follower::default();
        let t = Instant::now();
        let broke =
            dropped("error: Cannot build '/nix/store/x-a.drv'. builder failed with exit code 1.");
        f.asked(at("a"), &broke, "", None, t);
        assert!(!f.should_ask(&at("a"), "", t + DROPPED_AGAIN, EVERY));
    }

    #[test]
    fn what_says_its_builders_went_away() {
        let none = "Failed to find a machine for remote build!".to_owned();
        assert!(builders_dropped(
            "building the system on the machine's builders failed",
            std::slice::from_ref(&none)
        ));
        assert!(!builders_dropped("it didn't evaluate", &[none]));
        assert!(builders_dropped(
            "building the big pieces on the machine's builders said nothing for 4 minutes, so it \
             was stopped, which never build here",
            &[]
        ));
    }

    #[test]
    fn a_held_daemon_stops_it_until_the_machines_last_ending_changes() {
        let mut f = Follower::default();
        let t = Instant::now();
        let why = format!("{NEEDS_HOLD} nix-daemon.service, which waits for a hold");
        assert_eq!(
            f.asked(at("a"), &refused(&why), "refused", None, t),
            Some(why)
        );
        assert!(!f.should_ask(&at("b"), "refused", t + FAILED_AGAIN, EVERY));
        // A person's ask, with a hold sent, ended some other way.
        assert!(f.should_ask(&at("b"), "done", t + FAILED_AGAIN, EVERY));
    }

    /// Two programs in a folder, as a switch leaves an older and a newer
    /// update-to-main in the store, and a link to the newer, as the
    /// system's path gives it.
    fn programs() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let older = dir.path().join("older");
        let newer = dir.path().join("newer");
        let linked = dir.path().join("linked");
        fs::write(&older, "").unwrap();
        fs::write(&newer, "").unwrap();
        std::os::unix::fs::symlink(&newer, &linked).unwrap();
        let (older, newer) = (older.canonicalize().unwrap(), newer.canonicalize().unwrap());
        (dir, older, newer, linked)
    }

    #[test]
    fn a_follower_starts_the_newer_program_the_system_links_to() {
        let (_dir, older, newer_one, linked) = programs();
        assert_eq!(newer(Some(&linked), Some(&older)), Some(newer_one));
    }

    #[test]
    fn a_follower_starts_nothing_when_the_system_holds_it_already() {
        let (_dir, _older, newer_one, linked) = programs();
        assert_eq!(newer(Some(&linked), Some(&newer_one)), None);
    }

    #[test]
    fn a_follower_starts_nothing_from_a_program_that_leads_nowhere() {
        let (dir, older, _newer, _linked) = programs();
        let missing = dir.path().join("not-yet");
        assert_eq!(newer(Some(&missing), Some(&older)), None);
        assert_eq!(newer(None, Some(&older)), None);
    }

    #[test]
    fn a_follower_that_cant_name_itself_starts_nothing() {
        let (_dir, _older, _newer, linked) = programs();
        assert_eq!(newer(Some(&linked), None), None);
    }
}
