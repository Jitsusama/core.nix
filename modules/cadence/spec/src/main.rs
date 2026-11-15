//! cadence-spec serves Joel's axes to the launcher's Spec place, as ready
//! rows over the socket systemd holds for it, each watch told only the
//! rows that moved. The shell never rereads a file rewritten each tick
//! (Joel, 2026-11-10: "the daemons need to be the data layer, and they
//! need to serve the launcher"), so this daemon holds the rows.
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

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Shared};
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};

/// The daemon as the launcher knows it: Spec's rows and its headline.
const DOMAIN: Domain = Domain {
    name: "cadence-spec",
    kinds: &["row", "head"],
    sole: &[],
    named_only: &[],
};

/// The place its rows are in.
const PLACE: &str = "spec";

/// How long the rows last made stand before a watch has them made again.
const FRESH: Duration = Duration::from_mins(5);

fn help() -> Help {
    Help::new("cadence-spec", "serves Joel's axes to the launcher's Spec")
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
    let mut kept = Kept::default();
    let mut last: Option<Instant> = None;
    loop {
        let watched = shared.watches("row", PLACE) || shared.watches("head", PLACE);
        if watched && last.is_none_or(|at| at.elapsed() >= FRESH) {
            last = Some(Instant::now());
            if let Some((rows, head)) = made(asking)
                && !kept.take(rows, vec![head]).is_empty()
            {
                shared.change(kept.all().cloned().collect(), json!({}));
            }
        }
        // Woken as a watch begins or ends, else when the rows go stale.
        let ended = match waiting(watched, last.map(|at| at.elapsed())) {
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

/// How long to sleep before looking again, given whether Spec is watched
/// and how long ago the rows were last made; none to sleep until a watch
/// begins or ends. Stale rows nobody watches are left stale: waking for
/// them would find nothing to do and, with nothing left to wait, wake
/// again at once, spinning a core until a watch came.
fn waiting(watched: bool, since: Option<Duration>) -> Option<Duration> {
    watched.then(|| since.map_or(FRESH, |since| FRESH.saturating_sub(since)))
}

/// Spec's rows and headline as cadence makes them now, or none where it
/// failed or read nothing, so the last rows stay. cadence's own words on
/// why go to the journal, since its stderr is this daemon's.
fn made(asking: &Asking) -> Option<(Vec<Value>, Value)> {
    let ran = Command::new(&asking.cadence)
        .arg("axes")
        .arg("--served")
        .arg(&asking.words)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output();
    let ran = match ran {
        Ok(ran) if ran.status.success() => ran,
        Ok(ran) => {
            eprintln!(
                "cadence-spec: cadence ended {}; Spec keeps its last rows",
                ran.status
            );
            return None;
        }
        Err(why) => {
            eprintln!(
                "cadence-spec: {} couldn't run: {why}; Spec keeps its last rows",
                asking.cadence.display()
            );
            return None;
        }
    };
    let rows = parsed(&ran.stdout);
    if rows.is_none() {
        eprintln!("cadence-spec: cadence's rows didn't read as rows; Spec keeps its last");
    }
    rows
}

/// The rows and headline in what `cadence axes --served` printed.
fn parsed(printed: &[u8]) -> Option<(Vec<Value>, Value)> {
    let mut made: Value = serde_json::from_slice(printed).ok()?;
    let head = made.get_mut("head").filter(|head| head.is_object())?.take();
    let rows = made.get_mut("rows")?.as_array_mut().map(std::mem::take)?;
    Some((rows, head))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_cadence_prints_reads_as_rows_and_a_headline() {
        let printed = br#"{"rows": [{"key": "row:spec:scale"}], "head": {"key": "head:spec"}}"#;
        let (rows, head) = parsed(printed).expect("rows and a headline");
        assert_eq!(rows, vec![json!({"key": "row:spec:scale"})]);
        assert_eq!(head, json!({"key": "head:spec"}));
    }

    #[test]
    fn unwatched_it_sleeps_until_a_watch_however_stale_the_rows() {
        let stale = Some(FRESH + Duration::from_secs(60));
        assert_eq!(
            waiting(false, stale),
            None,
            "stale rows nobody watches must not wake it at once, again and again"
        );
        assert_eq!(waiting(false, None), None, "never made, never watched");
        assert_eq!(
            waiting(true, Some(Duration::from_mins(2))),
            Some(Duration::from_mins(3)),
            "watched, it wakes as the rows go stale"
        );
    }

    #[test]
    fn anything_else_reads_as_nothing_so_the_last_rows_stay() {
        assert_eq!(parsed(b""), None, "nothing printed");
        assert_eq!(parsed(br#"{"rows": []}"#), None, "no headline");
        assert_eq!(
            parsed(br#"{"head": {}, "rows": {}}"#),
            None,
            "rows not a list"
        );
    }
}
