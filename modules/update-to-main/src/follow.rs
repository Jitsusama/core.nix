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
//! ask: again soon after a refusal for being busy, not again at heads that
//! failed until they move or a while has passed, sooner when what failed
//! was a builder going away mid-build, and not at all after a
//! refusal for a held daemon until the machine's last ending changes,
//! since only a person's ask, with a hold sent, gets that one through.

use crate::config::{Config, Hosted};
use crate::follows::Followed;
use crate::hosted;
use crate::line::Line;
use crate::update::NEEDS_HOLD;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

/// How often the branches are read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How soon an ask refused for being busy, such as during a call, is made
/// again.
const BUSY_AGAIN: Duration = Duration::from_secs(60);

/// How long heads whose update failed wait before they're asked for
/// again, in case what failed was the builders and not the commit.
const FAILED_AGAIN: Duration = Duration::from_mins(30);

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
/// five shell restarts in ten minutes (`tests/opening-on-niri-test.py`).
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
const DROPPED_AGAIN: Duration = Duration::from_mins(2);
const DROPPED_TRIES: u32 = 5;

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
    /// Whether the last ask was refused for being busy.
    busy: bool,
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
}

impl Follower {
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
        self.last.as_ref() != Some(heads) || since >= every || (self.busy && since >= BUSY_AGAIN)
    }

    /// Keeps how an ask made at `heads` ended, with the machine's last
    /// ending as its file holds it afterwards. Says why it now waits for a
    /// person, when it does.
    pub fn asked(
        &mut self,
        heads: Followed,
        ending: &Line,
        ended: &str,
        now: Instant,
    ) -> Option<String> {
        self.asked = Some(now);
        self.busy = false;
        self.last = Some(heads.clone());
        if matches!(ending, Line::Done { .. }) {
            self.switched = Some(now);
        }
        match ending {
            Line::Done { .. } | Line::Unchanged { .. } => {
                self.failed = None;
                self.dropped = 0;
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
                    FAILED_AGAIN
                };
                self.failed = Some((heads, now, wait));
                None
            }
            Line::Refused { why, .. } if why.starts_with(NEEDS_HOLD) => {
                self.held = Some(ended.to_owned());
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
/// again in `LOOK_EVERY`, once the broker can give one.
fn asking_nobody(command: &mut Command) -> &mut Command {
    command
        .env_remove("SSH_ASKPASS")
        .env_remove("GIT_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "credential.interactive")
        .env("GIT_CONFIG_VALUE_0", "false")
        .stdin(Stdio::null())
}

#[cfg(test)]
mod tests {
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

    fn at(commit: &str) -> Followed {
        Followed::from([("core".to_owned(), commit.to_owned())])
    }

    fn done() -> Line {
        Line::Unchanged {
            commit: "c".to_owned(),
            followed: Followed::new(),
        }
    }

    fn refused(why: &str) -> Line {
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
            followed: Followed::new(),
        };
        f.asked(at("a"), &switched, "", t);
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
        f.asked(at("a"), &done(), "", t);
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
        f.asked(at("a"), &done(), "", t);
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY));
    }

    #[test]
    fn a_main_it_cant_read_is_asked_for_every_so_often() {
        let mut f = Follower::default();
        let t = Instant::now();
        f.asked(at("a"), &done(), "", t);
        assert!(!f.should_ask(&at("a"), "", t + EVERY / 2, EVERY));
        assert!(f.should_ask(&at("a"), "", t + EVERY, EVERY));
    }

    #[test]
    fn a_busy_machine_is_asked_again_soon() {
        let mut f = Follower::default();
        let t = Instant::now();
        assert_eq!(f.asked(at("a"), &refused("a call is on"), "", t), None);
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("a"), "", t + BUSY_AGAIN, EVERY));
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
        f.asked(at("a"), &failed, "", t);
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
            f.asked(at("a"), &lost, "", t);
            assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
            t += DROPPED_AGAIN;
            assert!(f.should_ask(&at("a"), "", t, EVERY));
        }
        // Past its tries, it waits like any failure.
        f.asked(at("a"), &lost, "", t);
        assert!(!f.should_ask(&at("a"), "", t + DROPPED_AGAIN, EVERY));
        assert!(f.should_ask(&at("a"), "", t + FAILED_AGAIN, EVERY));
    }

    #[test]
    fn a_piece_that_fails_on_a_builder_isnt_asked_for_again_soon() {
        let mut f = Follower::default();
        let t = Instant::now();
        let broke =
            dropped("error: Cannot build '/nix/store/x-a.drv'. builder failed with exit code 1.");
        f.asked(at("a"), &broke, "", t);
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
        assert_eq!(f.asked(at("a"), &refused(&why), "refused", t), Some(why));
        assert!(!f.should_ask(&at("b"), "refused", t + FAILED_AGAIN, EVERY));
        // A person's ask, with a hold sent, ended some other way.
        assert!(f.should_ask(&at("b"), "done", t + FAILED_AGAIN, EVERY));
    }
}
