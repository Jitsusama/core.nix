//! The collector as its account runs it: a sender writes a line to the
//! socket, and `at` and `trace` read it back.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COLLECTOR: &str = env!("CARGO_BIN_EXE_collector");
const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";

/// The collector serving, stopped when the test ends however it ends.
struct Serving(Child);

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn ask(store: &Path, words: &[&str]) -> Output {
    Command::new(COLLECTOR)
        .args(words)
        .arg("--store")
        .arg(store)
        .env("TZ", "UTC")
        .output()
        .expect("collector runs")
}

/// Waits for `ready`, failing rather than hanging when it never comes.
fn until<T>(what: &str, ready: impl FnMut() -> Option<T>) -> T {
    within(10, what, ready)
}

fn within<T>(seconds: u64, what: &str, mut ready: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        if let Some(it) = ready() {
            return it;
        }
        assert!(Instant::now() < deadline, "{what} within {seconds} s");
        std::thread::yield_now();
    }
}

/// The kind of each line a `jsonl` answer gives, in order.
fn kinds(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| line["type"].as_str().map(str::to_owned))
        .collect()
}

fn pressure(some: u64) -> String {
    format!(
        "some avg10=0.00 avg60=0.00 avg300=0.00 total={some}\n\
         full avg10=0.00 avg60=0.00 avg300=0.00 total={}\n",
        some / 2
    )
}

// The machine's memory waits half a second once the collector has read
// it quiet, and the wait is kept as the kernel's span once three quiet
// seconds end it. Its first reading is taken before the socket is made,
// so a socket there means the quiet one was read.
#[test]
fn a_wait_the_kernel_counts_is_kept_as_its_span() {
    let dir = tempfile::tempdir().expect("a folder");
    let socket = dir.path().join("run/lines.sock");
    let store = dir.path().join("store");
    let root = dir.path().join("root");
    let memory = root.join("proc/pressure/memory");
    std::fs::create_dir_all(memory.parent().unwrap()).unwrap();
    std::fs::write(&memory, pressure(0)).unwrap();
    let _serving = Serving(
        Command::new(COLLECTOR)
            .args(["serve", "--socket"])
            .arg(&socket)
            .arg("--store")
            .arg(&store)
            .arg("--pressure")
            .arg(&root)
            .spawn()
            .expect("collector serves"),
    );
    until("the socket", || UnixStream::connect(&socket).ok());
    std::fs::write(&memory, pressure(500_000)).unwrap();

    // Its three quiet seconds come after the waiting one.
    let line = within(30, "the wait kept", || {
        let out = ask(&store, &["spans", "--around", "10m", "--format", "jsonl"]);
        let text = String::from_utf8(out.stdout).ok()?;
        text.lines()
            .find(|line| line.contains("memory waits in the machine"))
            .map(str::to_owned)
    });
    assert!(line.contains(r#""service":"kernel""#), "{line}");
    assert!(line.contains(r#""waited_ms":500"#), "{line}");
    assert!(line.contains(r#""full_ms":250"#), "{line}");
}

// app.slice's memory is read as the collector starts, below the account's
// manager its cgroup line names, and kept as the kernel's span.
#[test]
fn what_a_floored_slice_holds_is_kept_as_its_span() {
    let dir = tempfile::tempdir().expect("a folder");
    let socket = dir.path().join("run/lines.sock");
    let store = dir.path().join("store");
    let root = dir.path().join("root");
    let manager = "user.slice/user-1000.slice/user@1000.service";
    std::fs::create_dir_all(root.join("proc/self")).unwrap();
    std::fs::write(
        root.join("proc/self/cgroup"),
        format!("0::/{manager}/app.slice/collector.service\n"),
    )
    .unwrap();
    let app = root.join("sys/fs/cgroup").join(manager).join("app.slice");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("memory.stat"), "anon 6120587264\nfile 644874240\n").unwrap();
    std::fs::write(app.join("memory.swap.current"), "21367615488\n").unwrap();
    std::fs::write(app.join("memory.min"), "6690131968\n").unwrap();
    let _serving = Serving(
        Command::new(COLLECTOR)
            .args(["serve", "--socket"])
            .arg(&socket)
            .arg("--store")
            .arg(&store)
            .arg("--pressure")
            .arg(&root)
            .spawn()
            .expect("collector serves"),
    );

    let line = within(30, "what app.slice holds", || {
        let out = ask(&store, &["spans", "--around", "10m", "--format", "jsonl"]);
        let text = String::from_utf8(out.stdout).ok()?;
        text.lines()
            .find(|line| line.contains("memory held by app.slice"))
            .map(str::to_owned)
    });
    assert!(line.contains(r#""service":"kernel""#), "{line}");
    assert!(line.contains(r#""anon_bytes":6120587264"#), "{line}");
    assert!(line.contains(r#""swap_bytes":21367615488"#), "{line}");
    assert!(line.contains(r#""floor_bytes":6690131968"#), "{line}");
}

fn request(now: u128) -> String {
    let start = now - 2_000_000_000;
    format!(
        concat!(
            r#"{{"resourceSpans":[{{"resource":{{"attributes":[{{"key":"service.name","#,
            r#""value":{{"stringValue":"launcher"}}}}]}},"scopeSpans":[{{"spans":["#,
            r#"{{"traceId":"{t}","spanId":"00f067aa0ba902b7","name":"launcher.open","#,
            r#""startTimeUnixNano":"{s}","endTimeUnixNano":"{e}"}},"#,
            r#"{{"traceId":"{t}","spanId":"1111111111111111","parentSpanId":"00f067aa0ba902b7","#,
            r#""name":"launcher.layout","startTimeUnixNano":"{s}","endTimeUnixNano":"{m}","#,
            r#""attributes":[{{"key":"first_y","value":{{"intValue":"0"}}}}]}}]}}]}}]}}"#
        ),
        t = TRACE,
        s = start,
        m = start + 1_000_000,
        e = now
    )
}

#[test]
fn a_span_sent_is_read_back_at_its_moment_and_in_its_trace() {
    let dir = tempfile::tempdir().expect("a folder");
    let socket = dir.path().join("run/lines.sock");
    let store = dir.path().join("store");
    let _serving = Serving(
        Command::new(COLLECTOR)
            .args(["serve", "--socket"])
            .arg(&socket)
            .arg("--store")
            .arg(&store)
            .spawn()
            .expect("collector serves"),
    );
    let mut sender = until("the socket", || UnixStream::connect(&socket).ok());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    writeln!(sender, "{}", request(now)).expect("sent");
    // A line that isn't a request is dropped, and the next still taken.
    writeln!(sender, "not a request").expect("sent");
    drop(sender);

    let lines = until("both spans kept", || {
        let out = ask(&store, &["spans", "--around", "10m", "--format", "jsonl"]);
        let text = String::from_utf8(out.stdout).ok()?;
        // The kernel's own spans are kept beside them, and on a machine
        // with slices to read there are some, so only the sender's count.
        let sent = text
            .lines()
            .filter(|line| line.contains(r#""service":"launcher""#));
        (sent.count() == 2).then_some(text)
    });
    assert!(lines.contains(r#""name":"launcher.open""#), "{lines}");
    assert!(lines.contains(r#""first_y":0"#), "{lines}");
    assert_eq!(
        kinds(&lines).last().map(String::as_str),
        Some("end"),
        "the stream says it ended"
    );

    // As JSON the spans are one object, each one's times in RFC 3339.
    let whole = ask(
        &store,
        &["spans", "--at", "now", "--around", "10m", "--json"],
    );
    let whole: serde_json::Value = serde_json::from_slice(&whole.stdout).expect("one object");
    assert_eq!(whole["schema"], "collector.spans/1");
    let first = &whole["spans"][0];
    assert!(
        first["started_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "a time in RFC 3339: {first}"
    );
    assert!(first["took_ms"].is_number(), "{first}");

    let tree = ask(&store, &["traces", TRACE]);
    assert!(tree.status.success());
    let tree = String::from_utf8(tree.stdout).unwrap();
    let rows: Vec<&str> = tree.lines().collect();
    assert_eq!(rows.len(), 2, "{tree}");
    assert!(rows[0].contains("  launcher.open  launcher"), "{tree}");
    assert!(
        rows[1].contains("    launcher.layout  launcher"),
        "the child sits below: {tree}"
    );
    assert!(rows[1].contains("first_y=0"), "{tree}");

    let empty = ask(&store, &["spans", "--at", "2001-01-01 00:00"]);
    assert_eq!(
        String::from_utf8(empty.stdout).unwrap(),
        "No span was open then.\n"
    );
    let missing = ask(&store, &["traces", "0123456789abcdef0123456789abcdef"]);
    assert_eq!(
        missing.status.code(),
        Some(66),
        "a trace not kept isn't found"
    );
    let mistyped = ask(&store, &["tarce", TRACE]);
    assert_eq!(
        mistyped.status.code(),
        Some(64),
        "a mistyped command is asked wrongly"
    );

    // The words asked before each resource took its own still answer,
    // exactly as the new ones do.
    for (old, new) in [
        (
            vec!["at", "2001-01-01 00:00"],
            vec!["spans", "--at", "2001-01-01 00:00"],
        ),
        (vec!["trace", TRACE], vec!["traces", TRACE]),
    ] {
        let (was, is) = (ask(&store, &old), ask(&store, &new));
        assert_eq!(
            (was.status.code(), was.stdout, was.stderr),
            (is.status.code(), is.stdout, is.stderr),
            "{old:?} should answer as {new:?} does"
        );
    }
}
