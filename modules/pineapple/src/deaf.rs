//! Whether the safe word's Slack reader can hear Slack, said to Joel where
//! Slack isn't needed. Once [`ROUNDS`] rounds in a row read nothing, or
//! only one of its two searches, for a reason he can fix or should know
//! of (Slack refused the sign-in, none can be read, or an answer), it
//! writes a file of what waits on him, which For You shows as a row and
//! says as a notice (the shell's WaitingOnYou.qml); a round that reads
//! Slack again takes it away. Its phone alert goes through Slack, so that
//! can't say this: on 17 Nov Slack refused the reader for 25 minutes, and
//! only its journal said so.
//!
//! A round Slack couldn't be asked in, its network down or its rate
//! spent, counts neither way: nothing there is Joel's to do, and the
//! journal says it. An answer that came but can't be read counts as
//! deaf, though no sign-in mends it: its cause lasts (a gateway's
//! wording changed, its answer cut short past what can be asked again,
//! or a message among the newest shaped like its markup), so read as
//! nothing it would look like a quiet Slack. One search of the two that
//! can't be read is told by name, while the other's messages are still
//! judged: a cause only one search meets would otherwise keep both deaf
//! for as long as it lasts. One both searches meet still keeps both
//! deaf.
//!
//! A search a gateway's answers leave a gap in is read on past it once
//! Joel's row says so ([`crate::ears::Through`]), and that span is never
//! read. So it's said first in a row of its own ([`missed`]), kept a week
//! in a file the reader's stop leaves, until Joel says he has looked. A
//! busy stretch reads on each round, each time past a span of its own, so
//! a search's spans within [`MEET`] of one another are said as one row,
//! as parts of it where a read stretch lies between them. A
//! reader in shadow keeps both rows in its own folder, which For You never
//! reads, so they're its record until it writes where For You reads.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// How many rounds in a row are deaf before Joel is told: one is a
/// blip a sign-in renewed mid-round makes, two at the reader's 30 seconds
/// is a minute of stops unheard.
pub const ROUNDS: u32 = 2;

/// Whether Slack's error word refuses the sign-in itself: the slack
/// crate's own list, so this reader counts the words slack-unreads does.
pub use slack_people::refuses_the_sign_in;

/// Why a round read nothing, or only one search, that Joel should know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deafness {
    /// Slack refused the sign-in.
    Refused,
    /// No sign-in could be read: signed out, the keyring locked, or one
    /// that doesn't say who Joel is.
    NoSignIn,
    /// Slack answered in words that can't be read.
    Unreadable,
    /// One search's answer can't be read, so only the other's messages
    /// are judged.
    Unread(Search),
}

/// Each of a round's two searches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    /// For "pineapple", which finds a stop however it's addressed.
    Phrase,
    /// For Joel's tag, which finds someone asking him to stop his agents
    /// without the phrase.
    Tag,
}

/// What a round of the reader came to, as far as hearing goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// Both searches answered.
    Heard,
    /// Nothing was read, or one search wasn't, for a reason Joel can fix.
    Deaf(Deafness),
    /// Slack couldn't be asked: counted neither way.
    Unasked,
}

/// What changes in the file of what waits on Joel.
#[derive(Debug, PartialEq)]
pub enum Change {
    /// Write it, whole.
    Tell(Value),
    /// Take it away.
    Untell,
    /// Leave it.
    Keep,
}

/// How one search has fared, round by round: whether it's deaf, as
/// settled once a state has held [`ROUNDS`] rounds in a row, so a blip
/// either way leaves it as it was.
#[derive(Clone, Copy, Debug, Default)]
struct Fared {
    deaf: bool,
    last: bool,
    run: u32,
}

impl Fared {
    fn round(&mut self, deaf: bool) {
        self.run = if self.last == deaf {
            self.run.saturating_add(1)
        } else {
            1
        };
        self.last = deaf;
        if self.run >= ROUNDS {
            self.deaf = deaf;
        }
    }
}

/// The rounds read nothing in a row, and whether Joel was told, and why.
#[derive(Debug)]
pub struct Deaf {
    rounds: u32,
    /// The last round's why, and how many rounds in a row gave it.
    held: Option<(Deafness, u32)>,
    /// Each search on its own, so one deaf every round is told whatever
    /// its partner does.
    phrase: Fared,
    tag: Fared,
    told: Option<Deafness>,
    /// When this time of not hearing was first told, in milliseconds,
    /// which names its row, so each time is a row For You hasn't seen.
    since: Option<u64>,
    /// Whether a file this program didn't write may still stand, as one
    /// left by the reader before a restart may.
    left: bool,
}

impl Default for Deaf {
    fn default() -> Self {
        Deaf::new()
    }
}

impl Deaf {
    /// Not deaf yet, and what an earlier reader told still to be taken
    /// away once Slack is heard.
    #[must_use]
    pub fn new() -> Deaf {
        Deaf {
            rounds: 0,
            held: None,
            phrase: Fared::default(),
            tag: Fared::default(),
            told: None,
            since: None,
            left: true,
        }
    }

    /// That `change` couldn't be made, so the next round makes it again:
    /// a row not written is told again, one not taken away is taken
    /// away once Slack is heard.
    pub fn failed(&mut self, change: &Change) {
        match change {
            // What an earlier tell wrote may still stand, so it's taken
            // away once Slack is heard even if no tell lands before.
            Change::Tell(_) => {
                self.told = None;
                self.left = true;
            }
            Change::Untell => self.left = true,
            Change::Keep => {}
        }
    }

    /// Whether the row standing tells Joel that `search` can't be read,
    /// written rather than only meant.
    #[must_use]
    pub fn tells(&self, search: Search) -> bool {
        self.told
            .is_some_and(|told| told == Deafness::Unreadable || told == Deafness::Unread(search))
    }

    /// What the searches' own settled states say, when neither sign-in
    /// cause decides the round.
    fn settled(&self) -> Option<Deafness> {
        match (self.phrase.deaf, self.tag.deaf) {
            (true, true) => Some(Deafness::Unreadable),
            (true, false) => Some(Deafness::Unread(Search::Phrase)),
            (false, true) => Some(Deafness::Unread(Search::Tag)),
            (false, false) => None,
        }
    }

    /// What `round`, at `at` in milliseconds, changes. Joel is told once
    /// [`ROUNDS`] in a row are deaf, and again only when what's deaf has
    /// held as many in its new shape, so a blip is no row of its own, and
    /// untold once a round reads Slack whole. Each search is counted on
    /// its own, so one deaf every round is told even while its partner
    /// flickers. A change of why starts a time of its own, so it isn't
    /// hidden under a row Joel marked done.
    pub fn heard(&mut self, round: Round, at: u64) -> Change {
        match round {
            Round::Heard => {
                self.rounds = 0;
                self.held = None;
                self.phrase = Fared::default();
                self.tag = Fared::default();
                self.since = None;
                let left = std::mem::take(&mut self.left);
                if self.told.take().is_some() || left {
                    Change::Untell
                } else {
                    Change::Keep
                }
            }
            Round::Deaf(why) => {
                self.rounds = self.rounds.saturating_add(1);
                let held = match self.held {
                    Some((last, held)) if last == why => held.saturating_add(1),
                    _ => 1,
                };
                self.held = Some((why, held));
                // A sign-in's cause reads neither search, so each counts
                // as heard of nothing it says.
                let whole = matches!(why, Deafness::Refused | Deafness::NoSignIn);
                let covers =
                    |search| why == Deafness::Unreadable || why == Deafness::Unread(search);
                self.phrase.round(!whole && covers(Search::Phrase));
                self.tag.round(!whole && covers(Search::Tag));
                let lasting = if whole {
                    (held >= ROUNDS).then_some(why)
                } else {
                    self.settled()
                };
                let tell = match self.told {
                    None => (self.rounds >= ROUNDS).then(|| lasting.unwrap_or(why)),
                    Some(told) => lasting.filter(|lasting| *lasting != told),
                };
                let Some(why) = tell else {
                    return Change::Keep;
                };
                let changed = self.told.is_some();
                // For You keeps a row Joel marked done gone by name, and
                // a sign-in's signal marks one done whether it mended it
                // or not. So a row told for another why, narrower, wider
                // or other, is a time of its own. A row whose writing
                // failed isn't told, so it's written again as itself.
                if changed {
                    self.since = None;
                }
                self.told = Some(why);
                self.left = false;
                let since = *self.since.get_or_insert(at);
                Change::Tell(waits(why, since, at))
            }
            Round::Unasked => Change::Keep,
        }
    }
}

/// What a deaf reader asks of Joel, in the shape every quest's file of
/// what waits on him takes (`mesh_wait`'s), as slack-unreads' sign-in is:
/// one item he can do now, known done by the signal `signin:slack` given
/// after `since`, when this time of not hearing was first told. The row is
/// named for `since`: For You says an item it has seen once, and keeps
/// one Joel marked done gone, by name, so a later time needs its own.
#[must_use]
pub fn waits(why: Deafness, since: u64, at: u64) -> Value {
    const SIGN_IN: [&str; 3] = [
        "Press Sign In on Slack in Programs",
        "Sign in to Slack in the browser it opens",
        "Or type /slack-auth in a pi session",
    ];
    const UNHEARD: &str = "so a stop said in Slack isn't heard; one said elsewhere still is";
    const READ_JOURNAL: &str =
        "Ask an agent to read the reader's journal, which says what it couldn't read";
    let title = match why {
        Deafness::Unread(_) => "The Safe Word Can't Hear Part of Slack",
        _ => "The Safe Word Can't Hear Slack",
    };
    let (said, steps, done): (String, Vec<&str>, Value) = match why {
        Deafness::Refused => (
            format!("Slack refused the safe word's sign-in, {UNHEARD}"),
            SIGN_IN.to_vec(),
            json!({"signal": "signin:slack"}),
        ),
        Deafness::NoSignIn => (
            format!(
                "The safe word has no Slack sign-in it can read, signed out or with the \
                 keyring locked, {UNHEARD}"
            ),
            ["Unlock your keyring if it asks"]
                .into_iter()
                .chain(SIGN_IN)
                .collect(),
            json!({"signal": "signin:slack"}),
        ),
        // No sign-in mends it, so only Joel's word marks it done.
        Deafness::Unreadable => (
            format!("Slack's answers to the safe word can't be read, {UNHEARD}"),
            vec![READ_JOURNAL],
            json!({"by": "person"}),
        ),
        Deafness::Unread(Search::Phrase) => (
            "The safe word's search of Slack for pineapple can't be read, so a stop that \
             doesn't tag you isn't heard; one that tags you still is"
                .to_owned(),
            vec![READ_JOURNAL],
            json!({"by": "person"}),
        ),
        Deafness::Unread(Search::Tag) => (
            "The safe word's search of Slack for your tag can't be read, so a stop that tags \
             you without saying pineapple isn't heard; one saying it still is"
                .to_owned(),
            vec![READ_JOURNAL],
            json!({"by": "person"}),
        ),
    };
    json!({
        "v": 2,
        "updated": slack_people::messages::iso(at),
        "quest": "pineapple",
        "items": [{
            "id": format!("pineapple-slack-deaf-{since}"),
            "title": title,
            "why": said,
            "raisedAt": slack_people::messages::iso(since),
            "minutes": 1,
            "ready": true,
            "doneWhen": done,
            "steps": steps,
        }]
    })
}

/// Where For You's row is written, and a name passed over for it.
#[derive(Debug, PartialEq, Eq)]
pub struct Waiting {
    /// The file the row goes in.
    pub file: PathBuf,
    /// A name given that isn't a whole path, and so names nothing.
    pub refused: Option<PathBuf>,
}

/// Where For You's row goes: `named`, as the unit names the file its stop
/// removes (home.nix), when that's a whole path; else For You's own under
/// `state_home`. A name that isn't whole is kept, for the journal to say.
#[must_use]
pub fn waiting(named: Option<&OsStr>, state_home: &Path) -> Waiting {
    match named.map(PathBuf::from) {
        Some(named) if named.is_absolute() => Waiting {
            file: named,
            refused: None,
        },
        refused => Waiting {
            file: state_home.join("mission/waiting/pineapple.json"),
            refused,
        },
    }
}

/// Makes `change` to `file`: written in a `.writing` folder beside it
/// and renamed over it, so no reader sees half, or taken away.
///
/// # Errors
///
/// Why it couldn't be written or taken away.
pub fn make(file: &Path, change: &Change) -> std::io::Result<()> {
    match change {
        Change::Tell(waits) => {
            let aside = file
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".writing");
            std::fs::create_dir_all(&aside)?;
            let writing = aside.join(file.file_name().unwrap_or_default());
            std::fs::write(&writing, waits.to_string())?;
            std::fs::rename(&writing, file)
        }
        Change::Untell => match std::fs::remove_file(file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            done => done,
        },
        Change::Keep => Ok(()),
    }
}

/// How long a row saying what was never read is kept after the span it
/// names ends, in milliseconds: a week, long past when Joel would look for
/// a stop said then.
const MISSED_FOR: u64 = 7 * 24 * 60 * 60 * 1000;

/// What names each row saying what was never read.
const MISSED: &str = "pineapple-slack-missed-";

/// How near one search's spans never read come, in milliseconds, to be
/// said in one row: ten minutes, as long as the reader's own look back,
/// as one stretch Joel searches at once. Spans that keep coming that near
/// chain into one row however long the stretch, said as parts once any
/// of it was read; farther apart they're rows of their own.
pub const MEET: u64 = 10 * 60 * 1000;

/// The row saying what `search` read on past is never read, from `from`
/// to `to`, each in milliseconds since 1970 or unknown, as Joel reads it
/// when it's raised, `at`: as minutes before then, so no zone is named.
/// No round reads that span, so only Joel's word marks it done, and it's
/// named for its search and the span, an unknown start as 0 and end as
/// `at`, so each span is a row of its own, the same however often it's
/// written.
#[must_use]
pub fn missed(search: Search, from: Option<u64>, to: Option<u64>, at: u64) -> Value {
    said(search, from, to, at, true)
}

/// The row [`missed`] makes, or with `whole` false the one saying only
/// parts of its span were never read, as rows met in it left read
/// stretches between them.
fn said(search: Search, from: Option<u64>, to: Option<u64>, at: u64, whole: bool) -> Value {
    let ago = |then: u64| at.saturating_sub(then);
    let span = match (
        from.map(|from| ago(from).div_ceil(60_000)),
        to.map(|to| ago(to) / 60_000),
    ) {
        (Some(from), Some(to)) if to > 0 && to < from => {
            format!("from {from} to {} before this was raised", minutes(to))
        }
        (Some(from), _) => format!("in the {} before this was raised", minutes(from)),
        (None, _) => "before this was raised, since when it last read can't be told".to_owned(),
    };
    let (name, unheard, step) = match search {
        Search::Phrase => (
            "phrase",
            "said pineapple in Slack without tagging you",
            "Search Slack for pineapple, for a stop said then",
        ),
        Search::Tag => (
            "tag",
            "tagged you in Slack without saying pineapple",
            "Look in Slack's Activity for a stop that mentioned you then",
        ),
    };
    let what = if whole { "what" } else { "parts of what" };
    json!({
        "id": format!("{MISSED}{name}-{}-{}", from.unwrap_or(0), to.unwrap_or(at)),
        "title": "The Safe Word Missed Part of Slack",
        "why": format!(
            "The safe word never read {what} {unheard} {span}, as more was said than it could \
             read, so a stop said then wasn't heard"
        ),
        "whole": whole,
        "raisedAt": slack_people::messages::iso(at),
        "minutes": 2,
        "ready": true,
        "doneWhen": {"by": "person"},
        "steps": [step],
    })
}

fn minutes(count: u64) -> String {
    if count == 1 {
        "1 minute".to_owned()
    } else {
        format!("{count} minutes")
    }
}

/// Adds `row`, raised `at` in milliseconds, to the rows in `file` saying
/// what was never read, keeping those whose span ended in the
/// [`MISSED_FOR`] before. Its search's rows whose spans come within
/// [`MEET`] of its own are said with it as one row, raised `at`, spanning
/// them all: whole where they overlap, as a gap read on again after its
/// round was thrown away does, and otherwise as parts. Rows that meet
/// only what it spans once it has taken others are taken too, as a file
/// written before rows met holds. That row is named anew, so one Joel
/// said he'd looked at comes back holding what's new; one not made by
/// [`missed`] meets nothing, and one whose start can't be told meets only
/// its own name, so no dated row is lost into it.
///
/// `file` is a file of its own, which the reader's stop doesn't take
/// away as it does the row saying it can't hear, since what it says stays
/// true once the reader hears again or restarts. One not
/// there holds no row; one that can't be read, or holds no rows, is left
/// as it is, so no row it holds is lost.
///
/// # Errors
///
/// Why it couldn't be read or written.
pub fn keep_missed(file: &Path, row: Value, at: u64) -> std::io::Result<()> {
    let unread = |why: String| std::io::Error::new(std::io::ErrorKind::InvalidData, why);
    let kept = match std::fs::read(file) {
        Ok(read) => {
            let kept: Value = serde_json::from_slice(&read)
                .map_err(|error| unread(format!("it holds no rows it can read: {error}")))?;
            if !kept["items"].is_array() {
                return Err(unread("it holds no rows".to_owned()));
            }
            Some(kept)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let recent = |row: &Value| {
        row["id"]
            .as_str()
            .and_then(|id| id.strip_prefix(MISSED))
            .and_then(|named| named.rsplit('-').next())
            .and_then(|raised| raised.parse::<u64>().ok())
            .is_some_and(|raised| raised.saturating_add(MISSED_FOR) >= at)
    };
    let mut rows: Vec<Value> = kept
        .as_ref()
        .and_then(|kept| kept["items"].as_array())
        .into_iter()
        .flatten()
        .filter(|kept| recent(kept))
        .cloned()
        .collect();
    let mut own = span_of(&row);
    let mut met = false;
    if let Some(span) = own.as_mut() {
        loop {
            let before = rows.len();
            rows.retain(|earlier| match span_of(earlier) {
                Some(theirs) if span.meets(&theirs) => {
                    span.take(&theirs);
                    false
                }
                _ => true,
            });
            if rows.len() == before {
                break;
            }
            met = true;
        }
    }
    rows.push(match own {
        Some(span) if met => said(span.search, span.from, Some(span.to), at, span.whole),
        _ => row,
    });
    let file_of = json!({
        "v": 2,
        "updated": slack_people::messages::iso(at),
        "quest": "pineapple",
        "items": rows,
    });
    make(file, &Change::Tell(file_of))
}

/// The span a row saying what was never read names, read back from its
/// name and fields: its search, its start, if known, and end, in
/// milliseconds, and whether all of it was never read.
struct Span {
    search: Search,
    from: Option<u64>,
    to: u64,
    whole: bool,
}

impl Span {
    /// Whether `theirs` is the same search's and comes within [`MEET`];
    /// where either start can't be told, only the same span meets.
    fn meets(&self, theirs: &Span) -> bool {
        self.search == theirs.search
            && match (self.from, theirs.from) {
                (Some(own), Some(other)) => {
                    own <= theirs.to.saturating_add(MEET) && other <= self.to.saturating_add(MEET)
                }
                (None, None) => self.to == theirs.to,
                _ => false,
            }
    }

    /// Spans `theirs` too, staying whole only where both are and overlap.
    fn take(&mut self, theirs: &Span) {
        let overlap = self.from.unwrap_or(0) <= theirs.to && theirs.from.unwrap_or(0) <= self.to;
        self.whole = self.whole && theirs.whole && overlap;
        self.from = self
            .from
            .zip(theirs.from)
            .map(|(own, other)| own.min(other));
        self.to = self.to.max(theirs.to);
    }
}

fn span_of(row: &Value) -> Option<Span> {
    let (name, span) = row["id"].as_str()?.strip_prefix(MISSED)?.split_once('-')?;
    let (from, to) = span.split_once('-')?;
    let search = match name {
        "phrase" => Search::Phrase,
        "tag" => Search::Tag,
        _ => return None,
    };
    let from: u64 = from.parse().ok()?;
    Some(Span {
        search,
        from: (from != 0).then_some(from),
        to: to.parse().ok()?,
        whole: row["whole"].as_bool().unwrap_or(true),
    })
}
