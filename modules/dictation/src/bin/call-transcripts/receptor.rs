//! The calls receptor: how many of Joel's calls there were, said to the
//! realm's mycelium node, which draws it as a cell on the calls' outside
//! network (mycelium's PROTOCOL.md, Receptors).
//!
//! Only counts and fixed words leave: the calls started in the last hour
//! and since local midnight, taken from the book's own start times and
//! from when each Tuple call Joel missed rang (rings.rs, which reads when
//! alone and never who), and whether the book could be read. A missed
//! ring is a call that asked for Joel as much as one he took, so it
//! counts in each, and every one is addressed to him. No title, person,
//! event or folder of a call is ever an argument, so nothing a call's
//! files say reaches the node, whatever they hold. mycelium is run by its store path, its
//! arguments given one by one and never through a shell, by the speaker
//! every daemon's receptors share (../receptor-say).
//!
//! The service names the program and the realm in `MYCELIUM` and
//! `MYCELIUM_REALM` (modules/call-transcripts/home.nix); with either unset,
//! as on a machine in no realm, nothing is said.

use std::ffi::OsString;
use std::time::Duration;

use receptor_say::{Source, Speaker, Told};

use crate::book;
use crate::rings::{self, Rings};

/// How often the receptor says how it stands, which is also how late the
/// node lets it be before reading it as failing.
pub const EVERY: Duration = Duration::from_secs(60);

/// An hour, in the book's milliseconds.
const HOUR: u64 = 3_600_000;

/// How many calls started in the last hour, and since local midnight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub hour: u64,
    pub today: u64,
}

impl Counts {
    /// The calls of these start times, at `now`, the day having begun at
    /// `midnight`, both in milliseconds. The hour counts only today's,
    /// since mycelium refuses an hour past the day, as a call from the
    /// last hour of yesterday would make it just after midnight.
    pub fn of(started: impl IntoIterator<Item = u64>, now: u64, midnight: u64) -> Self {
        let hour_ago = now.saturating_sub(HOUR);
        started
            .into_iter()
            .filter(|&at| at <= now && at >= midnight)
            .fold(Self::default(), |counts, at| Self {
                hour: counts.hour + u64::from(at > hour_ago),
                today: counts.today + 1,
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

    /// The state, in mycelium's words.
    fn said(self) -> receptor_say::State {
        match self {
            Self::Reading => receptor_say::State::Reading,
            Self::Quiet => receptor_say::State::Quiet,
            Self::Failing => receptor_say::State::Failing,
        }
    }
}

/// Where the receptor says how it stands: mycelium, and the realm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receptor {
    speaker: Speaker,
}

impl Receptor {
    /// The receptor the service names, if it names one: an absolute
    /// program and a realm of mycelium's letters.
    pub fn named(program: Option<OsString>, realm: Option<OsString>) -> Option<Self> {
        Speaker::named(program, realm).map(|speaker| Self { speaker })
    }

    /// The receptor the service's environment names.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
        )
    }

    /// What the calls say: how they stand and their counts, every call
    /// being Joel's own, so each is addressed to him.
    fn told(state: State, counts: Counts) -> Told {
        Told {
            source: Source::Calls,
            state: state.said(),
            hour: counts.hour,
            today: counts.today,
            addressed: counts.today,
        }
    }

    /// What mycelium is given: fixed words, the realm the service named,
    /// and integers, each count held to the most mycelium takes. `say`
    /// hands the speaker the same, so only the tests read it here.
    #[cfg(test)]
    pub fn arguments(&self, state: State, counts: Counts, pid: u32) -> Vec<OsString> {
        self.speaker
            .arguments(Self::told(state, counts), EVERY, pid)
    }

    /// Says how the calls stand.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn say(&self, state: State, counts: Counts, pid: u32) -> Result<(), String> {
        self.speaker.say(&[Self::told(state, counts)], EVERY, pid)
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

/// The calls the book holds and the Tuple calls missed, counted at `now`.
fn counted(held: &book::Book, missed: &[u64], now: u64, midnight: u64) -> Counts {
    Counts::of(held.starts().chain(missed.iter().copied()), now, midnight)
}

/// Says how the calls stand, every `EVERY`, for as long as the daemon
/// runs. A failure is said once, when it starts, rather than each round,
/// and so is a Tuple file that can't be counted from, and its recovery.
pub fn keep(receptor: &Receptor, book: &book::Shared) {
    let pid = std::process::id();
    let tuple = rings::file();
    let mut following = rings::Following::default();
    let mut said: Option<String> = None;
    loop {
        let now = book::now();
        let midnight = midnight(now);
        let rang = tuple.as_deref().map_or(
            Rings::Unread("no absolute folder names where tuple-call keeps its calls"),
            rings::read,
        );
        let (missed, logged) = following.round(&rang, midnight);
        if let Some(line) = logged {
            eprintln!("call-transcripts: {line}");
        }
        let (counts, failing) = {
            let held = book.lock();
            (counted(&held, &missed, now, midnight), held.failing())
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

    use std::path::PathBuf;

    use super::*;

    const NOW: u64 = 1_800_000_000_000;
    const MIDNIGHT: u64 = NOW - 10 * HOUR;

    /// How many times a stand-in another test's fork holds busy is run
    /// again before the test fails.
    const BUSY_TRIES: usize = 1000;

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
    fn a_count_past_what_mycelium_takes_is_said_as_the_most_it_takes() {
        let said = receptor(PathBuf::from("/nix/store/x-mycelium/bin/mycelium")).arguments(
            State::Reading,
            Counts {
                hour: u64::from(u32::MAX) + 1,
                today: u64::MAX,
            },
            4242,
        );
        let counts: Vec<&str> = [8, 10, 12]
            .into_iter()
            .map(|at| said[at].to_str().unwrap())
            .collect();
        assert_eq!(
            counts, ["4294967295"; 3],
            "a count past u32, which mycelium refuses, wasn't held to it"
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

    /// What a stand-in in `folder` was given when told how the calls
    /// stand, a word a line.
    fn said_by(folder: &std::path::Path, state: State, counts: Counts) -> String {
        let (program, given) = stand_in(folder);
        let receptor = receptor(program);
        // Another test's child, forked while the stand-in was still open
        // for writing, holds it busy until that child execs, which is
        // moments; so a busy stand-in is run again rather than failed.
        let mut said = receptor.say(state, counts, 4242);
        for _ in 0..BUSY_TRIES {
            match &said {
                Err(why) if why.contains("os error 26") => {
                    std::thread::yield_now();
                    said = receptor.say(state, counts, 4242);
                }
                _ => break,
            }
        }
        said.unwrap();
        std::fs::read_to_string(given).unwrap()
    }

    #[test]
    fn only_today_s_calls_count_in_the_hour() {
        let midnight = NOW - HOUR / 2;
        assert_eq!(
            Counts::of([midnight - 1, midnight], NOW, midnight),
            Counts { hour: 1, today: 1 },
            "yesterday's call counted in the hour, past what mycelium takes"
        );
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
        let said = said_by(folder.path(), State::of(counts, held.failing()), counts);
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
    fn a_missed_tuple_call_is_counted_and_who_rang_never_reaches_the_receptor() {
        let folder = tempfile::tempdir().unwrap();
        let calls = folder.path().join("calls");
        std::fs::create_dir_all(&calls).unwrap();
        let recent = folder.path().join("recent.jsonl");
        let line = |missed: bool, since: u64| {
            serde_json::json!({
                "people": [{"id": "41", "name": "Skiff Tern", "email": "skiff@example.com"}],
                "since": since, "ended": since + 1, "missed": missed,
                "caller": "--realm personal",
            })
            .to_string()
        };
        std::fs::write(
            &recent,
            [
                line(true, NOW - 5),
                line(true, MIDNIGHT - 1),
                line(false, NOW - 9),
            ]
            .join("\n"),
        )
        .unwrap();
        let held = book::Book::read(&calls);
        let (missed, _) = rings::Following::default().round(&rings::read(&recent), MIDNIGHT);
        let counts = counted(&held, &missed, NOW, MIDNIGHT);
        assert_eq!(
            counts,
            Counts { hour: 1, today: 1 },
            "only today's missed call counts; one answered or yesterday's doesn't"
        );
        let said = said_by(folder.path(), State::of(counts, held.failing()), counts);
        for planted in ["personal", "Skiff", "Tern", "skiff", "example", "41"] {
            assert!(
                !said.contains(planted),
                "{planted} reached mycelium: {said}"
            );
        }
        let words: Vec<&str> = said.lines().collect();
        assert_eq!(
            words[6..13],
            ["reading", "--hour", "1", "--today", "1", "--addressed", "1"],
            "the missed call wasn't said as one addressed to Joel"
        );
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
