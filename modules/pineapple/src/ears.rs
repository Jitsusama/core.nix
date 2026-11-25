//! Where the Slack reader hears Slack: Slack's own API with Joel's
//! sign-in, or a tool gateway that reads Slack for him.
//!
//! A round asks only two things of either, a search for the newest
//! messages and what the message starting a thread says, so the reader
//! judges the same whichever it hears through. What either couldn't be
//! asked is said as the reader counts it: a sign-in refused, none to give,
//! or a Slack that couldn't be asked this round.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use slack_people::{Account, Slack};

use crate::deaf::{self, Deafness, Round};
use crate::gateway;
use crate::mcp::{Called, Gateway};
use crate::slack::{self, Message};

/// How many of the newest messages each search asks for.
pub const COUNT: u8 = 20;

/// How far before the last read a full answer's oldest message must lie,
/// in microseconds: Slack's search can find a message a while after a
/// newer one, so one said just before the last read may not have been
/// found by it.
const FOUND_LATE: u128 = 60 * 1_000_000;

/// How many times one search asks again for fewer after an answer cut
/// short, before it's said as one that can't be read.
const CUTS: usize = 4;

/// How many pages, each before the oldest message of the one after it, a
/// search reads back through to the last read before what's left is a
/// gap: five of twenty is a hundred messages since.
const PAGES: usize = 5;

/// The round in a row whose unanswered later page makes a search's gap,
/// the nine before it each counted neither way: at the reader's 30
/// seconds about five minutes when the page fails at once, and up to
/// three times that when each ask waits out the gateway's 30 seconds, so
/// it's counted in rounds; Joel's row comes the few rounds after that any
/// gap's does. What that page showed
/// before it failed is still judged for a stop, and only Joel's warning
/// waits, while a gateway shedding a burst of pages has the time to
/// answer them again. A round whose first page goes unanswered neither
/// counts nor ends the row; one that reads back to the last read, or on
/// past the gap, does.
const UNANSWERED: u32 = 10;

/// Why a search's pages don't reach back to the last read.
enum Short {
    /// A gap: what lies before the pages read is left unread.
    Gap(String),
    /// A later page went unanswered, which may be gone next round.
    Unanswered(String),
}

/// What a reader hears Slack through.
pub trait Ears {
    /// The newest messages `query` finds, oldest first; else whether the
    /// sign-in was refused, missing, or Slack couldn't be asked now.
    /// `back_to`, a Slack timestamp, is how far back of now an answer must
    /// reach for nothing it could find since the last read to go unread,
    /// which a gateway's answers are held to.
    ///
    /// # Errors
    ///
    /// What the round counts as: see [`Round`].
    fn search(&mut self, query: &str, back_to: &str) -> Result<Vec<Message>, Round>;

    /// What the message starting the thread `thread` in `channel` says,
    /// or none when it can't be read.
    fn root(&mut self, channel: &str, thread: &str) -> Option<String>;

    /// What the last search for `query` gave, though it couldn't be read:
    /// none unless these ears can give part of an answer.
    fn unread(&mut self, _query: &str) -> Unread {
        Unread::default()
    }

    /// Whether the search for `query` waits on a later page it left
    /// unanswered, as one whose first page goes unanswered doesn't: never
    /// unless these ears read in pages.
    fn waiting(&self, _query: &str) -> bool {
        false
    }
}

/// What a search gave that couldn't be read, kept for what it may say.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Unread {
    /// Messages an ask for one fewer agreed with, unread only as they
    /// leave a span before them: each is judged as any read is.
    pub agreed: Vec<Message>,
    /// Messages no second ask agreed with, any of which could be typed:
    /// only a stop among them is taken, as a stop may come from anyone,
    /// and never a resume.
    pub unagreed: Vec<Message>,
}

/// Slack's own API, signed in as Joel.
pub struct Api<'a> {
    pub slack: &'a Slack,
    pub account: &'a Account,
}

impl Ears for Api<'_> {
    /// Slack's own answers are never cut short, so how far back one
    /// reaches is never asked.
    fn search(&mut self, query: &str, _back_to: &str) -> Result<Vec<Message>, Round> {
        let count = COUNT.to_string();
        let asked = [
            ("query", query),
            ("count", count.as_str()),
            ("sort", "timestamp"),
            ("sort_dir", "desc"),
            ("highlight", "false"),
        ];
        let said = match self.slack.read(self.account, "search.messages", &asked) {
            Ok(said) => said,
            Err(why) => {
                eprintln!("<4>couldn't search Slack: {why:?}");
                return Err(Round::Unasked);
            }
        };
        slack::found(&said).map_err(|why| {
            eprintln!("<4>Slack refused a search: {why}");
            if deaf::refuses_the_sign_in(&why) {
                Round::Deaf(Deafness::Refused)
            } else {
                Round::Unasked
            }
        })
    }

    fn root(&mut self, channel: &str, thread: &str) -> Option<String> {
        let asked = [("channel", channel), ("ts", thread), ("limit", "1")];
        match self
            .slack
            .read(self.account, "conversations.replies", &asked)
        {
            Ok(said) => slack::root(&said),
            Err(why) => {
                eprintln!("<4>couldn't read a thread's first message: {why:?}");
                None
            }
        }
    }
}

/// A tool gateway reading Slack for Joel, its answers worded for a model
/// (src/gateway.rs reads them).
///
/// Its answers are prose a message's writer can type, so each is read
/// only when a second ask for one fewer agrees with it: a typed result
/// is numbered "of" one count, so the same text can't pass for both, as
/// it could for one answer the gateway cut short where the real results
/// were dropped. Each is asked for the newest, with no bound after, so
/// one Slack finds late is read as long as it's among them, and without
/// the replies the gateway would add, which are what make it cut an
/// answer short. A full one, or one ever cut, is read only when it
/// reaches back to the last read; one that doesn't is asked for the page
/// before its oldest message, which must hold that message, so none
/// between the two goes unread, up to [`PAGES`] pages. Short of it, or
/// with a page that can't be read, it's deaf until Joel's row says that
/// search can't be read and a row of its own names what it leaves unread,
/// and is then read on past it. A later page that goes unanswered counts
/// neither way, until the [`UNANSWERED`]th round in a row makes it one
/// such gap. What an answer it can't read, or whose page went
/// unanswered, gave is kept for the round ([`Unread`]), so a stop in it
/// isn't lost while the search is deaf or waits.
pub struct Through {
    gateway: Gateway,
    /// How many rounds each search's answer, by its query, has left
    /// unread what was said since the last read.
    gaps: HashMap<String, u32>,
    /// How many rounds in a row each search's later page, by its query,
    /// went unanswered.
    unanswered: HashMap<String, u32>,
    /// The searches, by query, whose gap Joel's row has told him of.
    told: HashSet<String>,
    /// What each search's last answer, by its query, gave that it
    /// couldn't read.
    unread: HashMap<String, Unread>,
    /// Writes the row of its own saying what a search, by its query, is
    /// about to read on past, and says whether it was written: a gap is
    /// read on only once it is, so the last read never moves past a span
    /// untold, through a restart too.
    record: Record,
}

/// Writes the row saying what a search, by its query, reads on past, and
/// says whether it was written.
pub type Record = Box<dyn FnMut(&str, Skipped) -> bool>;

/// What a search read on past, so never read: from a minute before the
/// last read to the oldest message its answer gave, each in microseconds
/// since 1970, or unknown when a stamp couldn't be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// Where what's never read begins.
    pub from: Option<u128>,
    /// Where it ends, from which on all was read.
    pub to: Option<u128>,
}

impl Through {
    /// Reading Slack through `gateway`, with `record` writing the row that
    /// says what a search reads on past, and saying whether it was.
    #[must_use]
    pub fn new(gateway: Gateway, record: Record) -> Through {
        Through {
            gateway,
            gaps: HashMap::new(),
            unanswered: HashMap::new(),
            told: HashSet::new(),
            unread: HashMap::new(),
            record,
        }
    }

    /// Whether the row Joel has, as written, says the search for `query`
    /// can't be read. A gap is read on only once it does, so a round the
    /// other search went unasked in, or a row that failed to write, never
    /// skips a span untold.
    pub fn told(&mut self, query: &str, told: bool) {
        if !told {
            self.told.remove(query);
        } else if self.gaps.contains_key(query) {
            self.told.insert(query.to_owned());
        }
    }

    /// Joel's own Slack id, as the gateway says who it reads as.
    ///
    /// # Errors
    ///
    /// What the round counts as, when it can't say.
    pub fn me(&mut self) -> Result<String, Round> {
        let said = self.called("slack_who_am_i", &json!({}))?;
        let id = inner(&said, "result")
            .lines()
            .find_map(|line| line.strip_prefix("User ID:"))
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty());
        id.ok_or_else(|| {
            eprintln!("<4>the gateway didn't say whose Slack it reads");
            Round::Deaf(Deafness::Unreadable)
        })
    }

    /// The text of the gateway's answer to a search for the `limit` newest
    /// messages `query` finds, said before the second `before` when there
    /// is one, in its detailed form, bots' included, without the replies
    /// it would add after each.
    fn newest(&mut self, query: &str, limit: usize, before: Option<u64>) -> Result<String, Round> {
        let mut asked = json!({
            "query": query,
            "limit": limit,
            "sort": "timestamp",
            "sort_dir": "desc",
            "include_bots": true,
            "include_context": false,
            "response_format": "detailed",
        });
        if let Some(before) = before {
            asked["before"] = json!(before.to_string());
        }
        let said = self.called("slack_search_public_and_private", &asked)?;
        Ok(inner(&said, "results"))
    }

    /// What `tool` says, or what the round counts it as: a failure naming
    /// one of Slack's own refusals of a sign-in is one.
    fn called(&mut self, tool: &str, arguments: &Value) -> Result<String, Round> {
        self.gateway.call(tool, arguments).map_err(|called| {
            let round = match &called {
                Called::Refused => Round::Deaf(Deafness::Refused),
                Called::NoSignIn(_) => Round::Deaf(Deafness::NoSignIn),
                Called::Unreadable(_) => Round::Deaf(Deafness::Unreadable),
                Called::Failed(why)
                    if why
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .any(deaf::refuses_the_sign_in) =>
                {
                    Round::Deaf(Deafness::Refused)
                }
                Called::Unreachable(_) | Called::Failed(_) => Round::Unasked,
            };
            eprintln!("<4>the gateway's {tool} said nothing: {called:?}");
            round
        })
    }

    /// The newest messages a search for `query` finds, read only when an
    /// ask for one fewer agrees, with how many it asked for at the last and
    /// each count it was cut to: cut short, it's asked again for as many as
    /// it gave, as the gateway cuts by size and an ask for as many as it
    /// gave may be cut again.
    fn agreed(
        &mut self,
        query: &str,
        before: Option<u64>,
    ) -> Result<(Vec<Message>, usize, String), Round> {
        // An answer it can't read is no quiet Slack: it isn't heard, and
        // since its cause lasts, it's said as deaf rather than let go.
        let unread = |why: String| {
            eprintln!("<4>the gateway's search for {query:?} can't be read: {why}");
            Round::Deaf(Deafness::Unreadable)
        };
        let count = usize::from(COUNT);
        let mut answer = self.newest(query, count, before)?;
        let mut asked = count;
        let mut cuts = Vec::new();
        // What each answer gave, as one cut by size drops its oldest, so a
        // stop an earlier one gave is kept though a later one cuts it.
        let mut gave = gateway::given(&answer);
        for _ in 0..CUTS {
            let Some(given) = gateway::cut_to(&answer).filter(|given| *given < asked) else {
                break;
            };
            cuts.push(given.to_string());
            asked = given;
            // What it gave before an ask went unanswered is kept for its
            // stops, as an answer that can't be read is.
            answer = match self.newest(query, asked, before) {
                Ok(answer) => answer,
                Err(round) => {
                    gave.sort_by_key(|message| slack::micros(&message.ts));
                    self.unagreed(query, gave);
                    return Err(round);
                }
            };
            for message in gateway::given(&answer) {
                if !gave.contains(&message) {
                    gave.push(message);
                }
            }
        }
        gave.sort_by_key(|message| slack::micros(&message.ts));
        // Still cut once it's asked again as often as it may, it's said so,
        // as a gateway that keeps cutting is no answer short by chance.
        let still = if cuts.len() == CUTS && gateway::cut_to(&answer).is_some() {
            format!("still cut after asking again {CUTS} times, ")
        } else {
            String::new()
        };
        // Each count it was cut to, so a journal shows every cut.
        let cuts = cuts.join(", then ");
        let cut_to = |why: String| cut_to(&cuts, asked, why);
        let found = match gateway::found(&answer) {
            Ok(found) => found,
            Err(why) => {
                self.unagreed(query, gave);
                return Err(unread(cut_to(format!("{still}{why}"))));
            }
        };
        if let Some(fewer) = found.len().checked_sub(1).filter(|fewer| *fewer > 0) {
            let again = match self.newest(query, fewer, before) {
                Ok(again) => again,
                Err(round) => {
                    self.unagreed(query, found);
                    return Err(round);
                }
            };
            let why = match gateway::found(&again) {
                Err(why) => Some(format!("then asked for one fewer, {why}")),
                Ok(again) if again[..] != found[1..] => Some(format!(
                    "then asked for one fewer, it gives {} other than the newest {fewer} of \
                     the {}",
                    again.len(),
                    found.len()
                )),
                Ok(_) => None,
            };
            if let Some(why) = why {
                self.unagreed(query, found);
                return Err(unread(cut_to(why)));
            }
        }
        Ok((found, asked, cuts))
    }

    /// Keeps what the answer for `query` gave, `given`, that no second ask
    /// agreed with.
    fn unagreed(&mut self, query: &str, given: Vec<Message>) {
        let unread = Unread {
            agreed: Vec::new(),
            unagreed: given,
        };
        self.unread.insert(query.to_owned(), unread);
    }

    /// `found`, an answer for `query` that may not hold all it finds and
    /// may not reach back to the last read, `back_to`, with the pages
    /// before its oldest message added, oldest first, and why it still
    /// doesn't reach back to it, if it doesn't: each page must be read and
    /// agreed, hold the oldest message of the one after it, so none between
    /// the two goes unread, and add some older; one unanswered is said as
    /// such. A refused or missing sign-in keeps its own meaning; as the
    /// last read stays put, what the pages before it found is read again
    /// once signed in.
    fn paged(
        &mut self,
        query: &str,
        back_to: &str,
        mut found: Vec<Message>,
    ) -> Result<(Vec<Message>, Option<Short>), Round> {
        for page in 2..=PAGES {
            let Some(oldest) = found.first().cloned() else {
                return Ok((found, None));
            };
            if reaches(&oldest.ts, back_to) {
                return Ok((found, None));
            }
            let Some(before) = slack::micros(&oldest.ts)
                .and_then(seconds)
                .map(|second| second.saturating_add(1))
            else {
                let why = "its oldest message has no time to ask before";
                return Ok((found, Some(Short::Gap(why.into()))));
            };
            // A page that can't be read leaves what's before the pages
            // read unread, as a gap, so the pages agreed are kept and the
            // search is read on once Joel is told; one unanswered may be
            // answered next round ([`UNANSWERED`]).
            let (given, _, cuts) = match self.agreed(query, Some(before)) {
                Ok(page) => page,
                Err(Round::Deaf(Deafness::Unreadable)) => {
                    let why = format!("its page before {before} can't be read");
                    return Ok((found, Some(Short::Gap(why))));
                }
                Err(Round::Unasked) => {
                    let why = format!("its page before {before} goes unanswered");
                    return Ok((found, Some(Short::Unanswered(why))));
                }
                Err(round) => return Err(round),
            };
            let meets = given.contains(&oldest);
            let whole = holds_all(given.len(), &cuts);
            let older: Vec<Message> = given
                .into_iter()
                .filter(|message| !found.contains(message))
                .collect();
            // A page that doesn't meet the one after it leaves unread what
            // was said between the two, though what it gives was agreed, so
            // it's kept to judge beside what's found.
            if !meets {
                let why = format!(
                    "its page before {before} doesn't hold {:?}, the oldest of the one after it",
                    oldest.ts
                );
                self.unread.entry(query.to_owned()).or_default().agreed = older;
                return Ok((found, Some(Short::Gap(why))));
            }
            if older.is_empty() && !whole {
                let why = format!("its page before {before} gives none older");
                return Ok((found, Some(Short::Gap(why))));
            }
            eprintln!(
                "<5>the gateway's search for {query:?} read page {page}, before {before}: {} \
                 more{}",
                older.len(),
                if cuts.is_empty() {
                    String::new()
                } else {
                    format!(", cut to {cuts} of {COUNT}")
                }
            );
            found.extend(older);
            found.sort_by_key(|message| slack::micros(&message.ts));
            if whole {
                return Ok((found, None));
            }
        }
        let reached = found
            .first()
            .is_none_or(|oldest| reaches(&oldest.ts, back_to));
        let why = (!reached).then(|| Short::Gap(format!("{PAGES} pages don't reach it")));
        Ok((found, why))
    }

    /// Whether a search for `query` whose later page went unanswered, as
    /// `short` says, waits for it: counted neither way, with what its
    /// pages gave, `found`, kept for the round to judge, so a stop on its
    /// first page is heard every round. On the [`UNANSWERED`]th round in
    /// a row, or with the search in a gap already, it's a gap, why it's
    /// short in `short`; a round whose pages answer ends the row.
    fn waits(&mut self, query: &str, found: &[Message], short: &mut Option<Short>) -> bool {
        let Some(Short::Unanswered(why)) = short.as_ref() else {
            self.unanswered.remove(query);
            return false;
        };
        // A gap, told or yet to be, is no blink: it stays one until it's
        // read on, so Joel's row isn't held back.
        if self.gaps.contains_key(query) {
            *short = Some(Short::Gap(why.clone()));
            return false;
        }
        let rounds = self.unanswered.entry(query.to_owned()).or_default();
        *rounds = rounds.saturating_add(1);
        let rounds = *rounds;
        if rounds >= UNANSWERED {
            let why = format!("{why}, {rounds} rounds in a row");
            *short = Some(Short::Gap(why));
            return false;
        }
        eprintln!(
            "<4>the gateway's search for {query:?} read its first pages, but {why}, {rounds} of \
             {UNANSWERED} rounds in a row, so what they gave is judged and it's searched from \
             as far back next round"
        );
        let Unread { agreed, unagreed } = self.unread.remove(query).unwrap_or_default();
        let mut agreed: Vec<Message> = found.iter().cloned().chain(agreed).collect();
        agreed.sort_by_key(|message| slack::micros(&message.ts));
        self.unread
            .insert(query.to_owned(), Unread { agreed, unagreed });
        true
    }

    /// That the search for `query` read back to the last read, or on past
    /// its gap: its gap, whether Joel's row told of it, and its row of
    /// unanswered rounds all end.
    fn caught_up(&mut self, query: &str) {
        self.gaps.remove(query);
        self.told.remove(query);
        self.unanswered.remove(query);
    }
}

/// Whether a message said at `oldest` lies a minute before the last read,
/// `back_to`, or more: one said in that minute may be found only later.
fn reaches(oldest: &str, back_to: &str) -> bool {
    slack::micros(oldest)
        .zip(slack::micros(back_to))
        .is_some_and(|(oldest, back_to)| oldest.saturating_add(FOUND_LATE) <= back_to)
}

/// Whether an answer giving `given` messages, cut to `cuts`, holds every
/// message its search finds: fewer than the [`COUNT`] first asked for, and
/// never cut, as one cut and then counting fewer contradicts the count it
/// was cut from.
fn holds_all(given: usize, cuts: &str) -> bool {
    cuts.is_empty() && given < usize::from(COUNT)
}

/// Slack's whole seconds in `micros`, as a gateway's search is bounded by.
fn seconds(micros: u128) -> Option<u64> {
    u64::try_from(micros / 1_000_000).ok()
}

/// Why an answer can't be read, saying first each count it was cut to,
/// `cuts`, and how many it `asked` for at the last, when it was cut.
fn cut_to(cuts: &str, asked: usize, why: String) -> String {
    if cuts.is_empty() {
        why
    } else {
        format!("cut to {cuts} of {COUNT}, then asked for {asked}, {why}")
    }
}

impl Ears for Through {
    fn search(&mut self, query: &str, back_to: &str) -> Result<Vec<Message>, Round> {
        let unread = |why: String| {
            eprintln!("<4>the gateway's search for {query:?} can't be read: {why}");
            Round::Deaf(Deafness::Unreadable)
        };
        self.unread.remove(query);
        let (found, asked, cuts) = self.agreed(query, None)?;
        let cut = !cuts.is_empty();
        let cut_to = |why: String| cut_to(&cuts, asked, why);
        // Fewer than asked for, never cut, is every message it finds. Any
        // other must reach a minute before the last read, even one holding
        // it, or its pages before must.
        let (mut found, mut short) = if holds_all(found.len(), &cuts) {
            (found, None)
        } else {
            self.paged(query, back_to, found)?
        };
        if self.waits(query, &found, &mut short) {
            return Err(Round::Unasked);
        }
        if let Some(Short::Gap(short)) = short {
            let oldest = found.first().map(|message| message.ts.clone());
            let unheard = format!(
                "back to {:?}, it leaves unread what was said from a minute before {back_to} to \
                 then, as {short}",
                oldest.as_deref().unwrap_or_default()
            );
            // What a page that doesn't meet the one after it gave was agreed,
            // so it's judged with the rest, though the span before the oldest
            // of the one after it stays unread; what a page that can't be
            // read gave is kept for its stops.
            let Unread { agreed, unagreed } = self.unread.remove(query).unwrap_or_default();
            found.extend(agreed);
            found.sort_by_key(|message| slack::micros(&message.ts));
            let unheard = cut_to(unheard);
            let gaps = self.gaps.entry(query.to_owned()).or_default();
            *gaps = gaps.saturating_add(1);
            let told = *gaps > deaf::ROUNDS && self.told.contains(query);
            // Read on only once Joel's row says it can't be read and a row
            // of its own names what it leaves unread, as the first goes once
            // Slack is heard, and this span never is.
            let skipped = Skipped {
                from: slack::micros(back_to).map(|at| at.saturating_sub(FOUND_LATE)),
                to: oldest.as_deref().and_then(slack::micros),
            };
            if !told || !(self.record)(query, skipped) {
                let found = Unread {
                    agreed: found,
                    unagreed,
                };
                self.unread.insert(query.to_owned(), found);
                return Err(unread(unheard));
            }
            eprintln!(
                "<4>the gateway's search for {query:?}, {unheard}, told in Joel's row, reads \
                 on: what was said then is never read"
            );
            // What a page that can't be read gave is still judged for its
            // stops in the round that reads on past it.
            let kept = Unread {
                agreed: Vec::new(),
                unagreed,
            };
            self.unread.insert(query.to_owned(), kept);
        } else if found.len() > asked {
            eprintln!(
                "<5>the gateway's search for {query:?} read {} messages in pages, back to the \
                 last read, at {back_to}",
                found.len()
            );
        } else if cut {
            eprintln!(
                "<5>the gateway cut its answer for {query:?} to {cuts} of {COUNT}, so it read \
                 the newest {asked}, back to the last read, at {back_to}"
            );
        }
        self.caught_up(query);
        Ok(found)
    }

    fn root(&mut self, channel: &str, thread: &str) -> Option<String> {
        let said = self
            .called(
                "slack_read_thread",
                &json!({ "channel_id": channel, "message_ts": thread, "limit": 1 }),
            )
            .ok()?;
        crate::gateway::root(&inner(&said, "messages"))
    }

    fn unread(&mut self, query: &str) -> Unread {
        self.unread.remove(query).unwrap_or_default()
    }

    fn waiting(&self, query: &str) -> bool {
        self.unanswered
            .get(query)
            .is_some_and(|rounds| *rounds < UNANSWERED)
    }
}

/// The text a gateway's tool wraps under `key` in a JSON object, or its
/// whole answer when it isn't wrapped.
fn inner(said: &str, key: &str) -> String {
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|value| value.get(key).and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| said.to_owned())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::{Gateway, Short, Skipped, Through, UNANSWERED};

    /// Ears through a gateway nothing asks.
    fn through() -> Through {
        let gateway = Gateway::new("http://127.0.0.1:9", OsStr::new("true"));
        Through::new(gateway, Box::new(|_: &str, _: Skipped| true))
    }

    /// Whether `through` waits on a round its search's later page goes
    /// unanswered.
    fn waits(through: &mut Through) -> bool {
        let mut short = Some(Short::Unanswered(
            "its page before 1 goes unanswered".into(),
        ));
        through.waits("tag", &[], &mut short)
    }

    #[test]
    fn a_page_that_answers_or_a_gap_read_on_starts_the_row_again() {
        let mut through = through();
        for _ in 2..UNANSWERED {
            assert!(waits(&mut through), "eight rounds in a row wait");
        }
        assert!(!through.waits("tag", &[], &mut None), "its pages answer");
        assert!(
            (1..UNANSWERED).all(|_| waits(&mut through)),
            "once its pages answer, nine rounds in a row wait again"
        );
        assert!(!waits(&mut through), "the tenth in a row is a gap");
        through.caught_up("tag");
        assert!(
            waits(&mut through),
            "read on past the gap, the next page unanswered waits again"
        );
    }
}
