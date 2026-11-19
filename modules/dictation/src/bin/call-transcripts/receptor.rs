//! The calls receptor: how many of Joel's calls there were, said to the
//! realm's mycelium node, which draws it as a cell on the calls' outside
//! network (mycelium's PROTOCOL.md, Receptors).
//!
//! Counts and fixed words leave, and one kind of pointer. The counts are
//! the calls started in the last hour and since local midnight, taken
//! from the book's own start times and from when each Tuple call Joel
//! missed rang (rings.rs, which reads when alone and never who), and the
//! words say whether the book could be read. A missed
//! ring is a call that asked for Joel as much as one he took, so it
//! counts in each, and every one is addressed to him. Beside them goes
//! the calendar event of each call a summary was written of, by its id
//! alone, so the node can tell the live cells whose quests claim that
//! event (mycelium's PROTOCOL.md, Feeding): a pointer, never the
//! summary. No title, person, quote or folder of a call is ever an
//! argument: of what a call's files say, only its event id reaches the
//! node, and only in the narrow form `is_event` takes. The id and the
//! account it's named by are read from the call's meta.json as the
//! daemon wrote it, trusted as the daemon's, since what could write that
//! file as Joel could run mycelium itself. mycelium is run by
//! its store path, its arguments given one by one and never through a
//! shell, by the speaker every daemon's receptors share
//! (../receptor-say).
//!
//! The service names the program and the realm in `MYCELIUM` and
//! `MYCELIUM_REALM` (modules/call-transcripts/home.nix); with either unset,
//! as on a machine in no realm, nothing is said.
//!
//! A calendar event is the Google account's, so a call's is named only
//! when the account its meeting was on, as kept with the call
//! (meeting.rs), is the one the realm declares in `MYCELIUM_ACCOUNT`, as
//! the google receptor says only that account's counts. A call kept with
//! no account, or another, is never named, and none is with no account
//! declared. Who is signed in now isn't asked: it says nothing of whose
//! a past call was, and an account unheard for a round would take every
//! event away, for the node to feed each claimant again once it came
//! back.
//!
//! Long because its tests stay beside the one thing it does, deciding
//! what of Joel's calls may leave the machine, so each guard is read with
//! the test that would fail without it.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use receptor_say::{Declared, Source, Speaker, Told};

use crate::book;
use crate::rings::{self, Rings};

/// How often the receptor says how it stands, which is also how late the
/// node lets it be before reading it as failing.
pub const EVERY: Duration = Duration::from_secs(60);

/// An hour, in the book's milliseconds.
const HOUR: u64 = 3_600_000;

/// How long after a call ended its summary's event is named: a day short
/// of the 30 mycelium keeps an item, so the node lets an event go only
/// once it's no longer named, rather than keeping one named past its 30
/// days as new and telling its cell again.
const NAMED_FOR: u64 = 29 * 24 * HOUR;

/// The most events named at once: as many items as mycelium keeps.
const MOST_EVENTS: usize = 200;

/// The longest event id named: at this bound the most events still fit
/// the file mycelium reads, where its own bound of 1024 would not, and
/// one longer is left out rather than refusing the whole say.
const LONGEST_EVENT: usize = 200;

/// The fewest characters Google gives an event's own id.
const SHORTEST_EVENT: usize = 5;

/// Whether `event` is an id of Google's own that mycelium takes: its own
/// id in base32hex, lowercase `a` to `v` and digits, then any `_` parts
/// of digits and `R`, `T` or `Z` a recurring event's splits and instances
/// add, as mycelium's feed rule says. One it wouldn't take is left out,
/// since one refused refuses the counts beside it. So is one led by a
/// `_`, though mycelium takes it: Google makes that of an outside
/// invitation's iCalUID, in base32hex anyone can read back, and a UID can
/// carry an organizer's address or host. That closes the import path
/// alone: an id whoever made the event chose through Google's API can be
/// any base32hex, text included, and nothing here tells it apart.
fn is_event(event: &str) -> bool {
    if event.starts_with('_') {
        return false;
    }
    let mut parts = event.split('_');
    let own = parts.next().unwrap_or_default();
    event.len() <= LONGEST_EVENT
        && own.len() >= SHORTEST_EVENT
        && own
            .bytes()
            .all(|each| matches!(each, b'a'..=b'v' | b'0'..=b'9'))
        && parts.all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|each| matches!(each, b'0'..=b'9' | b'R' | b'T' | b'Z'))
        })
}

/// The events to name at `now`, of `summarized` calls as the book gives
/// them, each its event, when it ended and its folder: those that ended
/// within [`NAMED_FOR`] and not after now, whose summary is still in
/// their folder and was written since they last ended, and whose id
/// mycelium takes; the newest first, at most [`MOST_EVENTS`], each once.
fn named<'a>(
    summarized: impl IntoIterator<Item = (&'a str, u64, &'a Path)>,
    now: u64,
) -> Vec<String> {
    let mut held: Vec<(u64, &str)> = summarized
        .into_iter()
        .filter(|&(event, ended, folder)| {
            now.checked_sub(ended).is_some_and(|age| age < NAMED_FOR)
                && is_event(event)
                && written_since(&folder.join(book::SUMMARY), ended)
        })
        .map(|(event, ended, _)| (ended, event))
        .collect();
    held.sort_by(|one, other| other.cmp(one));
    let mut seen = std::collections::BTreeSet::new();
    held.into_iter()
        .filter(|&(_, event)| seen.insert(event))
        .take(MOST_EVENTS)
        .map(|(_, event)| event.to_owned())
        .collect()
}

/// Whether `summary` is a plain file written at or after `ended`, in
/// milliseconds since the epoch. So a call forgotten or moved away stops
/// being named, and a meeting rejoined after it was summed up isn't named
/// again on its first summary, only once the one for its last leg is
/// written. One that can't be looked at for now is as the book has it,
/// since only a summary gone should stop its naming: unnamed for a round,
/// the node would let it go and tell its cell again the next.
fn written_since(summary: &Path, ended: u64) -> bool {
    match std::fs::symlink_metadata(summary) {
        Ok(found) => {
            found.is_file()
                && found
                    .modified()
                    .ok()
                    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|at| u64::try_from(at.as_millis()).ok())
                    .is_some_and(|at| at >= ended)
        }
        Err(why) => !matches!(
            why.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
        ),
    }
}

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
    /// The Google account whose calls' events the realm may be told of.
    account: Option<Declared>,
}

impl Receptor {
    /// The receptor the service names, if it names one: an absolute
    /// program and a realm of mycelium's letters. It names no event
    /// until it's given the realm's account.
    pub fn named(program: Option<OsString>, realm: Option<OsString>) -> Option<Self> {
        Speaker::named(program, realm).map(|speaker| Self {
            speaker,
            account: None,
        })
    }

    /// The receptor, naming the events of the account `account` declares
    /// alone, if it is an address.
    pub fn for_account(self, account: Option<OsString>) -> Self {
        Self {
            account: Declared::address(account),
            ..self
        }
    }

    /// The receptor the service's environment names.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
        )
        .map(|receptor| receptor.for_account(std::env::var_os("MYCELIUM_ACCOUNT")))
    }

    /// Whether a call kept as `account`'s is the realm's to be told of:
    /// only the declared account's, and none with no account declared.
    fn owns(&self, account: &str) -> bool {
        self.account
            .as_ref()
            .is_some_and(|declared| declared.is(account))
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

    /// The words naming `events`, each after `--event`.
    fn events(events: &[String]) -> Vec<OsString> {
        events
            .iter()
            .flat_map(|event| [OsString::from("--event"), OsString::from(event)])
            .collect()
    }

    /// What mycelium is given: fixed words, the realm the service named,
    /// integers, each count held to the most mycelium takes, and the
    /// events named. `say` hands the speaker the same, so only the tests
    /// read it here.
    #[cfg(test)]
    pub fn arguments(
        &self,
        state: State,
        counts: Counts,
        events: &[String],
        pid: u32,
    ) -> Vec<OsString> {
        self.speaker
            .arguments_with(Self::told(state, counts), EVERY, pid, Self::events(events))
    }

    /// Says how the calls stand, and the events of those summarized.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn say(
        &self,
        state: State,
        counts: Counts,
        events: &[String],
        pid: u32,
    ) -> Result<(), String> {
        self.speaker
            .say_with(Self::told(state, counts), EVERY, pid, Self::events(events))
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

/// The events of the calls `summarized` to name at `now`, the book
/// standing as `state`: none unless the calls read whole. A book that
/// couldn't be read whole can't say which calls stand, so it names none,
/// and mycelium, which writes a calls receptor's events only on a reading
/// or quiet say, keeps those it had; on a reading or quiet say naming
/// none, it lets them all go.
fn chosen<'a>(
    state: State,
    summarized: impl IntoIterator<Item = (&'a str, u64, &'a Path)>,
    now: u64,
) -> Vec<String> {
    if matches!(state, State::Reading | State::Quiet) {
        named(summarized, now)
    } else {
        Vec::new()
    }
}

/// What `receptor` says of `book` at `now`, `missed` the Tuple calls
/// counted beside it: how the calls stand, their counts, and the events of
/// the declared account's summarized calls it may name. Nothing but the
/// book and the account kept with each call decides them.
fn round(
    receptor: &Receptor,
    book: &book::Shared,
    missed: &[u64],
    now: u64,
    midnight: u64,
) -> (State, Counts, Vec<String>) {
    let (counts, failing, summarized) = {
        let held = book.lock();
        let summarized: Vec<(String, u64, std::path::PathBuf)> = held
            .summarized(|account| receptor.owns(account))
            .map(|(event, ended, folder)| (event.to_owned(), ended, folder.to_owned()))
            .collect();
        (
            counted(&held, missed, now, midnight),
            held.failing(),
            summarized,
        )
    };
    // The summaries are looked for with the book let go, so a slow disk
    // holds up no one reading it.
    let state = State::of(counts, failing);
    let events = chosen(
        state,
        summarized
            .iter()
            .map(|(event, ended, folder)| (event.as_str(), *ended, folder.as_path())),
        now,
    );
    (state, counts, events)
}

/// Says how the calls stand, every `EVERY`, for as long as the daemon
/// runs. A failure is said once, when it starts, rather than each round,
/// and so is a Tuple file that can't be counted from, and its recovery.
pub fn keep(receptor: &Receptor, book: &book::Shared) {
    let pid = std::process::id();
    if receptor.account.is_none() {
        eprintln!(
            "call-transcripts: no Google account is declared as an address, so no call's event \
             is named"
        );
    }
    // The book reads failing for the daemon's life once it has, so this is
    // said once, here, rather than left for a reader to work out.
    if book.lock().failing() {
        eprintln!(
            "call-transcripts: the calls' folder, or a call's record in it, couldn't be read \
             when this started, so no call's event is named until a start reads them all"
        );
    }
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
        let (missed, logged) = following.round(&rang, midnight, now);
        if let Some(line) = logged {
            eprintln!("call-transcripts: {line}");
        }
        let (state, counts, events) = round(receptor, book, &missed, now, midnight);
        let outcome = receptor.say(state, counts, &events, pid).err();
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
            &[],
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
            &[],
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
        said_naming(folder, state, counts, &[])
    }

    /// What a stand-in in `folder` was given when told how the calls
    /// stand and the `events` named, a word a line.
    fn said_naming(
        folder: &std::path::Path,
        state: State,
        counts: Counts,
        events: &[String],
    ) -> String {
        let (program, given) = stand_in(folder);
        let receptor = receptor(program);
        // Another test's child, forked while the stand-in was still open
        // for writing, holds it busy until that child execs, which is
        // moments; so a busy stand-in is run again rather than failed.
        let mut said = receptor.say(state, counts, events, 4242);
        for _ in 0..BUSY_TRIES {
            match &said {
                Err(why) if why.contains("os error 26") => {
                    std::thread::yield_now();
                    said = receptor.say(state, counts, events, 4242);
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
        let (missed, _) = rings::Following::default().round(&rings::read(&recent), MIDNIGHT, NOW);
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
                .say(State::Quiet, Counts::default(), &[], 1)
                .is_err(),
            "a mycelium that refused wasn't a failure"
        );
        assert!(
            receptor(folder.path().join("missing"))
                .say(State::Quiet, Counts::default(), &[], 1)
                .is_err(),
            "a missing mycelium wasn't a failure"
        );
    }

    /// An event's id as Google gives a recurring one's instance.
    const EVENT: &str = "6kq3ehb1c4p0bt2oq1i5u4tq4c_20261118T150000Z";
    /// The account the receptor's realm declares, which a call is kept as
    /// unless it says whose it is.
    const ACCOUNT: &str = "jsama@example.com";

    /// A call kept in `calls` under `name`, with `meta` as its record
    /// and, when `summary` is some, that summary written beside it.
    fn kept(calls: &std::path::Path, name: &str, meta: &serde_json::Value, summary: Option<&str>) {
        let folder = calls.join(name);
        std::fs::create_dir_all(&folder).unwrap();
        // A call is the declared account's unless its meta says whose.
        let mut meta = meta.clone();
        meta.as_object_mut()
            .expect("a call's meta is an object")
            .entry("account")
            .or_insert(serde_json::json!(ACCOUNT));
        std::fs::write(folder.join("meta.json"), meta.to_string()).unwrap();
        if let Some(summary) = summary {
            written(&folder.join(book::SUMMARY), summary, NOW);
        }
    }

    /// `summary` written to `file`, as written at `at`.
    fn written(file: &Path, summary: &str, at: u64) {
        std::fs::write(file, summary).unwrap();
        std::fs::File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + Duration::from_millis(at))
            .unwrap();
    }

    #[test]
    fn only_the_declared_account_s_calls_are_named_whoever_is_signed_in_now() {
        let folder = tempfile::tempdir().unwrap();
        let calls = folder.path().join("calls");
        // The declared account's call, then another account's after
        // signing in as it, then the declared one's again after signing
        // back, then one kept before calls kept their account.
        for (event, ended, account) in [
            ("abcdef1", NOW - 40, serde_json::json!(ACCOUNT)),
            ("abcdef2", NOW - 30, serde_json::json!("skiff@example.com")),
            ("abcdef3", NOW - 20, serde_json::json!("JSama@example.com")),
            ("abcdef4", NOW - 10, serde_json::Value::Null),
        ] {
            kept(
                &calls,
                &format!("2026-11-18 1500 Planning {event}"),
                &serde_json::json!({
                    "key": format!("call:{event}"), "started": NOW - HOUR, "ended": ended,
                    "title": "Planning", "event": event, "account": account,
                }),
                Some("Planned."),
            );
        }
        // The declared account's, but not summarized.
        kept(
            &calls,
            "2026-11-18 1500 Planning abcdef6",
            &serde_json::json!({
                "key": "call:abcdef6", "started": NOW - HOUR, "ended": NOW - 5,
                "title": "Planning", "event": "abcdef6",
            }),
            None,
        );
        let held = book::Shared::new(book::Book::read(&calls));
        // A round as keep runs it, with nothing of Google's asked: who is
        // signed in now can't change what it names.
        let naming = |receptor: &Receptor| round(receptor, &held, &[], NOW, MIDNIGHT);
        let declared = receptor(PathBuf::from("/nowhere/mycelium"))
            .for_account(Some("jsama@example.com".into()));
        let (state, _, events) = naming(&declared);
        assert_eq!(state, State::Reading);
        assert_eq!(
            events,
            ["abcdef3", "abcdef1"],
            "only the declared account's summarized calls, as Google spells it, are named"
        );
        let undeclared = receptor(PathBuf::from("/nowhere/mycelium"));
        assert!(
            naming(&undeclared).2.is_empty(),
            "a realm that declares no account was told an event"
        );
        let unsound = receptor(PathBuf::from("/nowhere/mycelium")).for_account(Some("".into()));
        assert!(
            naming(&unsound).2.is_empty(),
            "an account that is no address was taken"
        );
        let first = calls.join("2026-11-18 1500 Planning abcdef1");
        let choosing = |state: State| chosen(state, [("abcdef1", NOW - 40, first.as_path())], NOW);
        assert_eq!(
            choosing(State::Quiet),
            ["abcdef1"],
            "a quiet book named nothing"
        );
        assert!(
            choosing(State::Failing).is_empty(),
            "a book that couldn't be read whole named an event"
        );
    }

    #[test]
    fn a_summarized_call_is_named_by_its_event_alone() {
        let folder = tempfile::tempdir().unwrap();
        let calls = folder.path().join("calls");
        kept(
            &calls,
            "2026-11-18 1500 Pay review with Skiff",
            &serde_json::json!({
                "key": format!("call:{EVENT}"), "started": NOW - HOUR, "ended": NOW - 5,
                "title": "Pay review with Skiff", "person": "skiff@example.com",
                "event": EVENT,
            }),
            Some("Skiff said: --realm personal, raise the budget"),
        );
        let held = book::Book::read(&calls);
        let events = named(held.summarized(|account| account == ACCOUNT), NOW);
        assert_eq!(events, [EVENT], "the summarized call's event wasn't named");
        let counts = Counts::of(held.starts(), NOW, MIDNIGHT);
        let said = said_naming(
            folder.path(),
            State::of(counts, held.failing()),
            counts,
            &events,
        );
        for planted in [
            "Pay", "review", "Skiff", "skiff", "example", "personal", "budget", "with",
        ] {
            assert!(
                !said.contains(planted),
                "{planted} reached mycelium: {said}"
            );
        }
        let words: Vec<&str> = said.lines().collect();
        assert_eq!(
            words[words.len() - 2..],
            ["--event", EVENT],
            "the event wasn't said after the counts"
        );
    }

    #[test]
    fn a_call_with_no_event_no_summary_or_not_ended_names_nothing() {
        let folder = tempfile::tempdir().unwrap();
        let calls = folder.path().join("calls");
        let meta = |key: &str, event: Option<&str>, ended: Option<u64>| {
            serde_json::json!({"key": key, "started": NOW - HOUR, "ended": ended,
                "event": event})
        };
        kept(
            &calls,
            "no-event",
            &meta("call:at:1", None, Some(NOW - 5)),
            Some("a"),
        );
        kept(
            &calls,
            "no-summary",
            &meta("call:abcdef1", Some("abcdef1"), Some(NOW - 5)),
            None,
        );
        let held = book::Book::read(&calls);
        assert!(
            named(held.summarized(|account| account == ACCOUNT), NOW).is_empty(),
            "a call with no event or no summary was named"
        );
        let unended = [("abcdef2", NOW + 1, calls.join("no-event"))];
        assert!(
            named(unended.iter().map(|(e, at, f)| (*e, *at, f.as_path())), NOW).is_empty(),
            "a call ending after now was named"
        );
    }

    #[test]
    fn an_event_is_named_while_its_summary_stands_within_its_days_and_only_as_an_id() {
        let folder = tempfile::tempdir().unwrap();
        let with = folder.path().join("with");
        std::fs::create_dir_all(&with).unwrap();
        written(&with.join(book::SUMMARY), "kept", NOW);
        // A meeting rejoined after its first summary: it last ended
        // after that summary was written.
        let rejoined = folder.path().join("rejoined");
        std::fs::create_dir_all(&rejoined).unwrap();
        written(&rejoined.join(book::SUMMARY), "first leg", NOW - 10);
        let gone = folder.path().join("gone");
        let linked = folder.path().join("linked");
        std::fs::create_dir_all(&linked).unwrap();
        std::os::unix::fs::symlink(with.join(book::SUMMARY), linked.join(book::SUMMARY)).unwrap();
        let held = [
            (EVENT, NOW - 5, with.as_path()),
            ("abcdef1", NOW - 5, gone.as_path()),
            ("abcdef2", NOW - 5, linked.as_path()),
            ("abcdef3", NOW - NAMED_FOR, with.as_path()),
            ("abcdef4", NOW - NAMED_FOR + 1, with.as_path()),
            ("abcdef5", NOW - 5, rejoined.as_path()),
            ("abcdef6", NOW - 10, rejoined.as_path()),
            ("Pay review", NOW - 5, with.as_path()),
            ("pay_review", NOW - 5, with.as_path()),
            (EVENT, NOW - 9, with.as_path()),
        ];
        assert_eq!(
            named(held, NOW),
            [EVENT, "abcdef6", "abcdef4"],
            "a summary gone, linked or from before the last end, one past its days, a title \
             or a repeat was named"
        );
    }

    #[test]
    fn an_event_whose_summary_cant_be_looked_at_for_now_stays_named() {
        let folder = tempfile::tempdir().unwrap();
        let shut = folder.path().join("shut");
        std::fs::create_dir_all(&shut).unwrap();
        written(&shut.join(book::SUMMARY), "kept", NOW);
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o000)).unwrap();
        let events = named([(EVENT, NOW - 5, shut.as_path())], NOW);
        std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            events,
            [EVENT],
            "a summary that couldn't be looked at unnamed it"
        );
    }

    #[test]
    fn only_an_id_mycelium_takes_is_named() {
        let folder = tempfile::tempdir().unwrap();
        written(&folder.path().join(book::SUMMARY), "kept", NOW);
        let longest = "a".repeat(LONGEST_EVENT);
        for taken in [
            longest.as_str(),
            "abcde",
            "abc123def_R20261101T150000_20261118T150000Z",
        ] {
            assert_eq!(
                named([(taken, NOW - 5, folder.path())], NOW),
                [taken],
                "{taken} wasn't named"
            );
        }
        let long = "a".repeat(LONGEST_EVENT + 1);
        for refused in [
            long.as_str(),
            "",
            "abcd",
            "_abcd",
            "__abcdef",
            "_6tlnaqrle5p6cpb4dhmj4phpehk62qbd5l9mqr39dpq7arbg",
            "_6tlnaqrle5p6cpb4dhmj4phpe_20261118T150000Z",
            "abcdefw",
            "Abcdef",
            "abcdef_",
            "abcdef_x",
            "abcdef__20261118",
            "abc-def",
            "--realm",
        ] {
            assert!(
                named([(refused, NOW - 5, folder.path())], NOW).is_empty(),
                "{refused:?} was named"
            );
        }
    }

    #[test]
    fn the_newest_events_are_named_and_no_more_than_mycelium_keeps() {
        let folder = tempfile::tempdir().unwrap();
        written(&folder.path().join(book::SUMMARY), "kept", NOW);
        let ids: Vec<String> = (0..=MOST_EVENTS)
            .map(|each| format!("abcde{each:04}"))
            .collect();
        let held = ids.iter().enumerate().map(|(at, id)| {
            let at = u64::try_from(at).unwrap();
            (id.as_str(), NOW - 10_000 + at, folder.path())
        });
        let events = named(held, NOW);
        assert_eq!(
            events.len(),
            MOST_EVENTS,
            "more events were named than kept"
        );
        assert_eq!(events[0], ids[MOST_EVENTS], "the newest wasn't first");
        assert!(
            !events.contains(&ids[0]),
            "the oldest was named over a newer one"
        );
        // The newest named twice, beside itself, still takes one place,
        // so a repeat is dropped before the most are counted.
        let held = ids
            .iter()
            .enumerate()
            .map(|(at, id)| {
                let at = u64::try_from(at).unwrap();
                (id.as_str(), NOW - 10_000 + at, folder.path())
            })
            .chain([(ids[MOST_EVENTS].as_str(), NOW - 9_800, folder.path())]);
        let events = named(held, NOW);
        assert_eq!(events.len(), MOST_EVENTS, "a repeat took a place");
        assert_eq!(
            events[MOST_EVENTS - 1],
            ids[1],
            "a repeat pushed out a call"
        );
    }
}
