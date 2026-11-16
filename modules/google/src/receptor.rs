//! The mail and calendar receptors: how much of Joel's mail wants him and
//! how many meetings he has, said to the realm's mycelium node, which
//! draws each as a cell on its source's outside network (mycelium's
//! PROTOCOL.md, Receptors).
//!
//! They're counted from what a look already holds, never asked of Google,
//! so they can't change how often Google is asked. Only counts and fixed
//! words leave: no subject, sender, title, guest or id is ever an
//! argument, so nothing mail or an invitation says reaches the node,
//! whatever it holds. mycelium is run by its store path, its arguments
//! given one by one and never through a shell, by the speaker every
//! daemon's receptors share (../receptor-say).
//!
//! The service names the program and the realm in `MYCELIUM` and
//! `MYCELIUM_REALM` (home.nix); with either unset, as on a machine in no
//! realm, nothing is said. The realm is the service's, never anything an
//! account holds.
//!
//! Only the account the machine declares for its realm, in
//! `MYCELIUM_ACCOUNT`, is said in it: the address Google gives as the
//! primary calendar's id must be that one, so a personal account signed
//! in here is never counted into the work realm. With none declared, or
//! another signed in, no counts are said: receptors said before are said
//! off once, with none, so the map stops drawing what they last read.

use std::ffi::OsString;
use std::time::Duration;

use jiff::Zoned;
use receptor_say::{Declared, Speaker};
/// The sources a receptor reads, and how it stands, in mycelium's words;
/// these read mail and the calendar.
pub use receptor_say::{Source, State as Reading};

use crate::calendar::{Answer, Meeting};
use crate::mail::Mail;
use crate::state::{Standing, State};

/// An hour, in the state's milliseconds.
const HOUR: i64 = 3_600_000;

/// A day, in the state's milliseconds.
const DAY: i64 = 24 * HOUR;

/// The sources these receptors read.
const SOURCES: [Source; 2] = [Source::Mail, Source::Calendar];

/// What one receptor says: its source, how it stands, and its counts.
/// Reading is Google's word for it: Google refused the sign-in, or
/// couldn't be reached, is failing, the counts what it last said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Told {
    pub source: Source,
    pub reading: Reading,
    /// What came or comes within the hour.
    pub hour: u64,
    /// What's today's.
    pub today: u64,
    /// Of today's, what waits on Joel.
    pub addressed: u64,
}

impl Told {
    /// What the shared speaker is told of it.
    fn said(self) -> receptor_say::Told {
        receptor_say::Told {
            source: self.source,
            state: self.reading,
            hour: self.hour,
            today: self.today,
            addressed: self.addressed,
        }
    }
}

/// What each receptor says of `state`, at `now`, the day having begun at
/// `midnight`, both in milliseconds; nothing while no account is signed
/// in.
pub fn told(state: &State, now: i64, midnight: i64) -> Vec<Told> {
    let (hour, today) = mail(&state.mail, now, midnight);
    let (meeting, meetings_today, unanswered) = meetings(&state.meetings, now, midnight);
    let said = |source, hour, today, addressed| {
        reading(state.account, today > 0).map(|reading| Told {
            source,
            reading,
            hour,
            today,
            addressed,
        })
    };
    // All the mail that wants Joel is his own to answer.
    [
        said(Source::Mail, hour, today, today),
        said(Source::Calendar, meeting, meetings_today, unanswered),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// When `now`'s day began, in milliseconds: its local midnight, or a day
/// before it where the zone can't say.
pub fn midnight(now: &Zoned) -> i64 {
    let at = now.timestamp().as_millisecond();
    now.start_of_day()
        .map_or(at - DAY, |day| day.timestamp().as_millisecond())
}

/// The mail that wants Joel: that which came within the hour, and today.
fn mail(held: &[Mail], now: i64, midnight: i64) -> (u64, u64) {
    let came = |since: i64| count(held.iter().filter(|mail| (since..=now).contains(&mail.at)));
    (came(now - HOUR + 1), came(midnight))
}

/// How many there are, as mycelium counts.
fn count<T>(items: impl Iterator<Item = T>) -> u64 {
    u64::try_from(items.count()).unwrap_or(u64::MAX)
}

/// Today's meetings Joel hasn't declined, all-day ones aside: those under
/// way or starting within the hour, all of them, and those he hasn't
/// answered yet.
fn meetings(held: &[Meeting], now: i64, midnight: i64) -> (u64, u64, u64) {
    let today: Vec<&Meeting> = held
        .iter()
        .filter(|meeting| !meeting.all_day && meeting.answer != Some(Answer::No))
        .filter(|meeting| (midnight..midnight + DAY).contains(&meeting.start))
        .collect();
    let soon = today
        .iter()
        .filter(|meeting| meeting.start < now + HOUR && meeting.end > now);
    let unanswered = today
        .iter()
        .filter(|meeting| meeting.answer == Some(Answer::Waiting));
    (count(soon), count(today.iter()), count(unanswered))
}

/// Where the receptors say how they stand: mycelium, and the realm, and
/// the one account they're said for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receptors {
    speaker: Speaker,
    account: Option<Declared>,
}

/// What came of telling the receptors once, or why it failed.
pub type Spoken = Result<Spoke, String>;

/// Whether the receptors fell silent with what `came`, having been said
/// as of what `was`: the declared account gone, or another signed in. The
/// node would draw each as it last was until three of its rounds passed,
/// so they're said off once, at once.
pub fn fell_silent(was: Option<&Spoken>, came: &Spoken) -> bool {
    let silent = |spoken: &Spoken| matches!(spoken, Ok(Spoke::NoAccount | Spoke::NotItsAccount));
    silent(came) && was.is_some_and(|was| !silent(was))
}

/// What each receptor says as it falls silent: off, its counts none.
fn silent() -> Vec<Told> {
    SOURCES
        .map(|source| Told {
            source,
            reading: Reading::Off,
            hour: 0,
            today: 0,
            addressed: 0,
        })
        .to_vec()
}

/// What came of telling the receptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spoke {
    /// Each said how it stands.
    Said,
    /// No account is declared for the realm, so none was said.
    NoAccount,
    /// The account signed in isn't the one declared, or Google didn't say
    /// whose it is, so none was said.
    NotItsAccount,
}

impl Receptors {
    /// The receptors the service names, if it names them: an absolute
    /// program and a realm of mycelium's letters.
    pub fn named(program: Option<OsString>, realm: Option<OsString>) -> Option<Self> {
        Speaker::named(program, realm).map(|speaker| Self {
            speaker,
            account: None,
        })
    }

    /// These receptors, said only for `account`, an address, if it is one.
    #[must_use]
    pub fn for_account(self, account: Option<OsString>) -> Self {
        Self {
            account: Declared::address(account),
            ..self
        }
    }

    /// The receptors the service's environment names.
    pub fn from_env() -> Option<Self> {
        Self::named(
            std::env::var_os("MYCELIUM"),
            std::env::var_os("MYCELIUM_REALM"),
        )
        .map(|receptors| receptors.for_account(std::env::var_os("MYCELIUM_ACCOUNT")))
    }

    /// Says how each receptor stands in `state` at `now`, the next said
    /// within `every`, if the account Google says is `signed_in` is the
    /// one declared.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn tell(
        &self,
        state: &State,
        signed_in: Option<&str>,
        now: &Zoned,
        every: Duration,
        pid: u32,
    ) -> Result<Spoke, String> {
        let Some(account) = &self.account else {
            return Ok(Spoke::NoAccount);
        };
        if !signed_in.is_some_and(|signed_in| account.is(signed_in)) {
            return Ok(Spoke::NotItsAccount);
        }
        let at = now.timestamp().as_millisecond();
        self.say(&told(state, at, midnight(now)), every, pid)?;
        Ok(Spoke::Said)
    }

    /// What mycelium is given for one receptor: fixed words, the realm
    /// the service named, and integers, each count held to the most
    /// mycelium takes, so a count past it is said as that, never refused.
    pub fn arguments(&self, told: Told, every: Duration, pid: u32) -> Vec<OsString> {
        self.speaker.arguments(told.said(), every, pid)
    }

    /// Says each receptor off, its counts none: no account's mail or
    /// meetings, so nothing of the account signed in instead.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told.
    pub fn fall_silent(&self, every: Duration, pid: u32) -> Result<(), String> {
        self.say(&silent(), every, pid)
    }

    /// Says how each receptor stands, the next said within `every`.
    ///
    /// # Errors
    ///
    /// When mycelium can't be run, or refuses what it's told, for any one
    /// of them.
    pub fn say(&self, told: &[Told], every: Duration, pid: u32) -> Result<(), String> {
        let said: Vec<receptor_say::Told> = told.iter().map(|one| one.said()).collect();
        self.speaker.say(&said, every, pid)
    }
}

/// Whether `standing` is one the receptors say anything of, and how.
fn reading(standing: Standing, any: bool) -> Option<Reading> {
    match standing {
        Standing::None => None,
        Standing::SignedOut | Standing::Unreachable => Some(Reading::Failing),
        Standing::Ready if any => Some(Reading::Reading),
        Standing::Ready => Some(Reading::Quiet),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;
    const MIDNIGHT: i64 = NOW - 10 * HOUR;

    fn letter(at: i64) -> Mail {
        Mail {
            id: "m".into(),
            from: String::new(),
            address: String::new(),
            to: String::new(),
            cc: String::new(),
            others: Vec::new(),
            subject: String::new(),
            invitation: None,
            sent: false,
            went: Vec::new(),
            at,
            page: String::new(),
            body: String::new(),
            attachments: Vec::new(),
            done: None,
        }
    }

    fn meeting(start: i64, end: i64, answer: Option<Answer>) -> Meeting {
        Meeting {
            id: "e".into(),
            title: String::new(),
            start,
            end,
            all_day: false,
            repeats: None,
            series: None,
            answer,
            link: None,
            kind: None,
            page: None,
            place: None,
            about: None,
            attachments: Vec::new(),
            dial: None,
            holds: false,
            through: None,
            organizer: None,
            guests: Vec::new(),
            reminders: Vec::new(),
            done: None,
            given: None,
        }
    }

    fn ready() -> State {
        State {
            account: Standing::Ready,
            ..State::default()
        }
    }

    #[test]
    fn mail_is_counted_in_the_last_hour_and_since_midnight() {
        let held: Vec<Mail> = [
            NOW - 1,
            NOW - HOUR + 1,
            NOW - HOUR - 1,
            MIDNIGHT,
            MIDNIGHT - 1,
        ]
        .into_iter()
        .map(letter)
        .collect();
        assert_eq!(mail(&held, NOW, MIDNIGHT), (2, 4));
        assert_eq!(mail(&[], NOW, MIDNIGHT), (0, 0));
    }

    #[test]
    fn todays_meetings_are_counted_but_declined_and_all_day_ones_arent() {
        let mut all_day = meeting(MIDNIGHT, MIDNIGHT + DAY, None);
        all_day.all_day = true;
        let held = [
            // Under way.
            meeting(NOW - HOUR, NOW + 1, Some(Answer::Yes)),
            // Starting within the hour, unanswered.
            meeting(NOW + HOUR - 1, NOW + 2 * HOUR, Some(Answer::Waiting)),
            // Later today, unanswered.
            meeting(NOW + 2 * HOUR, NOW + 3 * HOUR, Some(Answer::Waiting)),
            // Over, this morning; his own.
            meeting(MIDNIGHT + HOUR, MIDNIGHT + 2 * HOUR, None),
            // Declined, under way.
            meeting(NOW - 1, NOW + HOUR, Some(Answer::No)),
            all_day,
            // Tomorrow's, and yesterday's.
            meeting(MIDNIGHT + DAY + HOUR, MIDNIGHT + DAY + 2 * HOUR, None),
            meeting(MIDNIGHT - 2 * HOUR, MIDNIGHT - HOUR, None),
        ];
        assert_eq!(meetings(&held, NOW, MIDNIGHT), (2, 4, 2));
    }

    #[test]
    fn each_source_reads_quiet_at_none_and_failing_once_google_isnt_answering() {
        let mut state = ready();
        let quiet = told(&state, NOW, MIDNIGHT);
        assert_eq!(
            quiet
                .iter()
                .map(|one| (one.source, one.reading))
                .collect::<Vec<_>>(),
            [
                (Source::Mail, Reading::Quiet),
                (Source::Calendar, Reading::Quiet)
            ]
        );
        state.mail = vec![letter(NOW - 1)];
        state.meetings = vec![meeting(NOW + 2 * HOUR, NOW + 3 * HOUR, None)];
        let some = told(&state, NOW, MIDNIGHT);
        assert_eq!(
            some,
            [
                Told {
                    source: Source::Mail,
                    reading: Reading::Reading,
                    hour: 1,
                    today: 1,
                    addressed: 1,
                },
                Told {
                    source: Source::Calendar,
                    reading: Reading::Reading,
                    hour: 0,
                    today: 1,
                    addressed: 0,
                },
            ]
        );
        for standing in [Standing::SignedOut, Standing::Unreachable] {
            state.account = standing;
            assert!(
                told(&state, NOW, MIDNIGHT)
                    .iter()
                    .all(|one| one.reading == Reading::Failing),
                "{standing:?}"
            );
        }
        state.account = Standing::None;
        assert_eq!(told(&state, NOW, MIDNIGHT), []);
    }

    #[test]
    fn mycelium_is_given_fixed_words_the_realm_and_integers() {
        let receptors = Receptors::named(
            Some("/nix/store/x-mycelium/bin/mycelium".into()),
            Some("work".into()),
        )
        .unwrap();
        let said = receptors.arguments(
            Told {
                source: Source::Calendar,
                reading: Reading::Reading,
                hour: 1,
                today: 3,
                addressed: 2,
            },
            Duration::from_secs(120),
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
                "calendar",
                "--state",
                "reading",
                "--hour",
                "1",
                "--today",
                "3",
                "--addressed",
                "2",
                "--every",
                "120",
                "--pid",
                "4242",
            ]
        );
    }

    #[test]
    fn a_count_past_what_mycelium_takes_is_said_as_the_most_it_takes() {
        let receptors = Receptors::named(
            Some("/nix/store/x-mycelium/bin/mycelium".into()),
            Some("work".into()),
        )
        .unwrap();
        let said = receptors.arguments(
            Told {
                source: Source::Mail,
                reading: Reading::Reading,
                hour: u64::from(u32::MAX) + 1,
                today: u64::MAX,
                addressed: u64::from(u32::MAX),
            },
            Duration::from_secs(60),
            4242,
        );
        let counts: Vec<&str> = [8, 10, 12]
            .into_iter()
            .map(|at| said[at].to_str().unwrap())
            .collect();
        assert_eq!(
            counts, ["4294967295"; 3],
            "a count past u32 wasn't held to it"
        );
    }

    #[test]
    fn falling_silent_says_each_source_off_and_counts_nothing() {
        let receptors = Receptors::named(
            Some("/nix/store/x-mycelium/bin/mycelium".into()),
            Some("work".into()),
        )
        .unwrap();
        let said: Vec<Vec<String>> = silent()
            .into_iter()
            .map(|one| {
                receptors
                    .arguments(one, Duration::from_secs(60), 4242)
                    .into_iter()
                    .map(|word| word.into_string().unwrap())
                    .collect()
            })
            .collect();
        let off = |source: &str| {
            [
                "--realm",
                "work",
                "receptors",
                "say",
                source,
                "--state",
                "off",
                "--hour",
                "0",
                "--today",
                "0",
                "--addressed",
                "0",
                "--every",
                "60",
                "--pid",
                "4242",
            ]
            .map(str::to_owned)
            .to_vec()
        };
        assert_eq!(
            said,
            [off("mail"), off("calendar")],
            "falling silent didn't say each source off with no counts"
        );
    }

    #[test]
    fn the_receptors_fall_silent_only_as_they_stop_being_said() {
        let said: Spoken = Ok(Spoke::Said);
        let failed: Spoken = Err("mycelium refused".to_owned());
        let cases = [
            (Some(&said), Ok(Spoke::NotItsAccount), true),
            (Some(&said), Ok(Spoke::NoAccount), true),
            (Some(&failed), Ok(Spoke::NotItsAccount), true),
            (Some(&said), Ok(Spoke::Said), false),
            (Some(&said), Err("gone".to_owned()), false),
            (None, Ok(Spoke::NotItsAccount), false),
            (None, Ok(Spoke::NoAccount), false),
        ];
        for (was, came, silent) in &cases {
            assert_eq!(
                fell_silent(*was, came),
                *silent,
                "after {was:?}, {came:?} was judged wrongly"
            );
        }
        let not_its: Spoken = Ok(Spoke::NotItsAccount);
        assert!(
            !fell_silent(Some(&not_its), &Ok(Spoke::NotItsAccount)),
            "receptors already silent were said off again"
        );
    }

    #[test]
    fn only_an_absolute_program_and_a_realm_of_mycelium_s_letters_are_taken() {
        let program = || Some(OsString::from("/nix/store/x-mycelium/bin/mycelium"));
        let taken = [
            (program(), Some("work".into())),
            (program(), Some("personal-2".into())),
            (Some("mycelium".into()), Some("work".into())),
            (program(), Some("work; rm".into())),
            (program(), Some("--realm".into())),
            (program(), Some(OsString::new())),
            (program(), None),
            (None, Some("work".into())),
        ]
        .map(|(program, realm)| Receptors::named(program, realm).is_some());
        assert_eq!(
            taken,
            [true, true, false, false, false, false, false, false]
        );
    }
}
