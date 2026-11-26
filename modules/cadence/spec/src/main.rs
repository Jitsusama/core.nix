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
//! 4443be2d).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, mpsc};
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

/// A place it serves: its name, how cadence is asked for its rows, and how
/// long the rows last made stand before a watch has them made again.
struct Place {
    name: &'static str,
    flag: &'static str,
    fresh: Duration,
}

/// The places it serves, Spec first.
const PLACES: [Place; 2] = [
    Place {
        name: "spec",
        flag: "--served",
        fresh: Duration::from_mins(5),
    },
    Place {
        name: "asked",
        flag: "--asked",
        fresh: Duration::from_mins(1),
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
        let mut moved = false;
        let mut wait: Option<Duration> = None;
        for (at, place) in PLACES.iter().enumerate() {
            let watched = shared.watches("row", place.name) || shared.watches("head", place.name);
            if watched && last[at].is_none_or(|made| made.elapsed() >= place.fresh) {
                last[at] = Some(Instant::now());
                if let Some((rows, head)) = made(asking, place)
                    && !kept[at].take(rows, vec![head]).is_empty()
                {
                    moved = true;
                }
            }
            let since = last[at].map(|made| made.elapsed());
            if let Some(this) = waiting(watched, since, place.fresh) {
                wait = Some(wait.map_or(this, |other| other.min(this)));
            }
        }
        // Every place's rows are handed at once, since a record left out
        // is told as gone.
        if moved {
            let all = kept.iter().flat_map(Kept::all).cloned().collect();
            shared.change(all, json!({}));
        }
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

/// A place's rows and headline as cadence makes them now, or none where
/// it failed or read nothing, so the last rows stay. cadence's own words
/// on why go to the journal, since its stderr is this daemon's.
fn made(asking: &Asking, place: &Place) -> Option<(Vec<Value>, Value)> {
    let ran = Command::new(&asking.cadence)
        .arg("axes")
        .arg(place.flag)
        .arg(&asking.words)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output();
    let ran = match ran {
        Ok(ran) if ran.status.success() => ran,
        Ok(ran) => {
            eprintln!(
                "cadence-spec: cadence ended {}; {} keeps its last rows",
                ran.status, place.name
            );
            return None;
        }
        Err(why) => {
            eprintln!(
                "cadence-spec: {} couldn't run: {why}; {} keeps its last rows",
                asking.cadence.display(),
                place.name
            );
            return None;
        }
    };
    let rows = parsed(&ran.stdout, place.name);
    if rows.is_none() {
        eprintln!(
            "cadence-spec: cadence's rows didn't read as rows; {} keeps its last",
            place.name
        );
    }
    rows
}

/// The rows and headline in what cadence printed for `place`, the
/// headline as the whole record served beside the rows, its fields under
/// its kind, as every place's reader reads one (ReadyRows.qml's held);
/// served bare, Spec's head read as nothing (Y2YBTS, 2026-11-14).
fn parsed(printed: &[u8], place: &str) -> Option<(Vec<Value>, Value)> {
    let mut made: Value = serde_json::from_slice(printed).ok()?;
    let head = made.get_mut("head").filter(|head| head.is_object())?.take();
    let rows = made.get_mut("rows")?.as_array_mut().map(std::mem::take)?;
    let key = format!("head:{place}");
    Some((rows, json!({"key": key, "kind": "head", "head": head})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_cadence_prints_reads_as_rows_and_a_headline() {
        let printed = br#"{"rows": [{"key": "row:spec:scale"}],
            "head": {"key": "head:spec", "said": "all in spec"}}"#;
        let (rows, head) = parsed(printed, "spec").expect("rows and a headline");
        assert_eq!(rows, vec![json!({"key": "row:spec:scale"})]);
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
        let fresh = PLACES[0].fresh;
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
    fn each_place_is_asked_of_cadence_by_its_own_flag_and_keeps_its_own_head() {
        let names: Vec<(&str, &str)> = PLACES.iter().map(|p| (p.name, p.flag)).collect();
        assert_eq!(names, [("spec", "--served"), ("asked", "--asked")]);
        let printed = br#"{"rows": [], "head": {"said": "3 open"}}"#;
        let (_, head) = parsed(printed, "asked").expect("a headline");
        assert_eq!(head["key"], "head:asked");
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
