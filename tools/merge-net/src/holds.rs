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

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The groups a hold for the whole machine goes to.
const EVERYONE: [&str; 2] = ["#machine", "#all"];

/// Words naming a hold's subject that the net never touches. "Quest" is
/// none, since "for every quest" opens holds on anything.
const ELSEWHERE: [&str; 5] = ["conclude", "concluding", "retire", "retiring", "slack"];

/// Words that, right beside a subject, restate what is held, as "conclude
/// or retire no quest" and "send Slack only through" do; "my conclude
/// hold, no change" restates nothing.
const RESTATES: [&str; 5] = ["no", "nothing", "not", "never", "only"];

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
const HOLDING: [&str; 19] = [
    "no", "nothing", "not", "never", "stop", "don", "won", "until", "till", "unless", "but",
    "except", "wait", "avoid", "hold", "while", "once", "after", "before",
];

/// The starts of words naming what happens to the machine, which only a
/// list after "doesn't hold" frees: "the laptop reboots and Slack is fine"
/// holds.
const EVENTS: [&str; 13] = [
    "reboot", "boot", "suspend", "sleep", "shut", "power", "halt", "kernel", "upgrad", "roll",
    "firmware", "flash", "freez",
];

/// What a sentence says to leave alone what follows, up to its next such
/// phrase or clause (`AFTER_ENDS`). `HOLDING` is read in what remains once
/// these are taken out.
const FREES_AFTER: [&str; 3] = ["doesn't hold", "does not hold", "don't hold"];

/// Where what "doesn't hold" frees ends: a clause of its own begins.
const AFTER_ENDS: [&str; 6] = [", and ", " and ", ", so ", ", but ", ", while ", ", then "];

/// Where what "is fine" frees begins, read back from it. "And" is none, so
/// "merge-net and the trains carry on" frees both.
const BEFORE_BEGINS: [&str; 5] = [", and ", ", so ", ", but ", ", while ", ", then "];

/// What a sentence says to leave alone what comes before it, back to its
/// last such phrase or clause (`BEFORE_BEGINS`), unless that names one of
/// the `EVENTS`.
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
    let mut kept = vec![true; sentence.len()];
    for (index, &(start, end, after)) in found.iter().enumerate() {
        let (from, to) = if after {
            let phrase = found.get(index + 1).map_or(sentence.len(), |next| next.0);
            let clause = AFTER_ENDS
                .iter()
                .filter_map(|ends| sentence[end..].find(ends).map(|at| end + at))
                .min()
                .unwrap_or(sentence.len());
            (start, phrase.min(clause))
        } else {
            let phrase = index.checked_sub(1).map_or(0, |last| found[last].1);
            let clause = BEFORE_BEGINS
                .iter()
                .filter_map(|begins| sentence[..start].rfind(begins).map(|at| at + begins.len()))
                .max()
                .unwrap_or(0);
            let from = phrase.max(clause).min(start);
            if event(&sentence[from..start]) {
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
    let holds_its_work = lower
        .split(['.', ';', '!', '?', ':'])
        .any(|sentence| nets_work(&held_part(sentence, |rest| names(rest, &HOLDING), event)));
    holds_its_work || !names(&lower, &ELSEWHERE)
}

/// Whether some sentence of `text` names a subject the net never touches
/// and restates what is held of it.
fn restates(text: &str) -> bool {
    text.to_ascii_lowercase()
        .split(['.', ';', '!', '?', ':'])
        .any(|sentence| {
            let words: Vec<&str> = sentence
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|word| !word.is_empty())
                .collect();
            words.windows(2).any(|pair| {
                pair.iter().any(|word| ELSEWHERE.contains(word))
                    && pair.iter().any(|word| RESTATES.contains(word))
            })
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

/// The text of the first hold on the whole machine still standing among
/// the messages in `folder` sent since `since` (milliseconds, as each
/// file's name begins).
pub fn standing(folder: &Path, since: u64) -> Option<String> {
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
    for hold in passed_by(&messages) {
        let id = hold["id"].as_str().unwrap_or_default();
        if said.get_or_insert_with(HashSet::new).insert(id.to_owned()) {
            let text: String = hold["text"]
                .as_str()
                .unwrap_or_default()
                .chars()
                .take(120)
                .collect();
            eprintln!("merge-net: passing by hold {id}, not about its work: {text}");
        }
    }
    still_held(&messages)
}

/// The holds the net has said it passes by. A misread hold would otherwise
/// be as silent as the stall it fixes, so each is said once, in the net's
/// journal.
static SAID: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// The holds on the whole machine among `messages` that no later one from
/// its sender answers or replaces.
fn standing_holds(messages: &[Value]) -> impl Iterator<Item = &Value> {
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

/// The first hold on the whole machine among `messages` still standing and
/// about the net's work.
pub fn still_held(messages: &[Value]) -> Option<String> {
    standing_holds(messages)
        .find(|hold| about(hold, messages))
        .map(|hold| hold["text"].as_str().unwrap_or_default().to_owned())
}

/// The holds on the whole machine among `messages` still standing that the
/// net passes by.
fn passed_by(messages: &[Value]) -> Vec<&Value> {
    standing_holds(messages)
        .filter(|hold| !about(hold, messages))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hold(id: &str, from: &str, to: &str) -> Value {
        json!({ "id": id, "from": from, "to": [to], "intent": "hold", "text": id })
    }

    fn saying(text: &str) -> Value {
        json!({ "id": "h", "from": "A", "to": ["#machine"], "intent": "hold", "text": text })
    }

    // The four holds that stopped the net from 15:26 to past 18:35 on 19
    // November, while 21 merges landed: none of them is about its work.
    #[test]
    fn a_hold_on_concluding_or_on_slack_leaves_the_net_alone() {
        for text in [
            "P9R1KI hold: conclude or retire no quest, on either machine, until I end this \
             hold. Concluded quests' folders are being deleted. Everything else carries on.",
            "P9R1KI, renewing the conclude hold: conclude or retire no quest, on either \
             machine, until I end this hold. Documents are fine. The quest-share hotfix \
             lands about 18:30.",
            "P9R1KI, Joel's rule from 17:29, on both work machines: read and send Slack only \
             through the Tool Gateway's slack tools (mcp__tool_gateway slack_*). Use no \
             `slack` tool from the harness, and no direct slack.com call, until I end this \
             hold.",
            // A hold that says what it leaves alone names the net's work
            // only to free it.
            "P9R1KI, renewing the Slack hold: read and send Slack only through the Tool \
             Gateway's slack tools. This covers Slack only. It doesn't hold builds, merges, \
             switches or anything else, so merge-net and the trains carry on.",
            "WCJQSG (renewing 45a21014): conclude or retire no quest, on either machine, \
             until I end this hold. Concluding or retiring a document is fine. This covers \
             concluding and retiring quests only. It doesn't hold builds, merges, switches, \
             reboots, uploads, pi host cuts or anything else. It stays until the quest-share \
             hotfix is live on both machines (about 20:45) and the restore of the 16 \
             set-aside quests is checked, on the laptop by about 21:30.",
        ] {
            assert_eq!(still_held(&[saying(text)]), None, "held by: {text}");
        }
    }

    // The 20:07 renewal as sent, with a typographic apostrophe, passes.
    #[test]
    fn a_typographic_apostrophe_frees_as_a_plain_one_does() {
        let text = "WCJQSG: conclude or retire no quest. It doesn\u{2019}t hold builds, merges, \
                    switches or reboots.";
        assert_eq!(still_held(&[saying(text)]), None);
    }

    // Each case is read with a subject the net never touches beside it too,
    // so what holds it is the net's work it names, not a hold naming none.
    #[test]
    fn a_hold_on_the_nets_work_still_stops_it() {
        let subjected = |text: &str| format!("Conclude no quest. {text}");
        for text in [
            // A freeing phrase frees only its own clause, and only a list
            // after "doesn't hold" frees a reboot.
            "It doesn't hold Slack, and the laptop reboots at 21:00.",
            "It doesn't hold Slack and the laptop reboots at 21:00.",
            "The laptop reboots tonight and Slack is fine.",
            "Rebooting the laptop is fine.",
            // A freeing sentence saying when holds something after all.
            "Switches are fine once the hotfix is live.",
            "Merges carry on after 21:00.",
            "Builds are fine before 21:00, not after.",
            "Builds are fine till 21:00.",
            "Merges carry on unless Joel says so.",
            "Switches are fine while Joel is away.",
        ] {
            let text = subjected(text);
            assert_eq!(
                still_held(&[saying(&text)]),
                Some(text.clone()),
                "not held by: {text}"
            );
        }
        for text in [
            "16V6F4, for every quest: the laptop rebooted; start no rebuild until I end this.",
            "P9R1KI, for every quest: Joel is shutting the laptop down. Start nothing.",
            "P9R1KI, for every quest: we're deploying the core lock, so hold off.",
            // A sentence that frees one thing and holds another holds.
            "P9R1KI, for every quest: documents are fine but start no builds.",
            "P9R1KI, for every quest: don't hold back, stop every build now.",
            "P9R1KI, for every quest: Slack is fine, except that nothing merges until 21:00.",
            "16V6F4: the input update's switch restarts nix-daemon on the laptop. Start no new \
             nix build, eval or nix-offload there run until I end this hold.",
            "Integration (16V6F4): the laptop switches now, and it restarts nix-daemon.",
            "P9R1KI: Joel is rebooting the laptop in about 15 minutes. Park anything that would \
             die: local VM tests, gates.",
            "P9R1KI, for every quest: merge nothing into any repository, on either machine, \
             until I end this hold at 17:00.",
            "P9R1KI, for every quest: Joel is on a call. Start nothing heavy on the laptop.",
        ] {
            for text in [text.to_owned(), subjected(text)] {
                assert_eq!(
                    still_held(&[saying(&text)]),
                    Some(text.clone()),
                    "not held by: {text}"
                );
            }
        }
        for text in [
            // A hold that names neither its subject nor the net's work is
            // read as being about everything, so the net fails closed.
            "P9R1KI: stop now, until I end this hold.",
            // One that names a conclude and the net's work is about both.
            "P9R1KI: conclude no quest, and start no build, until I end this hold.",
            // A sentence freeing one thing frees nothing in the others.
            "P9R1KI: conclude no quest and start no VM tests; documents are fine. \
             Everything else carries on.",
            // "For every quest" names no subject, so a hold worded in no
            // word the net knows still stops it.
            "P9R1KI, for every quest: Joel is flashing new firmware. Hold off for now.",
            "P9R1KI, for every quest: hold everything until 21:00.",
            // What a sentence frees is what its phrase is said of, so a
            // reboot beside it still holds.
            "P9R1KI: retire no quest. Slack is fine, and the laptop reboots at 21:00.",
            "P9R1KI: retire no quest. It doesn't hold Slack; the laptop goes down at 21:00.",
            "P9R1KI: conclude nothing. Documents are fine till the kernel upgrade.",
            "P9R1KI: conclude nothing while the core lock lands on main.",
            "P9R1KI: retire no quest. A rollback of the laptop is coming; boot nothing new.",
        ] {
            assert_eq!(
                still_held(&[saying(text)]),
                Some(text.to_owned()),
                "not held by: {text}"
            );
        }
    }

    // A short renewal keeps the subject of the hold it replaces, and a hold
    // the net passes by doesn't hide one standing after it.
    #[test]
    fn a_renewal_keeps_what_it_renews_and_a_later_hold_still_counts() {
        let first = json!({ "id": "h1", "from": "A", "to": ["#machine"], "intent": "hold",
            "text": "A: start no VM tests until I end this." });
        let renewed = json!({ "id": "h2", "from": "A", "to": ["#machine"], "intent": "hold",
            "supersedes": "h1", "text": "A, renewing my conclude hold until 21:00." });
        assert_eq!(
            still_held(&[first, renewed.clone()]),
            Some("A, renewing my conclude hold until 21:00.".to_owned()),
            "a renewal naming its subject but not restating it dropped the builds"
        );
        // "No change" beside the hold, not its subject, restates nothing.
        for text in [
            "A, renewing my conclude hold, no change, until 21:00.",
            "A, renewing my conclude hold until 21:00; nothing else changed.",
        ] {
            let renewed = json!({ "id": "h2", "from": "A", "to": ["#machine"],
                "intent": "hold", "supersedes": "h1", "text": text });
            let first = json!({ "id": "h1", "from": "A", "to": ["#machine"],
                "intent": "hold", "text": "A: start no VM tests until I end this." });
            assert_eq!(still_held(&[first, renewed]), Some(text.to_owned()));
        }
        // One whose hold isn't to be found, as one sent before a reboot,
        // can't say what it renews, so it stops the net.
        assert_eq!(
            still_held(std::slice::from_ref(&renewed)),
            Some("A, renewing my conclude hold until 21:00.".to_owned())
        );
        let fresh = json!({ "id": "s", "from": "A", "to": ["#machine"], "intent": "hold",
            "text": "A: the Slack token rotates until 21:00." });
        assert_eq!(
            still_held(&[fresh]),
            None,
            "a hold renewing none was read as one"
        );
        // One restating its subject outright drops what the first mentioned.
        let conclude = json!({ "id": "c1", "from": "A", "to": ["#machine"], "intent": "hold",
            "text": "A: conclude no quest; at 18:00 both machines switch." });
        let restated = json!({ "id": "c2", "from": "A", "to": ["#machine"], "intent": "hold",
            "supersedes": "c1", "text": "A, renewing: conclude or retire no quest." });
        assert_eq!(still_held(&[conclude, restated]), None);
        let slack = saying("A: send Slack only through the gateway's tools.");
        let builds = json!({ "id": "b", "from": "B", "to": ["#machine"], "intent": "hold",
            "text": "B, for every quest: start no builds." });
        let both = [slack, builds];
        assert_eq!(
            still_held(&both),
            Some("B, for every quest: start no builds.".to_owned())
        );
        assert_eq!(
            passed_by(&both),
            [&both[0]],
            "the Slack hold wasn't the one passed by"
        );
    }

    #[test]
    fn a_hold_on_everyone_stands_until_its_sender_answers_it() {
        let held = hold("h1", "A", "#machine");
        assert_eq!(
            still_held(std::slice::from_ref(&held)),
            Some("h1".to_owned())
        );
        let other = json!({ "id": "x", "from": "B", "re": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held.clone(), other]), Some("h1".to_owned()));
        let ended = json!({ "id": "e", "from": "A", "re": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held.clone(), ended]), None);
        let replaced = json!({ "id": "r", "from": "A", "supersedes": "h1", "intent": "fyi" });
        assert_eq!(still_held(&[held, replaced]), None);
    }

    #[test]
    fn a_hold_on_one_session_leaves_the_net_alone() {
        assert_eq!(still_held(&[hold("h", "A", "@B")]), None);
        assert_eq!(still_held(&[hold("h", "A", "#all")]), Some("h".to_owned()));
    }

    #[test]
    fn only_holds_sent_since_the_boot_are_read() {
        let folder = std::env::temp_dir().join(format!("merge-net-holds-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let write = |name: &str, message: &Value| {
            fs::write(folder.join(name), message.to_string()).unwrap();
        };
        write("1000-A-1-aa.json", &hold("old", "A", "#machine"));
        assert_eq!(standing(&folder, 500), Some("old".to_owned()));
        assert_eq!(standing(&folder, 2000), None);
        write("3000-A-1-bb.json", &hold("new", "A", "#machine"));
        assert_eq!(standing(&folder, 2000), Some("new".to_owned()));
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn an_answer_filed_in_a_day_folder_still_ends_its_hold() {
        let folder = std::env::temp_dir().join(format!("merge-net-days-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("2026-11-09")).unwrap();
        let held = hold("1000-A-1-aa", "A", "#all");
        fs::write(folder.join("1000-A-1-aa.json"), held.to_string()).unwrap();
        assert_eq!(
            standing(&folder, 500),
            Some("1000-A-1-aa".to_owned()),
            "a hold with no answer stands"
        );
        let ended =
            json!({ "id": "2000-A-1-bb", "from": "A", "re": "1000-A-1-aa", "intent": "fyi" });
        fs::write(
            folder.join("2026-11-09/2000-A-1-bb.json"),
            ended.to_string(),
        )
        .unwrap();
        assert_eq!(
            standing(&folder, 500),
            None,
            "the answer in the day's folder ends the hold above it"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}
