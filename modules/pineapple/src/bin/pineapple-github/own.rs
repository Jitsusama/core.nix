//! Joel's own words on GitHub, which GitHub never notifies him of, so his
//! stop where nobody else said anything is found by searching for the
//! word. A search's index can fall behind what was said, by more than a
//! minute and by far more in an incident, so the search asks from an hour
//! before the last whole one, and a stop it indexed that late is still
//! heard. A stop of his that a read finds from the hour before then stops
//! nothing, and the journal says so once a run. Indexed more than an hour
//! late, it's heard only if the search finds its thread by a later change
//! within the hour of it, and told of within two, as a search finds a
//! thread by its last change and reads it back two hours; it must see
//! that change, and the word already indexed there, in time, so on a
//! thread nothing brings back by then it's missed unsaid. A thread run
//! past 30 pages is read without its newest, so a stop there is missed,
//! the journal saying only that the read was cut. How far it reached is
//! kept across a restart (src/progress.rs).

use std::collections::{HashMap, HashSet};

use pineapple::github::{self, Said, Thread, found_when};
use pineapple::progress::Own;
use pineapple::{Joel, Platform, Saying, Verdict, judge};

use super::{GitHub, Unread, searched};

/// How far before the last whole search Joel's own words are searched for
/// and judged, and how far before that a thread found is read, in
/// seconds. A stop heard an hour late beats one never heard,
/// while one older than that he may have moved past. What that finds
/// again is known by its key, so nothing is judged twice.
const OWN_LAG: u64 = 60 * 60;

/// The words that stop, as the journal names them.
const SAFE_WORD: &str = "pineapple override";

/// Where Joel's own words were last searched from whole, which a failed
/// search leaves where it was; when each pull request or issue that
/// search found had last changed, so one is read again only once it has;
/// how far back a first start may judge; and which of his words too old
/// to judge the journal has already named.
pub(super) struct Searched {
    from: u64,
    floor: u64,
    read: HashMap<String, String>,
    told: HashSet<String>,
}

/// What one search found: each pull request or issue to read, when each
/// it named had last changed, and whether it was whole.
pub(super) struct Found {
    pub(super) threads: Vec<Thread>,
    pub(super) seen: HashMap<String, String>,
    pub(super) whole: bool,
}

impl Searched {
    /// Searched from where it was `kept`, or with nothing kept from
    /// `since`, where the reader's rounds begin. A first start judges no
    /// further back than its own look back and a search's lag, so nothing
    /// said long before the reader ran stops anything, and a restart keeps
    /// to that; one with nothing kept of it judges the whole hour, so a
    /// stop said before it is heard.
    pub(super) fn new(since: u64, kept: Option<Own>, first: bool, lag: u64) -> Self {
        let floor = if first { since.saturating_sub(lag) } else { 0 };
        let Own { from, floor } = kept.unwrap_or(Own { from: since, floor });
        Self {
            from,
            floor,
            read: HashMap::new(),
            told: HashSet::new(),
        }
    }

    /// How far it has searched, to keep across a restart.
    pub(super) fn kept(&self) -> Own {
        Own {
            from: self.from,
            floor: self.floor,
        }
    }

    /// The farthest back this round searches for and judges Joel's words.
    pub(super) fn reach(&self) -> u64 {
        self.from.saturating_sub(OWN_LAG).max(self.floor)
    }

    /// How far back a pull request or issue the search found is read: an
    /// hour before what it reaches, for [`Searched::too_old`] to see a stop
    /// a later index lost, rather than its whole life, whose pages would
    /// cost Joel's rate limit on every change to a long thread.
    pub(super) fn read_from(&self) -> u64 {
        self.reach().saturating_sub(OWN_LAG)
    }

    /// The pull requests and issues where Joel takes part that name the
    /// word and changed since they were last read, as a search finds
    /// them. One that fails doesn't cost what notified him: it's searched
    /// from as far back next round.
    pub(super) fn search(&self, github: &GitHub<'_>, site: &str, login: &str) -> Found {
        let query = format!(
            "pineapple involves:{login} updated:>={}",
            github::stamp(self.reach())
        );
        let mut found = Found {
            threads: Vec::new(),
            seen: HashMap::new(),
            whole: true,
        };
        match searched(github, &query) {
            Ok(answer) => {
                // One read since and unchanged holds nothing new.
                for (thread, at) in found_when(&answer, site) {
                    let key = thread.key();
                    if self.read.get(&key) != Some(&at) {
                        found.threads.push(thread);
                    }
                    found.seen.insert(key, at);
                }
            }
            Err(Unread::Gone(why) | Unread::Failed(why)) => {
                // Its reach stays put, so the journal says how far back
                // it has fallen.
                eprintln!(
                    "<4>couldn't search {} for what Joel said, so it's searched from {} \
                     again next round: {why}",
                    github.name,
                    github::stamp(self.reach())
                );
                found.whole = false;
            }
        }
        found
    }

    /// Moves on to a round begun at `began`, once its search was whole.
    pub(super) fn moved_on(&mut self, began: u64, seen: HashMap<String, String>) {
        self.from = began;
        self.read = seen;
    }

    /// Says in the journal, once a run, Joel's own stop a read found from
    /// the hour before what this round reaches, as the judge would take
    /// it, which nothing has judged and nothing will: a later search
    /// reaches no further back. Older ones, as a review read whole may
    /// hold, were judged long since or before the reader kept keys.
    pub(super) fn too_old(&mut self, said: &Said, joel: &Joel, judged: &[String]) {
        let reach = self.reach();
        let stops = || {
            let saying = Saying {
                platform: Platform::Markdown,
                author: &said.author,
                by_bot: said.by_bot,
                text: &said.text,
            };
            let judgement = judge(&saying, joel);
            judgement.by_joel && judgement.verdict == Verdict::Stop
        };
        let unjudged =
            (self.read_from()..reach).contains(&said.at) && !judged.contains(&said.key) && stops();
        if unjudged && self.told.insert(said.key.clone()) {
            eprintln!(
                "<4>{} holds Joel's {SAFE_WORD} from {}, before {}, the farthest back \
                 his words are judged, so it stops nothing",
                said.key,
                github::stamp(said.at),
                github::stamp(reach)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use pineapple::progress::Own;

    use super::{OWN_LAG, Searched};

    /// When the reader's rounds begin in these cases, in seconds.
    const SINCE: u64 = 100_000;

    /// A search's lag, as a first start is given it.
    const LAG: u64 = 600;

    #[test]
    fn a_first_start_reaches_back_only_to_its_look_back() {
        let searched = Searched::new(SINCE, None, true, LAG);
        assert_eq!(searched.reach(), SINCE - LAG, "its look back and a lag");
        assert_eq!(
            searched.read_from(),
            SINCE - LAG - OWN_LAG,
            "what it finds is read from an hour before that"
        );
    }

    #[test]
    fn a_file_with_nothing_kept_reaches_the_whole_hour() {
        let searched = Searched::new(SINCE, None, false, LAG);
        assert_eq!(searched.reach(), SINCE - OWN_LAG, "the whole hour");
        assert_eq!(
            searched.read_from(),
            SINCE - 2 * OWN_LAG,
            "read an hour before"
        );
    }

    #[test]
    fn a_restart_keeps_the_reach_a_failed_search_held_back() {
        let kept = Own {
            from: SINCE - 2 * OWN_LAG,
            floor: 0,
        };
        let searched = Searched::new(SINCE, Some(kept), false, LAG);
        assert_eq!(searched.kept(), kept, "kept as it was");
        assert_eq!(
            searched.reach(),
            SINCE - 3 * OWN_LAG,
            "an hour before the search it held back, not an hour before the rounds begin"
        );
    }

    #[test]
    fn a_kept_floor_holds_back_the_reach() {
        let kept = Own {
            from: SINCE,
            floor: SINCE - LAG,
        };
        let searched = Searched::new(SINCE, Some(kept), false, LAG);
        assert_eq!(
            searched.reach(),
            SINCE - LAG,
            "no further back than its floor"
        );
    }
}
