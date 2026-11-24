//! Whether the machine is held: a hold sent to every session, as before a
//! reboot, asks that nothing new starts until its sender ends it. The net
//! is no session, so it reads the mesh's messages itself, from the folder
//! `MERGE_NET_MESH` names, or the harness's under XDG's state. A hold
//! ends when its sender answers it or sends one in its place, or when the
//! machine boots after it was sent. The harness files older messages into
//! a folder for each day and may leave a hold above its answer, so both
//! levels are read: on 11 November one Monday hold, whose end had gone
//! into its day's folder, stood again and held the net for half an hour.
//!
//! Not every hold on the machine is about the net's work. On 19 November
//! holds on concluding quests and on reaching Slack stopped it for over
//! three hours while 21 merges landed. So a hold passes the net by only
//! when it names a subject that isn't the net's (`ELSEWHERE`) and none of
//! the net's work (`NETS_WORK`); any other hold stops it, so one that says
//! too little fails closed. The net's work is matched by the start of a
//! word (`NETS_WORK`), so "rebooted" and "deploying" count, and by a few
//! words matched whole (`NETS_WORDS`); a hold that stops everything, as
//! "start nothing", names the net's work too. A sentence saying what goes
//! on, as "it doesn't hold builds" or "Slack is fine", frees only what its
//! phrase is said of in its own clause (`FREES_AFTER`, `FREES_BEFORE`), so
//! a reboot beside it still holds, and frees nothing when it holds
//! something as well (`HOLDING`), as "documents are fine but start no
//! builds" does. Only a list after "doesn't hold" frees an event such as a
//! reboot (`EVENTS`). A renewal that doesn't restate its subject, as "no
//! quest" or "Slack only" beside it do, is about whatever the hold it
//! replaces was about, and stops the net when that hold can't be found.
//! This reads
//! the hold's words, a stopgap until a hold carries its scope in a field of
//! its own.
//!
//! A hold stands only in its window, as the mesh keeps it: from its
//! `starts` until its `ends`, each an ISO time, or for two hours from when
//! it was sent where it says no end (`LONGEST_MS`). On 24 November a hold
//! sent at 10:58 for a call from 12:15 to 13:10 held the net from 11:30,
//! once the hold before it ended, and it sat 33 minutes before the call. A
//! time that can't be read holds as though the hold had said none.

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The groups a hold for the whole machine goes to.
const EVERYONE: [&str; 2] = ["#machine", "#all"];

/// The longest a hold that says no end stands, in milliseconds, as the
/// mesh's peers keep it: past it, it holds nothing.
const LONGEST_MS: u64 = 2 * 60 * 60 * 1000;

/// Words naming a hold's subject that the net never touches. "Quest" is
/// none, since "for every quest" opens holds on anything.
const ELSEWHERE: [&str; 5] = ["conclude", "concluding", "retire", "retiring", "slack"];

/// How a sentence opens that says when its hold ends rather than what it
/// holds, as "It stays until the Mac's update lands" does.
const TERMS: [&str; 7] = [
    "it stays",
    "this stays",
    "the hold stays",
    "it lasts",
    "this lasts",
    "it ends when",
    "this ends when",
];

/// Words that, in a sentence saying when its hold ends, say it holds
/// something too, as "it stays until 22:00: start no builds" does. Words
/// of time, as "after" is, are no such word there.
const TERM_HOLDS: [&str; 12] = [
    "no", "nothing", "not", "never", "don", "won", "hold", "stop", "wait", "avoid", "leave",
    "pause",
];

/// The net's own names, which hold it in a sentence freeing nothing and
/// are neutral in one freeing something, since a hold names the net there
/// only to say it carries on.
const NETS_NAMES: [&str; 2] = ["merge-net", "merge net"];

/// The starts of words naming what the net does, what a hold on it would
/// be about, or a hold on everything.
const NETS_WORK: [&str; 41] = [
    "build", "rebuild", "eval", "offload", "test", "merg", "revert", "switch", "restart", "reboot",
    "suspend", "sleep", "shut", "power", "halt", "heavy", "park", "push", "upload", "fetch",
    "quiet", "load", "daemon", "deploy", "compil", "cargo", "disk", "memory", "stop", "paus",
    "freez", "drain", "job", "run", "start", "boot", "kernel", "upgrad", "roll", "firmware",
    "flash",
];

/// Phrases naming the net's work in words too common to count alone, as
/// "lands" and "down" are.
const NETS_PHRASES: [&str; 5] = ["on main", "to main", "goes down", "go down", "going down"];

/// Short words naming the net's work, matched whole, since their starts
/// begin everyday words too, as "gate" begins the Tool Gateway.
const NETS_WORDS: [&str; 10] = [
    "nix", "vm", "vms", "cpu", "net", "gate", "gates", "ci", "ram", "kvm",
];

/// Words that, in a sentence freeing something, say it holds something
/// too, so the sentence is read for what it holds after all.
const HOLDING: [&str; 20] = [
    "no", "nothing", "not", "never", "stop", "don", "won", "until", "till", "unless", "but",
    "except", "wait", "avoid", "hold", "while", "once", "after", "before", "from",
];

/// The starts of words naming what happens to the machine, which only a
/// list after "doesn't hold" frees: "the laptop reboots and Slack is fine"
/// holds.
const EVENTS: [&str; 13] = [
    "reboot", "boot", "suspend", "sleep", "shut", "power", "halt", "kernel", "upgrad", "roll",
    "firmware", "flash", "freez",
];

/// What a sentence says to leave alone what follows, up to its next such
/// phrase or clause (`AFTER_ENDS`): a list, whose items are at most `FEW`
/// words each, since a longer one may be a clause of its own. `HOLDING` is
/// read in what remains once these are taken out.
const FREES_AFTER: [&str; 3] = ["doesn't hold", "does not hold", "don't hold"];

/// Where what "doesn't hold" frees ends: a clause of its own begins.
const AFTER_ENDS: [&str; 6] = [", and ", " and ", ", so ", ", but ", ", while ", ", then "];

/// The most words an item of what "doesn't hold" frees may have, as "pi
/// host cuts" does; "the laptop reboots at 21:00" is a clause.
const FEW: usize = 3;

/// Words opening a clause rather than an item of a list, after its first:
/// "Slack, the laptop reboots" frees only Slack. An item of more than one
/// word naming the net's work is a clause too, as "the Mac reboots" is
/// with no article.
const CLAUSE_OPENS: [&str; 7] = ["the", "a", "an", "my", "our", "its", "this"];

/// The most words before "is fine" it frees, as "concluding or retiring a
/// document is fine" does.
const FEW_BEFORE: usize = 5;

/// Where what "is fine" frees begins, read back from it.
const BEFORE_BEGINS: [&str; 7] = [
    ", and ", " and ", ", so ", ", but ", ", while ", ", then ", ", ",
];

/// What a sentence says to leave alone what comes before it, back to its
/// last such phrase or clause (`BEFORE_BEGINS`), unless that is more than
/// `FEW_BEFORE` words or names one of the `EVENTS`.
const FREES_BEFORE: [&str; 6] = [
    "not held",
    "carry on",
    "carries on",
    "is fine",
    "are fine",
    "unaffected",
];

/// What of `sentence` is left holding once what it frees is taken out: the
/// words after "doesn't hold", or before "carries on", as far as the phrase
/// or clause beside it. A sentence still naming a holding word once its
/// phrases are gone is left whole, and what comes before "carries on" is
/// kept when it names an `event`.
fn held_part(
    sentence: &str,
    holding: impl Fn(&str) -> bool,
    event: impl Fn(&str) -> bool,
    work: impl Fn(&str) -> bool,
) -> String {
    let mut found: Vec<(usize, usize, bool)> = FREES_AFTER
        .iter()
        .map(|phrase| (phrase, true))
        .chain(FREES_BEFORE.iter().map(|phrase| (phrase, false)))
        .flat_map(|(phrase, after)| {
            sentence
                .match_indices(phrase)
                .map(move |(at, _)| (at, at + phrase.len(), after))
        })
        .collect();
    if found.is_empty() {
        return sentence.to_owned();
    }
    found.sort_unstable();
    let mut unphrased = sentence.to_owned();
    for phrase in FREES_AFTER.iter().chain(&FREES_BEFORE) {
        unphrased = unphrased.replace(phrase, " ");
    }
    if holding(&unphrased) {
        return sentence.to_owned();
    }
    let count = |part: &str| {
        part.split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .count()
    };
    let mut kept = vec![true; sentence.len()];
    for (index, &(start, end, after)) in found.iter().enumerate() {
        let (from, to) = if after {
            let phrase = found.get(index + 1).map_or(sentence.len(), |next| next.0);
            let clause = AFTER_ENDS
                .iter()
                .filter_map(|ends| sentence[end..].find(ends).map(|at| end + at))
                .min()
                .unwrap_or(sentence.len());
            // What it frees is a list, of items split by ", " and " or ";
            // it ends before the first item of more than `FEW` words, as a
            // clause spliced on is, or a later one that opens as a clause
            // does.
            let mut to = phrase.min(clause);
            let mut item = end;
            while item < to {
                let (stops, next) = [", ", " or "]
                    .iter()
                    .filter_map(|split| sentence[item..to].find(split).map(|at| (at, split.len())))
                    .min()
                    .map_or((to, to), |(at, len)| (item + at, item + at + len));
                // The item without the word or comma that splits it off.
                let part = &sentence[item..stops];
                let opens = part
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .find(|word| !word.is_empty())
                    .is_some_and(|word| CLAUSE_OPENS.contains(&word));
                let clause = count(part) > 1 && work(part);
                if count(part) > FEW || (item > end && opens) || clause {
                    to = item;
                }
                item = next;
            }
            (start, to)
        } else {
            let phrase = index.checked_sub(1).map_or(0, |last| found[last].1);
            let clause = BEFORE_BEGINS
                .iter()
                .filter_map(|begins| sentence[..start].rfind(begins).map(|at| at + begins.len()))
                .max()
                .unwrap_or(0);
            let from = phrase.max(clause).min(start);
            if count(&sentence[from..start]) > FEW_BEFORE || event(&sentence[from..start]) {
                continue;
            }
            (from, end)
        };
        kept[from.min(start)..to.max(end)].fill(false);
    }
    sentence
        .char_indices()
        .map(|(at, c)| if kept[at] { c } else { ' ' })
        .collect()
}

/// Whether a hold's `text` is about the net's work: a sentence that holds
/// something names some of it, or the hold names no subject the net is
/// known to be clear of.
fn about_the_net(text: &str) -> bool {
    // A typographic apostrophe is read as a plain one, so "doesn’t hold"
    // frees as "doesn't hold" does.
    let lower = text.to_ascii_lowercase().replace('\u{2019}', "'");
    let words = |part: &str| -> Vec<String> {
        part.split(|c: char| !c.is_ascii_alphanumeric())
            .map(str::to_owned)
            .collect()
    };
    let names = |part: &str, set: &[&str]| words(part).iter().any(|w| set.contains(&w.as_str()));
    let nets_work = |part: &str| {
        // Padded, so "on main" never matches "on maintenance".
        let spaced: Vec<String> = words(part).into_iter().filter(|w| !w.is_empty()).collect();
        let spaced = format!(" {} ", spaced.join(" "));
        NETS_PHRASES
            .iter()
            .any(|phrase| spaced.contains(&format!(" {phrase} ")))
            || words(part).iter().any(|word| {
                NETS_WORDS.contains(&word.as_str())
                    || NETS_WORK.iter().any(|start| word.starts_with(start))
            })
    };
    let event = |part: &str| {
        words(part)
            .iter()
            .any(|word| EVENTS.iter().any(|start| word.starts_with(start)))
    };
    // A colon splits no sentence, since it splits a time in two too. A
    // sentence saying when its hold ends is read only for a word that holds
    // something beside the net's work, since its end is often a switch.
    let holds_its_work = lower.split(['.', ';', '!', '?']).any(|sentence| {
        // Its own opening, as "the hold stays", holds nothing.
        if let Some(term) = TERMS
            .iter()
            .find(|term| sentence.trim_start().starts_with(*term))
        {
            let rest = &sentence.trim_start()[term.len()..];
            return names(rest, &TERM_HOLDS) && nets_work(rest);
        }
        // The net's name holds it in a sentence freeing nothing, and is
        // neutral in one freeing something, as "so merge-net and the
        // trains carry on" is.
        let frees = FREES_AFTER
            .iter()
            .chain(&FREES_BEFORE)
            .any(|p| sentence.contains(p));
        if !frees && NETS_NAMES.iter().any(|name| sentence.contains(name)) {
            return true;
        }
        let mut unnamed = sentence.to_owned();
        for name in NETS_NAMES {
            unnamed = unnamed.replace(name, "it");
        }
        let held = held_part(&unnamed, |rest| names(rest, &HOLDING), event, nets_work);
        // Kept whole, the sentence holds something too, and its name for
        // the net is what it holds, as "Slack is fine, but hold merge-net
        // until 22:00" does.
        (held == unnamed && NETS_NAMES.iter().any(|name| sentence.contains(name)))
            || nets_work(&held)
    });
    holds_its_work || !names(&lower, &ELSEWHERE)
}

/// Whether `text` restates what it holds: "no quest", or "only" right
/// beside a subject the net never touches, as "conclude or retire no
/// quest" and "this covers Slack only" do. "My Slack hold, no change"
/// restates nothing.
fn restates(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.windows(2).any(|pair| {
        (pair[0] == "no" && pair[1].starts_with("quest"))
            || (ELSEWHERE.contains(&pair[0]) && pair[1] == "only")
            || (pair[0] == "only" && ELSEWHERE.contains(&pair[1]))
    })
}

/// Whether `hold` is about the net's work, or is a renewal saying too
/// little to stand alone whose hold it replaces is, or can't be found. A
/// renewal restating its subject, as "conclude or retire no quest" does,
/// says all it holds, so a switch the hold it replaces mentioned in passing
/// is dropped; one naming it only, as "renewing the conclude hold until
/// 21:00" does, keeps what that hold held.
fn about(hold: &Value, messages: &[Value]) -> bool {
    let mut at = hold;
    // A chain of renewals is short; the bound keeps a loop of them finite.
    for _ in 0..=messages.len().min(64) {
        let text = at["text"].as_str().unwrap_or_default();
        if about_the_net(text) {
            return true;
        }
        if restates(text) || !at["supersedes"].is_string() {
            return false;
        }
        let renewed = messages
            .iter()
            .find(|earlier| earlier["from"] == at["from"] && earlier["id"] == at["supersedes"]);
        match renewed {
            Some(earlier) => at = earlier,
            None => return true,
        }
    }
    true
}

/// The mesh's messages folder.
pub fn messages() -> PathBuf {
    if let Some(folder) = std::env::var_os("MERGE_NET_MESH").filter(|folder| !folder.is_empty()) {
        return PathBuf::from(folder);
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|folder| !folder.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        );
    state.join("pi/agentic-harness.pi/mesh-workflow/messages")
}

/// When the machine booted, in milliseconds since the epoch, from
/// /proc/stat's `btime`; none where it can't be read.
pub fn booted() -> Option<u64> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let seconds: u64 = stat
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    Some(seconds * 1000)
}

/// What stops the net: a hold's text, or a pause's, and who it's from, so
/// cadence can ask the holder when one stops the net too long.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub why: String,
    /// The mesh address of the session whose hold it is, or nothing where
    /// none holds it, as a pause file holds it.
    pub from: Option<String>,
}

/// The first hold on the whole machine still standing at `now` among the
/// messages in `folder` sent since `since` (both milliseconds, as each
/// file's name begins).
pub fn standing(folder: &Path, since: u64, now: u64) -> Option<Held> {
    let entries: Vec<fs::DirEntry> = fs::read_dir(folder).ok()?.filter_map(Result::ok).collect();
    let days: Vec<fs::DirEntry> = entries
        .iter()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|day| fs::read_dir(day.path()).ok())
        .flat_map(|day| day.filter_map(Result::ok))
        .collect();
    let mut sent: Vec<(u64, Value)> = days
        .into_iter()
        .chain(entries)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let at: u64 = name.split('-').next()?.parse().ok()?;
            (at >= since).then_some((at, entry.path()))
        })
        .filter_map(|(at, path)| {
            let text = fs::read_to_string(path).ok()?;
            Some((at, serde_json::from_str(&text).ok()?))
        })
        .collect();
    sent.sort_by_key(|(at, _)| *at);
    let messages: Vec<Value> = sent.into_iter().map(|(_, message)| message).collect();
    let mut said = SAID.lock().unwrap_or_else(PoisonError::into_inner);
    for hold in ahead(&messages, now) {
        let id = hold["id"].as_str().unwrap_or_default();
        if said
            .get_or_insert_with(HashSet::new)
            .insert(format!("{id} ahead"))
        {
            let starts = hold["starts"].as_str().unwrap_or_default();
            eprintln!("merge-net: hold {id} holds nothing until {starts}, when it starts");
        }
    }
    for hold in passed_by(&messages, now) {
        let id = hold["id"].as_str().unwrap_or_default();
        if said.get_or_insert_with(HashSet::new).insert(id.to_owned()) {
            // A newline in a hold would otherwise read as a journal line of
            // its own.
            let plain = |said: &str, most: usize| -> String {
                said.chars()
                    .take(most)
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .collect()
            };
            let text = plain(hold["text"].as_str().unwrap_or_default(), 240);
            let id = plain(id, 40);
            eprintln!("merge-net: passing by hold {id}, not about its work: {text}");
        }
    }
    held_by(&messages, now)
}

/// The holds the net has said it passes by. A misread hold would otherwise
/// be as silent as the stall it fixes, so each is said once, in the net's
/// journal.
static SAID: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// The holds on the whole machine among `messages` standing at `now`: in
/// their window, and answered or replaced by no later one from their
/// sender.
fn standing_holds(messages: &[Value], now: u64) -> impl Iterator<Item = &Value> {
    unended(messages).filter(move |hold| in_window(hold, now))
}

/// The holds on the whole machine among `messages` sent ahead, which start
/// after `now` and so hold nothing yet.
fn ahead(messages: &[Value], now: u64) -> impl Iterator<Item = &Value> {
    unended(messages).filter(move |hold| time(hold, "starts").is_some_and(|starts| starts > now))
}

/// Whether `now` falls in `hold`'s window: from its start, or always where
/// it says none, until its end, or `LONGEST_MS` after it was sent where it
/// says none. A hold whose sending time can't be read has no end.
fn in_window(hold: &Value, now: u64) -> bool {
    let started = time(hold, "starts").is_none_or(|starts| starts <= now);
    let ends =
        time(hold, "ends").or_else(|| time(hold, "at").map(|at| at.saturating_add(LONGEST_MS)));
    started && ends.is_none_or(|ends| now < ends)
}

/// The time `hold`'s `field` says, in milliseconds since 1970, or none
/// where it says none or can't be read.
fn time(hold: &Value, field: &str) -> Option<u64> {
    hold[field].as_str().and_then(iso_ms)
}

/// The milliseconds since 1970 of an ISO time as the mesh writes one,
/// `2026-11-24T17:14:15.133Z`, or with an offset such as `-05:00`; none
/// for anything else.
fn iso_ms(text: &str) -> Option<u64> {
    let (date, clock) = text.split_once('T')?;
    let mut parts = date.splitn(3, '-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (parts.next()??, parts.next()??, parts.next()??);
    let zone = clock.find(['Z', '+', '-'])?;
    let (clock, offset) = clock.split_at(zone);
    let offset_s = match offset.split_at(1) {
        ("Z", "") => 0,
        (sign, rest) if sign == "+" || sign == "-" => {
            let (hours, minutes) = rest.split_once(':')?;
            let span = hours.parse::<i64>().ok()? * 3600 + minutes.parse::<i64>().ok()? * 60;
            if sign == "-" { -span } else { span }
        }
        _ => return None,
    };
    let (whole, fraction) = clock.split_once('.').unwrap_or((clock, "0"));
    if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let millis: i64 = format!("{:0<3}", &fraction[..fraction.len().min(3)])
        .parse()
        .ok()?;
    let mut parts = whole.splitn(3, ':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (parts.next()??, parts.next()??, parts.next()??);
    let fits = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && (0..24).contains(&hour)
        && (0..60).contains(&minute)
        && (0..=60).contains(&second);
    if !fits {
        return None;
    }
    let seconds = days_since_1970(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from((seconds - offset_s) * 1000 + millis).ok()
}

/// The days from 1970's first day to `year`, `month` and `day`, by Howard
/// Hinnant's civil-to-days arithmetic, as terminal-kit's `time` goes back.
fn days_since_1970(year: i64, month: i64, day: i64) -> i64 {
    // Counted from 1 March, so a leap day ends its year.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468
}

/// The holds on the whole machine among `messages` that no later one from
/// its sender answers or replaces, whatever their window.
fn unended(messages: &[Value]) -> impl Iterator<Item = &Value> {
    messages.iter().filter(|hold| {
        let everyone = hold["to"].as_array().is_some_and(|to| {
            to.iter()
                .any(|group| EVERYONE.iter().any(|name| group == name))
        });
        let ended = || {
            messages.iter().any(|later| {
                later["from"] == hold["from"]
                    && (later["re"] == hold["id"] || later["supersedes"] == hold["id"])
            })
        };
        hold["intent"] == "hold" && everyone && !ended()
    })
}

/// The first hold on the whole machine among `messages` still standing at
/// `now` and about the net's work.
fn held_by(messages: &[Value], now: u64) -> Option<Held> {
    standing_holds(messages, now)
        .find(|hold| about(hold, messages))
        .map(|hold| Held {
            why: hold["text"].as_str().unwrap_or_default().to_owned(),
            from: hold["from"]
                .as_str()
                .filter(|from| !from.is_empty())
                .map(str::to_owned),
        })
}

/// The text of `held_by`'s hold, which is all most cases read, at a time
/// no window in them leaves out.
#[cfg(test)]
fn still_held(messages: &[Value]) -> Option<String> {
    held_by(messages, 0).map(|held| held.why)
}

/// The holds on the whole machine among `messages` still standing that the
/// net passes by.
fn passed_by(messages: &[Value], now: u64) -> Vec<&Value> {
    standing_holds(messages, now)
        .filter(|hold| !about(hold, messages))
        .collect()
}

#[cfg(test)]
mod tests;
