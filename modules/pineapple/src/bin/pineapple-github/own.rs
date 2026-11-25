//! Joel's own words on GitHub, which GitHub never notifies him of, so his
//! stop where nobody else said anything is found by searching for the
//! word. A search's index can fall behind what was said, by more than a
//! minute and by far more in an incident, so the search asks from an hour
//! before the last whole one, and a stop it indexed that late is still
//! heard. Words of his naming the safe word that a read finds from before
//! then stop nothing, and the journal says so once a run, so a stop lost
//! to a late index is never lost unsaid.

use std::collections::{HashMap, HashSet};

use pineapple::github::{self, Said, Thread, found_when};

use super::{GitHub, Unread, searched};

/// How far before the last whole search Joel's own words are searched for
/// and read, in seconds. A stop heard an hour late beats one never heard,
/// while one older than that he may have moved past. What that finds
/// again is known by its key, so nothing is judged twice.
pub(super) const OWN_LAG: u64 = 60 * 60;

/// The words that stop, as Joel says them, for telling which of his
/// words a read couldn't judge are worth a line in the journal.
const SAFE_WORD: &str = "pineapple override";

/// Where Joel's own words were last searched from whole, which a failed
/// search leaves where it was; when each pull request or issue that
/// search found had last changed, so one is read again only once it has;
/// how far back a first start may read; and which of his words too old
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
    /// Searched from `since`, where the reader's rounds begin. A first
    /// start reads no further back than its own look back and a search's
    /// lag, so it judges nothing said long before the reader ran; a
    /// restart reads the whole hour, so a stop said before it is heard.
    pub(super) fn new(since: u64, first: bool, lag: u64) -> Self {
        Self {
            from: since,
            floor: if first { since.saturating_sub(lag) } else { 0 },
            read: HashMap::new(),
            told: HashSet::new(),
        }
    }

    /// The farthest back this round searches for and reads Joel's words.
    pub(super) fn reach(&self) -> u64 {
        self.from.saturating_sub(OWN_LAG).max(self.floor)
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
                eprintln!(
                    "<4>couldn't search {} for what Joel said, so it's searched from as \
                     far back next round: {why}",
                    github.name
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

    /// Says in the journal, once a run, Joel's own safe word a read found
    /// from before what this round reaches, nothing has judged, and
    /// nothing will: a later search reaches no further back.
    pub(super) fn too_old(&mut self, said: &Said, login: &str, judged: &[String]) {
        let reach = self.reach();
        let unjudged = said.at < reach
            && said.author.eq_ignore_ascii_case(login)
            && said.text.to_lowercase().contains(SAFE_WORD)
            && !judged.contains(&said.key);
        if unjudged && self.told.insert(said.key.clone()) {
            eprintln!(
                "<4>{} holds Joel's {SAFE_WORD} from {}, before {}, the farthest back \
                 his words are read, so it stops nothing",
                said.key,
                github::stamp(said.at),
                github::stamp(reach)
            );
        }
    }
}
