//! The order of Joel's workspaces, kept as he sets it and put back when a
//! screen comes or goes. niri gives a screen coming back the workspaces
//! first made on it, and a screen going sends its own after the others':
//! on 11 November 2026 a monitor plugged back in, and the laptop's lid
//! shut, left Joel's seven workspaces in the order 19, 21, 23, 20, 2, 22,
//! 30, his apps' workspace fifth rather than first, since it and two
//! others had been made on the laptop's screen.
//!
//! So the order is one list across every screen, kept at each change of
//! the workspaces while the screens stay the same, Joel's own moves
//! included: each screen's workspaces fill the places in it theirs held,
//! so it keeps how two screens' interleave. Only a change of the screens
//! puts each screen's workspaces back in it, never undoing a move of his.
//! An empty workspace has no place, since niri keeps one last on each
//! screen and drops any other.
//!
//! The shell takes a workspace niri left on another screen back to its
//! own once that's back (Sessions.qml's comeHome), 2 s after the last
//! change, so one changing screens while the order settles is put in its
//! place too. Each move is made from what niri lists just before it, so a
//! workspace moved meanwhile never leaves a move made for where it was.
//!
//! niri also moves the focus off a window whose workspace leaves the
//! focused screen, so the window Joel had focused as the screens changed
//! is focused again once his workspaces are back, unless he focused
//! another while they moved. niri lists the workspaces before it says
//! where the focus went, so as the screens change the focus heard is
//! still his.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long the order is held after the last change of the workspaces,
/// once a screen came or went, for it to be put back and the shell to
/// take workspaces home, so the order niri lists meanwhile, its own
/// rather than Joel's, isn't kept as his.
pub const SETTLING: Duration = Duration::from_secs(5);

/// How long niri is waited on for each ask.
const ASKING: Duration = Duration::from_secs(1);

/// The most moves one putting back makes, past which niri isn't following
/// them and the rest is left as it is.
const MOVES: usize = 64;

/// Which putting back is the newest: an older one stops at its next move.
static NEWEST: AtomicU64 = AtomicU64::new(0);

/// What one putting back puts back: the order kept, and the window Joel
/// had focused as the screens changed, if any, to focus again after.
#[derive(Debug, PartialEq, Eq)]
pub struct Putting {
    pub kept: Vec<u64>,
    pub focus: Option<u64>,
}

/// The workspaces' order, and what it was kept with.
#[derive(Debug, Default)]
pub struct Order {
    /// Every workspace holding a window, by id, in the order Joel last
    /// had them, across every screen.
    kept: Vec<u64>,
    /// The screens holding workspaces as the order was kept.
    screens: BTreeSet<String>,
    /// Each workspace holding a window, by id, with its screen, as last
    /// heard.
    held: Vec<(u64, String)>,
    /// Until when the order is held while it's put back.
    settling: Option<Instant>,
}

/// One workspace as niri lists it: its id, its screen, where it is on it,
/// counted from 1, and whether it holds a window.
struct Placed<'a> {
    id: u64,
    screen: &'a str,
    at: u64,
    held: bool,
}

/// The workspaces niri lists, each screen's top to bottom.
fn placed(workspaces: &[Value]) -> Vec<Placed<'_>> {
    let mut listed: Vec<Placed> = workspaces
        .iter()
        .filter_map(|one| {
            Some(Placed {
                id: one["id"].as_u64()?,
                screen: one["output"].as_str()?,
                at: one["idx"].as_u64()?,
                held: !one["active_window_id"].is_null(),
            })
        })
        .collect();
    listed.sort_by(|a, b| a.screen.cmp(b.screen).then(a.at.cmp(&b.at)));
    listed
}

impl Order {
    /// Hears the workspaces niri lists now, with the window focused, and
    /// gives what's to be put back: none unless the screens holding them
    /// changed since the order was kept, or a workspace changed screens
    /// while it settles. Only the screens changing focuses a window again,
    /// so a window Joel focuses while the order settles stays focused.
    pub fn heard(
        &mut self,
        workspaces: &[Value],
        focused: Option<u64>,
        now: Instant,
    ) -> Option<Putting> {
        let listed = placed(workspaces);
        let screens: BTreeSet<String> = listed.iter().map(|one| one.screen.to_owned()).collect();
        let mut held: Vec<(u64, String)> = listed
            .iter()
            .filter(|one| one.held)
            .map(|one| (one.id, one.screen.to_owned()))
            .collect();
        held.sort();
        let moved = held != self.held;
        self.held = held;
        let came = !self.screens.is_empty() && screens != self.screens;
        self.screens = screens;
        let settling = self.settling.is_some_and(|until| now < until);
        if came || (settling && moved) {
            self.settling = Some(now + SETTLING);
            let focus = focused.filter(|_| came);
            return Some(Putting {
                kept: self.kept.clone(),
                focus,
            });
        }
        if settling {
            // Held until the workspaces rest, so the shell's taking them
            // home, 2 s after the last change, always comes inside it.
            self.settling = Some(now + SETTLING);
        } else {
            self.settling = None;
            self.keep(&listed);
        }
        None
    }

    /// Keeps the order niri lists: each screen's workspaces, top to
    /// bottom, in the places theirs held in the order, those gone let go
    /// and those new after every other.
    fn keep(&mut self, listed: &[Placed]) {
        let held: Vec<&Placed> = listed.iter().filter(|one| one.held).collect();
        let screen_of = |id: u64| held.iter().find(|one| one.id == id).map(|one| one.screen);
        self.kept.retain(|id| screen_of(*id).is_some());
        let mut kept = self.kept.clone();
        let screens: BTreeSet<&str> = held.iter().map(|one| one.screen).collect();
        for screen in screens {
            let mut theirs = held
                .iter()
                .filter(|one| one.screen == screen && self.kept.contains(&one.id))
                .map(|one| one.id);
            for (place, id) in kept.iter_mut().zip(&self.kept) {
                if screen_of(*id) == Some(screen)
                    && let Some(next) = theirs.next()
                {
                    *place = next;
                }
            }
        }
        kept.extend(
            held.iter()
                .map(|one| one.id)
                .filter(|id| !self.kept.contains(id)),
        );
        self.kept = kept;
    }
}

/// The next move putting the workspaces niri lists in the order kept:
/// the highest on its screen that's out of place goes to its place, so
/// each lands above those moved after it. One the order doesn't hold, as
/// the empty one niri keeps last, stays after those it does, as niri has
/// them. None once each screen's are in order.
pub fn next(kept: &[u64], workspaces: &[Value]) -> Option<Value> {
    let listed = placed(workspaces);
    let rank = |id: u64| kept.iter().position(|one| *one == id).unwrap_or(usize::MAX);
    let screens: BTreeSet<&str> = listed.iter().map(|one| one.screen).collect();
    screens.into_iter().find_map(|screen| {
        let now: Vec<u64> = listed
            .iter()
            .filter(|one| one.screen == screen)
            .map(|one| one.id)
            .collect();
        let mut wanted = now.clone();
        wanted.sort_by_key(|id| rank(*id));
        let index = now.iter().zip(&wanted).position(|(is, was)| is != was)?;
        Some(json!({"Action": {"MoveWorkspaceToIndex": {
            "index": index + 1,
            "reference": {"Id": wanted[index]}
        }}}))
    })
}

/// Puts the workspaces back in the order kept, on a thread of its own so
/// no event waits on it: each move made from what niri lists just before
/// it, until they're in order or a newer putting back comes, and then the
/// window Joel had focused focused again. A move niri refuses, or doesn't
/// answer, is said in the journal and ends the moves.
pub fn put_back(niri: &Path, putting: Option<Putting>) {
    let Some(putting) = putting else {
        return;
    };
    let mine = NEWEST.fetch_add(1, Ordering::SeqCst) + 1;
    let niri: PathBuf = niri.to_owned();
    std::thread::spawn(move || putting_back(&niri, &putting, &NEWEST, mine));
}

/// One putting back, while `mine` is the `newest`: the moves, then the
/// focus. niri keeps each screen's active workspace through the moves, so
/// the focus goes back once, after them. Where the focus is before them
/// is where niri put it as the screens changed, so one elsewhere after
/// them is Joel's own press, which stays, as does the focus of a newer
/// putting back that took over the moves (Y2YBTS, 11 November 2026).
fn putting_back(niri: &Path, putting: &Putting, newest: &AtomicU64, mine: u64) {
    let put = putting.focus.map_or(Focus::Unsaid, |_| focused(niri));
    move_back(niri, &putting.kept, newest, mine);
    let Some(window) = putting.focus else {
        return;
    };
    let now = focused(niri);
    if put == Focus::Unsaid || now == Focus::Unsaid {
        eprintln!(
            "<4>niri didn't say where its focus was, so window {window} wasn't focused again"
        );
        return;
    }
    if newest.load(Ordering::SeqCst) != mine || (now != Focus::Nowhere && now != put) {
        return;
    }
    let again = json!({"Action": {"FocusWindow": {"id": window}}});
    if let Some(why) = refused(niri, &again) {
        eprintln!("<4>niri didn't focus window {window} again as the screens changed: {why}");
    }
}

/// Where niri's focus is, as it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    /// niri didn't say.
    Unsaid,
    /// No window is focused.
    Nowhere,
    /// This window is.
    On(u64),
}

/// Where niri's focus is now.
fn focused(niri: &Path) -> Focus {
    let Some(answer) = answer(niri, &json!("FocusedWindow")) else {
        return Focus::Unsaid;
    };
    match answer.get("Ok").and_then(|ok| ok.get("FocusedWindow")) {
        None => Focus::Unsaid,
        Some(window) => window["id"].as_u64().map_or(Focus::Nowhere, Focus::On),
    }
}

/// Moves the workspaces into the order kept, one at a time, until they're
/// in it or a newer putting back than `mine` comes.
fn move_back(niri: &Path, kept: &[u64], newest: &AtomicU64, mine: u64) {
    for _ in 0..MOVES {
        if newest.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(listed) = answer(niri, &json!("Workspaces")) else {
            eprintln!("<4>niri didn't list the workspaces to put them back in order");
            return;
        };
        let workspaces = listed["Ok"]["Workspaces"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        let Some(one) = next(kept, workspaces) else {
            return;
        };
        if let Some(why) = refused(niri, &one) {
            eprintln!("<4>niri didn't put a workspace back in its place ({one}): {why}");
            return;
        }
    }
}

/// Why niri didn't do one request, or none where it did.
fn refused(niri: &Path, request: &Value) -> Option<String> {
    match answer(niri, request) {
        None => Some("no answer".to_owned()),
        Some(answer) => answer["Err"].as_str().map(str::to_owned),
    }
}

/// niri's answer to one request, or none where it didn't answer in time.
fn answer(niri: &Path, request: &Value) -> Option<Value> {
    let mut stream = UnixStream::connect(niri).ok()?;
    stream.set_read_timeout(Some(ASKING)).ok()?;
    // The request goes as one line in one write: serde_json says a value
    // in many small pieces, each its own write on a bare socket, and a
    // reader taking it a read at a time (tests/screens-off-on-niri-test.py's
    // proxy) saw a request cut in two.
    stream.write_all(format!("{request}\n").as_bytes()).ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

#[cfg(test)]
mod tests;
