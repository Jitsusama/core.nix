//! Brings a machine's pi sessions back after it restarts.
//!
//! A restart ends every session the node holds, and nothing in mycelium
//! starts them again, so on a Mac each came back only when someone typed
//! `pi --session` for it. This program is run each minute. Each run asks
//! the node of every realm it's given which pi sessions it holds here, and
//! keeps them, with the folder each runs in, under
//! `$XDG_STATE_HOME/sessions-come-back/<realm>.json`. The first run after a
//! newer boot finds what it kept from before and starts each one again,
//! through `mycelium start` on this machine, as `pi --session <id>` in its
//! own folder, which is the form the node takes as a session brought back.
//!
//! A session that ended while the machine ran isn't held at the next runs,
//! so it drops out of what's kept and doesn't come back: only a restart
//! brings anything back. It drops out only once three runs in a row
//! haven't seen it, since a shutdown ends sessions before it ends this
//! agent, and a run in between mustn't read them as ended by hand. A session whose folder is gone is left, since pi
//! would start in the home folder and ask to fork it. One the node won't
//! start yet, as while it's still linking after the boot, is tried again
//! each run, for a quarter of an hour.
//!
//!   sessions-come-back MYCELIUM REALM...    keeps, or brings back, each realm's sessions
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, and a failure that says what to try.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use terminal_kit::{Exit, Failure, Help};

/// How far apart two readings of the boot time may be and still be one
/// boot: the clock's corrections move what the kernel says by a second or
/// two, and no machine restarts in under a minute.
const SAME_BOOT_SECONDS: u64 = 60;

/// How many runs a session waits to be started again before it's left,
/// a quarter of an hour at one a minute: past that, the node isn't coming.
const TRIES: u32 = 15;

/// How many runs in a row a session goes unseen before it counts as ended
/// while the machine ran, rather than by the shutdown a run caught midway.
const GONE: u32 = 3;

/// A session the node held here: its id, and the folder it runs in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Session {
    id: String,
    folder: String,
}

/// One waiting to be started again, with the runs it has waited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Waiting {
    session: Session,
    tries: u32,
}

/// One held lately but not at the last runs, with how many in a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Unseen {
    session: Session,
    runs: u32,
}

/// What a realm's run keeps between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Kept {
    /// When the machine booted, in seconds since the epoch.
    boot: u64,
    /// The machine's name in the realm, which a start names to land here.
    machine: Option<String>,
    /// The sessions held here at the last run.
    sessions: Vec<Session>,
    /// The sessions held before the last run but not since, for fewer
    /// than GONE runs.
    #[serde(default)]
    unseen: Vec<Unseen>,
    /// The sessions a restart ended, still to be started again.
    #[serde(default)]
    waiting: Vec<Waiting>,
}

/// What the node holds here now: the machine's name and its pi sessions.
#[derive(Debug, Default, PartialEq, Eq)]
struct Held {
    machine: Option<String>,
    sessions: Vec<Session>,
}

/// A line of `mycelium holds`: a program a holder keeps on this machine.
#[derive(Debug, Deserialize)]
struct Hold {
    id: String,
    #[serde(default)]
    command: Vec<String>,
    #[serde(default)]
    folder: String,
}

/// What `mycelium sessions` says of a session, of every machine's.
#[derive(Debug, Deserialize)]
struct Listed {
    id: String,
    #[serde(default)]
    held: Option<String>,
    #[serde(default)]
    machine: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Listing {
    #[serde(rename = "Sessions", default)]
    sessions: Vec<Listed>,
}

#[derive(Debug, Deserialize)]
enum Answer {
    Ok(Listing),
    Err(serde_json::Value),
}

/// Whether a hold's program is pi, by its file's name, since a session
/// may be held as `pi` or by its path. Any other program, such as a
/// sprout's wrapper, would start its work over, so it isn't brought back.
fn runs_pi(command: &[String]) -> bool {
    command
        .first()
        .and_then(|program| Path::new(program).file_name())
        .is_some_and(|name| name == "pi")
}

/// The pi sessions held here, joining the holds this machine keeps to the
/// node's sessions by the hold's id, for the session's own id.
fn held(holds: &str, sessions: &str) -> Result<Held, String> {
    let listing = match serde_json::from_str::<Answer>(sessions).map_err(|e| e.to_string())? {
        Answer::Ok(listing) => listing,
        Answer::Err(why) => return Err(format!("the node said {why}")),
    };
    let mut found = Held::default();
    for line in holds.lines().filter(|line| !line.trim().is_empty()) {
        let hold: Hold = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if !runs_pi(&hold.command) {
            continue;
        }
        let Some(listed) = listing
            .sessions
            .iter()
            .find(|listed| listed.held.as_deref() == Some(hold.id.as_str()))
        else {
            continue;
        };
        if found.machine.is_none() {
            found.machine.clone_from(&listed.machine);
        }
        found.sessions.push(Session {
            id: listed.id.clone(),
            folder: hold.folder,
        });
    }
    Ok(found)
}

/// What to keep after this run's look, before any start: what's held now,
/// and what's waiting, which a newer boot fills with what was held before
/// it. A session held again no longer waits.
fn next(kept: Option<Kept>, boot: u64, now: &Held) -> Kept {
    let Some(before) = kept else {
        return Kept {
            boot,
            machine: now.machine.clone(),
            sessions: now.sessions.clone(),
            unseen: Vec::new(),
            waiting: Vec::new(),
        };
    };
    let held_now = |id: &str| now.sessions.iter().any(|s| s.id == id);
    let mut waiting = before.waiting;
    let mut unseen = Vec::new();
    if before.boot.abs_diff(boot) > SAME_BOOT_SECONDS {
        let lately = before.unseen.into_iter().map(|u| u.session);
        for session in before.sessions.into_iter().chain(lately) {
            if !waiting.iter().any(|w| w.session.id == session.id) {
                waiting.push(Waiting { session, tries: 0 });
            }
        }
    } else {
        let missed = before
            .sessions
            .into_iter()
            .map(|session| Unseen { session, runs: 0 })
            .chain(before.unseen);
        for Unseen { session, runs } in missed {
            if !held_now(&session.id) && runs + 1 < GONE {
                unseen.push(Unseen {
                    session,
                    runs: runs + 1,
                });
            }
        }
    }
    waiting.retain(|w| !held_now(&w.session.id));
    Kept {
        boot,
        machine: now.machine.clone().or(before.machine),
        sessions: now.sessions.clone(),
        unseen,
        waiting,
    }
}

/// Starts each waiting session whose folder is there, and says what it
/// did with each. One started, or one whose folder is gone, waits no
/// more; one the node wouldn't start waits for another run, up to TRIES.
fn bring_back(
    kept: &mut Kept,
    there: impl Fn(&str) -> bool,
    mut start: impl FnMut(&str, &Session) -> Result<(), String>,
    mut say: impl FnMut(String),
) {
    let Some(machine) = kept.machine.clone() else {
        if !kept.waiting.is_empty() {
            say("no session has said which machine this is, so none is started yet".into());
        }
        return;
    };
    kept.waiting.retain_mut(|waiting| {
        let session = &waiting.session;
        if !there(&session.folder) {
            say(format!(
                "{} isn't started again: its folder, {}, is gone",
                session.id, session.folder
            ));
            return false;
        }
        match start(&machine, session) {
            Ok(()) => {
                say(format!(
                    "{} started again in {}",
                    session.id, session.folder
                ));
                false
            }
            Err(why) => {
                waiting.tries += 1;
                let left = waiting.tries < TRIES;
                say(format!(
                    "{} wasn't started again ({why}); {}",
                    session.id,
                    if left {
                        "trying again next run"
                    } else {
                        "leaving it"
                    }
                ));
                left
            }
        }
    });
}

/// When this machine booted, in seconds since the epoch: a Mac's
/// kern.boottime, `{ sec = 1795097530, usec = 0 } ...`, or Linux's btime.
fn boot() -> Result<u64, Failure> {
    let unread = |why: String| {
        Failure::new(
            Exit::Unavailable,
            "it couldn't read when the machine booted",
        )
        .because(why)
    };
    if let Ok(stat) = fs::read_to_string("/proc/stat") {
        return stat
            .lines()
            .find_map(|line| line.strip_prefix("btime "))
            .and_then(|seconds| seconds.trim().parse().ok())
            .ok_or_else(|| unread("/proc/stat has no btime".into()));
    }
    let asked = Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.boottime"])
        .output()
        .map_err(|e| unread(e.to_string()))?;
    booted(&String::from_utf8_lossy(&asked.stdout))
        .ok_or_else(|| unread("sysctl's kern.boottime has no sec".into()))
}

/// The seconds in a Mac's kern.boottime.
fn booted(boottime: &str) -> Option<u64> {
    let (_, after) = boottime.split_once("sec = ")?;
    after
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Where each realm's record is kept.
fn state() -> Result<PathBuf, Failure> {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's nowhere to keep the sessions")
                .because("neither XDG_STATE_HOME nor HOME is set")
        })?;
    Ok(base.join("sessions-come-back"))
}

/// What a mycelium command printed, or why it couldn't say.
fn ask(mycelium: &str, realm: &str, words: &[&str]) -> Result<String, String> {
    let asked = Command::new(mycelium)
        .args(["--realm", realm])
        .args(words)
        .output()
        .map_err(|e| e.to_string())?;
    if !asked.status.success() {
        return Err(String::from_utf8_lossy(&asked.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&asked.stdout).into_owned())
}

/// One realm's run: look, keep, and start what a restart ended.
fn run_realm(mycelium: &str, realm: &str, boot: u64, folder: &Path) -> Result<(), Failure> {
    let unanswered = |why: String| {
        Failure::new(
            Exit::TryAgain,
            format!("the {realm} realm's node didn't say what it holds"),
        )
        .because(why)
        .try_this(format!("mycelium --realm {realm} holds"))
    };
    let holds = ask(mycelium, realm, &["holds"]).map_err(unanswered)?;
    let sessions = ask(mycelium, realm, &["sessions"]).map_err(unanswered)?;
    let now = held(&holds, &sessions).map_err(|why| {
        Failure::new(Exit::BadInput, "the node's sessions couldn't be read").because(why)
    })?;
    let file = folder.join(format!("{realm}.json"));
    let before = fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str::<Kept>(&text).ok());
    let mut kept = next(before, boot, &now);
    bring_back(
        &mut kept,
        |folder| Path::new(folder).is_dir(),
        |machine, session| {
            let need = format!("machine={machine}");
            let words = [
                "start",
                "--need",
                &need,
                "--folder",
                &session.folder,
                "--",
                "pi",
                "--session",
                &session.id,
            ];
            let said = ask(mycelium, realm, &words)?;
            if said.contains("\"Started\"") {
                Ok(())
            } else {
                Err(said.trim().to_string())
            }
        },
        |line| eprintln!("{realm}: {line}"),
    );
    let unkept = |why: String| {
        Failure::new(Exit::Failed, "it couldn't keep the sessions")
            .because(why)
            .try_this(format!("check that {} can be written", file.display()))
    };
    fs::create_dir_all(folder).map_err(|e| unkept(e.to_string()))?;
    let text = serde_json::to_string_pretty(&kept).map_err(|e| unkept(e.to_string()))?;
    let partial = file.with_extension("json.partial");
    fs::write(&partial, text).map_err(|e| unkept(e.to_string()))?;
    fs::rename(&partial, &file).map_err(|e| unkept(e.to_string()))
}

/// Every realm's run, each on its own, so one realm's node being away
/// keeps none of the others' sessions from coming back.
fn run(mycelium: &str, realms: &[String]) -> Result<(), Failure> {
    let boot = boot()?;
    let folder = state()?;
    let mut failed = None;
    for realm in realms {
        if let Err(failure) = run_realm(mycelium, realm, boot, &folder) {
            failed.get_or_insert(failure);
        }
    }
    failed.map_or(Ok(()), Err)
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "sessions-come-back",
        "brings a machine's pi sessions back after it restarts",
    )
    .example(
        "sessions-come-back mycelium work",
        "what its agent runs each minute, keeping or bringing back the work realm's sessions",
    )
    .exit(Exit::Failed, "it couldn't keep the sessions")
    .exit(Exit::BadInput, "the node's sessions couldn't be read")
    .exit(
        Exit::Unavailable,
        "it couldn't read when the machine booted",
    )
    .exit(Exit::TryAgain, "a realm's node didn't say what it holds")
    .exit(Exit::Config, "there's nowhere to keep the sessions")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [mycelium, realms @ ..] = context.args.as_slice() else {
            return Err(
                Failure::new(Exit::Usage, "it takes mycelium, as a path, and realms")
                    .try_this("sessions-come-back mycelium work"),
            );
        };
        if realms.is_empty() {
            return Err(Failure::new(Exit::Usage, "it takes at least one realm")
                .try_this("sessions-come-back mycelium work"));
        }
        run(mycelium, realms)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOLDS: &str = concat!(
        r#"{"id":"4d783094","command":["pi","--session","01a1-a"],"folder":"/Users/j/src"}"#,
        "\n",
        r#"{"id":"f0520da9","command":["/etc/profiles/bin/pi","--session","/x/01a1-b.jsonl"],"folder":"/Users/j"}"#,
        "\n",
        r#"{"id":"eaad11f8","command":["sh","-c","exec \"$@\"","sprout","pi"],"folder":"/Users/j"}"#,
        "\n",
        r#"{"id":"0d0d0d0d","command":["pi"],"folder":"/Users/j"}"#,
        "\n",
    );

    const SESSIONS: &str = r#"{"Ok":{"Sessions":[
        {"id":"01a1-a","machine":"mac","held":"4d783094","state":"idle"},
        {"id":"01a1-b","machine":"mac","held":"f0520da9"},
        {"id":"01a1-c","machine":"mac","held":"eaad11f8"},
        {"id":"01a1-d","machine":"other","held":"77777777"},
        {"id":"01a1-e","machine":"mac"}
    ]}}"#;

    fn session(id: &str, folder: &str) -> Session {
        Session {
            id: id.into(),
            folder: folder.into(),
        }
    }

    fn now(sessions: &[Session]) -> Held {
        Held {
            machine: Some("mac".into()),
            sessions: sessions.to_vec(),
        }
    }

    #[test]
    fn the_pi_sessions_held_here_are_found_by_their_holds() {
        assert_eq!(
            held(HOLDS, SESSIONS),
            Ok(Held {
                machine: Some("mac".into()),
                sessions: vec![
                    session("01a1-a", "/Users/j/src"),
                    session("01a1-b", "/Users/j")
                ],
            }),
            "a pi held by name or path is found by its hold, and a sprout's wrapper, a hold \
             the node names no session for and another machine's session are not"
        );
    }

    #[test]
    fn a_node_that_answers_with_an_error_is_not_read_as_holding_nothing() {
        assert!(
            held(HOLDS, r#"{"Err":"no node"}"#).is_err(),
            "an Err answer is a failure, so what's kept isn't emptied"
        );
    }

    #[test]
    fn the_first_run_keeps_what_is_held_and_brings_nothing_back() {
        let a = session("a", "/f");
        assert_eq!(
            next(None, 1000, &now(std::slice::from_ref(&a))),
            Kept {
                boot: 1000,
                machine: Some("mac".into()),
                sessions: vec![a],
                ..Kept::default()
            },
        );
    }

    fn holding(sessions: &[Session]) -> Kept {
        Kept {
            boot: 1000,
            machine: Some("mac".into()),
            sessions: sessions.to_vec(),
            ..Kept::default()
        }
    }

    #[test]
    fn a_session_that_ends_while_the_machine_runs_does_not_come_back() {
        let mut kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
        for _ in 0..GONE {
            let after = next(kept, 1001, &now(&[session("a", "/f")]));
            assert_eq!(
                after.waiting,
                vec![],
                "only a restart sets a session waiting"
            );
            assert_eq!(
                after.sessions,
                vec![session("a", "/f")],
                "what's kept is what's held"
            );
            kept = Some(after);
        }
        let restarted = next(kept, 5000, &Held::default());
        assert_eq!(
            restarted.waiting,
            vec![Waiting {
                session: session("a", "/f"),
                tries: 0
            }],
            "b went unseen for {GONE} runs before the restart, so it ended by itself"
        );
    }

    #[test]
    fn a_session_the_shutdown_ended_before_the_last_run_still_comes_back() {
        let caught = next(
            Some(holding(&[session("a", "/f"), session("b", "/g")])),
            1000,
            &now(&[session("a", "/f")]),
        );
        let restarted = next(Some(caught), 5000, &Held::default());
        assert_eq!(
            restarted.waiting,
            vec![
                Waiting {
                    session: session("a", "/f"),
                    tries: 0
                },
                Waiting {
                    session: session("b", "/g"),
                    tries: 0
                },
            ],
            "a run midway through the shutdown that missed b doesn't keep b from coming back"
        );
    }

    #[test]
    fn a_session_unseen_for_a_run_and_then_held_again_is_held() {
        let missed = next(Some(holding(&[session("a", "/f")])), 1000, &now(&[]));
        let again = next(Some(missed), 1000, &now(&[session("a", "/f")]));
        assert_eq!(
            (again.sessions, again.unseen),
            (vec![session("a", "/f")], vec![])
        );
    }

    #[test]
    fn a_restart_sets_every_session_held_before_it_waiting() {
        let kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
        let after = next(kept, 5000, &Held::default());
        assert_eq!(
            after,
            Kept {
                boot: 5000,
                machine: Some("mac".into()),
                sessions: vec![],
                unseen: vec![],
                waiting: vec![
                    Waiting {
                        session: session("a", "/f"),
                        tries: 0
                    },
                    Waiting {
                        session: session("b", "/g"),
                        tries: 0
                    },
                ],
            },
            "the machine's name is kept from before the boot, when nothing is held yet"
        );
    }

    #[test]
    fn a_session_held_again_no_longer_waits() {
        let kept = Some(Kept {
            boot: 5000,
            machine: Some("mac".into()),
            sessions: vec![],
            unseen: vec![],
            waiting: vec![
                Waiting {
                    session: session("a", "/f"),
                    tries: 2,
                },
                Waiting {
                    session: session("b", "/g"),
                    tries: 0,
                },
            ],
        });
        let after = next(kept, 5000, &now(&[session("a", "/f")]));
        assert_eq!(
            after.waiting,
            vec![Waiting {
                session: session("b", "/g"),
                tries: 0
            }]
        );
    }

    #[test]
    fn a_second_restart_before_they_are_back_keeps_them_waiting_once() {
        let kept = Some(Kept {
            boot: 5000,
            machine: Some("mac".into()),
            sessions: vec![session("c", "/h")],
            unseen: vec![],
            waiting: vec![Waiting {
                session: session("a", "/f"),
                tries: 3,
            }],
        });
        let after = next(kept, 9000, &Held::default());
        assert_eq!(
            after.waiting,
            vec![
                Waiting {
                    session: session("a", "/f"),
                    tries: 3
                },
                Waiting {
                    session: session("c", "/h"),
                    tries: 0
                },
            ],
        );
    }

    #[test]
    fn the_clock_moving_the_boot_time_a_little_is_not_a_restart() {
        let kept = Some(holding(&[session("a", "/f")]));
        assert_eq!(
            next(kept, 1000 + SAME_BOOT_SECONDS, &Held::default()).waiting,
            vec![]
        );
    }

    fn waiting(ids: &[&str]) -> Kept {
        Kept {
            boot: 5000,
            machine: Some("mac".into()),
            sessions: vec![],
            unseen: vec![],
            waiting: ids
                .iter()
                .map(|id| Waiting {
                    session: session(id, &format!("/{id}")),
                    tries: 0,
                })
                .collect(),
        }
    }

    #[test]
    fn each_waiting_session_is_started_on_this_machine_in_its_folder() {
        let mut kept = waiting(&["a", "b"]);
        let mut started = Vec::new();
        bring_back(
            &mut kept,
            |_| true,
            |machine, s| {
                started.push(format!("{machine} {} {}", s.folder, s.id));
                Ok(())
            },
            |_| {},
        );
        assert_eq!(started, vec!["mac /a a", "mac /b b"]);
        assert_eq!(kept.waiting, vec![], "a started session waits no more");
    }

    #[test]
    fn a_session_whose_folder_is_gone_is_left_and_said() {
        let mut kept = waiting(&["a", "b"]);
        let mut said = Vec::new();
        let mut started = Vec::new();
        bring_back(
            &mut kept,
            |folder| folder != "/a",
            |_, s| {
                started.push(s.id.clone());
                Ok(())
            },
            |line| said.push(line),
        );
        assert_eq!(
            started,
            vec!["b"],
            "pi would ask to fork a session started in the home"
        );
        assert_eq!(said[0], "a isn't started again: its folder, /a, is gone");
        assert_eq!(kept.waiting, vec![]);
    }

    #[test]
    fn one_the_node_refuses_is_tried_again_until_it_has_waited_too_long() {
        let mut kept = waiting(&["a"]);
        for run in 1..TRIES {
            bring_back(
                &mut kept,
                |_| true,
                |_, _| Err("no link yet".into()),
                |_| {},
            );
            assert_eq!(kept.waiting.len(), 1, "still waiting after run {run}");
        }
        bring_back(
            &mut kept,
            |_| true,
            |_, _| Err("no link yet".into()),
            |_| {},
        );
        assert_eq!(kept.waiting, vec![], "left after {TRIES} runs");
    }

    #[test]
    fn nothing_is_started_before_the_machine_is_known() {
        let mut kept = waiting(&["a"]);
        kept.machine = None;
        let mut started = 0;
        bring_back(
            &mut kept,
            |_| true,
            |_, _| {
                started += 1;
                Ok(())
            },
            |_| {},
        );
        assert_eq!(
            (started, kept.waiting.len()),
            (0, 1),
            "it waits to know where to start"
        );
    }

    #[test]
    fn a_macs_boot_time_is_read_from_kern_boottime() {
        assert_eq!(
            booted("{ sec = 1795097530, usec = 412000 } Thu Nov 19 12:42:10 2026\n"),
            Some(1_795_097_530)
        );
        assert_eq!(booted("nothing here"), None);
    }
}
