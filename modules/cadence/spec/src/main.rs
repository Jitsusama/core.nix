//! cadence-spec serves Joel's axes to the launcher's Spec place, and what
//! he asked for to its Asked For, as ready rows over the socket systemd
//! holds for it, each watch told only the rows that moved. The shell
//! never rereads a file rewritten each tick (Joel, 2026-11-10: "the
//! daemons need to be the data layer, and they need to serve the
//! launcher"), so this daemon holds the rows.
//!
//! cadence makes them: `cadence axes --served WORDS` reads the axes and
//! prints Spec's rows and headline (../src/axes/spec.rs), so what an axis
//! reads lives in one place. That takes seconds and about half a second of
//! processor, so this asks for it only while something watches Spec, and
//! at most once in five minutes, the pace cadence's own tick keeps; a
//! watch begun between reads is answered from the rows last made. A read
//! that fails, or finds nothing, leaves the last rows served rather than
//! an empty Spec (80ECO2, 2026-11-14, 2768bcbb), and cadence says why in
//! the journal, where its processor time goes too.
//!
//! Asked For's rows are made the same way, by `cadence axes --asked WORDS`
//! (../src/axes/asked.rs), on their own: watched apart, kept apart and
//! made again after a minute, since they read one file in milliseconds
//! and an ask's owner may write it at any time (1JQPPC, 2026-11-26,
//! 4443be2d). Each place's read has a deadline of its own, past which
//! cadence is stopped with all it started and the place keeps its last
//! rows, so a Spec read that hangs, on a git lock or a niri that doesn't
//! answer, holds Asked For back for no longer than Spec's deadline. A
//! place whose words say an unread headline, as Asked For's do, serves it
//! over those rows, so a read that stopped vouches for no row late or due.
//! The
//! places are read one after the other, Asked For first, so both watched
//! at once see Asked For's rows before Spec's read begins; Asked For
//! watched while a Spec read is already going waits out that read, its
//! seconds.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Shared};
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};

/// The daemon as the launcher knows it: each place's rows and headline.
const DOMAIN: Domain = Domain {
    name: "cadence-spec",
    kinds: &["row", "head"],
    sole: &[],
    named_only: &[],
};

/// A place it serves: its name, how cadence is asked for its rows, how
/// long the rows last made stand before a watch has them made again, and
/// how long a read may take before it's stopped.
struct Place {
    name: &'static str,
    flag: &'static str,
    fresh: Duration,
    deadline: Duration,
}

/// The places it serves, read in this order: Asked For first, since it
/// takes milliseconds and Spec seconds, so Asked For's first answer never
/// waits out a Spec read begun beside it (round 6 on #6875, F119).
const PLACES: [Place; 2] = [
    Place {
        name: "asked",
        flag: "--asked",
        fresh: Duration::from_mins(1),
        // It reads two files in milliseconds.
        deadline: Duration::from_secs(10),
    },
    Place {
        name: "spec",
        flag: "--served",
        fresh: Duration::from_mins(5),
        // It takes seconds, so a minute is a read gone wrong.
        deadline: Duration::from_mins(1),
    },
];

fn help() -> Help {
    Help::new(
        "cadence-spec",
        "serves Joel's axes and what he asked for to the launcher",
    )
    .command(
        "SOCKET CADENCE WORDS",
        "serves on SOCKET the rows CADENCE makes with WORDS",
    )
    .example(
        "cadence-spec $XDG_RUNTIME_DIR/daemons/cadence-spec.sock cadence words.json",
        "what its unit runs, on the socket systemd holds",
    )
    .exit(Exit::Failed, "it couldn't listen on SOCKET")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [socket, cadence, words] = context.args.as_slice() else {
            return Err(
                Failure::new(Exit::Usage, "it takes SOCKET, CADENCE, then WORDS")
                    .try_this("cadence-spec --help"),
            );
        };
        serving(
            Path::new(socket),
            &Asking {
                cadence: PathBuf::from(cadence),
                words: PathBuf::from(words),
            },
        )
    })
}

/// How the rows are asked of cadence.
struct Asking {
    cadence: PathBuf,
    words: PathBuf,
}

/// Serves on `socket` for as long as the program runs.
fn serving(socket: &Path, asking: &Asking) -> Result<(), Failure> {
    let started = Instant::now();
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (wake, woken) = mpsc::channel();
    shared.on_watches(Box::new(move || {
        let _ = wake.send(());
    }));
    let listener = serve::listen(socket).map_err(|why| {
        Failure::new(
            Exit::Failed,
            format!("it can't listen on {}", socket.display()),
        )
        .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    // Health reads a daemon's start to ready from this line.
    eprintln!(
        "cadence-spec: ready in {} ms",
        started.elapsed().as_millis()
    );
    let mut kept: [Kept; 2] = Default::default();
    let mut last: [Option<Instant>; 2] = [None; 2];
    loop {
        let mut waits: [Option<Duration>; 2] = [None; 2];
        for (at, place) in PLACES.iter().enumerate() {
            let watched = shared.watches("row", place.name) || shared.watches("head", place.name);
            if watched && last[at].is_none_or(|made| made.elapsed() >= place.fresh) {
                last[at] = Some(Instant::now());
                let made = made(asking, place);
                let moved = took(&mut kept[at], made, || {
                    unread_head(&asking.words, place.name)
                });
                // Handed as soon as this place moves, before the next
                // place's read, so Asked For isn't held back by Spec's;
                // every place's rows are handed at once, since a record
                // left out is told as gone.
                if moved {
                    let all = kept.iter().flat_map(Kept::all).cloned().collect();
                    shared.change(all, json!({}));
                }
            }
            let since = last[at].map(|made| made.elapsed());
            waits[at] = waiting(watched, since, place.fresh);
        }
        let wait = soonest(waits);
        // Woken as a watch begins or ends, else when the soonest rows
        // watched go stale.
        let ended = match wait {
            Some(wait) => matches!(
                woken.recv_timeout(wait),
                Err(mpsc::RecvTimeoutError::Disconnected)
            ),
            None => woken.recv().is_err(),
        };
        if ended {
            return Ok(());
        }
    }
}

/// How long to sleep before looking at a place again, given whether it's
/// watched, how long ago its rows were last made and how long they stand;
/// none to sleep until a watch begins or ends. Stale rows nobody watches
/// are left stale: waking for them would find nothing to do and, with
/// nothing left to wait, wake again at once, spinning a core until a watch
/// came.
fn waiting(watched: bool, since: Option<Duration>, fresh: Duration) -> Option<Duration> {
    watched.then(|| since.map_or(fresh, |since| fresh.saturating_sub(since)))
}

/// The soonest of each place's waits, so no place watched goes stale while
/// another's longer wait is slept; none where no place is watched.
fn soonest(waits: [Option<Duration>; 2]) -> Option<Duration> {
    waits.into_iter().flatten().min()
}

/// What cadence made of a place: its rows in the order shown, or none
/// where it said to keep the last, and its headline as a whole record.
#[derive(Debug, PartialEq)]
struct Made {
    rows: Option<Vec<Value>>,
    head: Value,
}

/// Whether what cadence made of a place moved what it serves, taken into
/// that place's own `kept`: its rows, or the last it served where it said
/// keep, beside its new headline. Where cadence made nothing of a place,
/// the headline saying its list is unread (`unread`) is served beside the
/// last rows it served, if any: a place that waits on its headline, as
/// Asked For does, stops asking (round 5, F91), and one whose rows say
/// what's late doesn't go on saying it in normal colours from a read that
/// stopped (#6875's eighth round, 147). A place whose words say no unread
/// headline, as Spec's don't, keeps all it served.
fn took(kept: &mut Kept, made: Option<Made>, unread: impl FnOnce() -> Option<Value>) -> bool {
    match made {
        Some(Made { rows, head }) => {
            let rows = rows.unwrap_or_else(|| last_rows(kept));
            !kept.take(rows, vec![head]).is_empty()
        }
        None => unread().is_some_and(|head| {
            let rows = last_rows(kept);
            !kept.take(rows, vec![head]).is_empty()
        }),
    }
}

/// The headline saying a place's list is unread, in the launcher's words
/// for it (`<place>:head`'s unread), as cadence says it; none where those
/// words don't say one, as Spec's don't, or can't be read.
fn unread_head(words: &Path, place: &str) -> Option<Value> {
    // Words that can't be read serve no unread headline, and the place keeps
    // what it served, as one whose words say none does; the journal says why.
    let read: Value = std::fs::read(words)
        .map_err(|error| error.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string()))
        .map_err(|why| eprintln!("cadence-spec: {}'s words: {why}", words.display()))
        .ok()?;
    let id = format!("{place}:head");
    let thing = read["things"]
        .as_array()?
        .iter()
        .find(|thing| thing["id"] == id.as_str())?;
    let said = thing["said"]["unread"].as_str()?;
    let key = format!("head:{place}");
    Some(json!({"key": key, "kind": "head",
        "head": {"key": key, "place": place, "said": said, "glyph": "failed"}}))
}

/// The rows a place last served, in their order, to serve again beside a
/// new headline, as when Asked For's list is unread (round 4, F57).
fn last_rows(kept: &Kept) -> Vec<Value> {
    let mut rows: Vec<&Value> = kept
        .all()
        .filter(|record| record["kind"] == "row")
        .map(|record| &record["row"])
        .collect();
    rows.sort_by_key(|row| row["order"].as_i64());
    rows.into_iter().cloned().collect()
}

/// A place's rows and headline as cadence makes them now, or none where
/// it failed, read nothing or ran past the place's deadline, so the last
/// rows stay. cadence's own words on why go to the journal, since its
/// stderr is this daemon's.
fn made(asking: &Asking, place: &Place) -> Option<Made> {
    let mut command = Command::new(&asking.cadence);
    command
        .arg("axes")
        .arg(place.flag)
        .arg(&asking.words)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let printed = match within(&mut command, place.deadline) {
        Ok(printed) => printed,
        Err(why) => {
            eprintln!("cadence-spec: {why}; {} keeps its last rows", place.name);
            return None;
        }
    };
    let rows = parsed(&printed, place.name);
    if rows.is_none() {
        eprintln!(
            "cadence-spec: cadence's rows didn't read as rows; {} keeps its last",
            place.name
        );
    }
    rows
}

/// What `command` printed, where it ended well within `deadline`; else
/// why not, the command stopped where it ran past. Its output is read as
/// it comes, so a long answer can't fill the pipe and hold it up, and
/// waited for no longer than the deadline, so a child of its own still
/// holding the pipe can't either. It runs in a process group of its own,
/// so stopping it stops what it started too: cadence's hangs are in its
/// own `git` and `niri msg`, which would otherwise be left running, one
/// more on each read that hung (round 2, F21). Its group is stopped once
/// it ends too, however it ended, before it's reaped, so nothing it left
/// keeps running or holds the pipe (round 3, F35).
fn within(command: &mut Command, deadline: Duration) -> Result<Vec<u8>, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .process_group(0)
        .spawn()
        .map_err(|why| format!("{program} couldn't run: {why}"))?;
    let Some(mut out) = child.stdout.take() else {
        let _ = stopped(&mut child);
        return Err("its output wasn't kept".to_owned());
    };
    let (sent, reading) = mpsc::channel();
    thread::spawn(move || {
        let mut printed = Vec::new();
        let _ = sent.send(out.read_to_end(&mut printed).map(|_| printed));
    });
    let started = Instant::now();
    loop {
        match ended(&child) {
            Ok(false) if started.elapsed() < deadline => thread::sleep(Duration::from_millis(20)),
            Ok(false) => {
                let _ = stopped(&mut child);
                return Err(format!(
                    "{program} ran past {} s, so it was stopped",
                    deadline.as_secs()
                ));
            }
            Ok(true) => break,
            Err(why) => {
                let _ = stopped(&mut child);
                return Err(format!("{program} couldn't be waited on: {why}"));
            }
        }
    }
    let status = stopped(&mut child).map_err(|why| format!("{program} wasn't reaped: {why}"))?;
    if !status.success() {
        return Err(format!("cadence ended {status}"));
    }
    match reading.recv_timeout(deadline.saturating_sub(started.elapsed())) {
        Ok(Ok(printed)) => Ok(printed),
        Ok(Err(why)) => Err(format!("cadence's output couldn't be read: {why}")),
        Err(_) => Err(format!(
            "cadence's output stayed open past {} s",
            deadline.as_secs()
        )),
    }
}

/// Whether `child` has ended, seen without reaping it, so the number of
/// the group it leads stays its own until its group is stopped.
fn ended(child: &Child) -> std::io::Result<bool> {
    // SAFETY: siginfo_t is plain data, for which all zeroes is a value.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: a plain system call on the child's own number, `info` ours
    // and as large as it asks; WNOWAIT leaves the child to be reaped.
    let asked = unsafe {
        libc::waitid(
            libc::P_PID,
            child.id(),
            &raw mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if asked != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: waitid filled `info`, whose si_pid is 0 while none ended.
    Ok(unsafe { info.si_pid() } != 0)
}

/// Stops `child` and everything it started, its whole process group, and
/// reaps it, so nothing it left is a zombie, saying how it ended.
fn stopped(child: &mut Child) -> std::io::Result<std::process::ExitStatus> {
    if let Ok(group) = libc::pid_t::try_from(child.id()) {
        // SAFETY: a plain system call, to the group the child leads
        // (process_group(0)), signalled before the child is reaped, so its
        // number can't have gone to another.
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
    child.wait()
}

/// The rows and headline in what cadence printed for `place`, the
/// headline as the whole record served beside the rows, its fields under
/// its kind, as every place's reader reads one (ReadyRows.qml's held);
/// served bare, Spec's head read as nothing (Y2YBTS, 2026-11-14). Where
/// it says `keep` in place of rows, the last rows stay beside the new
/// headline.
fn parsed(printed: &[u8], place: &str) -> Option<Made> {
    let mut made: Value = serde_json::from_slice(printed).ok()?;
    let head = made.get_mut("head").filter(|head| head.is_object())?.take();
    let rows = if made["keep"] == true {
        None
    } else {
        Some(made.get_mut("rows")?.as_array_mut().map(std::mem::take)?)
    };
    let key = format!("head:{place}");
    Some(Made {
        rows,
        head: json!({"key": key, "kind": "head", "head": head}),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_cadence_prints_reads_as_rows_and_a_headline() {
        let printed = br#"{"rows": [{"key": "row:spec:scale"}],
            "head": {"key": "head:spec", "said": "all in spec"}}"#;
        let Made { rows, head } = parsed(printed, "spec").expect("rows and a headline");
        assert_eq!(rows, Some(vec![json!({"key": "row:spec:scale"})]));
        assert_eq!(
            head,
            json!({"key": "head:spec", "kind": "head",
                "head": {"key": "head:spec", "said": "all in spec"}}),
            "the headline is served as a whole record, its fields under its kind, as every \
             place's reader reads it (ReadyRows.qml's held)"
        );
    }

    #[test]
    fn unwatched_it_sleeps_until_a_watch_however_stale_the_rows() {
        let fresh = Duration::from_mins(5);
        let stale = Some(fresh + Duration::from_secs(60));
        assert_eq!(
            waiting(false, stale, fresh),
            None,
            "stale rows nobody watches must not wake it at once, again and again"
        );
        assert_eq!(
            waiting(false, None, fresh),
            None,
            "never made, never watched"
        );
        assert_eq!(
            waiting(true, Some(Duration::from_mins(2)), fresh),
            Some(Duration::from_mins(3)),
            "watched, it wakes as the rows go stale"
        );
    }

    #[test]
    fn it_sleeps_the_soonest_wait_of_the_places_watched() {
        let minute = Duration::from_mins(1);
        let four = Duration::from_mins(4);
        assert_eq!(soonest([Some(four), Some(minute)]), Some(minute));
        assert_eq!(soonest([Some(minute), Some(four)]), Some(minute));
        assert_eq!(
            soonest([None, Some(four)]),
            Some(four),
            "a place unwatched waits on nothing"
        );
        assert_eq!(
            soonest([None, None]),
            None,
            "nothing watched sleeps till a watch"
        );
    }

    #[test]
    fn each_place_is_asked_of_cadence_by_its_own_flag_and_keeps_its_own_head() {
        let names: Vec<(&str, &str)> = PLACES.iter().map(|p| (p.name, p.flag)).collect();
        assert_eq!(
            names,
            [("asked", "--asked"), ("spec", "--served")],
            "Asked For, read in milliseconds, before Spec's seconds"
        );
        let printed = br#"{"rows": [], "head": {"said": "3 open"}}"#;
        let Made { head, .. } = parsed(printed, "asked").expect("a headline");
        assert_eq!(head["key"], "head:asked");
    }

    #[test]
    fn a_headline_that_says_keep_is_served_beside_the_last_rows() {
        let mut kept = Kept::default();
        let rows = vec![json!({"key": "row:asked:b"}), json!({"key": "row:asked:a"})];
        let head = json!({"key": "head:asked", "kind": "head", "head": {"said": "2 open"}});
        kept.take(rows, vec![head]);
        let printed = br#"{"keep": true, "head": {"said": "can't read the list now"}}"#;
        let made = parsed(printed, "asked").expect("a headline");
        assert_eq!(made.rows, None, "no rows of its own");
        let keys: Vec<Value> = last_rows(&kept)
            .iter()
            .map(|row| row["key"].clone())
            .collect();
        assert_eq!(
            keys,
            ["row:asked:b", "row:asked:a"],
            "in the order last shown"
        );
        // As the serving loop takes it, into this place's own kept.
        assert!(took(&mut kept, Some(made), || None), "the headline moved");
        assert_eq!(last_rows(&kept).len(), 2, "and its rows stay beside it");
        assert_eq!(
            kept.get("head:asked").expect("its headline")["head"]["said"],
            "can't read the list now"
        );
        assert_eq!(last_rows(&Kept::default()), Vec::<Value>::new());
    }

    #[test]
    fn a_place_cadence_made_nothing_of_is_headed_unread_beside_the_rows_it_served() {
        let unread = || {
            Some(json!({"key": "head:asked", "kind": "head",
            "head": {"said": "can't read the list now", "glyph": "failed"}}))
        };
        let mut cold = Kept::default();
        assert!(
            took(&mut cold, None, unread),
            "a place never headed is headed unread"
        );
        assert_eq!(
            cold.get("head:asked").expect("its headline")["head"]["glyph"],
            "failed"
        );
        assert!(last_rows(&cold).is_empty(), "with no rows");
        let mut headed = Kept::default();
        headed.take(
            vec![json!({"key": "row:asked:a"})],
            vec![json!({"key": "head:asked", "kind": "head", "head": {"said": "1 open"}})],
        );
        assert!(
            took(&mut headed, None, unread),
            "one headed before is headed unread too"
        );
        assert_eq!(
            headed.get("head:asked").expect("its headline")["head"]["glyph"],
            "failed",
            "a read that stopped says so rather than the last headline"
        );
        let rows: Vec<Value> = last_rows(&headed)
            .iter()
            .map(|row| row["key"].clone())
            .collect();
        assert_eq!(rows, ["row:asked:a"], "beside the rows it served");
        assert!(
            !took(&mut headed, None, unread),
            "a second stopped read moves nothing"
        );
        let mut spec = Kept::default();
        spec.take(
            vec![json!({"key": "row:spec:a"})],
            vec![json!({"key": "head:spec", "kind": "head", "head": {"said": "1 due"}})],
        );
        assert!(
            !took(&mut spec, None, || None),
            "a place with no unread words keeps all it served"
        );
        assert_eq!(
            spec.get("head:spec").expect("its headline")["head"]["said"],
            "1 due"
        );
    }

    #[test]
    fn the_unread_headline_is_the_launchers_words_for_it() {
        let words = std::env::temp_dir().join(format!("cadence-spec-words-{}", std::process::id()));
        std::fs::write(
            &words,
            r#"{"things": [{"id": "asked:head", "said": {"unread": "can't read it"}}]}"#,
        )
        .expect("the words written");
        let head = unread_head(&words, "asked").expect("Asked For's unread headline");
        assert_eq!(head["head"]["said"], "can't read it");
        assert_eq!(head["head"]["glyph"], "failed");
        assert_eq!(unread_head(&words, "spec"), None, "Spec's words say none");
        let _ = std::fs::remove_file(&words);
    }

    #[test]
    fn a_read_past_its_deadline_is_stopped_and_one_within_it_is_kept() {
        let shell = std::env::var("BASH").unwrap_or_else(|_| "sh".to_owned());
        let mut quick = Command::new(&shell);
        quick.args(["-c", "echo made"]).stdout(Stdio::piped());
        assert_eq!(
            within(&mut quick, Duration::from_secs(5)),
            Ok(b"made\n".to_vec())
        );
        // What hangs is a child of its own, as cadence's git or niri msg
        // would be, its number written down to look for after.
        let folder = std::env::temp_dir().join(format!("cadence-spec-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the hung child's number");
        let numbered = folder.join("hung");
        let mut hung = Command::new(&shell);
        hung.args([
            "-c",
            &format!("sleep 30 & echo $! > {}; wait", numbered.display()),
        ])
        .stdout(Stdio::piped());
        let started = Instant::now();
        let stopped = within(&mut hung, Duration::from_millis(500));
        assert!(
            stopped.as_ref().is_err_and(|why| why.contains("ran past")),
            "a hung cadence is said to have run past its time, not {stopped:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a hung read is stopped at its deadline, not waited out"
        );
        let left = std::fs::read_to_string(&numbered).expect("the hung child's number");
        let proc = format!("/proc/{}/stat", left.trim());
        // Gone, or dead and waiting on whoever reaps it now.
        let ended = || match std::fs::read_to_string(&proc) {
            Err(_) => true,
            Ok(stat) => stat.split(' ').nth(2) == Some("Z"),
        };
        let mut gone = ended();
        for _ in 0..50 {
            if gone {
                break;
            }
            thread::sleep(Duration::from_millis(20));
            gone = ended();
        }
        let _ = std::fs::remove_dir_all(&folder);
        assert!(
            gone,
            "what the read started is stopped with it, not left running"
        );
        let mut failing = Command::new(&shell);
        failing.args(["-c", "exit 3"]).stdout(Stdio::piped());
        assert!(
            within(&mut failing, Duration::from_secs(5)).is_err(),
            "a cadence ending in failure is no answer"
        );
    }

    #[test]
    fn what_a_read_left_holding_its_output_is_stopped_once_it_ends() {
        let shell = std::env::var("BASH").unwrap_or_else(|_| "sh".to_owned());
        let folder = std::env::temp_dir().join(format!("cadence-spec-left-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the left child's number");
        let numbered = folder.join("left");
        // It ends at once, leaving a child of its own holding its output.
        let mut leaving = Command::new(&shell);
        leaving
            .args([
                "-c",
                &format!("sleep 30 & echo $! > {}", numbered.display()),
            ])
            .stdout(Stdio::piped());
        let started = Instant::now();
        let read = within(&mut leaving, Duration::from_secs(5));
        assert_eq!(read, Ok(Vec::new()), "read once what it left is stopped");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "never waiting on what it left to let go of the pipe"
        );
        let left = std::fs::read_to_string(&numbered).expect("the left child's number");
        let proc = format!("/proc/{}/stat", left.trim());
        let ended = || match std::fs::read_to_string(&proc) {
            Err(_) => true,
            Ok(stat) => stat.split(' ').nth(2) == Some("Z"),
        };
        let mut gone = ended();
        for _ in 0..50 {
            if gone {
                break;
            }
            thread::sleep(Duration::from_millis(20));
            gone = ended();
        }
        let _ = std::fs::remove_dir_all(&folder);
        assert!(gone, "what it left is stopped, not left running");
    }

    #[test]
    fn anything_else_reads_as_nothing_so_the_last_rows_stay() {
        assert_eq!(parsed(b"", "spec"), None, "nothing printed");
        assert_eq!(parsed(br#"{"rows": []}"#, "spec"), None, "no headline");
        assert_eq!(
            parsed(br#"{"head": {}, "rows": {}}"#, "spec"),
            None,
            "rows not a list"
        );
    }
}
