//! The listener's runs, over session logs, ledgers and a routes file made
//! in a folder of their own.

use super::*;
use serde_json::json;
use std::fs;
use std::io::Write;

/// 20:00 UTC on 16 November 2026.
const AT: i64 = 1_794_859_200_000;

const CELLS: &str = r#"{"Ok":{"Sessions":[
    {"id":"s-live","address":"ABC123","state":"busy"},
    {"id":"s-void","address":"VOID01","state":"idle"}]}}"#;

struct Place {
    root: PathBuf,
}

impl Place {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("pain-hear-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sessions")).expect("a folder");
        fs::create_dir_all(root.join("state")).expect("a folder");
        fs::create_dir_all(root.join("void")).expect("a folder");
        Self { root }
    }

    fn ears(&self) -> Ears {
        Ears {
            sessions: self.root.join("sessions"),
            state: self.root.join("state"),
            ledgers: vec![self.root.join("asks.jsonl"), self.root.join("misses.jsonl")],
            void: self.root.join("void"),
            at: AT,
            offset_s: -5 * 3600,
        }
    }

    fn routes(&self) {
        fs::write(
            self.root.join("state/routes.json"),
            r#"{"from":"EAR001","fallback":"ORCH01","ledgers":"ORCH01"}"#,
        )
        .expect("routes written");
    }

    fn log(&self, folder: &str, session: &str, cwd: &Path, lines: &[String]) -> PathBuf {
        let dir = self.root.join("sessions").join(folder);
        fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join(format!("2026-11-16T10-00-00-000Z_{session}.jsonl"));
        let header = json!({"type": "session", "version": 3, "id": session,
                            "timestamp": "2026-11-16T10:00:00.000Z", "cwd": cwd});
        let mut text = header.to_string() + "\n";
        for line in lines {
            text += line;
            text += "\n";
        }
        fs::write(&path, text).expect("a log written");
        path
    }

    fn heard(&self) -> Vec<Heard> {
        fs::read_to_string(self.root.join("state/heard.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).expect("a heard line"))
            .collect()
    }
}

fn user(id: &str, stamp: &str, text: &str) -> String {
    json!({"type": "message", "id": id, "parentId": "p", "timestamp": stamp,
           "message": {"role": "user", "content": [{"type": "text", "text": text}],
                       "timestamp": 0}})
    .to_string()
}

fn tool(name: &str) -> Vec<String> {
    vec![
        json!({"type": "message", "id": "a", "parentId": "p",
               "timestamp": "2026-11-16T19:29:00.000Z",
               "message": {"role": "assistant", "timestamp": 0,
                           "content": [{"type": "toolCall", "id": "c", "name": name,
                                        "arguments": {}}]}})
        .to_string(),
        json!({"type": "message", "id": "r", "parentId": "a",
               "timestamp": "2026-11-16T19:29:01.000Z",
               "message": {"role": "toolResult", "toolCallId": "c", "toolName": name,
                           "isError": true, "timestamp": 0,
                           "content": [{"type": "text", "text": "boom"}]}})
        .to_string(),
    ]
}

/// A day of his: two pains and praise in a live session, one older than a
/// day, one the ledgers already carry, one in the void and one in a lab.
fn a_day(place: &Place) -> PathBuf {
    fs::write(
        place.root.join("asks.jsonl"),
        r#"{"at":"x","words":"the launcher is way too tight ... on the right","owner":"Y"}"#,
    )
    .expect("a ledger");
    let mut live = tool("bash");
    live.extend([
        user("old", "2026-11-15T10:00:00.000Z", "this fucking thing"),
        user("e1", "2026-11-16T19:30:00.000Z", "no, that's broken again"),
        user("e2", "2026-11-16T19:31:00.000Z", "I love this page!"),
        user("e3", "2026-11-16T19:40:00.000Z", "status update"),
        user(
            "e4",
            "2026-11-16T19:45:00.000Z",
            "the launcher is way too tight on the right hand side, wtf",
        ),
    ]);
    let path = place.log("src", "s-live", Path::new("/home/j/src"), &live);
    place.log(
        "void",
        "s-void",
        &place.root.join("void"),
        &[user(
            "v1",
            "2026-11-16T19:50:00.000Z",
            "[Joel is asking from outside this window: answer first.]\n\nwhy is the launcher \
             so slow\n\nAttached:\n- window: kitty · zsh",
        )],
    );
    place.log(
        "lab",
        "s-lab",
        Path::new("/c/quest-workspace/Q/tmp"),
        &[user("l1", "2026-11-16T19:50:00.000Z", "no, wrong")],
    );
    path
}

#[test]
fn a_first_run_sends_each_new_pain_to_its_owner_once() {
    let place = Place::new("first");
    place.routes();
    a_day(&place);
    let mut sent: Vec<(String, String)> = Vec::new();
    let hour = hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.to_string(), text.to_string()));
        true
    })
    .expect("a run");
    let to: Vec<&str> = sent.iter().map(|(to, _)| to.as_str()).collect();
    assert_eq!(to.iter().filter(|t| **t == "ABC123").count(), 1, "one finding: {sent:?}");
    assert_eq!(to.iter().filter(|t| **t == "ORCH01").count(), 2, "void and ledgers: {sent:?}");
    assert!(!to.contains(&"VOID01"), "nothing goes to the void");
    let live = &sent.iter().find(|(to, _)| to == "ABC123").expect("sent").1;
    assert!(live.contains("broken again") && live.contains("status update"), "{live}");
    assert!(live.contains("bash failed"), "the tool that failed before it: {live}");
    assert!(!live.contains("love"), "praise isn't pain: {live}");
    assert!(!live.contains("fucking"), "older than a day isn't heard: {live}");
    assert!(!live.contains("tight"), "what the ledgers carry goes to no one: {live}");
    let heard = place.heard();
    assert_eq!(heard.len(), 4, "{heard:?}");
    assert!(heard.iter().any(|h| h.carried && h.entry == "e4"));
    assert!(!heard.iter().any(|h| h.entry == "l1"), "a lab's words aren't his");
    assert_eq!(hour.heard, 4);
    assert_eq!(hour.routed, 3);
    assert_eq!(hour.open, 3);
    let kept: Hour = serde_json::from_str(
        &fs::read_to_string(place.root.join("state/hour.json")).expect("hour.json"),
    )
    .expect("an hour");
    assert_eq!(kept, hour);
}

#[test]
fn a_later_run_sends_only_what_is_new() {
    let place = Place::new("later");
    place.routes();
    let live = a_day(&place);
    let mut count = 0;
    hear(&place.ears(), CELLS, &mut |_, _| {
        count += 1;
        true
    })
    .expect("a run");
    assert_eq!(count, 3);
    let mut sent: Vec<(String, String)> = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    assert!(sent.is_empty(), "nothing new, nothing sent: {sent:?}");
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&live)
        .expect("the log");
    writeln!(file, "{}", user("e5", "2026-11-16T19:58:00.000Z", "hold up, not that")).expect("a line");
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    assert_eq!(sent.len(), 2, "the new pain and the ledgers' note: {sent:?}");
    assert!(sent[0].1.contains("hold up") && sent[1].1.contains("hold up"));
    assert_eq!(place.heard().len(), 5);
}

#[test]
fn a_pain_that_did_not_go_is_sent_on_the_next_run() {
    let place = Place::new("failed");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| false).expect("a run");
    assert_eq!(hour.routed, 0);
    assert_eq!(hour.open, 3);
    let mut to: Vec<String> = Vec::new();
    hear(&place.ears(), CELLS, &mut |t, _| {
        to.push(t.into());
        true
    })
    .expect("a run");
    assert_eq!(to.iter().filter(|t| *t == "ABC123").count(), 1, "{to:?}");
}

#[test]
fn with_no_routes_nothing_is_sent_and_every_pain_waits() {
    let place = Place::new("unrouted");
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |to, _| panic!("sent to {to}")).expect("a run");
    assert_eq!(hour.heard, 4);
    assert_eq!(hour.routed, 0);
    assert_eq!(hour.open, 3);
}

#[test]
fn what_he_hits_most_leads_the_hour() {
    let place = Place::new("worst");
    place.routes();
    place.log(
        "src",
        "s-live",
        Path::new("/home/j/src"),
        &[
            user("a", "2026-11-16T19:10:00.000Z", "the launcher is so slow to open"),
            user("b", "2026-11-16T19:20:00.000Z", "make the bar green"),
            user("c", "2026-11-16T19:50:00.000Z", "why is the launcher still slow to open"),
            user("d", "2026-11-16T19:55:00.000Z", "status update"),
        ],
    );
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.worst_hits, 2, "{hour:?}");
    assert!(hour.worst.as_deref().is_some_and(|w| w.contains("slow to open")), "{hour:?}");
}
