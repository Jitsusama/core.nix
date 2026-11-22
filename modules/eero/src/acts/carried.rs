//! How the daemon carries out an act a listener asked for, by its two
//! rules: an act whose asker has gone before the daemon reaches it is let
//! go undone, and one whose ask can't be logged isn't done. It takes the
//! act to do as a function, so the rules are tested apart from eero's
//! session, which the daemon alone opens.

use std::io;
use std::path::Path;

use crate::acts::{Act, Came, changes};
use crate::listeners::Asked;

/// What became of an act asked.
#[derive(Debug)]
pub enum Carried {
    /// Who asked it left before the daemon reached it, so it was let go.
    Gone,
    /// Its ask couldn't be logged, so it wasn't done.
    Unlogged(io::Error),
    /// It was done, and came to this.
    Came(Came),
}

/// Carries out `asked` with `do_it`: let go undone where its asker no
/// longer `waits`, not done where its ask can't be written to `log`, and
/// otherwise logged before it's done and again after, so one cut short
/// still leaves a line. The rule covers every act, the speed test and
/// conferencing too, since each act the daemon does is to be found in the
/// log; an act let go is logged as `gone` where it can be.
pub fn carried(
    log: Result<&Path, &io::Error>,
    waits: bool,
    asked: &Asked,
    do_it: impl FnOnce(Act) -> Came,
) -> Carried {
    let unlogged = |error: &io::Error| io::Error::new(error.kind(), error.to_string());
    if !waits {
        eprintln!(
            "eero: let {:?} go undone, since who asked it has gone",
            asked.act
        );
        if let Err(error) = log
            .map_err(unlogged)
            .and_then(|log| changes::gone(log, asked, changes::now()))
        {
            eprintln!("eero: couldn't log {:?} let go: {error}", asked.act);
        }
        return Carried::Gone;
    }
    let log = match log.map_err(unlogged) {
        Ok(log) => log,
        Err(error) => return Carried::Unlogged(error),
    };
    if let Err(error) = changes::asked(log, asked, changes::now()) {
        return Carried::Unlogged(error);
    }
    let came = do_it(asked.act);
    if let Err(error) = changes::ended(log, asked, &came, changes::now()) {
        eprintln!("{}", unended_line(asked, &came, &error));
    }
    Carried::Came(came)
}

/// What the journal says of an act whose ask couldn't be logged: the act
/// and who asked it, since the change log is down and the journal is then
/// the only record of either, as the CLI's pointer to it promises. The
/// reason is kept to one line, so no line break in it writes an entry of
/// its own.
pub fn unlogged_line(asked: &Asked, error: &io::Error) -> String {
    let peer = asked.peer.as_ref();
    format!(
        "eero: {:?} wasn't done, since its ask couldn't be logged (pid {:?}, uid {:?}, unit \
         {:?}): {}",
        asked.act,
        peer.and_then(|peer| peer.pid),
        peer.map(|peer| peer.uid),
        peer.and_then(|peer| peer.unit.as_deref()),
        terminal_kit::layout::one_line(&error.to_string()),
    )
}

/// What the journal says of an act done whose end couldn't be logged:
/// what it came to, and the address an unreservation took away, since the
/// failed line is cut back off and its undo would be found nowhere else.
fn unended_line(asked: &Asked, came: &Came, error: &io::Error) -> String {
    let held = came
        .held
        .map(|held| format!(", having held {held}"))
        .unwrap_or_default();
    format!(
        "eero: couldn't log how {:?} went, which was {}{held}: {}",
        asked.act,
        changes::result(&came.done),
        terminal_kit::layout::one_line(&error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;
    use crate::acts::{Done, Mac};
    use crate::listeners::Peer;

    /// A log in a folder of the test's own, which starts empty.
    fn log(name: &str) -> (PathBuf, PathBuf) {
        let folder =
            std::env::temp_dir().join(format!("eero-carried-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let log = folder.join("changes.jsonl");
        (folder, log)
    }

    fn asked() -> Asked {
        Asked {
            listener: 1,
            id: json!(1),
            act: Act::Unreserve {
                mac: Mac::parse("02:00:00:00:00:38").expect("a MAC"),
            },
            peer: Some(Peer {
                pid: Some(4242),
                uid: 1000,
                unit: None,
            }),
        }
    }

    #[test]
    fn the_journal_keeps_who_asked_and_what_came_where_the_log_cant() {
        let error = io::Error::other("disk full\neero: forged");
        let unlogged = unlogged_line(&asked(), &error);
        let came = Came {
            done: Ok(Done::Made),
            held: Some("10.0.0.208".parse().expect("an address")),
        };
        let unended = unended_line(&asked(), &came, &error);
        for line in [&unlogged, &unended] {
            assert!(
                !line.contains('\n') && line.contains("02:00:00:00:00:38"),
                "a journal line broke in two or lost the MAC as written: {line:?}"
            );
        }
        assert!(
            unlogged.contains("4242") && unlogged.contains("1000"),
            "an unlogged ask's journal line lost who asked: {unlogged:?}"
        );
        assert!(
            unended.contains("made") && unended.contains("10.0.0.208"),
            "an unlogged end's journal line lost what came or its undo: {unended:?}"
        );
    }

    /// Each line's result and pid, in order, or none where there's no log.
    fn results(log: &Path) -> Vec<(String, Value)> {
        std::fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .map(|line| {
                let line: Value = serde_json::from_str(line).expect("a line is JSON");
                (
                    line["result"].as_str().unwrap_or_default().to_owned(),
                    line["pid"].clone(),
                )
            })
            .collect()
    }

    #[test]
    fn an_act_is_logged_as_asked_before_it_is_done_and_after() {
        let (folder, log) = log("done");
        let mut lines_when_done = None;
        let carried = carried(Ok(&log), true, &asked(), |_| {
            lines_when_done = Some(results(&log).len());
            Came {
                done: Ok(Done::Made),
                held: Some("10.0.0.208".parse().expect("an address")),
            }
        });
        assert!(
            matches!(
                carried,
                Carried::Came(Came {
                    done: Ok(Done::Made),
                    ..
                })
            ),
            "an act asked and logged wasn't carried out: {carried:?}"
        );
        assert_eq!(
            lines_when_done,
            Some(1),
            "the act was done before its ask was logged"
        );
        assert_eq!(
            results(&log),
            [("asked".into(), json!(4242)), ("made".into(), json!(4242))],
            "the act's lines aren't its ask then its result, by who asked"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_act_whose_ask_cant_be_logged_is_never_done() {
        let (folder, _) = log("unlogged");
        let nowhere = folder.join("no folder").join("changes.jsonl");
        let missing = io::Error::other("no state folder");
        for (log, why) in [
            (Ok(nowhere.as_path()), "a log that can't be written"),
            (Err(&missing), "no log at all"),
        ] {
            let mut done = false;
            let carried = carried(log, true, &asked(), |_| {
                done = true;
                Came::of(Ok(Done::Made))
            });
            assert!(
                !done && matches!(carried, Carried::Unlogged(_)),
                "an act was done with {why}: {carried:?}"
            );
        }
        let left: Vec<_> = std::fs::read_dir(&folder)
            .expect("the test's folder reads")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(
            left,
            Vec::<std::ffi::OsString>::new(),
            "an unlogged act wrote, or made a folder, somewhere unasked"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_act_whose_asker_has_gone_is_let_go_and_logged_so() {
        let (folder, log) = log("gone");
        let mut done = false;
        let carried = carried(Ok(&log), false, &asked(), |_| {
            done = true;
            Came::of(Ok(Done::Made))
        });
        assert!(
            !done && matches!(carried, Carried::Gone),
            "an act whose asker had gone was done: {carried:?}"
        );
        assert_eq!(
            results(&log),
            [("gone".into(), json!(4242))],
            "an act let go left no line saying so"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }
}
