//! A line the hour holds to a fix shown closes once its figure is back in
//! and its owner has shown the fix, never on its figure alone: of 1,282
//! findings closed by 23 November, 1,088 closed on a figure gone quiet,
//! which shows no fix. The owner shows it with `cadence owe --done`, or
//! with a reply to the hour's question naming the change, once that change
//! runs here; a reply `None: <why>` closes it as needing none, counted
//! apart. One back a week with no word closes, counted back unexplained,
//! and a fall again reopens its own line. 1JQPPC, the loop's keeper, ruled
//! this shape on 27 November.

use std::collections::{BTreeMap, BTreeSet};

use super::{Class, DAY, Entry, FINDINGS, QUEUE, UNTESTED, causes, rounds, routed, told_late};
use crate::findings::shown::{Ended, How, Live, Said, Word};
use crate::mesh::{Message, stamped_at};

#[cfg(test)]
mod tests;

/// How many days a line may be back in with no word of its fix before it
/// closes, counted back unexplained: long enough for an owner to answer,
/// short enough that the ledger can't grow for good.
const UNEXPLAINED_DAYS: i64 = 7;

/// How much of a reply a line keeps as its word, in characters.
const SAID_AT_MOST: usize = 300;

/// The lines with a rule of their own, each closing on what shows its fix
/// already: a queue on its next change, a miss and an untested miss once
/// a check merges and runs, a finding of the work system's on its owner's
/// landing note, and a class of cause held for the week once its fix is
/// named. A merge's fruitless rounds and a date told late are events that
/// age out of their measures, with no fix to show.
const OWN_RULE: [&str; 8] = [
    QUEUE,
    causes::KEY,
    "miss:",
    "miss-held:",
    FINDINGS,
    UNTESTED,
    rounds::FRUITLESS,
    told_late::KEY,
];

/// Whether the hour's line filed as `key` on `axis` closes only on a fix
/// shown: every drag and red line the hour files, but those with a rule of
/// their own and those filed for each line's owner. A line routed to its
/// owner names each cell or file it holds, which goes only once that one
/// is dealt with, so its going shows the fix; and another of theirs gone
/// red within the day is asked anew, which a line held open would stop.
#[must_use]
pub fn held_to_a_fix(key: &str, axis: &str) -> bool {
    let routed = [Class::Clean, Class::Cells]
        .into_iter()
        .any(|class| routed(class) && class.name() == axis);
    !routed
        && key
            .strip_prefix("hour:")
            .is_some_and(|key| !OWN_RULE.iter().any(|rule| key.starts_with(rule)))
}

/// What an owner is told of how a line filed as `key` closes, naming the
/// key as `cadence owe` reads it, quoted so the shell reads all of it.
pub(super) fn closes_by(key: &str) -> String {
    let done = causes::shell_quoted(key);
    format!(
        "It closes once you name its fix and it's back in: run `cadence owe --done {done} \
         'FIX'`, or answer naming the change, as `core #123`, which counts once it runs here; \
         answer `None:` and why if it needs no fix."
    )
}

/// Closes `entry`, open and held to a fix shown, at the hour run on `date`
/// at `now`, in seconds, once `back` says its figure is back in and its fix
/// is shown: named with `--done`, or a change its owner's reply named
/// running here. One whose owner said it needs no fix closes too, and one
/// back `UNEXPLAINED_DAYS` with no word, each counted apart. Until then it
/// is marked back, so it goes late to nobody, and as it is open it is
/// asked nothing more; a fall again forgets when it came back.
pub(super) fn close(entry: &mut Entry, back: bool, date: &str, now: i64) {
    if !back {
        entry.back = None;
        return;
    }
    let since = entry.back.get_or_insert_with(|| date.to_owned());
    let unexplained = days_between(since, date).is_some_and(|days| days >= UNEXPLAINED_DAYS);
    let how = match entry.shown.word.as_ref().map(|word| &word.said) {
        Some(
            Said::Fix(_)
            | Said::Change {
                live: Live::Runs, ..
            },
        ) => Some(How::Fixed),
        Some(Said::Unneeded(_)) => Some(How::Unneeded),
        _ => unexplained.then_some(How::Unexplained),
    };
    if let Some(how) = how {
        entry.open = false;
        entry.shown.ended = Some(Ended { how, at: now });
    }
}

/// The days from the date `from` to the date `to`, if both read as dates.
fn days_between(from: &str, to: &str) -> Option<i64> {
    let from = from.parse::<jiff::civil::Date>().ok()?;
    let to = to.parse::<jiff::civil::Date>().ok()?;
    Some(i64::from(from.until(to).ok()?.get_days()))
}

/// The open lines held to a fix shown in `ledger`.
fn held(ledger: &[Entry]) -> impl Iterator<Item = &Entry> {
    ledger
        .iter()
        .filter(|e| e.open && super::is_asked(e) && held_to_a_fix(&e.key, &e.axis))
}

/// The ids of the questions the open lines held to a fix shown were asked
/// in, whose replies may name their fixes.
#[must_use]
pub fn asked(ledger: &[Entry]) -> BTreeSet<String> {
    held(ledger).flat_map(|e| e.asked.iter().cloned()).collect()
}

/// Who may name the fix of a line asked in one of `questions`: its owner,
/// each it was put to, and each its question went to.
#[must_use]
pub fn senders(ledger: &[Entry], questions: &[Message]) -> BTreeSet<String> {
    let mut senders: BTreeSet<String> = held(ledger)
        .flat_map(|e| e.owner.iter().chain(&e.put_to).cloned())
        .collect();
    senders.extend(questions.iter().flat_map(to_whom));
    senders
}

/// Each address `question` went to, without the mesh's `@`.
fn to_whom(question: &Message) -> impl Iterator<Item = String> + '_ {
    question
        .to
        .iter()
        .map(|to| to.strip_prefix('@').unwrap_or(to).to_owned())
}

/// Takes each of `replies` to one of the hour's `questions` as the word
/// of the fix of the line it answers, the latest word standing: the whole
/// reply where the question held one line, else the item numbered as the
/// question numbered the line. A word counts only from the line's owner,
/// whoever it was put to, or whoever its question went to; anyone else's
/// is left out, and said.
pub fn hear(ledger: &mut [Entry], questions: &[Message], replies: &[Message]) {
    let by_id: BTreeMap<&str, &Message> = questions.iter().map(|q| (q.id.as_str(), q)).collect();
    let mut replies: Vec<&Message> = replies.iter().collect();
    replies.sort_by_key(|reply| stamped_at(&reply.id));
    for reply in replies {
        let Some(question) = reply.re.as_deref().and_then(|re| by_id.get(re)) else {
            continue;
        };
        let at = stamped_at(&reply.id).unwrap_or_default() / 1000;
        for entry in ledger.iter_mut().filter(|e| {
            e.open
                && super::is_asked(e)
                && held_to_a_fix(&e.key, &e.axis)
                && e.asked.contains(&question.id)
        }) {
            let Some(said) =
                item_for(&question.text, &reply.text, &reply.from, &entry.key).and_then(word_of)
            else {
                continue;
            };
            let may = entry.owner.as_deref() == Some(reply.from.as_str())
                || entry.put_to.contains(&reply.from)
                || to_whom(question).any(|to| to == reply.from);
            if !may {
                eprintln!(
                    "cadence: {} answered {} of {}, whose owner it isn't, so its word is left out",
                    reply.from.replace(char::is_control, "?"),
                    reply.id,
                    entry.key
                );
                continue;
            }
            if entry.shown.word.as_ref().is_none_or(|word| word.at < at) {
                entry.shown.word = Some(Word { said, at });
            }
        }
    }
}

/// What of `reply`, sent by `from`, answers the line filed as `key` in
/// `question`: all of it where the question held one line naming the key,
/// else the item numbered as the question numbered the line naming it.
/// A reply opening with its sender's address is read past it.
fn item_for(question: &str, reply: &str, from: &str, key: &str) -> Option<String> {
    let needle = flat(&causes::shell_quoted(key));
    let reply = reply
        .strip_prefix(from)
        .and_then(|rest| rest.strip_prefix(':'))
        .unwrap_or(reply)
        .trim();
    let numbered: Vec<(usize, &str)> = question
        .split("\n\n")
        .filter_map(|paragraph| {
            let (number, said) = paragraph.split_once(". ")?;
            Some((number.parse().ok()?, said))
        })
        .collect();
    if numbered.is_empty() {
        return question.contains(&needle).then(|| reply.to_owned());
    }
    let (number, _) = numbered.iter().find(|(_, said)| said.contains(&needle))?;
    items(reply)
        .into_iter()
        .find(|(n, _)| n == number)
        .map(|(_, item)| item)
}

/// `text` with its whitespace runs made one space, as a question is sent.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The items of a reply answering line by line, each by the number it
/// opens with, as `2.`, `2:` or `2)`, with the lines up to the next.
fn items(reply: &str) -> Vec<(usize, String)> {
    let mut items: Vec<(usize, String)> = Vec::new();
    for line in reply.lines() {
        let opened = line.trim_start().trim_start_matches(['-', '*', ' ']);
        let digits = opened.len()
            - opened
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .len();
        let rest = opened[digits..].strip_prefix(['.', ':', ')']);
        match (opened[..digits].parse::<usize>(), rest) {
            (Ok(number), Some(rest)) => items.push((number, rest.trim().to_owned())),
            _ => {
                if let Some((_, item)) = items.last_mut() {
                    item.push(' ');
                    item.push_str(line.trim());
                }
            }
        }
    }
    items
}

/// The word an item of a reply says of its line's fix: none needed, as
/// `None:` and why, or the change that fixed it, as `core #123`. Anything
/// else names no fix. Each keeps at most `SAID_AT_MOST` of its words.
fn word_of(item: String) -> Option<Said> {
    let item = flat(item.trim_start_matches(['*', '_', ' ']));
    let said = |text: &str| text.trim().chars().take(SAID_AT_MOST).collect::<String>();
    if item
        .get(..5)
        .is_some_and(|opens| opens.eq_ignore_ascii_case("none:"))
    {
        return Some(Said::Unneeded(said(&item[5..])));
    }
    let names_a_change = item
        .match_indices('#')
        .any(|(at, _)| item[at + 1..].starts_with(|c: char| c.is_ascii_digit()));
    names_a_change.then(|| Said::Change {
        text: said(&item),
        live: Live::No,
    })
}

/// Each reply naming a change that an open line keeps as its word, with
/// when it was said, once, for the hour to judge whether it runs here.
#[must_use]
pub fn changes(ledger: &[Entry]) -> Vec<(String, i64)> {
    let mut said: Vec<(String, i64)> = held(ledger)
        .filter_map(|e| match e.shown.word.as_ref()? {
            Word {
                said: Said::Change { text, .. },
                at,
            } => Some((text.clone(), *at)),
            _ => None,
        })
        .collect();
    said.sort();
    said.dedup();
    said
}

/// Marks each open line's change as `live` judges it; one it doesn't hold
/// stays as it was.
pub fn judge(ledger: &mut [Entry], live: &BTreeMap<String, Live>) {
    for entry in ledger.iter_mut().filter(|e| e.open) {
        if let Some(Word {
            said: Said::Change { text, live: was },
            ..
        }) = entry.shown.word.as_mut()
            && let Some(now) = live.get(text.as_str())
        {
            *was = *now;
        }
    }
}

/// The card's line of the lines held to a fix shown at `now`, in seconds:
/// those back with their fix unnamed, those whose named fix doesn't run
/// here yet, and those the week closed on a fix shown, as needing none, or
/// back unexplained. None while every count is nought.
#[must_use]
pub fn tally(ledger: &[Entry], now: i64) -> Option<String> {
    let open: Vec<&Entry> = held(ledger).collect();
    let awaiting = open.iter().filter(|e| e.shown.awaits_switch(now)).count();
    let unnamed = open
        .iter()
        .filter(|e| e.back.is_some() && !e.shown.awaits_switch(now))
        .count();
    let week = |how: How| {
        ledger
            .iter()
            .filter(|e| !e.open && held_to_a_fix(&e.key, &e.axis))
            .filter_map(|e| e.shown.ended)
            .filter(|ended| ended.how == how && ended.at > now - UNEXPLAINED_DAYS * DAY)
            .count()
    };
    let counts = [
        (unnamed, "back, its fix unnamed"),
        (awaiting, "its fix named, not yet live"),
        (week(How::Fixed), "closed on a fix shown this week"),
        (week(How::Unneeded), "back, no fix needed, this week"),
        (week(How::Unexplained), "back unexplained this week"),
    ];
    let said: Vec<String> = counts
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("{n} {what}"))
        .collect();
    (!said.is_empty()).then(|| format!("Lines held to a fix shown: {}.\n", said.join(", ")))
}
