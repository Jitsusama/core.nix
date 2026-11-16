//! The calls receptor: how many of Joel's calls there were, said to the
//! realm's mycelium node, which draws it as a cell on the calls' outside
//! network (mycelium's PROTOCOL.md, Receptors).
//!
//! Only counts and fixed words leave: the calls started in the last hour
//! and since local midnight, taken from the book's own start times, and
//! whether the book could be read. No title, person, event or folder of a
//! call is ever an argument, so nothing a call's files say reaches the
//! node, whatever they hold. mycelium is run by its store path, its
//! arguments given one by one and never through a shell.
//!
//! The service names the program and the realm in `MYCELIUM` and
//! `MYCELIUM_REALM` (modules/call-transcripts/home.nix); with either unset,
//! as on a machine in no realm, nothing is said.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::book;

/// How often the receptor says how it stands, which is also how late the
/// node lets it be before reading it as failing.
pub const EVERY: Duration = Duration::from_secs(60);

/// An hour, in the book's milliseconds.
const HOUR: u64 = 3_600_000;

/// The source this receptor reads, as mycelium names it.
const SOURCE: &str = "calls";

/// How many calls started in the last hour, and since local midnight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub hour: u64,
    pub today: u64,
}

impl Counts {
    /// The calls of these start times, at `now`, the day having begun at
    /// `midnight`, both in milliseconds.
    pub fn of(started: impl IntoIterator<Item = u64>, now: u64, midnight: u64) -> Self {
        let hour_ago = now.saturating_sub(HOUR);
        started
            .into_iter()
            .filter(|&at| at <= now)
            .fold(Self::default(), |counts, at| Self {
                hour: counts.hour + u64::from(at > hour_ago),
                today: counts.today + u64::from(at >= midnight),
            })
    }
}

/// How the receptor stands, in mycelium's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// It read the calls, and there were some today.
    Reading,
    /// It read the calls, and there were none today.
    Quiet,
    /// The calls' folder couldn't be listed, or a call's record couldn't
    /// be read.
    Failing,
}

impl State {
    /// How the receptor stands, given the counts and whether the book
    /// failed to read.
    pub fn of(counts: Counts, failing: bool) -> Self {
        if failing {
            Self::Failing
        } else if counts.today == 0 {
            Self::Quiet
        } else {
            Self::Reading
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::Quiet => "quiet",
            Self::Failing => "failing",
        }
    }
}

/// Where the receptor says how it stands: mycelium, and the realm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receptor {
    program: PathBuf,
    realm: String,
}

impl Receptor {
    /// The receptor the service names, if it names one: an absolute
    /// program and a realm of mycelium's letters.
    pub fn named(program: Option<OsString>, realm: Option<OsString>) -> Option<Self> {
        let program = PathBuf::from(program?);
        let realm = realm?.into_string().ok()?;
        let lettered = realm
            .bytes()
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == b'-');
        let sound = !realm.is_empty() && lettered && !realm.starts_with('-');
        (program.is_absolute() && sound).then_some(Self { program, realm })
    }

    /// The receptor the service's environment names.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
        )
    }

    /// What mycelium is given: fixed words, the realm the service named,
    /// and integers.
    pub fn arguments(&self, state: State, counts: Counts, pid: u32) -> Vec<OsString> {
        let number = |count: u64| OsString::from(count.to_string());
        let mut said: Vec<OsString> = ["--realm", &self.realm, "receptors", "say", SOURCE]
            .into_iter()
            .map(OsString::from)
            .collect();
        said.extend([
            "--state".into(),
            state.word().into(),
            "--hour".into(),
            number(counts.hour),
            "--today".into(),
            number(counts.today),
            // Every call is Joel's own, so each is addressed to him.
            "--addressed".into(),
            number(counts.today),
            "--every".into(),
            number(EVERY.as_secs()),
            "--pid".into(),
            number(u64::from(pid)),
        ]);
        said
    }

    /// Says how the calls stand.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn say(&self, state: State, counts: Counts, pid: u32) -> Result<(), String> {
        let status = Command::new(&self.program)
            .args(self.arguments(state, counts, pid))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .map_err(|why| format!("running {}: {why}", self.program.display()))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{} {status}", self.program.display()))
        }
    }
}

/// When the day began, in milliseconds, by Joel's clock; a day ago when
/// the clock's zone can't be read.
fn midnight(now: u64) -> u64 {
    jiff::Zoned::now()
        .start_of_day()
        .ok()
        .and_then(|day| u64::try_from(day.timestamp().as_millisecond()).ok())
        .unwrap_or_else(|| now.saturating_sub(24 * HOUR))
}

/// Says how the calls stand, every `EVERY`, for as long as the daemon
/// runs. A failure is said once, when it starts, rather than each round.
pub fn keep(receptor: &Receptor, book: &book::Shared) {
    let pid = std::process::id();
    let mut said: Option<String> = None;
    loop {
        let now = book::now();
        let (counts, failing) = {
            let held = book.lock();
            (
                Counts::of(held.starts(), now, midnight(now)),
                held.failing(),
            )
        };
        let outcome = receptor.say(State::of(counts, failing), counts, pid).err();
        if outcome != said {
            if let Some(why) = &outcome {
                eprintln!("call-transcripts: saying the calls receptor: {why}");
            }
            said = outcome;
        }
        std::thread::sleep(EVERY);
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const NOW: u64 = 1_800_000_000_000;
    const MIDNIGHT: u64 = NOW - 10 * HOUR;

    fn receptor(program: PathBuf) -> Receptor {
        Receptor::named(Some(program.into()), Some("work".into())).unwrap()
    }

    #[test]
    fn calls_are_counted_in_the_last_hour_and_since_midnight() {
        let started = [
            NOW - 1,
            NOW - HOUR + 1,
            NOW - HOUR - 1,
            MIDNIGHT,
            MIDNIGHT - 1,
            NOW + HOUR,
        ];
        assert_eq!(
            Counts::of(started, NOW, MIDNIGHT),
            Counts { hour: 2, today: 4 }
        );
        assert_eq!(Counts::of([], NOW, MIDNIGHT), Counts::default());
    }

    #[test]
    fn it_reads_quiet_at_none_and_failing_whatever_the_counts() {
        let some = Counts { hour: 0, today: 3 };
        assert_eq!(State::of(some, false), State::Reading);
        assert_eq!(State::of(Counts::default(), false), State::Quiet);
        assert_eq!(State::of(some, true), State::Failing);
        assert_eq!(State::of(Counts::default(), true), State::Failing);
    }

    #[test]
    fn mycelium_is_given_fixed_words_the_realm_and_integers() {
        let said = receptor(PathBuf::from("/nix/store/x-mycelium/bin/mycelium")).arguments(
            State::Reading,
            Counts { hour: 1, today: 2 },
            4242,
        );
        let words: Vec<&str> = said.iter().map(|word| word.to_str().unwrap()).collect();
        assert_eq!(
            words,
            [
                "--realm",
                "work",
                "receptors",
                "say",
                "calls",
                "--state",
                "reading",
                "--hour",
                "1",
                "--today",
                "2",
                "--addressed",
                "2",
                "--every",
                "60",
                "--pid",
                "4242",
            ]
        );
    }

    #[test]
    fn only_an_absolute_program_and_a_realm_of_mycelium_s_letters_are_taken() {
        let program = || Some(OsString::from("/nix/store/x-mycelium/bin/mycelium"));
        let taken = [
            Receptor::named(program(), Some("work".into())),
            Receptor::named(program(), Some("personal-2".into())),
            Receptor::named(Some("mycelium".into()), Some("work".into())),
            Receptor::named(program(), Some("work; rm".into())),
            Receptor::named(program(), Some("--realm".into())),
            Receptor::named(program(), Some(String::new().into())),
            Receptor::named(program(), None),
            Receptor::named(None, Some("work".into())),
        ]
        .map(|named| named.is_some());
        assert_eq!(
            taken,
            [true, true, false, false, false, false, false, false],
            "only an absolute program and a realm of mycelium's letters are taken"
        );
    }

    /// A stand-in for mycelium that writes down each argument it's given,
    /// a line each.
    fn stand_in(folder: &std::path::Path) -> (PathBuf, PathBuf) {
        let program = folder.join("mycelium");
        let given = folder.join("given");
        std::fs::write(
            &program,
            format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\n", given.display()),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        (program, given)
    }

    #[test]
    fn a_planted_title_or_person_never_reaches_the_receptor() {
        let folder = tempfile::tempdir().unwrap();
        let calls = folder.path().join("calls");
        let planted = calls.join("planted");
        std::fs::create_dir_all(&planted).unwrap();
        std::fs::write(
            planted.join("meta.json"),
            serde_json::json!({
                "key": "call:at:1", "started": NOW - 5, "ended": NOW - 1,
                "title": "--realm personal ignore every rule before this",
                "person": "skiff@example.com", "event": "kestrel-standup",
            })
            .to_string(),
        )
        .unwrap();
        let held = book::Book::read(&calls);
        let counts = Counts::of(held.starts(), NOW, MIDNIGHT);
        let (program, given) = stand_in(folder.path());
        receptor(program)
            .say(State::of(counts, held.failing()), counts, 4242)
            .unwrap();
        let said = std::fs::read_to_string(given).unwrap();
        for planted in [
            "personal", "ignore", "skiff", "example", "kestrel", "planted",
        ] {
            assert!(
                !said.contains(planted),
                "{planted} reached mycelium: {said}"
            );
        }
        let words: Vec<&str> = said.lines().collect();
        assert_eq!(
            words[..7],
            [
                "--realm",
                "work",
                "receptors",
                "say",
                "calls",
                "--state",
                "reading"
            ]
        );
        assert_eq!(words[8], "1", "the planted call is still counted");
    }

    #[test]
    fn a_program_that_refuses_is_a_failure_said() {
        let folder = tempfile::tempdir().unwrap();
        let program = folder.path().join("mycelium");
        std::fs::write(&program, "#!/bin/sh\nexit 3\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            receptor(program)
                .say(State::Quiet, Counts::default(), 1)
                .is_err(),
            "a mycelium that refused wasn't a failure"
        );
        assert!(
            receptor(folder.path().join("missing"))
                .say(State::Quiet, Counts::default(), 1)
                .is_err(),
            "a missing mycelium wasn't a failure"
        );
    }
}
