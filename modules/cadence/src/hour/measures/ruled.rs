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
//! A denial, as "not Joel's rule", claims nothing, and nor does a line
//! of cadence's own card passed on, as `[0000000a] ... · T6E7X6`.
//!
//! Cadence's own messages are left out, and so is the first answer to its
//! ask, re the ask or re the claim, from the cell asked. An answer takes
//! its claim off the count, whatever it says: cadence can't judge whether
//! the words given are his, so the answer is read by whoever reads the
//! mesh. A message whose id isn't well formed, or names another sender,
//! is read as no one's, since its asks would go astray.
//!
//! The mesh is a shared folder with nothing signed, so this counts the
//! cells that write through the harness as it is meant to be used: a
//! cell writing files by hand can send as cadence or as another cell.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::cells::short_id;
use super::cited::sentences;
use super::{Class, DAY, Measure, count, short};
use crate::mesh::{Mesh, Message, stamped_at};

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

/// The words that deny a claim they come just before, lower case.
const DENIALS: [&str; 10] = [
    "not", "isn't", "wasn't", "aren't", "weren't", "isnt", "wasnt", "never", "no", "nor",
];

/// The words naming him, lower case, with his name's possessive.
const HIM: [&str; 5] = ["joel", "joel's", "he", "his", "him"];

/// How far past now, in seconds, a message's stamp may stand and still be
/// read: a clock a little ahead of cadence's is a message, and one stamped
/// further is a file that would otherwise be named as torn every hour.
const AHEAD: i64 = 300;

/// How many torn files the card names by id, the rest counted.
const TORN_NAMED: usize = 3;

/// The measure, from the mesh's messages of the last day under
/// `state_home`'s harness folder, and cadence's own over the day before
/// too, since an ask sent late in a claim's day may be answered after
/// it. With no messages folder there, or one that can't be listed or
/// read, it's left unjudged, rather than reading the messages it missed
/// as none unsourced and closing their asks. A file that isn't a message
/// is named in what it says and read past.
#[must_use]
pub fn read(state_home: &Path, now: i64) -> Measure {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return unread("no mesh messages folder here");
    }
    let day = (now - DAY) * 1000;
    let wanted = |stem: &str| {
        stamped_at(stem).is_some_and(|at| at >= day) || stem.split('-').nth(1) == Some(CADENCE)
    };
    match mesh.sent_as_named((now - 2 * DAY) * 1000, (now + AHEAD) * 1000, wanted) {
        Ok((sent, torn)) if torn.is_empty() => unsourced(&sent, now),
        Ok((sent, torn)) => {
            let mut named: Vec<String> = torn.iter().take(TORN_NAMED).map(|t| clean(t)).collect();
            if torn.len() > TORN_NAMED {
                named.push(format!("and {} more", torn.len() - TORN_NAMED));
            }
            let mut m = unsourced(&sent, now);
            m.says = format!("{} (read past, not messages: {})", m.says, named.join(", "));
            m
        }
        Err(why) => unread(&why),
    }
}

/// The measure unjudged, saying `why` it can't be read.
#[must_use]
pub fn unread(why: &str) -> Measure {
    Measure {
        says: format!("{SAYS} (unread: {})", clean(why)),
        target: None,
        ..unsourced(&[], 0)
    }
}

/// `text` with no control character in it, so a file's name or a
/// sender's words can't write a line of the card that reads as cadence's.
fn clean(text: &str) -> String {
    text.replace(char::is_control, " ")
}

/// The ask to the sender of message `id` claiming his rule in `said`.
/// [`about`] reads the claim's id back from its opening words, so the two
/// are kept here together.
#[must_use]
pub fn ask(id: &str, said: &str) -> String {
    format!(
        "{ASKS}{id} {ASKED} or wants something, with no word of when he said it: \"{said}\". \
         Only his own words bind a cell. Please answer on the mesh, re this ask or re {id}, \
         with his words and when he said them, or say the claim is dropped and whose caution \
         it was. Your answer closes this ask; unanswered, it closes a day after the message."
    )
}

/// The words an ask opens with, before the claim's id.
const ASKS: &str = "cadence hour: your message ";

/// The words that follow the claim's id in an ask.
const ASKED: &str = "says Joel ruled";

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
        .filter_map(|m| Some((stamp(m)?, m)))
        .filter(|&(at, _)| at > now - DAY && at <= now)
        .filter_map(|(at, m)| Some((at, m, unbacked(&m.text)?)))
        .collect();
    found.sort_by_key(|&(at, _, _)| at);
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

/// When `m` was sent, in seconds, by the stamp its id and file name both
/// carry. A message's own `at` is left aside: its sender writes it, and
/// one set a day back would slip its claim out of the count.
fn stamp(m: &Message) -> Option<i64> {
    stamped_at(&m.id).map(|ms| ms / 1000)
}

/// The ids of the answers in `sent` to a claim of his rule, and the claims
/// they answer, by sender and short id. Only a claim this measure asked
/// about has an answer: the first reply, after the ask, from the cell it
/// asked, re its ask or re the claim the ask names. The ask, not the
/// claim, says who may answer, so an answer counts after its claim has
/// left the day. A later reply, one before the ask, one re a message never
/// asked about, or one re any other of cadence's, is judged as any other
/// message, so a cell can't keep claiming his rule under its first answer.
fn answers<'a>(sent: &[&'a Message]) -> (BTreeSet<&'a str>, BTreeSet<(&'a str, &'a str)>) {
    let asks: Vec<(&str, &str, &Message)> = sent
        .iter()
        .copied()
        .filter(|m| m.from == CADENCE)
        .filter_map(|m| Some((short_id(&m.id), about(&m.text)?, m)))
        .collect();
    let claim_of: BTreeMap<&str, &str> = asks.iter().map(|&(ask, claim, _)| (ask, claim)).collect();
    let asked_before = |claim: &str, of: &str, at: Option<i64>| {
        asks.iter().any(|&(_, about, ask)| {
            about == claim
                && ask.to.iter().any(|to| to == &format!("@{of}"))
                && stamped_at(&ask.id) < at
        })
    };
    let mut by_time = sent.to_vec();
    by_time.sort_by_key(|m| stamped_at(&m.id));
    let (mut answers, mut answered) = (BTreeSet::new(), BTreeSet::new());
    for m in by_time {
        let Some(re) = m.re.as_deref().map(short_id) else {
            continue;
        };
        let claim = claim_of.get(re).copied().unwrap_or(re);
        let first = !answered.contains(&(m.from.as_str(), claim));
        if first && asked_before(claim, &m.from, stamped_at(&m.id)) {
            answers.insert(m.id.as_str());
            answered.insert((m.from.as_str(), claim));
        }
    }
    (answers, answered)
}

/// The short id of the message one of this measure's asks is about, as
/// [`ask`] writes it. Any other of cadence's messages is about no claim.
fn about(ask: &str) -> Option<&str> {
    let (id, rest) = ask.strip_prefix(ASKS)?.split_once(' ')?;
    rest.starts_with(ASKED).then_some(id)
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

/// `text` with its curly apostrophes made straight, the lines of cadence's
/// card it passes on taken out, as [`uncarded`] does, and its quotes put
/// by stand-ins, as [`unquoted`] does.
fn plain(text: &str) -> String {
    unquoted(&uncarded(&text.replace(['\u{2018}', '\u{2019}'], "'")))
}

/// `text` with no line of cadence's card in it: neither a claim the card
/// names, as `[0000000a] I broke Joel's rule. · T6E7X6`, nor what the
/// measure says it counts. The card is made to be passed on, and a cell
/// passing it on claims nothing of its own.
fn uncarded(text: &str) -> String {
    let plain = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric());
    let carded = |line: &str| {
        let line = line.trim_start_matches(|c: char| c.is_whitespace() || c == '-' || c == '*');
        let named = line
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("] "))
            .is_some_and(|(id, _)| plain(id));
        named
            && line
                .rsplit_once(" · ")
                .is_some_and(|(_, from)| plain(from.trim_end()))
    };
    text.lines()
        .filter(|line| !carded(line))
        .collect::<Vec<_>>()
        .join("\n")
        .replace(SAYS, "...")
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
    // "Joel's wish list" names a list of his, the repository's own, and
    // claims nothing he said.
    let said = sentence
        .to_lowercase()
        .replace("joel's wish list", "joel's list");
    let names = CLAIMS.iter().any(|claim| at_word(&said, claim));
    let joel = words(&said).iter().any(|word| word == "joel");
    names || (joel && HIS.iter().any(|claim| at_word(&said, claim)))
}

/// Whether `words` holds `phrase` as whole words, or with a plural "s",
/// so "Joel's calls" claims and "Joel's calendar" doesn't, and with no
/// denial just before it, as [`denied`] reads one.
fn at_word(words: &str, phrase: &str) -> bool {
    words.match_indices(phrase).any(|(at, _)| {
        let before = words[..at].chars().next_back();
        let rest = &words[at + phrase.len()..];
        let after = rest.strip_prefix('s').unwrap_or(rest).chars().next();
        before.is_none_or(|c| !c.is_alphanumeric())
            && after.is_none_or(|c| !c.is_alphanumeric())
            && !denied(&words[..at])
    })
}

/// Whether the words before a claim end in a denial of it, as "not Joel's
/// rule" or "never his wish". A denial set off by a stop, as "No, Joel
/// wants it", answers something else and denies nothing that follows.
fn denied(before: &str) -> bool {
    before
        .split_whitespace()
        .next_back()
        .is_some_and(|word| word.ends_with(char::is_alphanumeric) && DENIALS.contains(&word))
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
    // A time ending in a colon heads what follows, as a sender's own
    // "T6E7X6, 10:08: Joel wants", and is when the sender wrote, not he.
    let heads = |at: usize| tokens[at].ends_with(':');
    let ends =
        |at: usize| !heads(at) && (timed(at) || (at > 0 && dated(&lower[at - 1], &lower[at])));
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
