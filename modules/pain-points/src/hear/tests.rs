//! The listener's runs, over session logs, ledgers and a routes file made
//! in a folder of their own.

use super::*;
use serde_json::json;
use std::fs;
use std::io::Write;

mod answered;
mod standing;
mod troubles;

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
            misses: self.root.join("misses.jsonl"),
            misses_from: 0,
            void: self.root.join("void"),
            messages: self.root.join("messages"),
            switches: self.root.join("switches.jsonl"),
            core: self.root.join("core.git"),
            git: "git".into(),
            at: AT,
            offset_s: -5 * 3600,
        }
    }

    fn routes(&self) {
        fs::write(
            self.root.join("state/routes.json"),
            r#"{"from":"EAR001","fallback":"ORCH01","ledgers":"ORCH01","never":["NEVER1"]}"#,
        )
        .expect("routes written");
    }

    fn log(&self, folder: &str, session: &str, cwd: &Path, lines: &[String]) -> PathBuf {
        self.log_from("10", folder, session, cwd, lines)
    }

    /// A log begun at `hour` on 16 November, as pi names it by its start.
    fn log_from(
        &self,
        hour: &str,
        folder: &str,
        session: &str,
        cwd: &Path,
        lines: &[String],
    ) -> PathBuf {
        let dir = self.root.join("sessions").join(folder);
        fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join(format!("2026-11-16T{hour}-00-00-000Z_{session}.jsonl"));
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

impl Drop for Place {
    // Each run's folders hold made-up words only, but they'd pile up in the
    // temp folder run after run.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
    let mut live = vec![user(
        "old",
        "2026-11-15T10:00:00.000Z",
        "this fucking thing",
    )];
    live.extend(tool("bash"));
    live.extend([
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
fn each_pain_sent_is_kept_before_the_next_send() {
    let place = Place::new("kept");
    place.routes();
    a_day(&place);
    let mut kept_by_then = Vec::new();
    let heard_at = place.root.join("state/heard.jsonl");
    hear(&place.ears(), CELLS, &mut |to, _| {
        if to == "ORCH01" {
            kept_by_then.push(fs::read_to_string(&heard_at).unwrap_or_default());
        }
        true
    })
    .expect("a run");
    assert!(
        kept_by_then
            .first()
            .is_some_and(|kept| kept.contains(r#""to":"ABC123""#)),
        "a run stopped before its keeper's note would send the owner's finding again: \
         {kept_by_then:?}"
    );
}

#[test]
fn a_send_counts_only_when_the_node_took_it() {
    assert!(node_took(r#"{"Ok":{"Sent":1}}"#), "taken");
    assert!(
        !node_took(r#"{"Err":"from: \"bad/x\" is not a mesh name"}"#),
        "mycelium exits 0 on a refusal, so its answer says"
    );
    assert!(!node_took(""), "no answer");
}

#[test]
fn a_command_that_leaves_its_output_open_is_not_waited_on() {
    let started = std::time::Instant::now();
    let shell = std::env::var("BASH").unwrap_or_else(|_| "sh".into());
    let mut leaving = Command::new(shell);
    leaving.args(["-c", "sleep 30 & echo hi"]);
    let _ = output_within(&mut leaving, std::time::Duration::from_millis(500));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "a child holding the pipe open doesn't hold the run"
    );
}

#[test]
fn a_command_printing_more_than_a_pipe_holds_is_read_whole() {
    let mut zeros = Command::new("head");
    zeros.args(["-c", "300000", "/dev/zero"]);
    let out = output_within(&mut zeros, std::time::Duration::from_secs(5));
    assert_eq!(out.map(|o| o.len()), Some(300_000));
}

/// read's state of several sessions' turns, each `(log, enter, wait_ms)`.
fn timed_logs(place: &Place, turns: &[(&str, i64, i64)]) {
    let mut logs = serde_json::Map::new();
    for (name, enter, wait_ms) in turns {
        let turns = json!([{"offset": 0, "enter": enter, "delivered": enter,
            "calls": [{"start": enter + wait_ms, "ms": 4000, "stop": "done"}],
            "again": false, "correction": false, "status": false}]);
        logs.insert(
            (*name).to_string(),
            json!({"end": 0, "open": false, "turns": turns}),
        );
    }
    let kept = json!({"reader": "r", "logs": logs});
    fs::write(place.root.join("state/pi-logs.json"), kept.to_string()).expect("written");
}

#[test]
fn a_session_carried_into_a_new_log_is_heard_once_and_goes_to_the_live_cell() {
    let place = Place::new("carried-on");
    place.routes();
    // The old log began first, as pi names it, and its session has ended.
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-old","address":"DEF456","state":"ended"},
        {"id":"s-new","address":"ABC123","state":"busy"}]}}"#;
    let said = [user(
        "a",
        "2026-11-16T19:50:00.000Z",
        "no, the drawer forgets its place",
    )];
    place.log_from("09", "src", "s-old", Path::new("/home/j/src"), &said);
    place.log_from("10", "src", "s-new", Path::new("/home/j/src"), &said);
    timed_logs(
        &place,
        &[
            (
                "2026-11-16T09-00-00-000Z_s-old.jsonl",
                1_794_858_600_000,
                180_000,
            ),
            (
                "2026-11-16T10-00-00-000Z_s-new.jsonl",
                1_794_858_600_000,
                180_000,
            ),
        ],
    );
    let mut sent: Vec<(String, String)> = Vec::new();
    let hour = hear(&place.ears(), cells, &mut |to, text| {
        // The keeper's note names each pain again; only findings count.
        if !text.contains("heard from Joel, for you to carry") {
            sent.push((to.into(), text.into()));
        }
        true
    })
    .expect("a run");
    let to_live = sent.iter().filter(|(to, _)| to == "ABC123").count();
    let drawer = sent.iter().filter(|(_, t)| t.contains("drawer")).count();
    let stalls = sent
        .iter()
        .map(|(_, t)| t.matches("turn of").count())
        .sum::<usize>();
    assert_eq!(
        (to_live, drawer, stalls, hour.heard),
        (1, 1, 1, 1),
        "the live cell carrying it on owns it: {sent:?}"
    );
}

#[test]
fn two_sessions_stalled_in_the_same_second_are_both_heard() {
    let place = Place::new("two-stalls");
    place.routes();
    place.log("src", "s-live", Path::new("/home/j/src"), &[]);
    place.log("elsewhere", "s-other", Path::new("/home/j/other"), &[]);
    timed_logs(
        &place,
        &[
            (
                "2026-11-16T10-00-00-000Z_s-live.jsonl",
                1_794_858_600_000,
                180_000,
            ),
            (
                "2026-11-16T10-00-00-000Z_s-other.jsonl",
                1_794_858_600_000,
                90_000,
            ),
        ],
    );
    let cells = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy"},
        {"id":"s-other","address":"GHI789","state":"busy"}]}}"#;
    let hour = hear(&place.ears(), cells, &mut |_, _| true).expect("a run");
    assert_eq!(hour.stalled, 2, "{:?}", place.heard());
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
    assert_eq!(
        to.iter().filter(|t| **t == "ABC123").count(),
        1,
        "one finding: {sent:?}"
    );
    assert!(hour.troubles.is_empty(), "a run that went well: {hour:?}");
    assert_eq!(
        to.iter().filter(|t| **t == "ORCH01").count(),
        2,
        "void and ledgers: {sent:?}"
    );
    assert!(!to.contains(&"VOID01"), "nothing goes to the void");
    let note = &sent
        .iter()
        .rfind(|(to, _)| to == "ORCH01")
        .expect("noted")
        .1;
    assert!(
        !note.contains("slow"),
        "the keeper isn't told again of what it was just sent: {note}"
    );
    let live = &sent.iter().find(|(to, _)| to == "ABC123").expect("sent").1;
    assert!(
        live.contains("broken again") && live.contains("status update"),
        "{live}"
    );
    assert!(
        live.contains("bash failed"),
        "the tool that failed before it: {live}"
    );
    assert!(!live.contains("love"), "praise isn't pain: {live}");
    assert!(
        !live.contains("fucking"),
        "older than a day isn't heard: {live}"
    );
    assert!(
        !live.contains("tight"),
        "what the ledgers carry goes to no one: {live}"
    );
    let heard = place.heard();
    assert_eq!(heard.len(), 4, "{heard:?}");
    assert!(
        heard.iter().any(|h| h.carried && h.entry == "e4"),
        "the ledger carries it: {heard:?}"
    );
    assert!(
        !heard.iter().any(|h| h.entry == "l1"),
        "a lab's words aren't his"
    );
    assert_eq!(hour.heard, 4);
    assert_eq!(hour.routed, 3);
    assert_eq!(hour.open, 2, "the void's pain is open to no cell");
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
    writeln!(
        file,
        "{}",
        user("e5", "2026-11-16T19:58:00.000Z", "hold up, not that")
    )
    .expect("a line");
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    assert_eq!(
        sent.len(),
        2,
        "the new pain and the ledgers' note: {sent:?}"
    );
    assert!(
        sent[0].1.contains("hold up") && sent[1].1.contains("hold up"),
        "sent again on a later run: {sent:?}"
    );
    assert_eq!(place.heard().len(), 5);
}

#[test]
fn a_pain_that_did_not_go_is_sent_on_the_next_run() {
    let place = Place::new("failed");
    place.routes();
    a_day(&place);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| false).expect("a run");
    assert_eq!(hour.routed, 0);
    assert_eq!(hour.open, 2, "the void's pain is open to no cell");
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
    assert_eq!(hour.open, 2, "the void's pain is open to no cell");
}

#[test]
fn a_pain_hours_old_when_first_heard_goes_to_the_ledgers_keeper_alone() {
    let place = Place::new("stale");
    place.routes();
    place.log(
        "src",
        "s-live",
        Path::new("/home/j/src"),
        &[
            user("old", "2026-11-16T15:00:00.000Z", "status update please"),
            user("new", "2026-11-16T19:50:00.000Z", "no, that's broken"),
        ],
    );
    let mut sent: Vec<(String, String)> = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    let live = &sent.iter().find(|(to, _)| to == "ABC123").expect("sent").1;
    assert!(
        live.contains("broken") && !live.contains("status"),
        "{live}"
    );
    let note = &sent.iter().find(|(to, _)| to == "ORCH01").expect("noted").1;
    assert!(
        note.contains("status update please") && note.contains("broken"),
        "{note}"
    );
    assert!(
        place.heard().iter().all(|h| h.routed_at.is_some()),
        "the old one went to the keeper: {:?}",
        place.heard()
    );
}

#[test]
fn a_short_reply_said_twice_is_no_pain() {
    let place = Place::new("short");
    place.log(
        "src",
        "s-live",
        Path::new("/home/j/src"),
        &[
            user("a", "2026-11-16T19:10:00.000Z", "i sent the message"),
            user("b", "2026-11-16T19:20:00.000Z", "sent the messages"),
        ],
    );
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.heard, 0, "{:?}", place.heard());
}

#[test]
fn a_finding_names_five_pains_the_most_hit_first() {
    let place = Place::new("five");
    place.routes();
    let pains = [
        "no, the bar is broken",
        "the launcher is stuck on its first row",
        "no, wrong colour for the notices",
        "why did the kitty drawer close by itself",
        "the map keeps losing my place when it redraws",
        "no, the clock shows yesterday",
    ];
    let said: Vec<String> = pains
        .iter()
        .enumerate()
        .map(|(n, words)| {
            let at = format!("2026-11-16T19:{:02}:00.000Z", 10 + n * 5);
            user(&format!("e{n}"), &at, words)
        })
        .chain([user(
            "e9",
            "2026-11-16T19:59:00.000Z",
            "the launcher is stuck on its first row again",
        )])
        .collect();
    place.log("src", "s-live", Path::new("/home/j/src"), &said);
    let mut sent: Vec<(String, String)> = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    let finding = &sent.iter().find(|(to, _)| to == "ABC123").expect("sent").1;
    assert!(
        finding.contains("\n5. ") && !finding.contains("\n6. "),
        "{finding}"
    );
    assert!(
        finding.find("stuck").unwrap_or(usize::MAX) < finding.find("the bar").unwrap_or(0),
        "the one said twice leads: {finding}"
    );
    let routed = place
        .heard()
        .iter()
        .filter(|h| h.routed_at.is_some())
        .count();
    assert_eq!(
        routed, 5,
        "those past the fifth, not named, wait for the next run"
    );
}

#[test]
fn a_session_this_realm_does_not_know_is_never_sent_however_old() {
    let place = Place::new("unknown");
    place.routes();
    let said = [user("a", "2026-11-16T15:00:00.000Z", "no, that's broken")];
    place.log("src", "s-other", Path::new("/home/j/src"), &said);
    let hour = hear(&place.ears(), CELLS, &mut |to, text| {
        panic!("another realm's words went to {to}: {text}")
    })
    .expect("a run");
    assert_eq!(hour.open, 1, "it waits, counted open");
}

#[test]
fn a_pain_the_keeper_carries_later_is_closed() {
    let place = Place::new("closed");
    place.routes();
    let words = "no, the drawer forgets its place";
    let said = [user("a", "2026-11-16T19:50:00.000Z", words)];
    place.log("src", "s-live", Path::new("/home/j/src"), &said);
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.open, 1, "{hour:?}");
    let line = json!({"at": "x", "words": words, "owner": "Y"}).to_string();
    fs::write(place.root.join("asks.jsonl"), line).expect("a ledger");
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a second run");
    assert_eq!(hour.open, 0, "carried since: {hour:?}");
}

#[test]
fn nothing_goes_to_a_cell_the_routes_say_never_to_whatever_its_case() {
    let place = Place::new("never");
    fs::write(
        place.root.join("state/routes.json"),
        r#"{"from":"EAR001","fallback":"void01","ledgers":"ORCH01","never":["VOID01"]}"#,
    )
    .expect("routes");
    a_day(&place);
    let mut live = 0;
    let hour = hear(&place.ears(), CELLS, &mut |to, text| {
        assert!(!to.eq_ignore_ascii_case("void01"), "sent the void: {text}");
        live += usize::from(to == "ABC123");
        true
    })
    .expect("a run");
    assert_eq!(live, 1, "the live cell still hears its own");
    assert!(
        hour.troubles.iter().any(|t| t.contains("void01")),
        "a fallback never sent to says so: {hour:?}"
    );
}

#[test]
fn a_command_that_hangs_gives_nothing_and_is_stopped() {
    let started = std::time::Instant::now();
    let mut sleeping = Command::new("sleep");
    sleeping.arg("30");
    assert_eq!(
        output_within(&mut sleeping, std::time::Duration::from_millis(200)),
        None
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "stopped, not waited on"
    );
    let mut echo = Command::new("echo");
    echo.arg("hi");
    assert_eq!(
        output_within(&mut echo, std::time::Duration::from_secs(5)).as_deref(),
        Some("hi\n")
    );
}

/// read's state of one session's turns: one begun at `enter` whose first
/// call waited `wait_ms` for pi.
fn timed(place: &Place, session: &str, enter: i64, wait_ms: i64) {
    let turns = json!([{"offset": 0, "enter": enter, "delivered": enter,
        "calls": [{"start": enter + wait_ms, "ms": 4000, "stop": "done"}],
        "again": false, "correction": false, "status": false}]);
    let name = format!("2026-11-16T10-00-00-000Z_{session}.jsonl");
    let kept = json!({"reader": "r", "logs": {name: {"end": 0, "open": false, "turns": turns}}});
    fs::write(place.root.join("state/pi-logs.json"), kept.to_string()).expect("written");
}

#[test]
fn a_turn_that_stalled_him_goes_to_its_cell_though_he_said_nothing() {
    let place = Place::new("stalled");
    place.routes();
    let said = [user("a", "2026-11-16T19:50:00.000Z", "make the bar green")];
    place.log("src", "s-live", Path::new("/home/j/src"), &said);
    // 19:50 UTC, and a three-minute wait for pi's first call.
    timed(&place, "s-live", 1_794_858_600_000, 180_000);
    let mut sent: Vec<(String, String)> = Vec::new();
    hear(&place.ears(), CELLS, &mut |to, text| {
        sent.push((to.into(), text.into()));
        true
    })
    .expect("a run");
    let finding = &sent.iter().find(|(to, _)| to == "ABC123").expect("sent").1;
    assert!(finding.contains("stalled"), "{finding}");
    assert!(
        finding.contains("lost him 120 s"),
        "read's own reckoning: {finding}"
    );
    assert!(
        !finding.contains("\"make the bar green\""),
        "what he said was no pain: {finding}"
    );
    assert!(
        !sent
            .iter()
            .any(|(to, text)| to == "ORCH01" && text.contains("turn of")),
        "a stall is no ask for the keeper: {sent:?}"
    );
    hear(&place.ears(), CELLS, &mut |to, text| {
        panic!("sent {to} again: {text}");
    })
    .expect("a second run");
}

#[test]
fn a_short_wait_is_no_stall() {
    let place = Place::new("short-wait");
    place.routes();
    let said = [user("a", "2026-11-16T19:50:00.000Z", "make the bar green")];
    place.log("src", "s-live", Path::new("/home/j/src"), &said);
    timed(&place, "s-live", 1_794_858_600_000, 20_000);
    hear(&place.ears(), CELLS, &mut |to, text| {
        panic!("a twenty-second wait sent {to}: {text}");
    })
    .expect("a run");
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
            user(
                "a",
                "2026-11-16T19:10:00.000Z",
                "the launcher is so slow to open",
            ),
            user("b", "2026-11-16T19:20:00.000Z", "make the bar green"),
            user(
                "c",
                "2026-11-16T19:50:00.000Z",
                "why is the launcher still slow to open",
            ),
            user("d", "2026-11-16T19:55:00.000Z", "status update"),
        ],
    );
    let hour = hear(&place.ears(), CELLS, &mut |_, _| true).expect("a run");
    assert_eq!(hour.worst_hits, 2, "{hour:?}");
    assert!(
        hour.worst
            .as_deref()
            .is_some_and(|w| w.contains("slow to open")),
        "{hour:?}"
    );
}
