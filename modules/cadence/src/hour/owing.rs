//! What the hour owes: its drag filed in the loop's ledger with each of
//! its owners, due a day on, or a week for a queue's change, and each drag
//! filed before closed once it is back in.
//!
//! Long because each kind of drag, Joel's misses, a merge's rounds, dates
//! told late, his asks landed unproven, the work system's findings, the worst queue and each
//! measure's lines by owner, is filed and closed by rules of its own, and
//! its tests sit beside them.

use std::collections::BTreeMap;

use crate::DAY;
use crate::findings::Entry;
use crate::owe::{self, Owed};

use super::lines::figure;
use super::measures::queues::{self, QUEUE};
use super::measures::{
    BLIND, Class, FINDINGS, HELD, Measure, UNRUN, UNSEEN, UNTESTED, apps, causes, drag,
    held_for_good, held_unseen, rounds, told_late, unproven,
};

/// How long a queue's ask has before it goes late: long enough for the
/// dated change it asks for to land.
const QUEUE_DUE: i64 = 7 * DAY;

/// One drag as the ledger files it: its key, owner, words, and what its
/// owner is told.
struct Filed {
    key: String,
    owner: String,
    says: String,
    text: String,
}

/// What a miss's owner is asked, by what holds it open, `held`: nothing yet
/// but a check, a measure still finding it, or its check not running here.
fn asked_of_owner(what: &str, held: &str) -> String {
    if held.contains(UNRUN) {
        format!(
            "cadence hour: a miss Joel found himself is yours: {what}. Its check, {held}, so \
             it stays open, and Joel isn't told it's fixed, until this machine runs it. \
             Please reply with when it will."
        )
    } else if held.is_empty() {
        format!(
            "cadence hour: a miss Joel found himself is yours: {what}. Which check would have \
             caught it? It stays open until that check merges; tell the orchestrator its PR, \
             for the misses ledger's check field."
        )
    } else {
        format!(
            "cadence hour: a miss Joel found himself is yours: {what}. Its check merged, but \
             the measure {held}, so it stays open until that measure is judged within its \
             target. Please reply with what keeps it there."
        )
    }
}

/// Each finding behind, filed apart as each miss is, so one past a day
/// reaches its owner though another of theirs is open; one held only by
/// the hour's blindness isn't put to them.
fn findings_filed(m: &Measure) -> Vec<Filed> {
    m.behind
        .iter()
        .filter(|line| !held_unseen(line))
        .filter_map(|line| line.rsplit_once(" · "))
        .map(|(what, owner)| {
            let short = what.split(' ').next().unwrap_or_default();
            // A landing note counts only in its finding's thread, so the
            // ask names the id to answer, as the mesh shows it, since a
            // reply to this ask lands on nothing. A first line holding
            // a doubt word reads as doubting its fix, so the ask says to
            // keep what was fixed off that line. The finding's words are
            // quoted, since any sender writes them, so they never read as
            // cadence's.
            let closes = format!(
                "It closes by itself once the changes a landing note answering {short} names \
                 run here, the note opening as \"Landed core #6000.\" does, naming each \
                 change with its repository, as \"core #6000 and core #6001\"; a note naming no \
                 change, or answering this ask, closes nothing, nor does one whose first line \
                 says still, but, though, although, however, yet, broken, fails, failing or \
                 failed, so say what it fixed on the lines after."
            );
            // One whose fix merged and waits to run has its cause and fix
            // said already, so only when it runs is asked, as for a miss.
            let text = match what.split_once(HELD) {
                Some((name, merged)) if merged.contains(UNRUN) => format!(
                    "cadence hour: a finding in #v2-symbiosis open past a day is yours: \
                     \"{name}\". Its fix, {merged}, so it stays open until this machine runs it. \
                     Please reply with when it will. {closes}"
                ),
                _ => format!(
                    "cadence hour: a finding in #v2-symbiosis open past a day, with no fix \
                     running here, is yours: \"{what}\". Please reply with its cause and fix \
                     within a day, or why it can't move. {closes}"
                ),
            };
            Filed {
                key: format!("hour:{FINDINGS}:{owner}:{short}"),
                owner: owner.to_owned(),
                says: format!("a work-system finding open past a day, yours: {what}"),
                text,
            }
        })
        .collect()
}

/// What the drag files: one entry for its measure, or, for Joel's misses,
/// one for each miss with that miss's own owner, since each owner names
/// the check its own miss needs.
fn filed(m: &Measure) -> Vec<Filed> {
    if m.class == Class::Miss {
        return m
            .behind
            .iter()
            .filter_map(|line| line.rsplit_once(" · "))
            // A miss held only because the hour couldn't look is the hour's
            // to fix, not its owner's, so it isn't put to them: a measure
            // unread, or what runs here unseen.
            .filter(|(_, owner)| {
                owner.split_once(HELD).is_none_or(|(_, why)| {
                    !BLIND.iter().any(|b| why.ends_with(b))
                        && why
                            .split_once(UNRUN)
                            .is_none_or(|(_, unrun)| !unrun.starts_with(UNSEEN))
                })
            })
            .map(|(what, owner)| {
                // A note after the owner, ", though its check ...", is no
                // address: it says the check merged and a measure still
                // holds the miss, so the owner isn't asked for a check again.
                let (owner, held) = owner.split_once(HELD).unwrap_or((owner, ""));
                // A held miss is its own finding, so the owner hears of the
                // hold though the ask for a check is still open.
                let key = if held.is_empty() {
                    format!("hour:miss:{what}")
                } else {
                    format!("hour:miss-held:{what}")
                };
                let text = asked_of_owner(what, held);
                Filed {
                    key,
                    owner: owner
                        .split(',')
                        .next()
                        .unwrap_or_default()
                        .trim()
                        .to_owned(),
                    says: format!("a miss Joel found himself: {what}"),
                    text,
                }
            })
            .collect();
    }
    let says = format!(
        "{}: {} against at most {}",
        m.says,
        figure(m.now),
        figure(m.target.unwrap_or_default())
    );
    if m.key.starts_with(rounds::FRUITLESS) {
        return vec![fruitless_asked(m, says)];
    }
    if m.key.starts_with(told_late::KEY) {
        return told_late_asked(m, &says);
    }
    if m.key == causes::KEY {
        return causes_asked(m);
    }
    if m.key == unproven::KEY {
        return unproven_asked(m, &says);
    }
    if m.key == FINDINGS {
        return findings_filed(m);
    }
    if m.key.starts_with(QUEUE) {
        return vec![queue_asked(m, says)];
    }
    if m.key == UNTESTED {
        return untested_asked(m, &says);
    }
    let owners = by_owner(m);
    if !owners.is_empty() {
        return owners
            .into_iter()
            .map(|(owner, files)| {
                let files = files.join(", ");
                let (things, closes) = match (m.class, m.key.as_str()) {
                    (Class::Cells, "reports-unasked") => ("reports", "none of them is left"),
                    (Class::Cells, _) => ("cells", "none of them is left"),
                    _ => ("files", "its figure is back in"),
                };
                Filed {
                    key: format!("hour:{}:{owner}", m.key),
                    says: format!("{says}, yours: {files}"),
                    text: format!(
                        "cadence hour: the biggest drag this hour is {says}, and these of \
                         its {things} are yours: {files}. Please reply with their cause and fix \
                         within a day, or why they can't move. It closes by itself once \
                         {closes}."
                    ),
                    owner,
                }
            })
            .collect();
    }
    vec![Filed {
        key: format!("hour:{}", m.key),
        owner: m.owner.clone(),
        text: format!(
            "cadence hour: the biggest drag this hour is yours, {says}. Please reply with \
             its cause and fix within a day, or why it can't move. It closes by itself once \
             its figure is back in."
        ),
        says,
    }]
}

/// The ask about a merge's rounds past five that caught nothing. Its
/// rounds can't be taken back, so its figure never comes in: the owner is
/// asked what they caught, and it closes as the merge leaves the day's
/// count.
fn fruitless_asked(m: &Measure, says: String) -> Filed {
    Filed {
        key: format!("hour:{}", m.key),
        owner: m.owner.clone(),
        text: format!(
            "cadence hour: {says}. Past round five a round raises only what would \
             mislead Joel or break something. Please reply with what those rounds \
             caught that the fixes record misses, or what keeps the next merge to five. \
             It closes by itself once the merge is a day old."
        ),
        says,
    }
}

/// The ask to its owner about each date moved too late to warn whoever
/// waited, each line naming its owner last as the misses' do; filed move
/// by move, so a later move of an owner whose ask is open is still told.
/// A move can't be taken back, so its figure never comes in by steering:
/// it closes once its line is gone, a day after the move.
fn told_late_asked(m: &Measure, says: &str) -> Vec<Filed> {
    m.behind
        .iter()
        .filter_map(|line| line.rsplit_once(" · "))
        .map(|(date, owner)| Filed {
            key: format!("hour:{}:{owner}:{date}", m.key),
            says: format!("{says}, yours: {date}"),
            text: format!(
                "cadence hour: {says}, and this move is yours: {date}. A date moved this \
                 late leaves whoever waits on it no time to plan round it. Please reply \
                 with what held the word back and what tells the next move sooner. It \
                 closes by itself a day after the move."
            ),
            owner: owner.to_owned(),
        })
        .collect()
}

/// The ask to each class's owner about a cause found on three or more
/// changes in a week, filed class by class, so each owner fixes the cause
/// once rather than each change's reviewer fixing it again. It closes once
/// its owner names the fix, and stays closed for the week its cases count
/// over, or once the class is found on fewer; a case after that brings the
/// same line back, counted as reopened. The class is quoted, since an
/// unfolded one is in a sender's own words, so it never reads as
/// cadence's. A measure that couldn't read the mesh asks nobody: its line
/// names no class.
fn causes_asked(m: &Measure) -> Vec<Filed> {
    m.behind
        .iter()
        .filter_map(|line| line.rsplit_once(" · "))
        .filter(|(line, _)| causes::class_of(line) != "unread")
        .map(|(line, owner)| {
            let class = causes::class_of(line);
            let found = line.get(class.len() + 1..).unwrap_or_default().trim();
            let key = format!("hour:{}:{class}", m.key);
            let done = causes::shell_quoted(&key);
            Filed {
                says: format!("a cause found again and again, yours: \"{class}\", {found}"),
                text: format!(
                    "cadence hour: a cause is found again and again, and it is yours: \
                     \"{class}\", {found}. Please reply within a day with the one fix that \
                     stops it at its root, such as a check that refuses it or a shared piece \
                     every change uses, or why it can't have one. Once it lands, name it with \
                     `cadence owe --done {done} FIX`, and it stays closed for the week its \
                     cases count over; until then it also closes once the class falls under \
                     three changes in a week."
                ),
                key,
                owner: owner.to_owned(),
            }
        })
        .collect()
}

/// The ask to the cell answering for each of his asks landed with no probe
/// passing since, each line naming the ask and when it was asked before its
/// state, and its cell last; filed ask by ask, so each cell hears of its
/// own. It closes once the ask's line is gone from the measure, read: its
/// probe passed, he confirmed it, or it left the week. The week's asks
/// naming no probe are one line, its keeper's, asked to write them.
fn unproven_asked(m: &Measure, says: &str) -> Vec<Filed> {
    m.behind
        .iter()
        .filter_map(|line| line.rsplit_once(" · "))
        .filter_map(|(ask, owner)| Some((ask.rsplit_once("; ")?, owner)))
        .map(|((ask, state), owner)| {
            let key = format!("hour:{}:{owner}:{ask}", m.key);
            // His words are in the key, so it's quoted to stay one word,
            // and nothing in it runs, when pasted.
            let done = causes::shell_quoted(&key);
            Filed {
                says: format!("{says}, yours: {ask}; {state}"),
                text: if ask == unproven::UNNAMED {
                    format!(
                        "cadence hour: {says}, and these are yours to write probes for: \
                         {ask}; {state}. No cell can prove what it was asked without one. \
                         Please write each one's probe, or say why they can't have one with \
                         `cadence owe --done {done} WHY`, which holds until their count \
                         moves. It closes by itself once none is left."
                    )
                } else {
                    format!(
                        "cadence hour: {says}, and this one's result is yours: {ask}; \
                         {state}. A merge isn't done until he has what he asked for. Please \
                         run its probe and fix what it shows, or say why it can't pass yet \
                         with `cadence owe --done {done} WHY`, which holds until its state \
                         moves. It closes by itself once its probe passes here, or once it's \
                         a week landed and left to the backfill."
                    )
                },
                key,
                owner: owner.to_owned(),
            }
        })
        .collect()
}

/// Each owner's ask for a test that runs to catch their misses again, in
/// the orchestrator's ruling (23 November): every miss gets a check that
/// runs. It closes once none of the measure's lines names them.
fn untested_asked(m: &Measure, says: &str) -> Vec<Filed> {
    by_owner(m)
        .into_iter()
        .map(|(owner, misses)| {
            let misses = misses.join("; ");
            Filed {
                key: format!("hour:{}:{owner}", m.key),
                says: format!("{says}, yours: {misses}"),
                text: format!(
                    "cadence hour: {says}, and these misses are yours: {misses}. Each needs \
                     a test that fails if it comes back, merged where its repository's own \
                     checks run it. Please name each within a day with `pain-points test \
                     --miss AT --test \"REPO PATH NAME\"`, AT being the miss's time as the \
                     ledger has it, or reply why it can't have one. It closes by itself once \
                     cadence finds each test where it runs."
                ),
                owner,
            }
        })
        .collect()
}

/// The worst queue's ask, in Joel's terms for a queue (20 November): what
/// it protects, whether the work could go without it, and a change weighed
/// against others, with its date.
fn queue_asked(m: &Measure, says: String) -> Filed {
    Filed {
        key: format!("hour:{}", m.key),
        owner: m.owner.clone(),
        text: format!(
            "cadence hour: the worst queue today is yours, {says}. Please reply within a day \
             with what it protects, whether the work could go without it, and the change that \
             keeps the protection without the wait, weighed against at least two designs \
             beside the obvious one, with its date. It closes by itself once its p90 is back \
             in, and goes late to the orchestrator only after a week, as its change takes its \
             time."
        ),
        says,
    }
}

/// A clean measure's files, or a cell measure's cells, by the owner each
/// is said with; empty otherwise, and for every other class.
pub fn by_owner(m: &Measure) -> BTreeMap<String, Vec<String>> {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if routed(m.class) {
        for (file, owner) in m.behind.iter().filter_map(|line| line.rsplit_once(" · ")) {
            owners
                .entry(owner.to_owned())
                .or_default()
                .push(file.to_owned());
        }
    }
    owners
}

/// Whether a class's lines each name their own owner, after ` · `, so its
/// drag is filed with each of them rather than with the measure's owner.
fn routed(class: Class) -> bool {
    matches!(class, Class::Clean | Class::Cells)
}

/// Whether the drag filed as `key` is back in: its miss no longer open,
/// or its measure read and within its target; a drag filed for each line's
/// owner carries that owner after its measure's key, and a cells one is
/// back in once none of the measure's lines names them, since a cell's
/// line goes only when it is fixed. A clean file's line can go as the day
/// turns, so a clean one waits for its zone. A measure that couldn't be
/// read says nothing, so it closes none. A merge whose rounds past five
/// caught nothing is back in once its line is gone, the merge a day old,
/// unless the rounds measure couldn't read the review record. A figure of
/// his apps gone from an hour that read their file whole was renamed or
/// dropped by its owner, so it's back in, as nothing would bring it back.
fn back_in(key: &str, measures: &[Measure]) -> bool {
    let key = key.strip_prefix("hour:").unwrap_or(key);
    if key.starts_with(rounds::FRUITLESS) {
        return !measures.iter().any(|m| m.key == key) && measures.iter().any(rounds::read_whole);
    }
    // A class of cause is its owner's until the measure, read, no longer
    // names it; a measure that couldn't read the mesh names only that.
    if let Some(class) = key
        .strip_prefix(causes::KEY)
        .and_then(|rest| rest.strip_prefix(':'))
    {
        return measures
            .iter()
            .find(|m| m.key == causes::KEY)
            .is_some_and(|m| {
                !m.behind
                    .iter()
                    .any(|line| causes::class_of(line) == "unread")
                    && !m.behind.iter().any(|line| causes::class_of(line) == class)
            });
    }
    // A date told late is its owner's until its line is gone from the
    // measure, read; with the asks list unread it says nothing, so stays
    // open.
    let late = measures.iter().find_map(|m| {
        let moved = key.strip_prefix(m.key.as_str())?.strip_prefix(':')?;
        m.key.starts_with(told_late::KEY).then_some((m, moved))
    });
    if let Some((m, moved)) = late {
        let line = moved
            .split_once(':')
            .map(|(owner, date)| format!("{date} · {owner}"));
        return m.target.is_some() && line.is_none_or(|line| !m.behind.contains(&line));
    }
    // An ask unproven is its cell's until its line, whatever its state
    // now, is gone from the measure, read; unjudged, it stays open.
    if let Some(rest) = key
        .strip_prefix(unproven::KEY)
        .and_then(|k| k.strip_prefix(':'))
    {
        let Some((owner, ask)) = rest.split_once(':') else {
            return false;
        };
        let (opens, ends) = (format!("{ask}; "), format!(" · {owner}"));
        return measures
            .iter()
            .find(|m| m.key == unproven::KEY)
            .is_some_and(|m| {
                m.target.is_some()
                    && !m
                        .behind
                        .iter()
                        .any(|line| line.starts_with(&opens) && line.ends_with(&ends))
            });
    }
    // An unread file leaves only its one line, never a figure, so a drag
    // isn't closed while its figure can't be seen; a figure written with no
    // reading is still its figure, unjudged. His apps aren't routed, so a
    // figure's drag is filed under its key alone.
    if key.starts_with(apps::FIGURE) && !measures.iter().any(|m| m.key == key) {
        return measures.iter().any(|m| m.key.starts_with(apps::FIGURE));
    }
    // The ask for a check closes once the check merged and only a measure
    // holds the miss; that hold is its own finding, closing once judged.
    let asked = key.strip_prefix("miss:").map(|what| (what, false));
    if let Some((what, held)) = asked.or_else(|| key.strip_prefix("miss-held:").map(|w| (w, true)))
    {
        // A ledger not read whole says nothing of which misses are gone.
        if measures
            .iter()
            .any(|m| m.class == Class::Miss && m.target.is_none())
        {
            return false;
        }
        let open = format!("{what} · ");
        return !measures
            .iter()
            .filter(|m| m.class == Class::Miss)
            .flat_map(|m| &m.behind)
            .filter_map(|line| line.strip_prefix(&open))
            .any(|by| by.contains(HELD) == held);
    }
    // A finding's drag carries its owner and its short id, and is back in
    // once the findings measure, read, holds no line of it but one held for
    // good, by a change cadence can't read: its owner has done what they
    // can, so it never goes late beside their name. An hour that can't
    // look keeps it open, so it isn't closed and filed again.
    let finding = key
        .strip_prefix(FINDINGS)
        .and_then(|rest| rest.strip_prefix(':'))
        .and_then(|rest| rest.rsplit_once(':'));
    if let Some((owner, short)) = finding {
        let (opens, ends) = (format!("{short} from "), format!(" · {owner}"));
        return measures
            .iter()
            .find(|m| m.key == FINDINGS)
            .is_some_and(|m| {
                m.target.is_some()
                    && !m.behind.iter().any(|line| {
                        line.starts_with(&opens) && line.ends_with(&ends) && !held_for_good(line)
                    })
            });
    }
    measures
        .iter()
        .find_map(|m| {
            if m.key == key {
                return Some((m, None));
            }
            let owner = key.strip_prefix(m.key.as_str())?.strip_prefix(':')?;
            routed(m.class).then_some((m, (m.class == Class::Cells).then_some(owner)))
        })
        .is_some_and(|(m, owner)| {
            m.target.is_some()
                && (m.past().is_none()
                    || owner.is_some_and(|owner| {
                        let named = format!(" · {owner}");
                        !m.behind.iter().any(|line| line.ends_with(&named))
                    }))
        })
}

/// Whether `key` is a class of cause its owner closed by naming its fix
/// within the week before `now`: its cases still count from before the
/// fix, so filing it again would ask for the fix just named. A bare
/// `--done`, which names none, holds nothing.
fn fixed_lately(ledger: &[Entry], key: &str, now: i64) -> bool {
    key.strip_prefix("hour:")
        .is_some_and(|key| key.starts_with(causes::KEY))
        && ledger.iter().any(|e| {
            e.key == key
                && e.fixed.is_some()
                && e.closed.is_some_and(|at| at > now - causes::WITHIN)
        })
}

/// Whether the ask unproven filed as `one` was closed by its cell, with
/// `cadence owe --done KEY WHY`, while its line said what it says now: the
/// cell has said why it can't pass yet, so it isn't asked again until the
/// line's state moves. A bare `--done`, which says no why, holds nothing;
/// nor does a close the hour made as its line went, or one whose line has
/// gone since (`unheld`), so a pass then the same failure is asked again;
/// nor does the measure's figure, which moves with every other ask.
fn held(ledger: &[Entry], one: &Filed) -> bool {
    let own = |says: &str| says.split_once(", yours: ").map(|(_, own)| own.to_owned());
    ledger
        .iter()
        .rev()
        .find(|e| e.key == one.key)
        .is_some_and(|e| {
            !e.open && e.closed.is_some() && e.fixed.is_some() && own(&e.words) == own(&one.says)
        })
}

/// Lets go each cell's close of an ask unproven whose line is gone from
/// the measure, read: its probe passed, or it left the week, so what it
/// said no longer holds, and the same failure after is asked again.
fn unheld(ledger: &mut [Entry], measures: &[Measure]) {
    let ours = format!("hour:{}:", unproven::KEY);
    for entry in ledger.iter_mut() {
        if !entry.open
            && entry.closed.is_some()
            && entry.key.starts_with(&ours)
            && back_in(&entry.key, measures)
        {
            entry.closed = None;
        }
    }
}

/// Files the hour's drag in the loop's ledger as a must of kind `drag`,
/// due a day on, unless it's open there already, and closes each drag
/// filed before that is back in. Says what to tell the owner of each one
/// newly filed. His apps under load lead the drag, but they came in ahead
/// of every class that was filed before them, so while they lead it each
/// of his apps' figures past its target goes to its own owner, as each
/// miss does, and the drag the rest of the hour would have is filed too:
/// his misses still reach their owners whatever his apps read. So does a
/// merge whose rounds past five caught nothing, which WCJQSG asked be filed
/// to whoever landed it, whatever else leads, each owner's findings in
/// the work system's group open past a day, which WCJQSG ruled count
/// against them as Joel's misses do, and the worst queue, once a day.
pub fn file(
    ledger: &mut Vec<Entry>,
    measures: &[Measure],
    date: &str,
    now: i64,
) -> Vec<(String, String)> {
    let closing: Vec<String> = ledger
        .iter()
        .filter(|e| e.open && e.kind.as_deref() == Some("drag"))
        .filter(|e| match e.key.strip_prefix("hour:") {
            Some(key) if key.starts_with(QUEUE) => queues::answered(key, measures),
            _ => back_in(&e.key, measures),
        })
        .map(|e| e.key.clone())
        .collect();
    for key in closing {
        owe::settle(ledger, &key);
    }
    unheld(ledger, measures);
    let rest: Vec<Measure> = measures
        .iter()
        .filter(|m| m.class != Class::Apps)
        .cloned()
        .collect();
    let mut dragging: Vec<&Measure> = drag(measures).into_iter().collect();
    if let Some(lead) = dragging.first().copied().filter(|m| m.class == Class::Apps) {
        dragging.extend(
            measures
                .iter()
                .filter(|m| m.class == Class::Apps && m.key != lead.key && m.past().is_some()),
        );
        dragging.extend(drag(&rest));
    }
    // Never the lead, which passes them over, so filed whatever leads; a
    // measure filed twice is owed once, as the ledger holds it open. A
    // miss with no test that runs is asked of its owner the same way.
    dragging.extend(measures.iter().filter(|m| {
        let beside = [FINDINGS, UNTESTED].contains(&m.key.as_str());
        let apart = m.key.starts_with(rounds::FRUITLESS)
            || m.key.starts_with(told_late::KEY)
            || m.key == causes::KEY
            || m.key == unproven::KEY;
        (apart || beside) && m.past().is_some()
    }));
    // Nor do the queues; the worst is asked for its next change once a
    // day, as Joel asked of them, whatever else leads.
    let asked_today = ledger
        .iter()
        .any(|e| e.key.starts_with(&format!("hour:{QUEUE}")) && e.first == date);
    dragging.extend(queues::worst(measures).filter(|_| !asked_today));
    let mut told = Vec::new();
    for (m, one) in dragging
        .into_iter()
        .flat_map(|m| filed(m).into_iter().map(move |f| (m, f)))
    {
        // An ask unproven keeps its line's state as it moves, so its cell's
        // close holds while the line says the same.
        let unproven = one.key.starts_with(&format!("hour:{}:", unproven::KEY));
        if let Some(open) = ledger.iter_mut().find(|e| e.open && e.key == one.key) {
            if unproven {
                open.words = one.says;
            }
            continue;
        }
        if fixed_lately(ledger, &one.key, now) || (unproven && held(ledger, &one)) {
            continue;
        }
        owe::owe(
            ledger,
            Owed {
                key: one.key,
                axis: m.class.name().to_owned(),
                owner: one.owner.clone(),
                // A queue's ask is answered by a change weighed against
                // others, which takes longer than a day to land.
                due: now
                    + if m.class == Class::Queues {
                        QUEUE_DUE
                    } else {
                        DAY
                    },
                says: one.says,
                kind: "drag".to_owned(),
            },
            date,
            now,
        );
        told.push((one.owner, one.text));
    }
    told
}

#[cfg(test)]
mod tests {
    use super::super::measure;
    use super::*;

    #[test]
    fn a_miss_cadence_cant_see_running_isnt_put_to_its_owner() {
        let mut m = measure("misses", Class::Miss, 1.0, Some(0.0));
        m.behind = vec![format!(
            "the host crashed · OSPHT4{HELD}core #5800{UNRUN}cadence can't see what runs here"
        )];
        assert!(
            filed(&m).is_empty(),
            "the owner was asked about what only the hour can mend"
        );
    }

    #[test]
    fn the_owner_of_a_miss_whose_check_doesnt_run_yet_is_asked_when_it_will() {
        let mut m = measure("misses", Class::Miss, 1.0, Some(0.0));
        m.behind = vec![format!(
            "the host crashed · OSPHT4{HELD}core #5800{UNRUN}the pi hosts don't load it yet"
        )];
        let filed = filed(&m);
        assert_eq!(filed.len(), 1, "the miss wasn't put to its owner");
        assert_eq!(filed[0].owner, "OSPHT4");
        assert_eq!(filed[0].key, "hour:miss-held:the host crashed");
        assert!(
            filed[0]
                .text
                .contains("core #5800 merged, but the pi hosts don't load it yet")
                && filed[0].text.contains("when it will")
                && !filed[0].text.contains("Which check"),
            "its owner was asked for a check again, or not when it will run: {}",
            filed[0].text
        );
    }

    #[test]
    fn the_worst_queue_is_asked_once_a_day_beside_the_drag() {
        let queue = |key: &str, now: f64, owner: &str| {
            let mut m = measure(&format!("{QUEUE}{key}"), Class::Queues, now, Some(60.0));
            m.owner = owner.to_owned();
            m
        };
        let slips = measure("slips", Class::Slip, 3.0, Some(0.0));
        let hour = [
            slips.clone(),
            queue("world-walks", 180.0, "WALKS"),
            queue("sends", 90.0, "SENDS"),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &hour, "2026-11-21", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["OWNER", "WALKS"],
            "the drag still leads, and the worst queue is asked"
        );
        assert!(
            told[1]
                .1
                .contains("what it protects, whether the work could go without it")
                && told[1]
                    .1
                    .contains("two designs beside the obvious one, with its date"),
            "asked in Joel's terms: {}",
            told[1].1
        );
        // Later that day another queue is the worst; one ask a day is all.
        let later = [
            slips,
            queue("world-walks", 30.0, "WALKS"),
            queue("sends", 90.0, "SENDS"),
        ];
        assert!(
            file(&mut ledger, &later, "2026-11-21", 4600).is_empty(),
            "a second queue asked the same day"
        );
        assert!(
            !ledger
                .iter()
                .any(|e| e.key == format!("hour:{QUEUE}world-walks") && e.open),
            "a queue back in closes its ask"
        );
        let told = file(&mut ledger, &later, "2026-11-22", 90_000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["SENDS"], "the next day's worst is asked");
    }

    #[test]
    fn a_queue_never_leads_the_drag_and_keeps_its_ask_while_quiet_or_unread() {
        let queue = measure(
            &format!("{QUEUE}world-walks"),
            Class::Queues,
            180.0,
            Some(60.0),
        );
        assert!(
            drag(std::slice::from_ref(&queue)).is_none(),
            "a queue led the drag"
        );
        let pains = measure("pains", Class::Pains, 2.0, Some(0.0));
        assert_eq!(
            drag(&[queue.clone(), pains]).map(|m| m.key.as_str()),
            Some("pains"),
            "a queue led ahead of Joel's pains"
        );
        let mut ledger = Vec::new();
        let told = file(
            &mut ledger,
            std::slice::from_ref(&queue),
            "2026-11-21",
            1000,
        );
        assert_eq!(told.len(), 1, "only the queue's ask: {told:?}");
        assert_eq!(ledger[0].due, 1000 + QUEUE_DUE, "due a week on");
        let unread = Measure {
            says: format!("x{}", queues::UNREAD),
            target: None,
            ..queue.clone()
        };
        file(&mut ledger, &[unread], "2026-11-22", 90_000);
        assert!(ledger[0].open, "a record unread closed the ask");
        let quiet = Measure {
            says: format!("x{}", queues::QUIET),
            target: None,
            ..queue
        };
        file(&mut ledger, &[quiet], "2026-11-22", 93_600);
        assert!(
            ledger[0].open,
            "a quiet queue closed the ask, though a wedged one reads the same"
        );
    }

    #[test]
    fn a_drag_is_filed_once_with_its_owner_and_closes_once_back_in() {
        let mut ledger = Vec::new();
        let out = [measure("slips", Class::Slip, 3.0, Some(0.0))];
        let told = file(&mut ledger, &out, "2026-11-14", 1000);
        let owners: Vec<String> = told.into_iter().map(|(to, _)| to).collect();
        assert_eq!(owners, ["OWNER"], "its owner is told");
        assert_eq!(ledger.len(), 1, "filed");
        assert_eq!(ledger[0].due, 1000 + DAY, "due a day on");
        assert!(
            file(&mut ledger, &out, "2026-11-14", 4600).is_empty(),
            "told once"
        );
        assert_eq!(ledger[0].due, 1000 + DAY, "its date doesn't move");
        let back = [measure("slips", Class::Slip, 0.0, Some(0.0))];
        assert!(
            file(&mut ledger, &back, "2026-11-14", 8200).is_empty(),
            "none told"
        );
        assert!(!ledger[0].open, "back in closes it");
    }

    fn recurring(lines: &[&str]) -> Measure {
        let mut m = measure(causes::KEY, Class::Functional, 0.0, Some(0.0));
        m.behind = lines.iter().map(|&line| line.to_owned()).collect();
        m.now = if lines.is_empty() { 0.0 } else { 1.0 };
        m
    }

    /// The keeper's most common class holds an apostrophe, and an unfolded
    /// one holds whatever its sender wrote, so the command each ask gives
    /// must reach `cadence owe` as the very key its line is filed under.
    #[test]
    fn the_command_an_ask_gives_reaches_owe_as_its_own_key() {
        let undone = "a test passes with the fix undone, or can't fail in core.nix: on 3 changes \
                      in a week, core.nix#1, #2, #3 · 80ECO2";
        let odd = "(unfolded) x'; touch gone; ' $(touch gone) \\ end: on 3 changes in a week, \
                   core.nix#4, #5, #6 · 1JQPPC";
        let mut ledger = Vec::new();
        let told = file(
            &mut ledger,
            &[recurring(&[undone, odd])],
            "2026-11-26",
            1000,
        );
        assert_eq!(told.len(), 2, "each class wasn't asked: {told:?}");
        let zone = jiff::tz::TimeZone::UTC;
        let here = tempfile::tempdir().expect("a folder for the shell");
        for (_, text) in &told {
            let command = text
                .split_once("`cadence owe ")
                .and_then(|(_, rest)| rest.split_once('`'))
                .map(|(command, _)| command)
                .expect("an ask that names its command");
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("printf '%s\\n' {command}"))
                .current_dir(here.path())
                .output()
                .expect("a shell to split the command");
            let words: Vec<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::to_owned)
                .collect();
            let asked = owe::parse(words.clone().into_iter(), &zone);
            assert!(
                matches!(&asked, Ok(owe::Asked::Done { key, fix })
                    if fix.as_deref() == Some("FIX") && ledger.iter().any(|e| &e.key == key)),
                "the ask's command, `{command}`, split by a shell as {words:?}, isn't a key a \
                 line is filed under: {asked:?}"
            );
        }
        assert!(
            std::fs::read_dir(here.path())
                .expect("its folder")
                .next()
                .is_none(),
            "pasting an ask's command ran what its sender wrote"
        );
    }

    #[test]
    fn each_class_of_cause_is_asked_of_its_owner_apart_and_reopens_when_it_comes_back() {
        let docs = "stale docs: on 3 changes in a week, core.nix#1, #2, #3 · AAAAAA";
        let racy = "racy test: on 4 changes in a week, core.nix#4, #5 · BBBBBB";
        let worse = [
            measure("slips", Class::Slip, 3.0, Some(0.0)),
            recurring(&[racy, docs]),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &worse, "2026-11-26", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["OWNER", "BBBBBB", "AAAAAA"],
            "a class of cause wasn't asked of its own owner beside what leads"
        );
        assert!(
            told[1].1.contains("\"racy test\", on 4 changes") && !told[1].1.contains("stale"),
            "an owner heard of a class not theirs, or not quoted as a sender's words: {}",
            told[1].1
        );
        let key = "hour:causes-recurring:racy test";
        assert!(
            ledger.iter().any(|e| e.key == key),
            "a class wasn't filed under its own key: {ledger:?}"
        );
        let unread = recurring(&["unread: no mesh messages folder here · 1JQPPC"]);
        let blind = file(&mut ledger, &[unread], "2026-11-26", 4600);
        assert!(
            blind.is_empty() && !ledger.iter().any(|e| e.key.ends_with(":unread")),
            "a mesh that couldn't be read was asked of the keeper as a cause: {blind:?}"
        );
        assert!(
            ledger
                .iter()
                .filter(|e| e.key.contains("causes"))
                .all(|e| e.open),
            "a class closed while the mesh couldn't be read"
        );
        file(&mut ledger, &[recurring(&[docs])], "2026-11-26", 8200);
        let open = |ledger: &[Entry], key: &str| ledger.iter().any(|e| e.open && e.key == key);
        assert!(
            !open(&ledger, key) && open(&ledger, "hour:causes-recurring:stale docs"),
            "a class found on fewer changes didn't close, or one still over did"
        );
        let again = file(&mut ledger, &[recurring(&[racy])], "2026-11-27", 90_000);
        assert_eq!(again.len(), 1, "a class come back wasn't asked again");
        let lines: Vec<&Entry> = ledger.iter().filter(|e| e.key == key).collect();
        assert_eq!(
            (
                lines.len(),
                lines[0].open,
                lines[0].reopened,
                lines[0].first.as_str()
            ),
            (1, true, 1, "2026-11-27"),
            "a class come back opened a second line rather than reopening its own"
        );
        // Its owner names the fix, as `cadence owe --done` does; the cases
        // from before it still count the class over for the week.
        // A bare `--done` names no fix, so it holds nothing.
        let at = ledger.iter().position(|e| e.key == key).expect("its line");
        ledger[at].open = false;
        ledger[at].closed = Some(90_000);
        let bare = file(&mut ledger, &[recurring(&[racy])], "2026-11-27", 91_800);
        assert!(
            bare.len() == 1 && ledger[at].open && ledger[at].reopened == 2,
            "a class closed with no fix named was held: {bare:?}"
        );
        ledger[at].open = false;
        ledger[at].closed = Some(90_000);
        ledger[at].fixed = Some("core #6000".to_owned());
        let fixed = file(&mut ledger, &[recurring(&[racy])], "2026-11-27", 93_600);
        assert!(
            fixed.is_empty() && !ledger[at].open && ledger[at].reopened == 2,
            "a class its owner closed by naming the fix was asked again: {fixed:?}"
        );
        let week = 90_000 + causes::WITHIN + 1;
        let past = file(&mut ledger, &[recurring(&[racy])], "2026-12-04", week);
        assert_eq!(
            (past.len(), ledger[at].open, ledger[at].reopened),
            (1, true, 3),
            "a class still over a week after its fix wasn't asked again"
        );
    }

    #[test]
    fn a_clean_drag_goes_to_each_files_owner_and_closes_once_back_in() {
        let mut suppressions = measure(
            "clean:core.nix:suppressions",
            Class::Clean,
            12.0,
            Some(10.0),
        );
        suppressions.behind = vec![
            "net/a.rs · PN2VZC".to_owned(),
            "net/b.rs · PN2VZC".to_owned(),
            "x.rs · Y9XISP".to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[suppressions.clone()], "2026-11-15", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["PN2VZC", "Y9XISP"], "each owner of a file is told");
        assert!(
            told[0].1.contains("net/a.rs, net/b.rs") && !told[0].1.contains("x.rs"),
            "each hears only of its own files: {}",
            told[0].1
        );
        suppressions.now = 10.0;
        file(&mut ledger, &[suppressions], "2026-11-15", 4600);
        assert!(
            ledger.iter().all(|e| !e.open),
            "back in closes every owner's"
        );
    }

    #[test]
    fn a_cells_drag_goes_to_each_cells_owner_and_closes_once_back_in() {
        let mut idle = measure("cells-idle", Class::Cells, 2.0, Some(0.0));
        idle.behind = vec![
            "AAAAAA idle 30 min: write it · ORCH".to_owned(),
            "BBBBBB idle 40 min: merge it · PARENT".to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[idle.clone()], "2026-11-17", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["ORCH", "PARENT"], "each cell's owner is told");
        assert!(
            told[0]
                .1
                .contains("these of its cells are yours: AAAAAA idle 30 min: write it")
                && !told[0].1.contains("BBBBBB"),
            "each hears only of its own cells: {}",
            told[0].1
        );
        idle.now = 0.0;
        file(&mut ledger, &[idle], "2026-11-17", 4600);
        assert!(
            ledger.iter().all(|e| !e.open),
            "back in closes every owner's"
        );
    }

    #[test]
    fn each_untested_miss_is_asked_of_its_owner_beside_the_drag_until_its_test_runs() {
        let lead = measure("slips", Class::Slip, 3.0, Some(0.0));
        let mut untested = measure(UNTESTED, Class::Cells, 2.0, Some(0.0));
        untested.behind = vec![
            "cat waits (2026-11-14T17:24:00-05:00), naming no test · XY9H5C".to_owned(),
            "bar froze (2026-11-15T19:42:00-05:00), naming no test · H2VYOB".to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[lead, untested.clone()], "2026-11-24", 1000);
        let asked: Vec<&(String, String)> = told
            .iter()
            .filter(|(_, text)| text.contains("pain-points test"))
            .collect();
        let owners: Vec<&str> = asked.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["H2VYOB", "XY9H5C"],
            "each miss's owner wasn't asked beside the lead"
        );
        assert!(
            asked[1].1.contains("cat waits (2026-11-14T17:24:00-05:00)")
                && !asked[1].1.contains("bar froze"),
            "an owner heard of another's miss, or not of their own: {}",
            asked[1].1
        );
        untested.behind.remove(0);
        untested.now = 1.0;
        file(&mut ledger, &[untested], "2026-11-24", 4600);
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open && e.key.starts_with("hour:misses-untested"))
            .filter_map(|e| e.owner.as_deref())
            .collect();
        assert_eq!(open, ["H2VYOB"], "an owner whose test runs was still asked");
    }

    #[test]
    fn an_owners_cells_drag_closes_once_its_lines_go_and_stays_open_through_an_unread_hour() {
        let mut idle = measure("cells-idle", Class::Cells, 2.0, Some(0.0));
        idle.behind = vec![
            "AAAAAA idle 30 min: write it · ORCH".to_owned(),
            "BBBBBB idle 40 min: merge it · PARENT".to_owned(),
        ];
        let mut ledger = Vec::new();
        file(&mut ledger, &[idle.clone()], "2026-11-17", 1000);
        idle.behind.remove(0);
        idle.now = 1.0;
        file(&mut ledger, &[idle.clone()], "2026-11-17", 4600);
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .filter_map(|e| e.owner.as_deref())
            .collect();
        assert_eq!(
            open,
            ["PARENT"],
            "an owner whose cells are fixed stays open"
        );
        idle.target = None;
        idle.behind.clear();
        idle.now = 0.0;
        file(&mut ledger, &[idle], "2026-11-17", 8200);
        assert!(
            ledger
                .iter()
                .any(|e| e.open && e.owner.as_deref() == Some("PARENT")),
            "an hour that couldn't read the cells closed a drag"
        );
    }

    #[test]
    fn a_clean_drag_stays_open_while_its_measure_is_past_though_its_owner_left_the_list() {
        let mut long = measure("clean:core.nix:long", Class::Clean, 12.0, Some(10.0));
        long.behind = vec!["a.rs · PN2VZC".to_owned(), "b.rs · Y9XISP".to_owned()];
        let mut ledger = Vec::new();
        file(&mut ledger, &[long.clone()], "2026-11-17", 1000);
        long.behind.remove(0);
        file(&mut ledger, &[long], "2026-11-17", 4600);
        assert!(
            ledger.iter().all(|e| e.open),
            "a clean owner's drag closed while the zone was still past its target"
        );
    }

    #[test]
    fn a_reports_drag_says_reports_and_how_it_closes() {
        let mut unasked = measure("reports-unasked", Class::Cells, 1.0, Some(0.0));
        unasked.behind = vec!["AAAAAA's RPRT-1, drafted 9 h ago, asked nothing · ORCH".to_owned()];
        let told = file(&mut Vec::new(), &[unasked], "2026-11-17", 1000);
        assert!(
            told[0].1.contains("these of its reports are yours")
                && told[0].1.contains("once none of them is left"),
            "a reports drag spoke of cells, or of a figure it doesn't wait for: {}",
            told[0].1
        );
    }

    #[test]
    fn each_owners_findings_are_filed_as_findings_whatever_leads() {
        let slips = measure("slips", Class::Slip, 3.0, Some(0.0));
        let mut found = measure(FINDINGS, Class::Cells, 1.0, Some(0.0));
        found.behind = vec!["ab12 from IKQ63G: the slots · A2VLJN".to_owned()];
        let told = file(&mut Vec::new(), &[slips, found], "2026-11-21", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["OWNER", "A2VLJN"],
            "a finding's owner wasn't filed while slips led the drag"
        );
        assert!(
            told[1]
                .1
                .contains("is yours: \"ab12 from IKQ63G: the slots\". Please reply")
                && told[1].1.contains("\"Landed core #6000.\"")
                && told[1].1.contains("a landing note answering ab12")
                && told[1].1.contains("say what it fixed on the lines after"),
            "a finding was put to its owner as a cell, or without what closes it, the id \
             to answer and the doubt words its note's first line can't hold: {}",
            told[1].1
        );
        // Its fix merged and waiting to run, only when it runs is asked.
        let mut waits = measure(FINDINGS, Class::Cells, 1.0, Some(0.0));
        waits.behind = vec![format!(
            "ab12 from IKQ63G: the slots{HELD}core #6001{UNRUN}the last switch doesn't carry \
             it · A2VLJN"
        )];
        let told = file(&mut Vec::new(), &[waits], "2026-11-21", 1000);
        assert!(
            told.len() == 1
                && told[0]
                    .1
                    .contains("is yours: \"ab12 from IKQ63G: the slots\". Its fix, core ")
                && told[0].1.contains("when it will")
                && !told[0].1.contains("cause and fix"),
            "a finding whose fix waits on a switch asked its owner for a fix again: {told:?}"
        );
    }

    #[test]
    fn each_finding_reaches_its_owner_once_and_closes_once_its_line_goes() {
        let mut found = measure(FINDINGS, Class::Cells, 1.0, Some(0.0));
        found.behind = vec!["ab12 from IKQ63G: the slots · A2VLJN".to_owned()];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[found.clone()], "2026-11-21", 1000);
        assert_eq!(
            told.len(),
            1,
            "a leading finding wasn't told once: {told:?}"
        );
        // Another of the same owner's, past a day while the first is open.
        found
            .behind
            .push("cd34 from CIJMZQ: the hosts · A2VLJN".to_owned());
        found.now = 2.0;
        let told = file(&mut ledger, &[found.clone()], "2026-11-21", 4600);
        assert!(
            told.len() == 1 && told[0].1.contains("cd34 from CIJMZQ"),
            "a second finding wasn't told while the owner's first was open: {told:?}"
        );
        found.behind.remove(0);
        found.now = 1.0;
        file(&mut ledger, &[found.clone()], "2026-11-21", 8200);
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            ["hour:symbiosis-findings:A2VLJN:cd34"],
            "a fixed finding's drag stayed open, or another's closed with it"
        );
        // Held by an hour that can't look, a main unread or a listing
        // failed, it stays on the card and its drag open, put to nobody;
        // so does an hour the mesh went unread.
        found.now = 0.0;
        for (blind, at) in [
            (
                ", though cadence couldn't read core's main to see its check".to_owned(),
                11_800,
            ),
            (
                format!("{HELD}core #6000{UNRUN}cadence can't list the pi hosts"),
                13_600,
            ),
        ] {
            found.behind = vec![format!("cd34 from CIJMZQ: the hosts{blind} · A2VLJN")];
            assert!(
                filed(&found).is_empty()
                    && file(&mut ledger, &[found.clone()], "2026-11-21", at).is_empty()
                    && ledger.iter().filter(|e| e.open).count() == 1,
                "a finding held by {blind:?} was put to its owner, or its drag closed"
            );
        }
        found.behind.clear();
        found.target = None;
        file(&mut ledger, &[found.clone()], "2026-11-21", 15_400);
        assert!(
            ledger.iter().filter(|e| e.open).count() == 1,
            "a drag closed while the hour couldn't look"
        );
        // Held for good, by a change cadence can't read, its owner has done
        // what they can, so its drag closes rather than going late.
        found.target = Some(0.0);
        found.behind = vec![
            "cd34 from CIJMZQ: the hosts, though cadence can't read a change its landing notes \
             name · A2VLJN"
                .to_owned(),
        ];
        assert!(
            file(&mut ledger, &[found], "2026-11-21", 19_000).is_empty()
                && ledger.iter().all(|e| !e.open),
            "a finding held for good was put to its owner, or its drag stayed open to go late"
        );
    }

    #[test]
    fn a_drag_on_a_figure_of_his_apps_renamed_closes_once_their_file_is_read() {
        let old = measure("apps:wait per wakeup", Class::Apps, 9.0, Some(2.0));
        let renamed = measure("apps:wait per 10 s", Class::Apps, 1.0, Some(2.0));
        let unread = measure("apps-under-load", Class::Apps, 1.0, Some(6.0));
        let mut ledger = Vec::new();
        file(&mut ledger, &[old], "2026-11-22", 1000);
        assert!(ledger[0].open, "the figure past its target wasn't filed");
        file(&mut ledger, &[unread], "2026-11-22", 4600);
        assert!(
            ledger[0].open,
            "a drag closed while its apps' file went unread"
        );
        let written_unread = measure("apps:wait per wakeup", Class::Apps, 0.0, None);
        let unread_figure = [written_unread, renamed.clone()];
        file(&mut ledger, &unread_figure, "2026-11-22", 5800);
        assert!(
            ledger[0].open,
            "a drag closed while its figure was written with no reading"
        );
        let longer = measure("apps:wait per wakeup p99", Class::Apps, 1.0, Some(2.0));
        file(&mut ledger, &[longer, renamed], "2026-11-22", 8200);
        assert!(
            !ledger[0].open,
            "a drag on a figure gone from a file read whole stayed open for good"
        );
    }

    #[test]
    fn while_his_apps_lead_the_drag_his_misses_still_reach_their_owners() {
        let mut apps = measure("apps:call fps p10", Class::Apps, 6.0, Some(0.0));
        apps.owner = "APPS".to_owned();
        let mut launcher = measure("apps:launcher", Class::Apps, 180.0, Some(120.0));
        launcher.owner = "LAUNCHER".to_owned();
        let passing = measure("apps:freezes", Class::Apps, 0.0, Some(0.0));
        let mut misses = measure("misses", Class::Miss, 1.0, Some(0.0));
        misses.behind = vec!["History paused · RCTRF8".to_owned()];
        let slips = measure("slips", Class::Slip, 3.0, Some(0.0));
        let mut ledger = Vec::new();
        let all = [apps, launcher, passing, misses, slips];
        let told = file(&mut ledger, &all, "2026-11-19", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["APPS", "LAUNCHER", "RCTRF8"],
            "each figure past its target and the misses weren't filed"
        );
        assert_eq!(ledger[2].axis, Class::Miss.name(), "filed as a miss");
    }

    #[test]
    fn a_merge_whose_rounds_past_five_fixed_nothing_goes_to_who_landed_it() {
        let mut misses = measure("misses", Class::Miss, 1.0, Some(0.0));
        misses.behind = vec!["History paused · RCTRF8".to_owned()];
        let mut fruitless = measure(
            &format!("{}core.nix#12", rounds::FRUITLESS),
            Class::Ship,
            7.0,
            Some(5.0),
        );
        fruitless.owner = "B8RBNW".to_owned();
        let mut ledger = Vec::new();
        let told = file(
            &mut ledger,
            &[misses, fruitless.clone()],
            "2026-11-19",
            1000,
        );
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["RCTRF8", "B8RBNW"],
            "the merge's rounds weren't filed beside the miss that leads"
        );
        // Alone, it still files, and never leads the drag.
        let mut alone = Vec::new();
        let median = measure(rounds::KEY, Class::Ship, 2.0, Some(rounds::TARGET));
        let both = [median.clone(), fruitless];
        assert!(drag(&both).is_none(), "a merge's rounds led the drag");
        assert_eq!(file(&mut alone, &both, "2026-11-19", 1000).len(), 1);
        // Still in the day's count an hour on, it stays open; a day on, its
        // line gone, it closes; with the record unread, it would have
        // stayed open.
        file(&mut alone, &both, "2026-11-19", 4600);
        assert!(alone[0].open, "closed while the merge was still counted");
        let mut unread = median.clone();
        unread.says = "median council rounds, of 0 with a review record, 0 with none, this \
                       machine's review records unread"
            .to_owned();
        file(&mut alone, &[unread], "2026-11-20", 90_000);
        assert!(alone[0].open, "closed though the review record went unread");
        file(&mut alone, &[median], "2026-11-20", 90_000);
        assert!(!alone[0].open, "never closed once the merge left the day");
    }

    #[test]
    fn a_date_told_late_goes_to_its_owner_move_by_move_and_closes_once_its_line_is_gone() {
        let mut late = measure(told_late::KEY, Class::Slip, 2.0, Some(0.0));
        late.behind = vec![
            "a fix, moved 6 min before its date · J7XHQN".to_owned(),
            "a switch, moved in its last minute · 16V6F4".to_owned(),
        ];
        let median = measure(rounds::KEY, Class::Ship, 2.0, Some(rounds::TARGET));
        let both = [median, late.clone()];
        assert!(drag(&both).is_none(), "a date told late led the drag");
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &both, "2026-11-25", 1000);
        let mut owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        owners.sort_unstable();
        assert_eq!(
            owners,
            ["16V6F4", "J7XHQN"],
            "a date told late wasn't put to each owner"
        );
        let fix = told
            .iter()
            .find(|(to, _)| to == "J7XHQN")
            .map(|(_, text)| text.as_str())
            .unwrap_or_default();
        assert!(
            fix.contains("this move is yours: a fix, moved 6 min before its date."),
            "the owner wasn't told which move was theirs: {fix}"
        );
        let mut unread = late.clone();
        unread.behind.clear();
        unread.target = None;
        file(&mut ledger, &[unread], "2026-11-25", 4600);
        assert!(
            ledger.iter().all(|e| e.open),
            "a move closed while the asks list went unread"
        );
        late.behind.remove(1);
        late.behind
            .push("a deploy, moved 2 min before its date · J7XHQN".to_owned());
        let told = file(&mut ledger, &[late], "2026-11-25", 8200);
        assert!(
            told.iter()
                .any(|(to, text)| to == "J7XHQN" && text.contains("a deploy, moved 2 min")),
            "a later move of an owner with an ask open went untold: {told:?}"
        );
        let mut open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        open.sort_unstable();
        assert_eq!(
            open,
            [
                format!(
                    "hour:{}:J7XHQN:a deploy, moved 2 min before its date",
                    told_late::KEY
                ),
                format!(
                    "hour:{}:J7XHQN:a fix, moved 6 min before its date",
                    told_late::KEY
                ),
            ],
            "a move closed while its line stood, or stayed open once it was gone"
        );
    }

    #[test]
    fn an_ask_unproven_goes_to_its_cell_whatever_leads_and_closes_once_its_line_is_gone() {
        let card = "show the card, asked 2026-11-26T09:00:00Z";
        let map = "fix the map, asked 2026-11-26T08:00:00Z";
        let mut unproven = measure(unproven::KEY, Class::Functional, 2.0, Some(0.0));
        unproven.behind = vec![
            format!("{card}; its probe hasn't run since it landed · 1JQPPC"),
            format!("{map}; its probe failed · Y2YBTS"),
        ];
        let miss = measure("misses", Class::Miss, 1.0, Some(0.0));
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[miss, unproven.clone()], "2026-11-26", 1000);
        let card_told = told
            .iter()
            .find(|(to, _)| to == "1JQPPC")
            .map(|(_, text)| text.as_str())
            .unwrap_or_default();
        assert!(
            card_told.contains(&format!(
                "yours: {card}; its probe hasn't run since it landed."
            )),
            "the cell wasn't told which ask was theirs, while a miss led: {told:?}"
        );
        assert!(
            card_told.contains(&format!(
                "`cadence owe --done 'hour:{}:1JQPPC:{card}' WHY`",
                unproven::KEY
            )),
            "the cell wasn't told how to close it: {card_told}"
        );
        assert!(
            told.iter().any(|(to, _)| to == "Y2YBTS"),
            "each ask's cell wasn't asked apart: {told:?}"
        );
        let mut unread = unproven.clone();
        unread.behind.clear();
        unread.target = None;
        file(&mut ledger, &[unread], "2026-11-26", 4600);
        assert!(
            ledger.iter().all(|e| e.open),
            "an ask closed while the measure went unjudged"
        );
        // Its probe ran and failed: the same ask, its state moved on.
        unproven.behind[0] = format!("{card}; its probe failed after switch 1200 · 1JQPPC");
        unproven.behind.remove(1);
        unproven.now = 1.0;
        let told = file(
            &mut ledger,
            std::slice::from_ref(&unproven),
            "2026-11-26",
            8200,
        );
        assert!(
            told.is_empty(),
            "an ask whose state moved was asked again: {told:?}"
        );
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            [format!("hour:{}:1JQPPC:{card}", unproven::KEY)],
            "an ask closed while its line stood, or stayed open once it was gone"
        );
    }

    #[test]
    fn a_cells_close_with_its_why_holds_while_its_line_stands_and_no_longer() {
        let card = "show the card, asked 2026-11-26T09:00:00Z";
        let key = format!("hour:{}:1JQPPC:{card}", unproven::KEY);
        let failed = format!("{card}; its probe failed · 1JQPPC");
        let other = "fix the map, asked 2026-11-26T08:00:00Z; its probe failed · Y2YBTS";
        let with = |lines: &[&str]| {
            let mut m = measure(unproven::KEY, Class::Functional, 1.0, Some(0.0));
            m.behind = lines.iter().map(|&line| line.to_owned()).collect();
            m.now = f64::from(u32::try_from(lines.len()).unwrap_or(u32::MAX));
            m
        };
        let asked = |told: &[(String, String)]| told.iter().any(|(to, _)| to == "1JQPPC");
        let mut ledger = Vec::new();
        let hour = |ledger: &mut Vec<Entry>, lines: &[&str], at: i64| {
            file(ledger, &[with(lines)], "2026-11-26", at)
        };
        assert!(
            asked(&hour(&mut ledger, &[&failed], 1000)),
            "it wasn't asked"
        );
        // A bare close says no why, so holds nothing.
        let bare = owe::done(&mut ledger, &key, None, 2000).map_err(|f| f.exit);
        assert!(bare.is_ok(), "its cell couldn't close it: {bare:?}");
        assert!(
            asked(&hour(&mut ledger, &[&failed], 4600)),
            "a bare close held it"
        );
        // Closed with its why, it holds while its line says the same,
        // whatever other asks do to the figure.
        let why = owe::done(&mut ledger, &key, Some("needs his login"), 5000);
        assert!(
            why.is_ok(),
            "its cell couldn't close it: {:?}",
            why.err().map(|f| f.exit)
        );
        let told = hour(&mut ledger, &[&failed, other], 8200);
        assert!(
            !asked(&told),
            "a close with its why was asked again: {told:?}"
        );
        // Its state moves, so it is asked again.
        let moved = format!("{card}; its probe failed after switch 1201 · 1JQPPC");
        assert!(
            asked(&hour(&mut ledger, &[&moved], 11800)),
            "a close held once its state moved"
        );
        // Closed again with its why, then its probe passes and the same
        // failure comes back: what it said no longer holds.
        let again = owe::done(&mut ledger, &key, Some("needs his login"), 12000);
        assert!(again.is_ok(), "its cell couldn't close it again");
        hour(&mut ledger, &[other], 15400);
        let told = hour(&mut ledger, &[&moved], 19000);
        assert!(
            asked(&told),
            "a cell's close held through a pass, so the same failure went unasked: {told:?}"
        );
    }

    #[test]
    fn the_weeks_asks_naming_no_probe_are_their_keepers_to_write_once_while_any_is_left() {
        let mut unproven = measure(unproven::KEY, Class::Functional, 3.0, Some(0.0));
        let keeper = unproven::KEEPER;
        unproven.behind = vec![
            format!("{}; 2 of them · {keeper}", unproven::UNNAMED),
            format!("9 older than a week, left to the probes' backfill · {keeper}"),
        ];
        let mut ledger = Vec::new();
        let told = file(
            &mut ledger,
            std::slice::from_ref(&unproven),
            "2026-11-26",
            1000,
        );
        assert_eq!(
            told.iter()
                .map(|(to, text)| (to.as_str(), text.contains("write probes for")))
                .collect::<Vec<_>>(),
            [(keeper, true)],
            "the keeper wasn't asked to write the probes once, or the backfill was filed"
        );
        unproven.behind[0] = format!("{}; 1 of them · {keeper}", unproven::UNNAMED);
        let told = file(
            &mut ledger,
            std::slice::from_ref(&unproven),
            "2026-11-26",
            4600,
        );
        assert!(told.is_empty(), "a count moving asked again: {told:?}");
        unproven.behind.remove(0);
        file(&mut ledger, &[unproven], "2026-11-26", 8200);
        assert!(
            ledger.iter().all(|e| !e.open),
            "the ask stayed open once every probe was written"
        );
    }

    #[test]
    fn each_miss_of_joels_goes_to_its_own_owner_and_closes_once_its_check_is_named() {
        let mut misses = measure("misses", Class::Miss, 2.0, Some(0.0));
        misses.behind = vec![
            "the map had no network · KNURBN".to_owned(),
            "History paused · RCTRF8".to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[misses.clone()], "2026-11-14", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["KNURBN", "RCTRF8"], "each its own owner");
        assert!(
            told[0].1.contains("Which check would have caught it?"),
            "{}",
            told[0].1
        );
        misses.behind.remove(1);
        misses.now = 1.0;
        assert!(
            file(&mut ledger, &[misses], "2026-11-14", 4600).is_empty(),
            "told once"
        );
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            ["hour:miss:the map had no network"],
            "History's closed"
        );
    }

    #[test]
    fn a_miss_noted_after_its_owner_still_goes_to_that_owner() {
        let mut misses = measure("misses", Class::Miss, 2.0, Some(0.0));
        misses.behind = vec![
            "the map drew cells alone · WCJQSG, though its check map-cells still finds it"
                .to_owned(),
            "the notice stayed up · 88LQC0, though cadence couldn't read world's main to see \
             its check"
                .to_owned(),
        ];
        let told = file(&mut Vec::new(), &[misses], "2026-11-17", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["WCJQSG", "88LQC0"],
            "a miss's note was taken as part of its owner's address"
        );
    }

    /// An hour that couldn't read the misses ledger whole says nothing of
    /// which misses are gone, so it settles none of their findings.
    #[test]
    fn an_unread_misses_ledger_settles_no_miss_finding() {
        let mut misses = measure("misses", Class::Miss, 1.0, Some(0.0));
        misses.behind = vec![
            "the map drew cells alone · WCJQSG".to_owned(),
            "History paused · H2VYOB, though its check map-cells still finds it".to_owned(),
        ];
        let mut ledger = Vec::new();
        file(&mut ledger, &[misses], "2026-11-17", 1000);
        let unread = measure("misses", Class::Miss, 0.0, None);
        file(&mut ledger, &[unread], "2026-11-17", 2000);
        let open = ledger.iter().filter(|e| e.open).count();
        assert_eq!(
            open, 2,
            "an unread ledger settled its misses' findings: {ledger:?}"
        );
    }

    /// A measure the hour couldn't read holds a miss on the card, but the
    /// miss's owner can't move the hour's blindness, so isn't asked to.
    #[test]
    fn a_miss_held_by_a_blind_measure_is_not_put_to_its_owner() {
        let mut misses = measure("misses", Class::Miss, 1.0, Some(0.0));
        misses.behind = vec![
            "a miss with no check · H2VYOB".to_owned(),
            "the map drew cells alone · WCJQSG, though its check map-cells can't judge it yet"
                .to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[misses], "2026-11-17", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(
            owners,
            ["H2VYOB"],
            "a miss held only because the hour couldn't look was put to its owner"
        );
    }

    #[test]
    fn a_miss_held_by_its_merged_check_asks_its_owner_for_no_new_check() {
        let mut misses = measure("misses", Class::Miss, 1.0, Some(0.0));
        misses.behind = vec![
            "the map drew cells alone · WCJQSG, though its check map-cells still finds it"
                .to_owned(),
        ];
        // The owner was asked for a check before it merged, and that
        // finding is still open; the hold is news to them all the same.
        let mut ledger = Vec::new();
        let mut asked = misses.clone();
        asked.behind = vec!["the map drew cells alone · WCJQSG".to_owned()];
        file(&mut ledger, &[asked], "2026-11-16", 0);
        let told = file(
            &mut ledger,
            std::slice::from_ref(&misses),
            "2026-11-17",
            1000,
        );
        let text = told.first().map_or("nothing", |(_, text)| text.as_str());
        assert!(
            !text.contains("Which check would have caught it") && text.contains("map-cells"),
            "a miss whose check merged asked its owner for a check again, or wasn't told: {text}"
        );
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            ["hour:miss-held:the map drew cells alone"],
            "the ask for a check stayed open once it merged, or the hold wasn't filed"
        );
        let again = file(&mut ledger, &[misses], "2026-11-17", 1500);
        let still: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert!(
            again.is_empty() && still == ["hour:miss-held:the map drew cells alone"],
            "a hold a second hour running was closed or told again: {again:?}, open {still:?}"
        );
        let mut gone = measure("misses", Class::Miss, 1.0, Some(0.0));
        gone.behind = vec!["another miss · WCJQSG".to_owned()];
        file(&mut ledger, &[gone], "2026-11-17", 2000);
        assert!(
            ledger
                .iter()
                .all(|e| !e.open || e.key == "hour:miss:another miss"),
            "a hold its measure judged gone stayed open: {:?}",
            ledger
                .iter()
                .filter(|e| e.open)
                .map(|e| &e.key)
                .collect::<Vec<_>>()
        );
    }
}
