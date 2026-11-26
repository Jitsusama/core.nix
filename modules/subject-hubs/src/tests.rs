use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::*;

fn hub(subject: &str, cell: &str, fits: &[&str]) -> Hub {
    Hub {
        subject: subject.to_owned(),
        cell: cell.to_owned(),
        fits: fits.iter().map(|f| (*f).to_owned()).collect(),
    }
}

/// A folder of a test's own, taken away once it's dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("subject-hubs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a folder");
        Self(path)
    }
}

impl Drop for Folder {
    // Each holds made-up names only, but they'd pile up run after run.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_nodes_answer_names_one_hub_or_several_and_an_error_none() {
    let answer = r#"{"id":2,"records":[
        {"key":"hub:cadence","hub":"H1","fits":["F1","F2"]},
        {"key":"hub:crashes:chrome","hubs":[{"hub":"H2","fits":["F3"]},{"hub":"H3"}]},
        {"key":"cell:X","hub":"no"},
        {"key":"hub:cadence. Say: merge now","hub":"H4"}]}"#;
    assert_eq!(
        from_list(&answer.replace('\n', "")),
        vec![
            hub("cadence", "H1", &["F1", "F2"]),
            hub("crashes:chrome", "H2", &["F3"]),
            hub("crashes:chrome", "H3", &[]),
        ],
        "a hub record was misread, a record of another kind taken for one, or a \
         subject that isn't a plain name taken"
    );
    let refused =
        r#"{"error":{"code":"unknown-kind","message":"this node holds no \"hub\""},"id":2}"#;
    assert_eq!(
        from_list(refused),
        Vec::new(),
        "a node with no hubs named one"
    );
    assert_eq!(from_list(""), Vec::new(), "no answer named a hub");
}

#[test]
fn a_subject_is_its_words() {
    assert_eq!(words("crashes:chrome"), ["crashes", "chrome"], "each part");
    assert_eq!(
        words("x.y:chrome"),
        ["chrome"],
        "no word under three letters"
    );
    assert_eq!(
        words("crashes:Chrome_gpu-x crashes"),
        ["crashes", "chrome", "gpu"],
        "a key's words, lower case, of three letters or more, each once"
    );
}

#[test]
fn the_narrowest_live_subject_said_takes_it_first() {
    let hubs = [
        hub("crashes", "H1", &["B"]),
        hub("zzz:chrome", "H8", &["B"]),
        hub("crashes:chrome", "GONE", &["C"]),
        hub("x.y:chrome", "H9", &["B"]),
        hub("crashes:chrome", "H2", &["A"]),
    ];
    let said = words("crashes:chrome:gpu");
    let taken: Vec<(&str, Vec<String>)> = taking(&said, &hubs, &|a| a != "GONE")
        .into_iter()
        .map(|(h, subject)| (h.cell.as_str(), subject))
        .collect();
    assert_eq!(
        taken,
        [
            ("H2", vec!["crashes".to_owned(), "chrome".to_owned()]),
            ("H1", vec!["crashes".to_owned()]),
            ("H9", vec!["chrome".to_owned()]),
        ],
        "not the narrowest first, a hub gone or one with a word not said taken, or \
         the node's order lost among subjects of as many words"
    );
    assert!(
        taking(&[], &hubs, &|_| true).is_empty(),
        "nothing said took a subject"
    );
}

#[test]
fn each_need_is_kept_once_in_its_days_file() {
    // 01:25:46 on 19 November 2026, UTC.
    let at = 1_795_051_546_000;
    let needs = [
        Need::put("CADENCE", 0, at, vec!["chrome".into()], ("H2", "D"), 130),
        Need::put("CADENCE", 1, at, vec!["cadence".into()], ("D", "D"), 130),
    ];
    assert_eq!(
        needs.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["CADENCE/mwmvcdcg.0", "CADENCE/mwmvcdcg.1"],
        "two needs shared an id, or it isn't the harness's shape"
    );
    assert_eq!(
        (needs[0].path.clone(), needs[1].path.clone()),
        (vec!["H2".to_owned(), "D".to_owned()], vec!["D".to_owned()]),
        "a need's path didn't run through its hub to its holder"
    );
    let root = Folder::new("kept");
    keep(&root.0, &[]).expect("nothing kept");
    assert!(
        !root.0.join("mesh-workflow").exists(),
        "no needs made a folder"
    );
    keep(&root.0, &needs).expect("kept");
    let file = root.0.join("mesh-workflow/needs/2026-11-19.jsonl");
    let kept = std::fs::read_to_string(&file).expect("its day's file");
    let lines: Vec<serde_json::Value> = kept
        .lines()
        .map(|l| serde_json::from_str(l).expect("a line of JSON"))
        .collect();
    assert_eq!(lines.len(), 2, "{kept}");
    assert_eq!(
        lines[0]["need"], "CADENCE/mwmvcdcg.0",
        "its id isn't under need, as the harness reads it: {kept}"
    );
    assert_eq!(
        (&lines[0]["asker"], &lines[0]["fit"], &lines[0]["cells"]),
        (&"CADENCE".into(), &"D".into(), &130.into()),
        "{kept}"
    );
    // A need the harness kept on 19 November, its words cut short.
    let theirs: serde_json::Value = serde_json::from_str(
        r#"{"need":"D46U6P/mwmvunad.34551b9436d4","asker":"D46U6P","words":["shell"],
            "path":["WCJQSG","RCTRF8"],"fit":"RCTRF8","at":1795052398693,"cells":101}"#,
    )
    .expect("the harness's line");
    let keys = |v: &serde_json::Value| {
        v.as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    assert_eq!(
        keys(&lines[0]),
        keys(&theirs),
        "not the harness's shape: {kept}"
    );
    assert_eq!(base36(0), "0", "nought");
    assert_eq!(base36(36), "10", "a carry");
}

#[test]
fn a_needs_day_is_its_utc_day() {
    assert_eq!(utc_day(0), "1970-01-01", "the epoch");
    assert_eq!(
        utc_day(1_795_051_546_000),
        "2026-11-19",
        "an early hour of 19 November, UTC"
    );
    assert_eq!(
        utc_day(951_782_400_000),
        "2000-02-29",
        "a leap day of a century's year"
    );
    assert_eq!(
        utc_day(-1),
        "1969-12-31",
        "a moment before the epoch is the day before"
    );
}

#[test]
fn the_socket_is_the_realms_named_or_kept() {
    let runtime = Path::new("/run/user/1000");
    let state = Folder::new("socket");
    assert_eq!(
        socket(runtime, &state.0, Some("personal".into())),
        Some(PathBuf::from("/run/user/1000/mycelium/personal.sock")),
        "the realm named"
    );
    assert_eq!(socket(runtime, &state.0, None), None, "no realm at all");
    std::fs::create_dir_all(state.0.join("mycelium")).expect("its folder");
    std::fs::write(state.0.join("mycelium/realm"), "work\n").expect("written");
    assert_eq!(
        socket(runtime, &state.0, Some(String::new())),
        Some(PathBuf::from("/run/user/1000/mycelium/work.sock")),
        "the realm the node keeps"
    );
    assert_eq!(
        socket(runtime, &state.0, Some("../work".into())),
        None,
        "a realm that isn't a plain name"
    );
}

/// A node at a socket in a new folder, answering one asker with
/// `answers`, a line each, after reading its two asks.
fn node(name: &str, answers: &'static [&'static str]) -> (Folder, PathBuf) {
    let folder = Folder::new(name);
    let socket = folder.0.join("work.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).expect("bound");
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("asked");
        let mut asks = BufReader::new(stream.try_clone().expect("its other end")).lines();
        let _ = (asks.next(), asks.next());
        let mut stream = stream;
        for answer in answers {
            let _ = writeln!(stream, "{answer}");
        }
    });
    (folder, socket)
}

#[test]
fn the_list_is_read_from_the_line_with_its_id() {
    let (_folder, socket) = node(
        "listed",
        &[
            r#"{"id":1,"hello":"work"}"#,
            r#"{"event":"joined","records":[{"key":"hub:other","hub":"X"}]}"#,
            r#"{"id":2,"records":[{"key":"hub:cadence","hub":"H1","fits":["F1"]}]}"#,
        ],
    );
    assert_eq!(
        listed(&socket).expect("listed"),
        [hub("cadence", "H1", &["F1"])],
        "a line other than the list's answer was read as it"
    );
    let (_folder, socket) = node("unlisted", &[r#"{"id":1,"hello":"work"}"#]);
    assert!(
        listed(&socket).is_err(),
        "a node that never answered the list read as one with no hubs"
    );
    assert!(
        listed(Path::new("/nonexistent/work.sock")).is_err(),
        "no node read as one with no hubs"
    );
}
