//! Messages on the mesh that say Joel ruled or wants something with no
//! word of his behind it. On 26 November a cell's caution about reaching
//! a test machine was passed on as Joel's rule and held against the work,
//! and of 92 messages claiming a rule or wish of his, only 22 carried his
//! words or a dated source near the claim. A claim with neither binds no
//! one, so each is asked of its sender: his dated words, or the claim
//! dropped.
//!
//! A message backs its claim when it quotes three words or more, as a
//! quote of his is given, or names a date, as "26 Nov" or "2026-11-26" or
//! "Thu 26", or a clock time beside Joel's name, as "Joel, 11:30" or
//! "Joel's 10:05 ruling". A claim inside quotes or backticks is words
//! about a claim, not one. Cadence's own messages, and answers to them,
//! are left out, since its asks name the claim they're about.

use std::collections::BTreeSet;
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
                    quote of his and no dated source";

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

/// How many words away from Joel's name a clock time still dates a claim.
const BESIDE: usize = 3;

/// How many words a quote holds at least to be one given as his.
const QUOTED: usize = 3;

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
    unsourced(&mesh.sent_since((now - DAY) * 1000), now)
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
/// wish of Joel's with nothing behind it, oldest first, each once, named
/// by its short id, the sentence claiming it and its sender, who is
/// asked for his words. The oldest one's sender answers for the measure.
#[must_use]
pub fn unsourced(sent: &[Message], now: i64) -> Measure {
    let cadence: BTreeSet<&str> = sent
        .iter()
        .filter(|m| m.from == CADENCE)
        .map(|m| short_id(&m.id))
        .collect();
    let answers_cadence = |m: &Message| {
        m.re
            .as_deref()
            .is_some_and(|re| cadence.contains(short_id(re)))
    };
    let mut seen = BTreeSet::new();
    let mut found: Vec<(i64, &Message, String)> = sent
        .iter()
        .filter(|m| m.from != CADENCE && !answers_cadence(m))
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
            .map_or_else(|| KEEPER.to_owned(), |(_, m, _)| clean(&m.from)),
        behind: found
            .iter()
            .map(|(_, m, claim)| {
                let (id, from) = (clean(short_id(&m.id)), clean(&m.from));
                format!("[{id}] {} · {from}", short(&clean(claim)))
            })
            .collect(),
    }
}

/// The first sentence of `text` claiming a rule or wish of Joel's, when
/// nothing in `text` backs it: no quote of three words or more, no date,
/// and no clock time beside his name.
#[must_use]
pub fn unbacked(text: &str) -> Option<String> {
    let text = text.replace(['\u{2018}', '\u{2019}'], "'");
    let (plain, quotes) = unquoted(&text);
    let claim = sentences(&plain).into_iter().find(|s| claims(s))?;
    let backed = quotes.iter().any(|quote| quote.split_whitespace().count() >= QUOTED)
        || dated(&text)
        || clocked(&text);
    (!backed).then(|| claim.trim().to_owned())
}

/// `text` with every span in double quotes or backticks taken out, and
/// the quoted spans, each a quote's words; a quote left open runs to the
/// end, as it does as it's read.
fn unquoted(text: &str) -> (String, Vec<String>) {
    let (mut plain, mut quotes) = (String::new(), Vec::new());
    let mut open: Option<(char, String)> = None;
    for c in text.chars() {
        match open.as_mut() {
            Some((close, quote)) if c == *close => {
                quotes.push(std::mem::take(quote));
                open = None;
                plain.push(' ');
            }
            Some((_, quote)) => quote.push(c),
            None => match c {
                '"' => open = Some(('"', String::new())),
                '\u{201c}' => open = Some(('\u{201d}', String::new())),
                '`' => open = Some(('`', String::new())),
                _ => plain.push(c),
            },
        }
    }
    if let Some((_, quote)) = open {
        quotes.push(quote);
    }
    (plain, quotes)
}

/// Whether `sentence` claims a rule or wish of Joel's: any of the words
/// naming him, or one by "he" or "his" in a sentence naming him.
fn claims(sentence: &str) -> bool {
    let said = sentence.to_lowercase();
    let names = CLAIMS.iter().any(|claim| at_word(&said, claim));
    names || (at_word(&said, "joel") && HIS.iter().any(|claim| at_word(&said, claim)))
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

/// Whether `text` names a date: a day and a month either way round, a
/// year, month and day, or a weekday and a day.
fn dated(text: &str) -> bool {
    let words = words(text);
    let day = |word: &str| {
        word.parse::<u8>()
            .is_ok_and(|day| (1..=31).contains(&day))
    };
    let month = |word: &str| named(&MONTHS, word);
    let weekday = |word: &str| named(&WEEKDAYS, word);
    let paired = words.windows(2).any(|pair| {
        let (a, b) = (pair[0].as_str(), pair[1].as_str());
        (day(a) && month(b)) || (month(a) && day(b)) || (weekday(a) && day(b))
    });
    paired || text.split_whitespace().any(iso_date)
}

/// Whether `word` is one of the names of any of `names`, whatever its case.
fn named(names: &[&[&str]], word: &str) -> bool {
    let word = word.to_lowercase();
    names.iter().any(|one| one.contains(&word.as_str()))
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

/// Whether a clock time, as `11:30`, sits within a few words of Joel's
/// name in `text`.
fn clocked(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let joel = |word: &str| at_word(&word.to_lowercase(), "joel");
    words.iter().enumerate().filter(|(_, w)| joel(w)).any(|(at, _)| {
        let near = at.saturating_sub(BESIDE)..(at + BESIDE + 1).min(words.len());
        words[near].iter().any(|word| clock(word))
    })
}

/// Whether `word` holds a clock time, an hour of one or two digits, a
/// colon and two digits.
fn clock(word: &str) -> bool {
    word.match_indices(':').any(|(at, _)| {
        let before = word[..at].chars().rev().take_while(char::is_ascii_digit).count();
        let after = word[at + 1..].chars().take_while(char::is_ascii_digit).count();
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
