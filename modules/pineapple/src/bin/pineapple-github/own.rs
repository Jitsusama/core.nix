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
//! thread nothing brings back by then it's missed unsaid. GitHub gives a
//! search at most a thousand threads, most lately changed first, so one
//! it cuts short is followed by a search of what changed before the
//! oldest it gave; past two thousand, the oldest are left unread, the
//! journal naming from when to when. A thread given more than 30 pages
//! of comments since an hour before what a read reaches (two hours while
//! its search is whole, longer while a failing one holds it back), or a
//! pull request more than 30 pages of reviews in its life, is read
//! without its newest, so a stop there is missed, the journal saying only
//! that the read was cut. How far it reached, and the spans it holds to
//! tell Joel of, are kept across a restart (src/progress.rs).

use std::collections::{HashMap, HashSet};

use pineapple::github::{self, Said, Thread, found_when};
use pineapple::progress::{Own, Untold};
use pineapple::{Joel, Platform, Saying, Verdict, judge, missed};

use super::{GitHub, Left, Unread, searched};

/// How far before the last whole search Joel's own words are searched for
/// and judged, and how far before that a thread found is read, in
/// seconds. A stop heard an hour late beats one never heard,
/// while one older than that he may have moved past. What that finds
/// again is known by its key, so nothing is judged twice. A thread GitHub
/// wouldn't let a round read is judged from where it was owed (owed.rs),
/// however late: a stop the reader never saw leans to stopping.
const OWN_LAG: u64 = 60 * 60;

/// How many searches a round makes for Joel's words: one cut short at
/// GitHub's thousand is followed by one of what changed before the oldest
/// it gave. Two take in a flood of two thousand in twenty pages, within
/// the thirty a minute his search limit allows; a larger one leaves the
/// oldest unread that round rather than hold every round back, as keeping
/// the reach keeps the flood in every later search. Reading two thousand
/// can still spend the hour's core limit, which fails the round.
const SEARCHES: usize = 2;

/// The words that stop, as the journal names them.
const SAFE_WORD: &str = "pineapple override";

/// How many rounds hold a span the search left out while Joel's row
/// saying so can't be written, each trying it again whether or not its
/// search answered, before the journal alone says so: 360, about three
/// hours at a round each half minute, about as long as a thread whose row
/// can't be written is owed (owed.rs). A site searched by author never
/// searches for his words, so never holds one. A round that stops before
/// it searches, as one that can't ask what changed does, tries none, so
/// holds them longer.
const UNTOLD_ROUNDS: u32 = 360;

/// Where Joel's own words were last searched from whole, which a failed
/// search leaves where it was; when each pull request or issue that
/// search found had last changed, so one is read again only once it has;
/// how far back a first start may judge; which of his words too old to
/// judge the journal has already named; and each stretch its searches
/// left out whose row in For You couldn't yet be written, kept across a
/// restart with how far it searched (src/progress.rs), as a restart after
/// mending the rows file is when it's likeliest to be written.
pub(super) struct Searched {
    from: u64,
    floor: u64,
    read: HashMap<String, String>,
    told: HashSet<String>,
    untold: Vec<Untold>,
}

/// What one search found: each pull request or issue to read, when each
/// it named had last changed, whether the round may move on from it (not
/// when its first search failed, or was cut short giving nothing it could
/// read, which holds as an outage does), and what it left out, each as
/// the journal says it and when it ends, for the journal and Joel once
/// the round knows where the next reaches, with whose words it searched.
pub(super) struct Found {
    pub(super) threads: Vec<Thread>,
    pub(super) seen: HashMap<String, String>,
    pub(super) answered: bool,
    left: Vec<(String, String)>,
    login: String,
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
        let Own {
            from,
            floor,
            untold,
        } = kept.unwrap_or_else(|| Own {
            from: since,
            floor,
            untold: Vec::new(),
        });
        Self {
            from,
            floor,
            read: HashMap::new(),
            told: HashSet::new(),
            untold,
        }
    }

    /// How far it has searched, and the spans it holds, to keep across a
    /// restart.
    pub(super) fn kept(&self) -> Own {
        Own {
            from: self.from,
            floor: self.floor,
            untold: self.untold.clone(),
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
    /// from as far back next round. One GitHub cuts short is followed by
    /// one of what changed before the oldest it gave, up to [`SEARCHES`].
    pub(super) fn search(&self, github: &GitHub<'_>, site: &str, login: &str) -> Found {
        let reach = github::stamp(self.reach());
        let mut found = Found {
            threads: Vec::new(),
            seen: HashMap::new(),
            answered: true,
            left: Vec::new(),
            login: login.to_owned(),
        };
        // Up to when the search before was cut short, none at first.
        let mut before: Option<String> = None;
        for searching in 1..=SEARCHES {
            let changed = before.as_ref().map_or_else(
                || format!(">={reach}"),
                |before| format!("{reach}..{before}"),
            );
            let query = format!("pineapple involves:{login} updated:{changed}");
            let answer = match searched(github, &query) {
                Ok(answer) => answer,
                // Its reach stays put, so the journal says how far back
                // it has fallen.
                Err(Unread::Gone(why) | Unread::Failed(why) | Unread::Limited(why))
                    if before.is_none() =>
                {
                    eprintln!(
                        "<4>couldn't search {} for what Joel said, so it's searched from {reach} \
                         again next round: {why}",
                        github.name
                    );
                    found.answered = false;
                    return found;
                }
                // A later one failing leaves what's older unread, as one
                // cut short would, rather than hold every round back on a
                // wide search that may keep failing.
                Err(Unread::Gone(why) | Unread::Failed(why) | Unread::Limited(why)) => {
                    let said = format!(
                        "<4>couldn't search {} for what Joel said changed {changed} ({why}), so \
                         threads the search before left out are",
                        github.name
                    );
                    found.left.push((said, before.unwrap_or_default()));
                    return found;
                }
            };
            let given = found_when(&answer, site);
            let oldest = oldest(&given);
            for (thread, at) in given {
                // One read since and unchanged holds nothing new, and one
                // a search before gave is read already.
                let key = thread.key();
                if self.read.get(&key) != Some(&at) && !found.seen.contains_key(&key) {
                    found.threads.push(thread);
                }
                found.seen.insert(key, at);
            }
            let gave = answer["items"].as_array().map_or(0, Vec::len);
            let all = answer["total_count"].as_u64().unwrap_or(0);
            if u64::try_from(gave).is_ok_and(|gave| gave >= all) {
                return found;
            }
            // A later search giving nothing it can read left out all it
            // searched, up to when the one before was cut short.
            let Some(oldest) = oldest.or_else(|| before.clone()) else {
                // The first can't say how far it got, so it holds, as
                // one that fails does, rather than skip all it found.
                eprintln!(
                    "<4>{}'s search for what Joel said found {all} but gave {gave}, none it \
                     could read, so it's searched from {reach} again next round",
                    github.name
                );
                found.answered = false;
                return found;
            };
            if searching == SEARCHES {
                let said = format!(
                    "<4>{}'s search for what Joel said found {all} but gave {gave}, back to \
                     {oldest}, so threads it left out, changed from {reach} to then, are",
                    github.name
                );
                found.left.push((said, oldest));
                return found;
            }
            before = Some(oldest);
        }
        found
    }

    /// Tries again each span held as its row couldn't be written, for up
    /// to [`UNTOLD_ROUNDS`], whether or not what the round `found` was
    /// answered, so a mended rows file doesn't wait on a search that may
    /// fail for hours. Then, once it was answered, moves on to the round
    /// begun at `began`, knowing what that read as read, what the round
    /// couldn't read owed (owed.rs), and says in the journal what its
    /// searches left out, from where the next round reaches, which only
    /// moving on settles, and tells Joel in a row of its own (left.rs), as
    /// nothing searches it again.
    pub(super) fn moved_on(&mut self, began: u64, found: Found, tell: &Left) {
        for untold in std::mem::take(&mut self.untold) {
            self.tell_of(untold, tell);
        }
        if !found.answered {
            return;
        }
        let reach = self.reach();
        self.from = began;
        self.read = found.seen;
        let next = github::stamp(self.reach());
        for (left, to) in found.left {
            eprintln!("{left} searched again only from {next}, where the next round reaches");
            // A thread changed again from where the next round reaches
            // is found again, but what was said on it before then is
            // older than that round judges, so the row runs to the
            // oldest the search gave, as far as a stop it missed may be.
            if let Some(to) = github::seconds(&to) {
                let login = found.login.clone();
                let untold = Untold {
                    login,
                    from: reach,
                    to,
                    rounds: 0,
                };
                self.tell_of(untold, tell);
            }
        }
    }

    /// Tells Joel of what `untold` names in its row, held to try again
    /// the next round while it can't be written, until [`UNTOLD_ROUNDS`]
    /// have tried it, when the journal alone says so. One that comes
    /// within [`missed::MEET`] of a span held for the same login is held
    /// as one stretch with it, from the lower start to the higher end, at
    /// the fewer rounds, as that stretch is still being left out: in a
    /// lasting flood every round's search is cut short, and held apart
    /// they'd be a span a round, each tried and its file read every round.
    /// Why it couldn't be written is said only on a stretch's first try.
    fn tell_of(&mut self, untold: Untold, tell: &Left) {
        let meets = |held: &Untold| {
            let near = missed::MEET / 1000;
            held.login == untold.login
                && held.from <= untold.to.saturating_add(near)
                && untold.from <= held.to.saturating_add(near)
        };
        let aloud = untold.rounds == 0 && !self.untold.iter().any(meets);
        if tell.own(&untold.login, untold.from, untold.to, aloud) {
            return;
        }
        let rounds = untold.rounds + 1;
        if rounds < UNTOLD_ROUNDS {
            match self.untold.iter_mut().find(|held| meets(held)) {
                Some(held) => {
                    held.from = held.from.min(untold.from);
                    held.to = held.to.max(untold.to);
                    held.rounds = held.rounds.min(rounds);
                }
                None => self.untold.push(Untold { rounds, ..untold }),
            }
            return;
        }
        eprintln!(
            "<3>couldn't write Joel's row in {rounds} rounds saying {}'s search for what he said \
             left out threads changed from {} to {}, so only the journal says so: search there \
             for a stop",
            untold.login,
            github::stamp(untold.from),
            github::stamp(untold.to)
        );
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

/// When the least lately changed of what a search `given` last changed,
/// as GitHub stamps it; one that doesn't say is passed over, as an empty
/// stamp sorts first and would ask what's older with no end. None where
/// none says.
fn oldest(given: &[(Thread, String)]) -> Option<String> {
    given
        .iter()
        .map(|(_, at)| at)
        .filter(|at| !at.is_empty())
        .min()
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pineapple::github::{self, Thread};
    use pineapple::progress::{Own, Untold};

    use super::{Found, OWN_LAG, Searched, UNTOLD_ROUNDS, oldest};
    use crate::left::Rows;

    /// When the reader's rounds begin in these cases, in seconds. Their
    /// spans end in 1970 while `Left` stamps its rows by the real clock,
    /// so a row they write lives only as the newest written: the next
    /// write finds it a week past its end and drops it. Each case reads
    /// only the row it wrote last.
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
    fn a_restart_keeps_the_reach_a_failed_search_held_back_and_the_spans_it_held() {
        let kept = Own {
            from: SINCE - 2 * OWN_LAG,
            floor: 0,
            untold: vec![Untold {
                login: "joel.example".to_owned(),
                from: SINCE - 4 * OWN_LAG,
                to: SINCE - 3 * OWN_LAG,
                rounds: 7,
            }],
        };
        let searched = Searched::new(SINCE, Some(kept.clone()), false, LAG);
        assert_eq!(searched.kept(), kept, "kept as it was, held spans and all");
        assert_eq!(
            searched.reach(),
            SINCE - 3 * OWN_LAG,
            "an hour before the search it held back, not an hour before the rounds begin"
        );
    }

    #[test]
    fn the_oldest_a_search_gave_passes_over_one_that_doesnt_say() {
        let thread = |number| Thread {
            site: "github".to_owned(),
            repo: "example/shop".to_owned(),
            number,
            pull: false,
        };
        let given = [
            (thread(1), "2026-11-26T05:00:00Z".to_owned()),
            (thread(2), String::new()),
            (thread(3), "2026-11-26T04:00:00Z".to_owned()),
        ];
        assert_eq!(
            oldest(&given).as_deref(),
            Some("2026-11-26T04:00:00Z"),
            "the least lately changed that says when"
        );
        assert_eq!(
            oldest(&given[1..2]),
            None,
            "none where none says, so the search before bounds it"
        );
    }

    /// A search that moves on, leaving out what changed before `oldest`,
    /// if anything, as one cut short twice does.
    fn moving_on(oldest: Option<u64>) -> Found {
        let said = "<4>GitHub's search for what Joel said found 2001 but gave 2000";
        Found {
            threads: Vec::new(),
            seen: HashMap::new(),
            answered: true,
            left: oldest
                .map(|oldest| (said.to_owned(), github::stamp(oldest)))
                .into_iter()
                .collect(),
            login: "joel.example".to_owned(),
        }
    }

    #[test]
    fn a_span_whose_row_cant_be_written_is_held_until_it_is() {
        let rows = Rows::new("own-unwritten");
        rows.break_home();
        let mut searched = Searched::new(SINCE, None, false, LAG);
        searched.moved_on(SINCE + 30, moving_on(Some(SINCE - 60)), &rows.left);
        let held = Untold {
            login: "joel.example".to_owned(),
            from: SINCE - OWN_LAG,
            to: SINCE - 60,
            rounds: 1,
        };
        assert_eq!(
            searched.untold,
            [held],
            "held while its row can't be written"
        );

        rows.free_home();
        searched.moved_on(SINCE + 60, moving_on(None), &rows.left);
        let written = std::fs::read_to_string(rows.home.join("pineapple/missed-github.json"))
            .unwrap_or_default();
        let named = format!(
            "pineapple-github-missed-own-{}-{}",
            (SINCE - OWN_LAG) * 1000,
            (SINCE - 60) * 1000
        );
        assert!(
            searched.untold.is_empty() && written.contains(&named),
            "the next round that moves on writes it, from where it was left out: {written}"
        );
    }

    #[test]
    fn a_held_span_is_written_in_a_round_whose_search_failed() {
        let rows = Rows::new("own-unanswered");
        rows.break_home();
        let mut searched = Searched::new(SINCE, None, false, LAG);
        searched.moved_on(SINCE + 30, moving_on(Some(SINCE - 60)), &rows.left);
        assert_eq!(
            searched.untold.len(),
            1,
            "held while its row can't be written"
        );

        rows.free_home();
        let unanswered = Found {
            answered: false,
            ..moving_on(None)
        };
        searched.moved_on(SINCE + 60, unanswered, &rows.left);
        let written = std::fs::read_to_string(rows.home.join("pineapple/missed-github.json"))
            .unwrap_or_default();
        assert!(
            searched.untold.is_empty() && written.contains("pineapple-github-missed-own-"),
            "a mended rows file waited on a search that failed: {written}"
        );
        assert_eq!(
            searched.kept().from,
            SINCE + 30,
            "a round whose search failed moved on"
        );
    }

    #[test]
    fn spans_left_out_within_a_meet_of_each_other_are_held_as_one_stretch() {
        let rows = Rows::new("own-stretch");
        rows.break_home();
        let mut searched = Searched::new(SINCE, None, false, LAG);
        searched.moved_on(SINCE + 30, moving_on(Some(SINCE - 60)), &rows.left);
        searched.moved_on(SINCE + 60, moving_on(Some(SINCE)), &rows.left);
        let held = Untold {
            login: "joel.example".to_owned(),
            from: SINCE - OWN_LAG,
            to: SINCE,
            rounds: 1,
        };
        assert_eq!(
            searched.untold,
            [held],
            "from the first's start to the second's end, at the fewer rounds"
        );
    }

    #[test]
    fn a_span_whose_row_stays_unwritten_is_let_go_after_its_rounds() {
        assert_eq!(
            UNTOLD_ROUNDS,
            360,
            "a rows file that stays broken holds a span about three hours"
        );
        let rows = Rows::new("own-untold");
        rows.break_home();
        let mut searched = Searched::new(SINCE, None, false, LAG);
        searched.moved_on(SINCE + 30, moving_on(Some(SINCE - 60)), &rows.left);
        for round in 2..UNTOLD_ROUNDS {
            searched.moved_on(SINCE + 30 * u64::from(round), moving_on(None), &rows.left);
        }
        assert_eq!(
            searched
                .untold
                .iter()
                .map(|held| held.rounds)
                .collect::<Vec<_>>(),
            [UNTOLD_ROUNDS - 1],
            "held a round short of its last"
        );
        searched.moved_on(
            SINCE + 30 * u64::from(UNTOLD_ROUNDS),
            moving_on(None),
            &rows.left,
        );
        assert!(
            searched.untold.is_empty(),
            "a rows file that stays broken holds it no longer, the journal alone saying so"
        );
    }

    #[test]
    fn a_kept_floor_holds_back_the_reach() {
        let kept = Own {
            from: SINCE,
            floor: SINCE - LAG,
            untold: Vec::new(),
        };
        let searched = Searched::new(SINCE, Some(kept), false, LAG);
        assert_eq!(
            searched.reach(),
            SINCE - LAG,
            "no further back than its floor"
        );
    }
}
