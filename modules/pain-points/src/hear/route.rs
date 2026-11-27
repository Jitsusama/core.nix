//! Where each pain goes, and what it says there. A pain goes to the cell
//! of the session he said it in, which was working on what hurt, as the
//! mycelium's sessions name it. One said to the void, or in a session
//! that has ended, goes to a holder of its subject through that subject's
//! hub, as the node names hubs: its session's purpose where it had one,
//! else his words. Only where no hub takes it does it go to the fallback
//! the routes file names, which once took every such pain and so was
//! their funnel; with none named it waits, counted open, as one said in a
//! session the mycelium doesn't know waits. The void is never sent
//! anything, nor is any cell the routes file says never to send to.
//!
//! What a finding says stays short and carries links rather than
//! context: the cell and the log entry it was heard at, so whoever reads
//! it opens the rest.

use crate::hear::heard::Heard;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// How many pains one finding names; the rest wait for the next run.
pub const NAMED_AT_MOST: usize = 5;

/// How many pains the ledgers' keeper's note names; it says how many more
/// there were.
pub const NOTED_AT_MOST: usize = 15;

/// His words are cut to this many characters in a finding.
const WORDS_AT_MOST: usize = 280;

/// A session's cell, as the mycelium knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub address: String,
    pub live: bool,
    /// What its session works on, as its forage names it, where it says.
    pub purpose: Option<String>,
}

/// What the routes file, beside the listener's state, says: who findings
/// are from, where those no live cell takes go, and who keeps the asks
/// and misses ledgers.
///
/// A key it doesn't know is refused rather than skipped, so "nevr" can't
/// leave the void unguarded.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routes {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default)]
    pub ledgers: Option<String>,
    /// Cells nothing is ever sent to, the void's among them, so a void
    /// begun somewhere its folder doesn't say is still never written to.
    #[serde(default)]
    pub never: Vec<String>,
    /// Cells whose work the ledgers' keeper leaves to them: of a pain sent
    /// to one, the keeper is told with its next turn, wanting no answer,
    /// rather than asked, since its owner answers it.
    #[serde(default)]
    pub told: Vec<String>,
}

impl Routes {
    /// Whether `address` is one nothing is ever sent to, in any case.
    pub fn refuses(&self, address: &str) -> bool {
        self.never.iter().any(|n| n.eq_ignore_ascii_case(address))
    }

    /// Whether the keeper is only told of a pain sent to `address`, in
    /// any case: never the fallback's, as a pain sent there went to no
    /// owner of its own.
    pub fn only_tells(&self, address: &str) -> bool {
        let fallback = self
            .fallback
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case(address));
        !fallback && self.told.iter().any(|n| n.eq_ignore_ascii_case(address))
    }

    /// Why routes.json can't be used, when it's there but can't be read.
    pub fn trouble(state: &Path) -> Option<String> {
        // Only a routes file that isn't there is a listener nobody set up;
        // one there but unreadable, or naming no one to send as, is a
        // listener set up wrong, which must not read as a quiet hour.
        let text = match fs::read(state.join("routes.json")) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => {
                return Some(format!(
                    "routes.json can't be read ({e}), so nothing was sent"
                ));
            }
        };
        match serde_json::from_slice::<Self>(&text) {
            Err(e) => Some(format!(
                "routes.json can't be read ({e}), so nothing was sent"
            )),
            Ok(routes) if routes.from.is_none() => {
                Some("routes.json names no one to send as, so nothing was sent".into())
            }
            Ok(_) => None,
        }
    }

    /// Whether anything may be sent: only with someone to send as, and
    /// cells named never to send to, as a routes file that names none
    /// most likely lost the void's line rather than meant it.
    pub fn sends(&self) -> bool {
        self.from.is_some() && !self.never.is_empty()
    }

    /// The routes file in the listener's state, or none named.
    pub fn read(state: &Path) -> Self {
        fs::read(state.join("routes.json"))
            .ok()
            .and_then(|text| serde_json::from_slice(&text).ok())
            .unwrap_or_default()
    }
}

/// Each session the mycelium knows, by its id, from what
/// `mycelium sessions` prints.
pub fn cells(printed: &str) -> BTreeMap<String, Cell> {
    #[derive(Deserialize)]
    struct Printed {
        #[serde(rename = "Ok")]
        ok: Known,
    }
    #[derive(Deserialize)]
    struct Known {
        #[serde(rename = "Sessions")]
        sessions: Vec<Session>,
    }
    #[derive(Deserialize)]
    struct Session {
        id: String,
        address: String,
        #[serde(default)]
        state: String,
        #[serde(default)]
        working_on: Option<String>,
    }
    serde_json::from_str::<Printed>(printed)
        .map(|p| p.ok.sessions)
        .unwrap_or_default()
        .into_iter()
        .map(|s| {
            let live = s.state != "ended";
            (
                s.id,
                Cell {
                    address: s.address,
                    live,
                    purpose: s.working_on.filter(|p| !p.trim().is_empty()),
                },
            )
        })
        .collect()
}

/// The address a pain said in `session` goes to, or none for now: its own
/// live cell, else the fallback when the session ended or is the void.
pub fn owner(
    session: &str,
    void: bool,
    cells: &BTreeMap<String, Cell>,
    routes: &Routes,
) -> Option<String> {
    // A session this realm's mycelium doesn't know may be another realm's,
    // whose words don't cross into this one, so it waits, counted open.
    cells.get(session)?;
    own(session, void, cells, routes).or_else(|| routes.fallback.clone())
}

/// The words a pain's subject is read from where no live cell of its own
/// takes it: its session's purpose, where the session had one and wasn't
/// the void or a cell never sent to, since that is what the work was;
/// else his words. A stall's words are the listener's own account, so
/// they name a subject only where the provider failed him, which the
/// provider's owner holds; another stall without a purpose names none.
pub fn said(pain: &Heard, cells: &BTreeMap<String, Cell>, routes: &Routes) -> Vec<String> {
    let purpose = cells
        .get(&pain.session)
        .filter(|cell| !pain.void && !routes.refuses(&cell.address))
        .and_then(|cell| cell.purpose.as_deref());
    if let Some(purpose) = purpose {
        return subject_hubs::words(purpose);
    }
    if super::stalled::is(pain) {
        return if pain.words.contains(super::stalled::PROVIDER_FAILING) {
            subject_hubs::words(super::stalled::PROVIDER_FAILING)
        } else {
            Vec::new()
        };
    }
    subject_hubs::words(&pain.words)
}

/// The hub that takes the pain keyed `key`, by the words `said` it names,
/// and the holder it goes to: of the best ranked live hub, as
/// subject-hubs' `taking` ranks them, the live holder holding the fewest
/// of `held`, never one the routes say never to send to. The pain's key
/// draws the lot among hubs that rank the same, so one pain meets them in
/// one order run after run, and pains spread over them. A holder
/// in `failed_at`, where a send of the pain failed, isn't given it again
/// while another fits, a broader hub's included. Counting failures into
/// the list instead would land back on one of them once the counts
/// reordered it (#6774's round, F1 and F7). A holder at `late_at`, where
/// a send of it ran past its limit and so may have gone, is given it
/// again while it fits, so no second holder works on it beside the first
/// (#6774's round, F16). A hub counts while its cell is among `live`,
/// every live cell, and gives the pain only to one of `takers`: a
/// session heard in the void holds none, though a hub it keeps still
/// stands for its holders (#7022's round, F3).
pub fn holder<'a>(
    (key, said): (&str, &[String]),
    hubs: &'a [subject_hubs::Hub],
    (live, takers): (&BTreeSet<String>, &BTreeSet<String>),
    held: &BTreeMap<String, usize>,
    routes: &Routes,
    failed_at: &[String],
    late_at: Option<&str>,
) -> Option<(&'a subject_hubs::Hub, &'a str, Vec<String>)> {
    let stands = |a: &str| live.contains(a) && !routes.refuses(a);
    let may_hold = |a: &str| takers.contains(a) && !routes.refuses(a);
    // Each hub that takes it, best ranked first, with its live holders
    // least-held first.
    let taking: Vec<(&subject_hubs::Hub, Vec<String>, Vec<&String>)> =
        subject_hubs::taking(key, said, hubs, &stands)
            .into_iter()
            .map(|(hub, subject)| {
                let mut fits: Vec<&String> = hub.fits.iter().filter(|f| may_hold(f)).collect();
                // Stable, so holders holding as many keep the node's order.
                fits.sort_by_key(|f| held.get(f.as_str()).copied().unwrap_or_default());
                (hub, subject, fits)
            })
            .collect();
    let pick = |fit_for: &dyn Fn(&str) -> bool| {
        taking.iter().find_map(|(hub, subject, fits)| {
            let fit = fits.iter().copied().find(|f| fit_for(f))?;
            Some((*hub, fit.as_str(), subject.clone()))
        })
    };
    // A late send may have landed, so its holder keeps the pain wherever
    // it still fits. Else it goes to a holder no send failed at, of any
    // hub that takes it, before going back to one that failed, so a hub
    // of one holder doesn't hold a pain a broader hub could take (#6774's
    // round, F6).
    pick(&|f| late_at == Some(f))
        .or_else(|| pick(&|f| !failed_at.iter().any(|x| x == f)))
        .or_else(|| pick(&|_| true))
}

/// Whether a pain said in `session` goes to its own cell: live, not the
/// void, and not one the routes say never to send to.
pub fn own(
    session: &str,
    void: bool,
    cells: &BTreeMap<String, Cell>,
    routes: &Routes,
) -> Option<String> {
    cells
        .get(session)
        .filter(|cell| cell.live && !void && !routes.refuses(&cell.address))
        .map(|cell| cell.address.clone())
}

/// His local time of day for a moment, as HH:MM.
fn clock(at: i64, offset_s: i64) -> String {
    let minutes = (at / 1000 + offset_s).rem_euclid(86_400) / 60;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Where a pain was heard, as a link into the mycelium where it can be.
fn place(pain: &Heard, cells: &BTreeMap<String, Cell>) -> String {
    match cells.get(&pain.session) {
        _ if pain.void => format!(
            "in the void (mycelium:session/{}#{})",
            pain.session, pain.entry
        ),
        Some(cell) if cell.live => format!("mycelium:cell/{}#{}", cell.address, pain.entry),
        Some(cell) => format!(
            "mycelium:cell/{}#{}, a session that has ended",
            cell.address, pain.entry
        ),
        None => format!(
            "mycelium:session/{}#{}, a session the mycelium doesn't know",
            pain.session, pain.entry
        ),
    }
}

/// The most of a window's title a finding quotes.
const TITLE_AT_MOST: usize = 80;

/// His words, or the node's, cut short where they run long.
pub(super) fn quoted(words: &str) -> String {
    // His words can't close their own quote and speak outside it, as a
    // window's title can't, nor carry a control character, which a send's
    // arguments can't hold.
    let flat = words
        .replace('"', "'")
        .replace(char::is_control, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if flat.chars().count() <= WORDS_AT_MOST {
        return format!("\"{flat}\"");
    }
    let cut: String = flat.chars().take(WORDS_AT_MOST).collect();
    format!("\"{cut}...\"")
}

/// What the keeper's note of new pains asks of it, so its thread is told
/// from a cell's: it asks nothing of an owner.
pub const FOR_THE_KEEPER: &str = "for you to carry into asks or misses";

/// The finding a cell is sent for its pains, worst first, each with the
/// link to where it was heard. Only tests ask for it with no pain put
/// through a hub; the listener says each one's subject.
#[cfg(test)]
pub fn finding(pains: &[&Heard], cells: &BTreeMap<String, Cell>, offset_s: i64) -> String {
    finding_held(pains, cells, offset_s, &BTreeMap::new())
}

/// `finding`, saying of each pain put to the cell through a hub, by its
/// key in `held_as`, the subject it holds that the pain went by, so it
/// knows why a pain from another's session is its own.
pub fn finding_held(
    pains: &[&Heard],
    cells: &BTreeMap<String, Cell>,
    offset_s: i64,
    held_as: &BTreeMap<String, String>,
) -> String {
    let mut said = String::from(
        "Joel was hurt, as pain-points hear heard it in his sessions: by what he said, or a \
         turn that stalled him. For each: \
         write a red case that shows it, fix it, and answer with a fix time in hours \
         (same day for a quick fix); if it isn't yours, send it to its owner. It stays open \
         until its fix runs here: once it lands, reply to this with \"merged at\" and the \
         commit, or name its #entry beside them in your landing note.",
    );
    for (n, pain) in pains.iter().take(NAMED_AT_MOST).enumerate() {
        // A subject is a cell's own word, which the node gives only as
        // one plain word of at most 80 characters, with no space or quote
        // mark: it is quoted as the name it is, so it reads as no
        // instruction of this finding's (#6774's round, F14).
        let to = held_as.get(&pain.key).map_or(String::new(), |subject| {
            format!(", put to you as a holder of the subject \"{subject}\"")
        });
        let mut parts = vec![line(n, pain, &to, cells, offset_s)];
        if let Some(window) = &pain.looking_at {
            // A title is whatever a program named its window, so it is
            // quoted as what it says rather than said as a fact, and cut.
            // Its own quote marks become single ones, so it can't close
            // the quote and speak outside it.
            let title: String = window
                .chars()
                .take(TITLE_AT_MOST)
                .map(|c| match c {
                    '"' => '\'',
                    c if c.is_control() => ' ',
                    c => c,
                })
                .collect();
            let cut = if title.chars().count() < window.chars().count() {
                "..."
            } else {
                ""
            };
            parts.push(format!(
                " He was looking at a window titled \"{title}{cut}\"."
            ));
        }
        if !pain.tools.is_empty() {
            parts.push(format!(" Before it: {}", pain.tools.join(", ")));
            if !pain.failed.is_empty() {
                let failed: Vec<String> =
                    pain.failed.iter().map(|t| format!("{t} failed")).collect();
                parts.push(format!(" ({})", failed.join(", ")));
            }
            parts.push(".".into());
        }
        said += &parts.concat();
    }
    said
}

/// One numbered pain: when, where, how it hurt, his words, and how often
/// he has said them.
fn line(n: usize, pain: &Heard, to: &str, cells: &BTreeMap<String, Cell>, offset_s: i64) -> String {
    let again = if pain.hits > 1 {
        format!(" He has said this {} times in a fortnight.", pain.hits)
    } else {
        String::new()
    };
    format!(
        "\n{}. {}, {}{to}, {}: {}.{again}",
        n + 1,
        clock(pain.at, offset_s),
        place(pain, cells),
        pain.kinds.join(", "),
        // A stall is the listener's own account, not his words to quote.
        if super::stalled::is(pain) {
            pain.words.clone()
        } else {
            quoted(&pain.words)
        }
    )
}

/// The note the ledgers' keeper is sent of new pains, so it can carry them
/// into asks or misses: the worst first, at most `NOTED_AT_MOST` of them.
/// One sent to its owner says so; one heard hours after he said it went to
/// no one else, since its owner has long moved on. One `asked` wants the
/// keeper's answer; one told of pains its list leaves to their owners
/// says there's nothing to answer.
pub fn ledger_note(
    pains: &[&Heard],
    cells: &BTreeMap<String, Cell>,
    offset_s: i64,
    asked: bool,
) -> String {
    let count = pains.len();
    let mut ranked = pains.to_vec();
    ranked.sort_by_key(|pain| (std::cmp::Reverse(pain.hits), pain.at));
    let noun = if count == 1 { "pain" } else { "pains" };
    let mut said = if asked {
        format!(
            "{count} new {noun} heard from Joel, {FOR_THE_KEEPER} where they belong; each sent \
             to its owner says so:"
        )
    } else {
        format!(
            "{count} new {noun} heard from Joel, each sent to an owner whose work your list \
             leaves to it, so nothing to answer; {FOR_THE_KEEPER} only if one is his ask:"
        )
    };
    for (n, pain) in ranked.iter().take(NOTED_AT_MOST).enumerate() {
        let to = pain
            .to
            .as_deref()
            .map_or(String::new(), |to| format!(", sent to {to}"));
        said += &line(n, pain, &to, cells, offset_s);
    }
    if count > NOTED_AT_MOST {
        let more = count - NOTED_AT_MOST;
        said = format!("{said}\nAnd {more} more, in pain-points' heard.jsonl.");
    }
    said
}

#[cfg(test)]
mod tests;
