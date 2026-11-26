//! Messages on the mesh that say Joel ruled or wants something with no
//! word of his behind it. On 26 November two cells told the mesh they had
//! broken Joel's rule by reaching a test machine, while the rule was an
//! orchestrator's own pivot, and the work it held back was his. A claim
//! with no word of his binds no one, so each is asked of its sender: his
//! dated words, or the claim dropped.
//!
//! A claim is backed when its sentence, or the one before or after it,
//! says when he said it: a date, as "26 Nov" or "2026-11-26" or "Thu
//! 26", or a clock time, following or leading his name, or he or his,
//! with only words of saying between, as "Joel, 11:30" or "he said on 22
//! Nov". A date anywhere else in a sentence dates something else, as a
//! deadline or the hour a rule was broken, and a quote with no date may
//! be the claimer's own words for the rule. A claim inside quotes or
//! backticks is words about a claim, not one. Each claim in a message is
//! judged, so one backed claim hides no other.
//!
//! Cadence's own messages are left out, and so is an answer, re its ask
//! or re the claim itself, from the claim's sender. An answer takes its
//! claim off the count, whatever it says: cadence can't judge whether the
//! words given are his, so the answer is read by whoever reads the mesh.
//! A message whose id isn't well formed, or names another sender, is
//! read as no one's, since its asks would go astray.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::cells::short_id;
use super::cited::sentences;
use super::holds::sent_at;
use super::{Class, DAY, Measure, count, short};
use crate::mesh::{Mesh, Message};

/// The key the count of claims with no word of his behind them takes.
pub const KEY: &str = "rulings-unsourced";

/// Who answers for the measure when none is unsourced, or the mesh
/// can't be read: the keeper of his rulings.
const KEEPER: &str = "1JQPPC";

/// The sender cadence's own messages go by.
const CADENCE: &str = "CADENCE";

/// What the measure counts, as the card says it.
const SAYS: &str = "messages in the last day saying Joel ruled or wants something, with no \
                    word of when he said it, unanswered";

/// The words that claim a rule or wish of Joel's, lower case, each read
/// only where a word starts.
const CLAIMS: [&str; 17] = [
    "joel's rule",
    "joel's standing rule",
    "joel's wish",
    "joel's will",
    "joel's call",
    "joel ruled",
    "joel rules",
    "joel wants",
    "joel wanted",
    "joel asked for",
    "joel asks for",
    "joel said",
    "joel says",
    "joel has said",
    "joel insists",
    "joel's orders",
    "per joel",
];

/// The words that claim one of his by "he" or "his", which count only in
/// a sentence naming Joel, since a message speaks of other people's rules
/// too. "He said" isn't among them: it tells what he said as often as it
/// claims a rule, as when a pain is told by what he said.
const HIS: [&str; 6] = [
    "his rule",
    "his ruling",
    "his wish",
    "he ruled",
    "he wants",
    "he asked for",
];

/// The months, each by its names, lower case: a word must be one whole,
/// so "declined" or "marked" never reads as a month.
const MONTHS: [&[&str]; 12] = [
    &["jan", "january"],
    &["feb", "february"],
    &["mar", "march"],
    &["apr", "april"],
    &["may"],
    &["jun", "june"],
    &["jul", "july"],
    &["aug", "august"],
    &["sep", "sept", "september"],
    &["oct", "october"],
    &["nov", "november"],
    &["dec", "december"],
];

/// The days of the week, each by its names, lower case.
const WEEKDAYS: [&[&str]; 7] = [
    &["mon", "monday"],
    &["tue", "tues", "tuesday"],
    &["wed", "wednesday"],
    &["thu", "thurs", "thursday"],
    &["fri", "friday"],
    &["sat", "saturday"],
    &["sun", "sunday"],
];

/// How many words of saying may stand between his name and a date or
/// clock time for it to be when he said it.
const BESIDE: usize = 3;

/// The words of saying that may stand between his name and when he said
/// it, as "Joel said on 22 Nov" or "his words of 10:05", lower case.
const SAYING: [&str; 17] = [
    "said", "says", "wrote", "asked", "ruled", "told", "on", "at", "in", "of", "from", "his",
    "words", "message", "verbatim", "it", "so",
];

/// The names of a rule, which stand between his name and its date only
/// as "Joel's rule of 22 Nov": "Joel's rule at 10:08" dates its breaking.
const RULES: [&str; 5] = ["rule", "ruling", "wish", "call", "orders"];

/// The words naming him, lower case, with his name's possessive.
const HIM: [&str; 5] = ["joel", "joel's", "he", "his", "him"];

/// The measure, from the mesh's messages of the last day under
/// `state_home`'s harness folder. With no messages folder there, it's
/// left unjudged, rather than reading no messages as none unsourced.
#[must_use]
pub fn read(state_home: &Path, now: i64) -> Measure {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return unread("no mesh messages folder here");
    }
    unsourced(&mesh.sent_since_as_named((now - DAY) * 1000), now)
}

/// The measure unjudged, saying `why` it can't be read.
#[must_use]
pub fn unread(why: &str) -> Measure {
    Measure {
        says: format!("{SAYS} (unread: {why})"),
        target: None,
        ..unsourced(&[], 0)
    }
}

/// The messages in `sent` from the day before `now` that claim a rule or
/// wish of Joel's with nothing behind it and no answer from their sender,
/// oldest first, each once, named by its short id, the sentence claiming
/// it and its sender, who is asked for his words. The oldest one's sender
/// answers for the measure.
#[must_use]
pub fn unsourced(sent: &[Message], now: i64) -> Measure {
    let sent: Vec<&Message> = sent.iter().filter(|m| well_named(m)).collect();
    let (answers, answered) = answers(&sent);
    let mut seen = BTreeSet::new();
    let mut found: Vec<(i64, &Message, String)> = sent
        .iter()
        .copied()
        .filter(|m| m.from != CADENCE && !answers.contains(m.id.as_str()))
        .filter(|m| !answered.contains(&(m.from.as_str(), short_id(&m.id))))
        .filter(|m| seen.insert(m.id.as_str()))
        .filter_map(|m| Some((sent_at(m)?, m)))
        .filter(|&(at, _)| at > now - DAY && at <= now)
        .filter_map(|(at, m)| Some((at, m, unbacked(&m.text)?)))
        .collect();
    found.sort_by_key(|&(at, _, _)| at);
    let clean = |text: &str| text.replace(char::is_control, " ");
    Measure {
        key: KEY.to_owned(),
        class: Class::Cells,
        says: SAYS.to_owned(),
        now: count(found.len()),
        target: Some(0.0),
        owner: found
            .first()
            .map_or_else(|| KEEPER.to_owned(), |(_, m, _)| m.from.clone()),
        behind: found
            .iter()
            .map(|(_, m, claim)| {
                let (id, from) = (short_id(&m.id), &m.from);
                format!("[{id}] {} · {from}", short(&clean(claim)))
            })
            .collect(),
    }
}

/// Whether `m`'s id is one the harness gives, of letters and digits as
/// `<ms>-<from>-<pid>-<short>`, naming its own sender: any other would
/// key its ask wrongly, or send it to someone who never sent it.
fn well_named(m: &Message) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let plain = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric());
    let parts: Vec<&str> = m.id.split('-').collect();
    matches!(parts.as_slice(), [ms, from, pid, id]
        if digits(ms) && plain(from) && *from == m.from && digits(pid) && plain(id))
}

/// The ids of the answers in `sent` to a claim of his rule, and the claims
/// they answer, by sender and short id. An answer is re one of cadence's
/// asks, or re a claiming message of its own sender's. One re an ask
/// answers the claim the ask names, and only for that claim's sender.
fn answers<'a>(sent: &[&'a Message]) -> (BTreeSet<&'a str>, BTreeSet<(&'a str, &'a str)>) {
    let asked: BTreeMap<&str, Option<&str>> = sent
        .iter()
        .copied()
        .filter(|m| m.from == CADENCE)
        .map(|m| (short_id(&m.id), about(&m.text)))
        .collect();
    let claimed: BTreeMap<&str, &str> = sent
        .iter()
        .copied()
        .filter(|m| m.from != CADENCE && claims_any(&m.text))
        .map(|m| (short_id(&m.id), m.from.as_str()))
        .collect();
    let (mut answers, mut answered) = (BTreeSet::new(), BTreeSet::new());
    for &m in sent {
        let Some(re) = m.re.as_deref().map(short_id) else {
            continue;
        };
        let own = claimed.get(re) == Some(&m.from.as_str());
        let about = asked.get(re).copied().flatten().or(own.then_some(re));
        if asked.contains_key(re) || own {
            answers.insert(m.id.as_str());
        }
        if let Some(about) = about {
            answered.insert((m.from.as_str(), about));
        }
    }
    (answers, answered)
}

/// The short id of the message one of cadence's asks is about, as its
/// text names it: "your message <short id> says".
fn about(ask: &str) -> Option<&str> {
    ask.split_once("your message ")?.1.split_whitespace().next()
}

/// Whether any sentence of `text` claims a rule or wish of Joel's.
fn claims_any(text: &str) -> bool {
    sentences(&plain(text))
        .iter()
        .any(|sentence| claims(sentence))
}

/// The first sentence of `text` claiming a rule or wish of Joel's that
/// nothing beside it backs: neither it nor the sentence before or after
/// it says when he said it. Every claim is judged, so a backed one hides
/// no other.
#[must_use]
pub fn unbacked(text: &str) -> Option<String> {
    let plain = plain(text);
    let said = sentences(&plain);
    let near = |at: usize| &said[at.saturating_sub(1)..(at + 2).min(said.len())];
    let backed = |at: usize| near(at).iter().any(|sentence| dated_by_him(sentence));
    (0..said.len())
        .find(|&at| claims(said[at]) && !backed(at))
        .map(|at| said[at].to_owned())
}

/// `text` with its curly apostrophes made straight and its quotes put by
/// stand-ins, as [`unquoted`] does.
fn plain(text: &str) -> String {
    unquoted(&text.replace(['\u{2018}', '\u{2019}'], "'"))
}

/// `text` with its bold and italic marks taken out, and every span in
/// double quotes or backticks put by its quotes around three dots, so no
/// claim inside one is read and no stop inside one ends a sentence. A
/// quote left open is put back as written, since a stray quote mark would
/// otherwise hide every claim after it.
fn unquoted(text: &str) -> String {
    let mut plain = String::new();
    let mut open: Option<(char, char, String)> = None;
    for c in text.chars() {
        match open.as_mut() {
            Some((_, close, _)) if c == *close => {
                plain.push(c);
                plain.push_str("...");
                plain.push(c);
                open = None;
            }
            Some((_, _, quote)) => quote.push(c),
            None => match c {
                '"' | '`' => open = Some((c, c, String::new())),
                '\u{201c}' => open = Some((c, '\u{201d}', String::new())),
                // Bold or italic marks stand between a stop and the space
                // after it, as in "**I ran them.** That", and would join
                // two sentences into one.
                '*' => {}
                _ => plain.push(c),
            },
        }
    }
    if let Some((opened, _, quote)) = open {
        plain.push(opened);
        plain.push_str(&quote);
    }
    plain
}

/// Whether `sentence` claims a rule or wish of Joel's: any of the words
/// naming him, or one by "he" or "his" in a sentence naming him.
fn claims(sentence: &str) -> bool {
    let said = sentence.to_lowercase();
    let names = CLAIMS.iter().any(|claim| at_word(&said, claim));
    let joel = words(&said).iter().any(|word| word == "joel");
    names || (joel && HIS.iter().any(|claim| at_word(&said, claim)))
}

/// Whether `words` holds `phrase` where a word starts.
fn at_word(words: &str, phrase: &str) -> bool {
    words.match_indices(phrase).any(|(at, _)| {
        words[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric())
    })
}

/// Whether `sentence` says when he said something: a date or a clock
/// time following or leading a word naming him, with no more than a few
/// words of saying between, as "Joel, 11:30", "he said on 22 Nov" or
/// "22 Nov, Joel". Any other word between makes it the date of something
/// else, as "Joel wants it by 26 Nov" or "I broke Joel's rule at 10:08".
fn dated_by_him(sentence: &str) -> bool {
    let tokens: Vec<&str> = sentence.split_whitespace().collect();
    let lower: Vec<String> = tokens.iter().map(|token| bare(token)).collect();
    let saying = |at: usize| {
        let word = lower[at].as_str();
        let named_by_date = lower
            .get(at + 1)
            .is_some_and(|next| ["of", "from"].contains(&next.as_str()));
        word.is_empty() || SAYING.contains(&word) || (RULES.contains(&word) && named_by_date)
    };
    let timed = |at: usize| clock(tokens[at]) || iso_date(tokens[at]);
    let starts = |at: usize| timed(at) || lower.get(at + 1).is_some_and(|b| dated(&lower[at], b));
    let ends = |at: usize| timed(at) || (at > 0 && dated(&lower[at - 1], &lower[at]));
    (0..tokens.len())
        .filter(|&at| HIM.contains(&lower[at].as_str()))
        .any(|him| {
            let after = (him + 1..tokens.len()).take(BESIDE + 1);
            let before = (0..him).rev().take(BESIDE + 1);
            reaches(after, starts, saying) || reaches(before, ends, saying)
        })
}

/// Whether, walking `path` away from his name, a word where `date` holds
/// comes before any word that isn't one of `saying`.
fn reaches(
    path: impl IntoIterator<Item = usize>,
    date: impl Fn(usize) -> bool,
    saying: impl Fn(usize) -> bool,
) -> bool {
    path.into_iter()
        .find(|&at| date(at) || !saying(at))
        .is_some_and(date)
}

/// `token` lower case, with what isn't a letter or digit taken off either
/// end, as "(Joel," is "joel" and "Joel's" stays "joel's".
fn bare(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Whether two words in a row name a date: a day and a month either way
/// round, or a weekday and a day.
fn dated(first: &str, then: &str) -> bool {
    let day = |word: &str| {
        let digits = ["st", "nd", "rd", "th"]
            .iter()
            .find_map(|ordinal| word.strip_suffix(ordinal))
            .unwrap_or(word);
        digits
            .parse::<u8>()
            .is_ok_and(|day| (1..=31).contains(&day))
    };
    let month = |word: &str| named(&MONTHS, word);
    let weekday = |word: &str| named(&WEEKDAYS, word);
    (day(first) && month(then)) || (month(first) && day(then)) || (weekday(first) && day(then))
}

/// Whether `word`, lower case, is one of the names of any of `names`.
fn named(names: &[&[&str]], word: &str) -> bool {
    names.iter().any(|one| one.contains(&word))
}

/// Whether `word` opens with a date as `2026-11-26`.
fn iso_date(word: &str) -> bool {
    let word = word.trim_start_matches(|c: char| !c.is_ascii_digit());
    let bytes = word.as_bytes();
    bytes.len() >= 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&at| bytes[at].is_ascii_digit())
}

/// Whether `word` holds a clock time, an hour of one or two digits, a
/// colon and two digits.
fn clock(word: &str) -> bool {
    word.match_indices(':').any(|(at, _)| {
        let before = word[..at]
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .count();
        let after = word[at + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .count();
        (1..=2).contains(&before) && after == 2
    })
}

/// The words of `text`, of letters and digits.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
