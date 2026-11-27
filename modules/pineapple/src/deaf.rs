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
//! journal says it. Through a tool gateway, past [`UNREACHED`] such
//! rounds in a row is told, since nothing else hears Slack while the
//! gateway is out, and only Joel marks it done. An answer that came but can't be read counts as
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
//! read. So it's said first in a row of its own ([`missed()`]), kept a week
//! in a file the reader's stop leaves, until Joel says he has looked. A
//! busy stretch reads on each round, each time past a span of its own, so
//! a search's spans within [`MEET`] of one another are said as one row,
//! as parts of it where a read stretch lies between them. A
//! reader in shadow keeps both rows in its own folder, which For You never
//! reads, so they're its record until it writes where For You reads.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::missed;

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
    /// The tool gateway Slack is heard through left its searches
    /// unanswered more than [`UNREACHED`] rounds in a row: unreached, or
    /// its tool failing or Slack's rate spent behind it.
    Unreached,
}

/// How many rounds in a row a reader over a tool gateway may leave Slack
/// unasked, about five minutes at its pace, before that's a deafness of
/// its own: the gateway is the one way it hears, so an outage of it
/// isn't a blink that a reader beside it covers.
pub const UNREACHED: u32 = 10;

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
    /// Slack couldn't be asked, neither search answered: counted neither
    /// way.
    Unasked,
    /// One search answered while the other blinked, or one waits on a
    /// later page: too little read to count either way, though Slack, or
    /// its gateway, was reached. It names the search left unasked, if one
    /// was and waits on no later page, beside one read whole.
    Partly(Option<Search>),
    /// The search named left unasked, waiting on no later page, beside one
    /// that waits on a later page, its first read: as [`Round::Partly`]
    /// naming it, though neither search was read whole.
    Waiting(Search),
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
    /// Whether Slack is heard through a tool gateway, whose sign-in isn't
    /// Slack's, so a row about it can't send Joel to sign in to Slack.
    gateway: bool,
    /// How many rounds in a row left Slack unasked.
    unasked: u32,
    /// The search left unasked beside one answered, and how many rounds
    /// in a row it was.
    alone: Option<(Search, u32)>,
    /// The search the round just before narrowed an unreached row to;
    /// any other round forgets it.
    turn: Option<Search>,
    /// The search the round just before left unasked beside one read
    /// whole; any other round forgets it.
    before: Option<Search>,
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
            gateway: false,
            unasked: 0,
            alone: None,
            turn: None,
            before: None,
        }
    }

    /// The same, for a reader hearing Slack through a tool gateway.
    #[must_use]
    pub fn through_gateway() -> Deaf {
        Deaf {
            gateway: true,
            ..Deaf::new()
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
        let turn = self.turn.take();
        let before = self.before.take();
        match round {
            Round::Heard => {
                self.rounds = 0;
                self.unasked = 0;
                self.alone = None;
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
                if why != Deafness::Unreached {
                    self.unasked = 0;
                    self.alone = None;
                }
                self.rounds = self.rounds.saturating_add(1);
                let held = match self.held {
                    Some((last, held)) if last == why => held.saturating_add(1),
                    _ => 1,
                };
                self.held = Some((why, held));
                // A sign-in's cause reads neither search, so each counts
                // as heard of nothing it says.
                let whole = matches!(
                    why,
                    Deafness::Refused | Deafness::NoSignIn | Deafness::Unreached
                );
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
                Change::Tell(if self.gateway {
                    waits_on_gateway(why, since, at)
                } else {
                    waits(why, since, at)
                })
            }
            Round::Partly(alone) => {
                let change = self.answered(alone, true, turn, before, at);
                self.before = alone;
                change
            }
            Round::Waiting(alone) => self.answered(Some(alone), false, turn, before, at),
            Round::Unasked if self.gateway => self.none_answered(at),
            Round::Unasked => Change::Keep,
        }
    }

    /// What a round read in part changes, `alone` the search it left
    /// unasked, if one, and `whole` whether the search it answered was read
    /// whole rather than only to a later page. The gateway answered it, with
    /// the sign-in it was given, so a row saying it's unreached, that the
    /// sign-in was refused or that there's none to read is taken away, as
    /// is one saying only the search it read whole can't be read; a
    /// deafness after is a time of its own, told anew even where Joel
    /// marked that row done, as after a round heard whole. A row saying
    /// it's unreached, though, is narrowed to the search still left unasked
    /// rather than taken away, since that search is still unheard, unless
    /// the round just before narrowed it to the other: each was answered.
    /// A row saying both searches can't be read, or one left from before,
    /// by a reader since restarted or an untelling that wasn't written,
    /// goes once this round and the one just before each read whole the
    /// search the other left unasked, as Slack was read whole across the
    /// two. A search read only to a later page may still hold what can't be
    /// read past it, so that round is no turn of the two, either side.
    fn answered(
        &mut self,
        alone: Option<Search>,
        whole: bool,
        turn: Option<Search>,
        before: Option<Search>,
        at: u64,
    ) -> Change {
        if let Some(search) =
            alone.filter(|_| self.gateway && self.told == Some(Deafness::Unreached))
            && turn.is_none_or(|last| last == search)
        {
            let run = match self.alone {
                Some((last, run)) if last == search => run.saturating_add(1),
                _ => 1,
            };
            let change = self.heard(Round::Deaf(Deafness::Unread(search)), at);
            self.alone = Some((search, run));
            self.turn = Some(search);
            return change;
        }
        let turned = whole && alone.zip(before).is_some_and(|(now, then)| now != then);
        let reached = match self.told {
            Some(Deafness::Unread(told)) => whole && alone.is_some_and(|left| left != told),
            Some(Deafness::Unreadable) => turned,
            None => self.left && turned,
            Some(Deafness::Unreached | Deafness::Refused | Deafness::NoSignIn) => true,
        };
        if reached {
            self.left = false;
            self.told = None;
            self.since = None;
            self.rounds = 0;
            self.held = None;
        }
        match self.partly(alone, at) {
            Change::Keep if reached => Change::Untell,
            change => change,
        }
    }

    /// What a round no search was answered in changes, through a gateway:
    /// a run of them is deaf as any cause is, since nothing else hears
    /// Slack while it lasts. A search left unasked alone is left unasked
    /// here too, so its run goes on, and a gateway answering the other
    /// now and then can't keep it untold. Once its partner too has gone
    /// unanswered [`ROUNDS`] in a row, the run only counts: a row saying
    /// the partner is still heard would mislead while nothing answers.
    fn none_answered(&mut self, at: u64) -> Change {
        self.unasked = self.unasked.saturating_add(1);
        if self.unasked > UNREACHED {
            return self.heard(Round::Deaf(Deafness::Unreached), at);
        }
        let Some((search, run)) = self.alone else {
            return Change::Keep;
        };
        if self.unasked >= ROUNDS {
            self.alone = Some((search, run.saturating_add(1)));
            return Change::Keep;
        }
        let unasked = self.unasked;
        let change = self.partly(Some(search), at);
        self.unasked = unasked;
        change
    }

    /// What a round leaving `alone` unasked, if one, changes: it ends a
    /// run of rounds unasked. Through a gateway, one search left unasked
    /// more than [`UNREACHED`] rounds in a row is deaf of its own, since a
    /// stop only it finds goes unheard, so it's told as that search unread
    /// until the row says it. Its partner answered in the run, so it's
    /// never told as both.
    fn partly(&mut self, alone: Option<Search>, at: u64) -> Change {
        self.unasked = 0;
        let run = match (alone, self.alone) {
            (Some(search), Some((last, run))) if search == last => run.saturating_add(1),
            _ => 1,
        };
        self.alone = alone.map(|search| (search, run));
        let Some(search) = alone.filter(|search| self.gateway && !self.tells(*search)) else {
            return Change::Keep;
        };
        if run <= UNREACHED {
            return Change::Keep;
        }
        // Told as a deaf round would be, its run kept for the next.
        let kept = self.alone;
        let change = self.heard(Round::Deaf(Deafness::Unread(search)), at);
        self.alone = kept;
        change
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
        Deafness::Unreached => (
            format!(
                "The tool gateway the safe word hears Slack through isn't answering its \
                 searches, {UNHEARD}"
            ),
            vec![
                "Ask an agent to read the reader's journal, which says why each search went \
                 unanswered: the gateway unreached, its tool failing or Slack's rate spent",
            ],
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

/// What [`waits`] asks, for a reader hearing Slack through a tool gateway:
/// a sign-in refused or missing is the gateway's, which no Slack sign-in
/// mends and no signal says is mended, so it says so, sends Joel to the
/// journal and is marked done by his word alone.
#[must_use]
pub fn waits_on_gateway(why: Deafness, since: u64, at: u64) -> Value {
    let mut told = waits(why, since, at);
    let said = match why {
        Deafness::Refused => "The tool gateway or Slack behind it refused the safe word's sign-in",
        Deafness::NoSignIn => "The safe word has no tool gateway sign-in it can read",
        Deafness::Unreadable | Deafness::Unread(_) | Deafness::Unreached => return told,
    };
    let item = &mut told["items"][0];
    item["why"] = json!(format!(
        "{said}, so a stop said in Slack isn't heard; one said elsewhere still is"
    ));
    item["steps"] = json!([
        "Ask an agent to read the reader's journal, which says why the gateway's sign-in failed"
    ]);
    item["doneWhen"] = json!({"by": "person"});
    told
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

/// What names each row saying what was never read.
const MISSED: &str = "pineapple-slack-missed-";

/// How near one search's spans never read come to be said in one row,
/// as pineapple's `missed` says of every reader's.
pub use crate::missed::MEET;

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

/// The row [`missed()`] makes, or with `whole` false the one saying only
/// parts of its span were never read, as rows met in it left read
/// stretches between them.
fn said(search: Search, from: Option<u64>, to: Option<u64>, at: u64, whole: bool) -> Value {
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
    let span = missed::when(from, to, at);
    missed::row(
        &missed::named(MISSED, name, from, to.unwrap_or(at)),
        "The Safe Word Missed Part of Slack",
        &format!(
            "The safe word never read {what} {unheard} {span}, as more was said than it could \
             read, so a stop said then wasn't heard"
        ),
        step,
        at,
        whole,
    )
}

/// Adds `row`, raised `at` in milliseconds, to the rows in `file` saying
/// what Slack's searches never read, as pineapple's `missed` keeps every
/// reader's: its search's rows whose spans come within [`MEET`] of its own
/// are said with it as one row, whole or in parts. What the row written
/// says is given back, so the journal says what Joel reads.
///
/// # Errors
///
/// Why it couldn't be read or written.
pub fn keep_missed(file: &Path, row: Value, at: u64) -> std::io::Result<String> {
    let phrase = format!("{MISSED}phrase-");
    let search = if row["id"].as_str().is_some_and(|id| id.starts_with(&phrase)) {
        Search::Phrase
    } else {
        Search::Tag
    };
    missed::keep(file, MISSED, row, at, |from, to, whole| {
        said(search, from, Some(to), at, whole)
    })
}
