//! Every open tab as this daemon serves them, Chrome's and kitty's, each
//! a row of their own place (`tab_rows.rs`), made from what each program
//! reading Chrome's session has open, kitty's open list and the windows
//! daemon's windows: read as a watch of them begins, Chrome's again as
//! the session folder changes and kitty's as kitty tells it changed, for
//! as long as one wants them (heard.rs, told.rs), so home's search finds
//! a tab Joel opened a moment ago; and made again as a window opens,
//! closes or is looked at (rows.rs), or an agent's comes or goes
//! (agents.rs). Go to the Tab and Close the Tab on one of kitty's are
//! asks of this daemon, each answered at once and done on a thread of
//! its own, so no other ask or watch waits on kitty.

use std::sync::Arc;

use programs::interface::{TabAct, TabAsk};
use programs::kitty_tabs::Kitty;
use programs::source::Without;
use programs::tab_rows::{self, PLACE};
use programs::{command, now_ms};
use serde_json::{Value, json};

use super::{Daemon, Run, Shared, agents, bookmarks, due, keep, lock, programs, start};

/// Whether a watch wants the tabs' rows, as a whole place.
pub fn watched(daemon: &Daemon) -> bool {
    daemon.watches.iter().any(|watch| {
        watch.keys.is_none()
            && watch.place.as_deref() == Some(PLACE)
            && watch.kinds.iter().any(|kind| kind == "row")
    })
}

/// Whether what Chrome has open is wanted for a place's rows: the tabs',
/// or the bookmarks', which leave out a page open now.
pub fn wanted(daemon: &Daemon) -> bool {
    watched(daemon) || bookmarks::watched(daemon)
}

/// Where kitty's open list is among the sources: the one served without
/// the agents' panes, a line a kitty (kitty's module, `without`).
pub fn kitty(shared: &Shared) -> Option<usize> {
    shared
        .sources
        .iter()
        .position(|source| source.without == Some(Without::Agents))
}

/// Reads what each program reading Chrome's session has open, as a watch
/// begins while the tabs or the bookmarks are watched, each on a thread
/// of its own: only what isn't held yet, since what is was read again at
/// every change of the session while they were watched, so a watch
/// beginning reads nothing more. kitty's list is read as it's due, or
/// never read yet, while the tabs are watched.
pub fn read(shared: &Arc<Shared>) {
    let (unread, listed) = {
        let daemon = lock(&shared.daemon);
        if !wanted(&daemon) {
            return;
        }
        let unread: Vec<usize> = shared
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| source.session.is_some())
            .filter(|(_, source)| {
                source
                    .opened_key()
                    .is_some_and(|key| !daemon.held.contains_key(&key))
            })
            .map(|(at, _)| at)
            .collect();
        let listed = kitty(shared).filter(|&at| {
            watched(&daemon)
                && (due(shared, &daemon, at)
                    || !daemon.held.contains_key(&shared.sources[at].list_key()))
        });
        (unread, listed)
    };
    for at in unread {
        start(shared, at, Run::Opened);
    }
    if let Some(at) = listed {
        start(shared, at, Run::List);
    }
}

/// What a source's command read or refused, as it's kept: the rows and
/// the head made again where it was what Chrome or kitty has open and the
/// tabs are watched.
pub fn heard(shared: &Shared, daemon: &mut Daemon, at: usize, what: Run) {
    let theirs = match what {
        Run::Opened => shared.sources[at].session.is_some(),
        Run::List => kitty(shared) == Some(at),
    };
    if theirs && watched(daemon) {
        make(shared, daemon);
    }
}

/// A folder of entries changed, so a program may be named anew: the tabs
/// are made again while a watch wants them, each said in its new name.
pub fn renamed(shared: &Shared, daemon: &mut Daemon) {
    if watched(daemon) {
        make(shared, daemon);
    }
}

/// Makes the rows again from what's held open, keeping what changed and
/// telling each watch only the rows that read differently and those gone.
/// kitty's are of niri's windows but the agents', whose panes its list
/// leaves out. The head names each program whose last read was refused.
/// Each program is named as its desktop entry names it, so as its
/// Programs row is, the entries read once as Programs' are.
pub fn make(shared: &Shared, daemon: &mut Daemon) {
    if daemon.catalogue.is_none() {
        daemon.catalogue = Some(programs(shared, daemon, None));
    }
    let words = shared
        .tab_words
        .naming(daemon.catalogue.as_deref().unwrap_or_default());
    let theirs = agents::windows(daemon);
    let windows: Option<Vec<&Value>> = daemon.windows.as_ref().map(|windows| {
        windows
            .iter()
            .filter(|(key, _)| !theirs.contains(*key))
            .map(|(_, window)| window)
            .collect()
    });
    let listed = kitty(shared).and_then(|at| {
        let source = &shared.sources[at];
        Some((source.app.as_deref()?, daemon.held.get(&source.list_key())?))
    });
    let tabs = listed.map(|(app, record)| Kitty {
        app,
        lines: record["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect(),
        clock: clock(),
        home: &shared.home,
    });
    let rows = tab_rows::rows(
        shared
            .sources
            .iter()
            .filter(|source| source.session.is_some())
            .filter_map(|source| {
                let app = source.app.as_deref()?;
                Some((app, daemon.held.get(&source.opened_key()?)?))
            }),
        tabs.as_ref(),
        windows.as_deref(),
        &words,
    );
    let kitty = kitty(shared);
    let silent = shared
        .sources
        .iter()
        .enumerate()
        .filter_map(|(at, source)| {
            let key = if source.session.is_some() {
                source.opened_key()?
            } else if kitty == Some(at) {
                source.list_key()
            } else {
                return None;
            };
            let refused = daemon.commands.get(&key)?["state"] == "refused";
            refused.then_some(source.app.as_deref()?)
        });
    let head = tab_rows::head(silent, windows.as_deref(), &words);
    let moved = daemon.tabs.take(rows, head.into_iter().collect());
    keep(daemon, moved.changed, &moved.gone);
}

/// What moves a time on the monotonic clock, niri's focus times', onto
/// the wall's, in milliseconds; none where the clock can't be read.
fn clock() -> i64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes only the timespec it's handed, which
    // lives for the call.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut now) } != 0 {
        return 0;
    }
    let monotonic = now.tv_sec.saturating_mul(1000) + now.tv_nsec / 1_000_000;
    i64::try_from(now_ms()).unwrap_or(i64::MAX) - monotonic
}

/// Goes to one of kitty's tabs, its OS window through niri and then the
/// tab inside it through kitty, or closes it, answered at once: kitty
/// tells its list changed as it's done, and the rows follow. Go's token
/// came with his press, which is all it's for here, since niri focuses a
/// window on its own ask.
pub fn act(id: &Value, asked: TabAsk) -> Value {
    let TabAsk {
        act,
        pid,
        tab,
        window,
    } = asked;
    let kitten = tool("PROGRAMS_KITTEN", "kitten");
    let to = format!("unix:@kitty-{pid}");
    let matched = format!("id:{tab}");
    let mut runs = Vec::new();
    if act == TabAct::Go
        && let Some(window) = window
    {
        let niri = tool("PROGRAMS_NIRI", "niri");
        runs.push(vec![
            niri,
            "msg".into(),
            "action".into(),
            "focus-window".into(),
            "--id".into(),
            window.to_string(),
        ]);
    }
    let verb = match act {
        TabAct::Go => "focus-tab",
        TabAct::Close => "close-tab",
    };
    runs.push(vec![
        kitten,
        "@".into(),
        "--to".into(),
        to,
        verb.into(),
        "--match".into(),
        matched,
    ]);
    std::thread::spawn(move || {
        for argv in runs {
            if let Err(why) = command::run(&argv) {
                eprintln!("a kitty tab's {verb} wasn't done: {why}");
                return;
            }
        }
    });
    json!({"id": id, "asked": verb})
}

/// A program this runs, by the variable naming it, else by its name.
fn tool(variable: &str, name: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| name.to_owned())
}
