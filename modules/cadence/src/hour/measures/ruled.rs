//! Messages on the mesh that say Joel ruled or wants something with no
//! word of his behind it. On 26 November two cells told the mesh they had
//! broken Joel's rule by reaching a test machine, while the rule was an
//! orchestrator's own pivot, and the work it held back was his. A claim
//! with no word of his binds no one, so each is asked of its sender: his
//! dated words, or the claim dropped.
//!
//! A claim is backed when its sentence, or the one before or after it,
//! says when he said it: a date, as "26 Nov" or "2026-11-26" or "Thu
//! 26", or a clock time, tied to his saying. It follows his name, he,
//! his or him, with only words of saying between; he, his or him stand
//! for Joel only in the claim's sentence, one naming him, or the one just
//! after opening with them, since elsewhere they may be anyone. It's tied
//! to his saying by nothing but a stop between, as "Joel, 11:30", unless
//! the cell said it to him, as "I told Joel 10:08", by a word of
//! his saying between or just after, as "he said on 22 Nov" or "Joel at
//! 10:05 said", or by the rule it dates, as "Joel's rule of 22 Nov"; or
//! it leads his name or he with a word of his saying next, as "On 22 Nov,
//! Joel ruled it". A date anywhere else dates something else, as a
//! deadline, the hour a rule was broken, or when a cell told him, and a
//! quote with no date may be the claimer's own words for the rule. A claim
//! inside quotes or backticks is words about a claim, not one. Each claim
//! in a message is judged, so one backed claim hides no other.
//!
//! A denial, as "not Joel's rule", claims nothing, and nor does a line
//! of cadence's own card passed on, as `[0000000a] ... · T6E7X6`, nor
//! what a measure says it counts, as `pains Joel said ...`, wherever the
//! card puts it.
//!
//! Cadence's own messages are left out, and so is every reply to its ask,
//! re the ask or re the claim, from the cell asked: the ask tells it to
//! say the claim is dropped, and the words it drops it in name the claim.
//! Any such reply answers, and takes its claim off the count, whatever
//! it says: cadence can't judge whether the words given are his, so the
//! answer is read by whoever reads the mesh. Its asks are read back over
//! [`ASKS_READ`] days, so an answer sent long after is still one. A
//! message whose id
//! isn't well formed, or names another sender, is read as no one's, since
//! its asks would go astray.
//!
//! The mesh is a shared folder with nothing signed, so this counts the
//! cells that write through the harness as it is meant to be used: a
//! cell writing files by hand can send as cadence or as another cell.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::cells::short_id;
use super::cited::sentences;
use super::{Class, DAY, Measure, count, pains, short};
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
///
/// "Joel's call" isn't among them: in this repository it names his phone
/// and video calls far more often than his decisions.
const CLAIMS: [&str; 16] = [
    "joel's rule",
    "joel's standing rule",
    "joel's wish",
    "joel's will",
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

/// The words of his saying, lower case: a date by he, his or him, or one
/// leading his name, is when he said it only beside one of these, since
/// "I told him at 10:08" or "At 10:08 he saw it" is someone else's hour.
const SAID: [&str; 9] = [
    "said", "says", "wrote", "asked", "ruled", "told", "words", "message", "verbatim",
];

/// The names of a rule, which stand between his name and its date only
/// as "Joel's rule of 22 Nov": "Joel's rule at 10:08" dates its breaking.
const RULES: [&str; 4] = ["rule", "ruling", "wish", "orders"];

/// The words that deny a claim they come just before, lower case.
const DENIALS: [&str; 10] = [
    "not", "isn't", "wasn't", "aren't", "weren't", "isnt", "wasnt", "never", "no", "nor",
];

/// The words naming him, lower case, with his name's possessive.
const HIM: [&str; 5] = ["joel", "joel's", "he", "his", "him"];

/// The words of a cell's telling him, lower case, just before his name:
/// a time straight after it, as "I told Joel 10:08", is the cell's.
const TO_HIM: [&str; 12] = [
    "told", "tell", "asked", "ask", "pinged", "ping", "messaged", "reminded", "remind", "showed",
    "sent", "send",
];

/// How many days of cadence's own asks are read back, so an answer to
/// one sent days late, as by a cell parked over a weekend, is still read
/// as an answer rather than judged as a claim. Only cadence's own files
/// are opened past the two days the claims are read over.
const ASKS_READ: i64 = 8;

/// The most characters of a claim the card and the ask give: words are
/// capped too, but a sentence with no space in it, or with marks that
/// look like none, would otherwise be given whole.
const CLAIM_LONGEST: usize = 160;

/// How far past now, in seconds, a message's stamp may stand and still be
/// read: a clock a little ahead of cadence's is a message, and one stamped
/// further is a file that would otherwise be named as torn every hour.
const AHEAD: i64 = 300;

/// How many torn files the card names by id, the rest counted.
const TORN_NAMED: usize = 3;

/// The most characters of a torn file's name the card gives: the
/// harness's own names are some forty long.
const TORN_LONGEST: usize = 48;

/// The measure, from the mesh's messages of the last two days under
/// `state_home`'s harness folder, and cadence's asks of the last
/// [`ASKS_READ`] days. Only the last day's messages are judged, but an
/// answer sent late judged as a claim would ask its sender about its own
/// answer, so the asks before are read too, and the day before the
/// claims for a reply re one. Only cadence's asks, replies, and those
/// naming him are kept, since nothing else can claim, ask or answer, and
/// two days of the mesh held whole would weigh on the hour. With no
/// messages folder there, or one that can't be listed or read, it's left
/// unjudged, rather than reading the messages it missed as none unsourced
/// and closing their asks. A file that isn't a message is named in what
/// it says and read past.
#[must_use]
pub fn read(state_home: &Path, now: i64) -> Measure {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return unread("no mesh messages folder here");
    }
    let ask = |m: &Message| m.from == CADENCE && about(&m.text).is_some();
    let used = |m: &Message| ask(m) || m.re.is_some() || m.text.to_lowercase().contains("joel");
    let two_days = (now - 2 * DAY) * 1000;
    let read = mesh
        .sent_as_named(two_days, (now + AHEAD) * 1000, used)
        .and_then(|(mut sent, mut torn)| {
            let before = (now - ASKS_READ * DAY) * 1000;
            let (asks, torn_asks) = mesh.sent_by_as_named(CADENCE, before, two_days - 1, ask)?;
            sent.extend(asks);
            torn.extend(torn_asks);
            Ok((sent, torn))
        });
    match read {
        Ok((sent, torn)) if torn.is_empty() => unsourced(&sent, now),
        Ok((sent, torn)) => {
            let mut named: Vec<String> = torn
                .iter()
                .take(TORN_NAMED)
                .map(|t| clean(t).chars().take(TORN_LONGEST).collect())
                .collect();
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

/// `text` with no control character in it, nor any mark that turns text
/// round or breaks a line unseen, so a file's name or a sender's words
/// can't write a line of the card that reads as cadence's, or seem to
/// name another sender.
fn clean(text: &str) -> String {
    let unseen = |c: char| {
        c.is_control()
            || FILLERS.contains(&c)
            || matches!(
                c,
                '\u{061c}'
                    | '\u{180e}'
                    | '\u{200b}'..='\u{200f}'
                    | '\u{2028}'..='\u{202e}'
                    | '\u{2060}'..='\u{2069}'
                    | '\u{feff}'
            )
    };
    text.replace(unseen, " ")
}

/// The ask to the sender of message `id` claiming his rule in `said`.
/// [`about`] reads the claim's id back from its opening words, so the two
/// are kept here together. Its own words claim nothing, as
/// [`unbacked`] reads them, so a cell passing it on isn't asked in turn.
#[must_use]
pub fn ask(id: &str, said: &str) -> String {
    format!(
        "{ASKS}{id} {ASKED}, with no word of when he said it: \"{said}\". \
         Only his own words bind a cell. Please answer on the mesh, re this ask or re {id}, \
         with his words and when he said them, or say the claim is dropped and whose caution \
         it was. Your answer closes this ask; unanswered, it closes a day after the message."
    )
}

/// The words an ask opens with, before the claim's id.
const ASKS: &str = "cadence hour: your message ";

/// The words that follow the claim's id in an ask.
const ASKED: &str = "claims a rule or wish of Joel's";

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
    // By the millisecond, and by id within one, so two claims sent in the
    // same second keep their order however the folders list them, and a
    // sender's oldest, the one it's asked about, never changes hands.
    found.sort_by(|(_, a, _), (_, b, _)| {
        (stamped_at(&a.id), &a.id).cmp(&(stamped_at(&b.id), &b.id))
    });
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
                let mut shown: String = short(&clean(claim));
                if shown.chars().count() > CLAIM_LONGEST {
                    shown = shown.chars().take(CLAIM_LONGEST).collect::<String>() + "...";
                }
                format!("[{id}] {shown} · {from}")
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

/// The ids of the replies in `sent` to an ask about a claim of his rule,
/// and the claims answered, by sender and short id. Only a claim this
/// measure asked about has an answer: any reply, after the ask, from the
/// cell it asked, re its ask or re the claim the ask names. None of them
/// is judged, since a cell dropping the claim names it, and in whatever
/// order they came, the claim is answered once. The ask, not the claim,
/// says who may answer, so an answer counts after its claim has left the
/// day. A reply before the ask, one re a message never asked about, or
/// one re any other of cadence's, is judged as any other.
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
    let (mut answers, mut answered) = (BTreeSet::new(), BTreeSet::new());
    for &m in sent {
        let Some(re) = m.re.as_deref().map(short_id) else {
            continue;
        };
        let claim = claim_of.get(re).copied().unwrap_or(re);
        if asked_before(claim, &m.from, stamped_at(&m.id)) {
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
    let names_him = |sentence: &str| words(&sentence.to_lowercase()).iter().any(|w| w == "joel");
    let opens_on_him = |sentence: &str| {
        let first = sentence.split_whitespace().next().map(bare);
        matches!(first.as_deref(), Some("he" | "his"))
    };
    // He, his or him stand for Joel in the claim's sentence, in one
    // naming him, or in the one after opening with them, as "He said so
    // on 22 Nov."; anywhere else they may be anyone, as "Sam agreed; on
    // 25 Nov he said so."
    let backs = |at: usize, by: usize| {
        let sentence = said[by];
        let he = by == at || names_him(sentence) || (by == at + 1 && opens_on_him(sentence));
        dated_by_him(sentence, he)
    };
    let backed =
        |at: usize| (at.saturating_sub(1)..(at + 2).min(said.len())).any(|by| backs(at, by));
    (0..said.len())
        .find(|&at| claims(said[at]) && !backed(at))
        .map(|at| said[at].to_owned())
}

/// `text` with its curly apostrophes made straight, the lines of cadence's
/// card it passes on taken out, as [`uncarded`] does, and its quotes put
/// by stand-ins, as [`unquoted`] does.
fn plain(text: &str) -> String {
    let text = text
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace(FILLERS, " ");
    unquoted(&uncarded(&text))
}

/// The marks that draw as a blank but are no space, as U+2800 or the
/// Hangul fillers: each is read as a space, so a claim spaced by them is
/// still one, and is given by its first words rather than whole.
const FILLERS: [char; 8] = [
    '\u{034f}', '\u{115f}', '\u{1160}', '\u{17b4}', '\u{17b5}', '\u{2800}', '\u{3164}', '\u{ffa0}',
];

/// `text` with no line of cadence's card in it: neither a claim the card
/// names, as `[0000000a] I broke Joel's rule. · T6E7X6`, nor a measure's
/// line, as `✓ pains Joel said that nobody has taken up: 0, at most 0` or
/// the drag's `Drag: ..., 3 against at most 0, ...`, nor what this
/// measure says it counts. The card is made to be passed on, and a cell
/// passing it on claims nothing of its own. Each is held to the card's
/// own shape, a short id of eight hex digits or a measure's figure, so a
/// line merely bracketed or ticked is still read.
fn uncarded(text: &str) -> String {
    let own = [SAYS, pains::SAYS];
    let plain = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric());
    let short = |id: &str| {
        id.len() == 8
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    let figured = |line: &str| {
        line.rsplit_once(": ")
            .is_some_and(|(_, figure)| figure.starts_with(|c: char| c.is_ascii_digit()))
    };
    let carded = |line: &str| {
        let line = line.trim_start_matches(|c: char| c.is_whitespace() || c == '-' || c == '*');
        let named = line
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("] "))
            .is_some_and(|(id, _)| short(id))
            && line
                .rsplit_once(" · ")
                .is_some_and(|(_, from)| plain(from.trim_end()));
        let measured = ["✓ ", "✕ ", "· "]
            .iter()
            .any(|mark| line.strip_prefix(mark).is_some_and(figured));
        let drag = line
            .strip_prefix("Drag: ")
            .is_some_and(|rest| rest.contains(" against at most "));
        named || measured || drag
    };
    let kept = text
        .lines()
        .filter(|line| !carded(line))
        .collect::<Vec<_>>()
        .join("\n");
    own.iter()
        .fold(kept, |kept, says| kept.replace(says, "..."))
}

/// `text` with its bold and italic marks taken out, and every span in
/// double quotes or backticks put by its quotes around three dots, so no
/// claim inside one is read and no stop inside one ends a sentence. A
/// quote left open is put back as written, since a stray quote mark would
/// otherwise hide every claim after it.
fn unquoted(text: &str) -> String {
    let mut plain = String::new();
    let mut open: Option<(char, char, String)> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        // A mark with a space or nothing after it opens no quote, as the
        // stray one in `I typed " by mistake. Joel wants "dark" mode.`,
        // which would otherwise pair with the next and hide what's between.
        let opens = chars.peek().is_some_and(|next| !next.is_whitespace());
        match open.as_mut() {
            Some((_, close, _)) if c == *close => {
                plain.push(c);
                plain.push_str("...");
                plain.push(c);
                open = None;
            }
            Some((_, _, quote)) => quote.push(c),
            None => match c {
                '"' | '`' if opens => open = Some((c, c, String::new())),
                '\u{201c}' if opens => open = Some((c, '\u{201d}', String::new())),
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

/// Whether `words` holds `phrase` as whole words, or with a plural "s"
/// or "es", so "Joel's calls" and "Joel's wishes" claim and "Joel's
/// calendar" doesn't, and with no denial just before it, as [`denied`]
/// reads one.
fn at_word(words: &str, phrase: &str) -> bool {
    words.match_indices(phrase).any(|(at, _)| {
        let before = words[..at].chars().next_back();
        let rest = &words[at + phrase.len()..];
        let plural = rest.strip_prefix("es").or_else(|| rest.strip_prefix('s'));
        let after = plural.unwrap_or(rest).chars().next();
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
/// time beside a word naming him, with no more than a few words of saying
/// between, and tied to his saying as the module's doc says. Any other
/// word between makes it the date of something else, as "Joel wants it
/// by 26 Nov" or "I broke Joel's rule at 10:08", and so does a word that
/// isn't his saying, as "At 10:08, Joel's rule was broken" or "I told him
/// at 10:08".
fn dated_by_him(sentence: &str, he: bool) -> bool {
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
    let said = |at: usize| {
        lower
            .get(at)
            .is_some_and(|word| SAID.contains(&word.as_str()))
    };
    // The word after him, past a "has" or "had", as "Joel has said".
    let says_next = |him: usize| {
        let next = lower.get(him + 1).map(String::as_str);
        said(him + 1 + usize::from(matches!(next, Some("has" | "had"))))
    };
    (0..tokens.len())
        .filter(|&at| HIM.contains(&lower[at].as_str()))
        .filter(|&at| he || lower[at].starts_with("joel"))
        .any(|him| {
            let named = lower[him].starts_with("joel");
            let to_him = him > 0 && TO_HIM.contains(&lower[him - 1].as_str());
            let after = (him + 1..tokens.len()).take(BESIDE + 1);
            let before = (0..him).rev().take(BESIDE + 1);
            // "I told Joel at 10:08" has only words of saying between, as
            // "Joel at 10:05 said" does: the date is his only when tied to
            // his saying, as "per Joel" ties it, or set straight after his
            // name.
            let past = |date: usize| date + if timed(date) { 1 } else { 2 };
            let per = him > 0 && lower[him - 1] == "per";
            let his_saying = |date: usize| {
                let between = him + 1..date;
                per || (named && !to_him && between.clone().all(|at| lower[at].is_empty()))
                    || between
                        .clone()
                        .any(|at| said(at) || RULES.contains(&lower[at].as_str()))
                    || said(past(date))
            };
            let follows = reached(after, starts, saying).is_some_and(his_saying);
            let leads = reached(before, ends, saying).is_some() && says_next(him);
            follows || leads
        })
}

/// Where, walking `path` away from his name, a word where `date` holds
/// comes before any word that isn't one of `saying`.
fn reached(
    path: impl IntoIterator<Item = usize>,
    date: impl Fn(usize) -> bool,
    saying: impl Fn(usize) -> bool,
) -> Option<usize> {
    path.into_iter()
        .find(|&at| date(at) || !saying(at))
        .filter(|&at| date(at))
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
