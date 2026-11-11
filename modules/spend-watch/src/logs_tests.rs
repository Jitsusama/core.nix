use super::*;

fn refusals() -> Vec<String> {
    vec!["over the allowed cap".into()]
}

fn lines(entries: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend_from_slice(entry.as_bytes());
        bytes.push(b'\n');
    }
    bytes
}

fn turn(at: &str, read: u32, write: u32, long: u32) -> String {
    let cost = serde_json::json!({
        "input": 0.01, "output": 0.1, "cacheRead": 0.2, "cacheWrite": 1.0, "total": 1.31,
    });
    let usage = serde_json::json!({
        "input": 10, "output": 100, "cacheRead": read, "cacheWrite": write,
        "cacheWrite1h": long, "cost": cost,
    });
    let message = serde_json::json!({
        "role": "assistant", "provider": "anthropic", "model": "opus", "usage": usage,
    });
    serde_json::json!({ "type": "message", "timestamp": at, "message": message }).to_string()
}

const HEADER: &str = concat!(
    r#"{"type":"session","id":"s1","timestamp":"2026-10-14T10:00:00Z","#,
    r#""cwd":"/home/someone/src/github.com/me/thing"}"#
);

fn turns(events: &[Event]) -> Vec<&Turn> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Turn(turn) => Some(turn),
            _ => None,
        })
        .collect()
}

#[test]
fn a_torn_line_waits_for_its_end() {
    let mut bytes = lines(&[HEADER]);
    bytes.extend_from_slice(br#"{"type":"message","timest"#);
    let mut carry = Carry::default();
    let (events, used) = read(&bytes, &mut carry, &refusals());
    assert_eq!(events.len(), 1);
    assert_eq!(used, HEADER.len() + 1);
}

// pi carries a session into a new log by copying its entries, with
// their times, ahead of the new header; the parent's log still holds
// them, so only what comes after the header is the new log's own.
fn carried(parent: &str) -> String {
    serde_json::json!({
        "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
        "cwd": "/home/someone/elsewhere", "parentSession": parent,
    })
    .to_string()
}

#[test]
fn a_carried_log_counts_only_its_own_turns() {
    let parent = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let bytes = lines(&[
        &carried(parent),
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
        &turn("2026-10-14T10:31:00Z", 50_000, 2_000, 0),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    let turns = turns(&events);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].session, "s2");
    // The copy is still the context the first turn of its own read.
    assert_eq!(turns[0].gap, Some(31.0 * 60.0));
    assert_eq!(turns[0].cause, None);
}

// A copy whose parent's log is gone is the only record of its turns.
#[test]
fn a_copy_without_its_parent_keeps_every_turn() {
    let bytes = lines(&[
        &carried("/nowhere/2026_gone.jsonl"),
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
        &turn("2026-10-14T10:31:00Z", 50_000, 2_000, 0),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    assert_eq!(turns(&events).len(), 2);
}

#[test]
fn the_first_write_is_fresh_and_a_pause_rebuilds() {
    let bytes = lines(&[
        HEADER,
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
        &turn("2026-10-14T10:01:00Z", 50_000, 2_000, 0),
        // Ten minutes later, past the five-minute cache: all rebuilt.
        &turn("2026-10-14T10:11:00Z", 0, 52_000, 0),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    let turns = turns(&events);
    assert_eq!(turns[0].cause, Some(Cause::Fresh));
    assert_eq!(turns[1].cause, None, "growth alone isn't a rebuild");
    assert_eq!(turns[2].cause, Some(Cause::Gap));
    assert_eq!(turns[2].rebuilt, 52_000.0);
    let cost = turns[2].rebuilt_cost;
    assert!((cost - 1.0).abs() < 1e-9, "the rebuild cost {cost}");
}

#[test]
fn an_hour_long_cache_survives_a_ten_minute_pause() {
    let bytes = lines(&[
        HEADER,
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 50_000),
        &turn("2026-10-14T10:10:00Z", 0, 50_000, 50_000),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    assert_eq!(turns(&events)[1].cause, Some(Cause::Unknown));
}

#[test]
fn a_compaction_is_its_own_turn_and_cause() {
    let compaction = concat!(
        r#"{"type":"compaction","timestamp":"2026-10-14T10:01:30Z","tokensBefore":300000,"#,
        r#""usage":{"input":900,"output":11000,"cacheRead":300000,"cacheWrite":0,"#,
        r#""cost":{"total":0.3}}}"#
    );
    let bytes = lines(&[
        HEADER,
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
        compaction,
        &turn("2026-10-14T10:02:00Z", 0, 20_000, 0),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    let turns = turns(&events);
    assert_eq!(turns[1].kind, Kind::Compaction);
    assert_eq!(turns[2].cause, Some(Cause::Compaction));
}

#[test]
fn a_quest_names_the_turns_after_it() {
    let quest = concat!(
        r#"{"type":"custom","customType":"quest-workflow","#,
        r#""timestamp":"2026-10-14T10:00:30Z","data":{"questId":"QEST-1"}}"#
    );
    let bytes = lines(&[
        HEADER,
        &turn("2026-10-14T10:00:00Z", 0, 1, 0),
        quest,
        &turn("2026-10-14T10:01:00Z", 0, 1, 0),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&bytes, &mut carry, &refusals());
    let turns = turns(&events);
    assert_eq!(turns[0].quest, None);
    assert_eq!(turns[1].quest.as_deref(), Some("QEST-1"));
}

// An agent working in a quest's workspace serves that quest, until a
// quest entry says otherwise; its session-info names it.
#[test]
fn an_agent_in_a_workspace_serves_its_quest() {
    let header = concat!(
        r#"{"type":"session","id":"a1","timestamp":"2026-10-14T10:00:00Z","#,
        r#""cwd":"/w/QEST-2/hub/agents/acp@team/team"}"#
    );
    let named = r#"{"type":"session_info","name":"acp@team"}"#;
    let quest = concat!(
        r#"{"type":"custom","customType":"quest-workflow","#,
        r#""timestamp":"2026-10-14T10:00:30Z","data":{"questId":"QEST-1"}}"#
    );
    let bytes = lines(&[
        header,
        named,
        &turn("2026-10-14T10:00:10Z", 0, 1, 0),
        quest,
        &turn("2026-10-14T10:01:00Z", 0, 1, 0),
    ]);
    let mut carry = Carry {
        kind: Some("swarm".into()),
        workspaces: Some("/w/".into()),
        ..Carry::default()
    };
    let (events, _) = read(&bytes, &mut carry, &refusals());
    assert_eq!(
        events[0],
        Event::Session {
            id: "a1".into(),
            cwd: "/w/QEST-2/hub/agents/acp@team/team".into(),
            at: crate::time::parse("2026-10-14T10:00:00Z").unwrap(),
            kind: Some("swarm".into()),
        }
    );
    assert_eq!(
        events[1],
        Event::Named {
            session: "a1".into(),
            name: "acp@team".into(),
        }
    );
    let turns = turns(&events);
    assert_eq!(turns[0].quest.as_deref(), Some("QEST-2"));
    assert_eq!(turns[1].quest.as_deref(), Some("QEST-1"));
}

/// A turn in a log whose entries have ids, with its parent's.
fn tree_turn(id: &str, parent: &str, at: &str) -> String {
    let mut entry: Value = serde_json::from_str(&turn(at, 0, 1, 0)).unwrap();
    entry["id"] = id.into();
    entry["parentId"] = parent.into();
    entry.to_string()
}

fn budget(id: &str, parent: &str, name: Option<&str>) -> String {
    serde_json::json!({
        "type": "custom", "customType": "budget", "id": id, "parentId": parent,
        "timestamp": "2026-11-07T02:20:33Z",
        "data": {"v": 1, "name": name, "label": "Delta team token"},
    })
    .to_string()
}

fn budgets(events: &[Event]) -> Vec<Option<&str>> {
    turns(events)
        .iter()
        .map(|turn| turn.budget.as_deref())
        .collect()
}

// Before any entry a turn's log says nothing of its budget; after one,
// it draws on the budget named, or the default for none.
#[test]
fn a_budget_entry_names_the_turns_after_it() {
    let bytes = lines(&[
        HEADER,
        &tree_turn("a", "", "2026-11-07T02:00:00Z"),
        &budget("b", "a", Some("delta")),
        &tree_turn("c", "b", "2026-11-07T02:21:00Z"),
        &budget("d", "c", None),
        &tree_turn("e", "d", "2026-11-07T02:22:00Z"),
    ]);
    let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
    assert_eq!(budgets(&events), [None, Some("delta"), Some("")]);
}

// A branch taken from further back draws on what its parent did, not
// on the entry last written, even when the branch is read later, from
// the log itself: the carry kept between reads holds no entry's id
// but the last.
#[test]
fn a_branch_draws_on_its_own_ancestors() {
    let first = lines(&[
        HEADER,
        &budget("a", "", None),
        &tree_turn("b", "a", "2026-11-07T02:00:00Z"),
        &budget("c", "b", Some("delta")),
        &tree_turn("d", "c", "2026-11-07T02:21:00Z"),
        &budget("e", "d", Some("aquifer")),
        &tree_turn("f", "e", "2026-11-07T02:22:00Z"),
    ]);
    let mut carry = Carry::default();
    let (events, _) = read(&first, &mut carry, &refusals());
    assert_eq!(budgets(&events), [Some(""), Some("delta"), Some("aquifer")]);
    let kept = serde_json::to_string(&carry).unwrap();
    assert!(!kept.contains("\"c\"") && !kept.contains("\"d\""), "{kept}");
    let mut carry: Carry = serde_json::from_str(&kept).unwrap();
    let later = lines(&[
        &tree_turn("g", "d", "2026-11-07T02:30:00Z"),
        &tree_turn("h", "g", "2026-11-07T02:31:00Z"),
        &tree_turn("i", "b", "2026-11-07T02:32:00Z"),
    ]);
    let earlier = || ancestry(first.as_slice());
    let (events, _) = read_after(&later, &mut carry, &refusals(), &earlier);
    assert_eq!(budgets(&events), [Some("delta"), Some("delta"), Some("")]);
}

// A message's id and parent are read from the start of its line alone,
// and a line laid out otherwise, or a budget entry, is read whole.
#[test]
fn an_ancestry_reads_a_line_s_start_where_it_can() {
    let long = "x".repeat(4000);
    let message =
        format!(r#"{{"type":"message","id":"b","parentId":"a","message":{{"text":"{long}"}}}}"#);
    assert_eq!(head_ids(message.as_bytes()), Some(("b", Some("a"))));
    let first = r#"{"type":"model_change","id":"a","parentId":null}"#;
    assert_eq!(head_ids(first.as_bytes()), Some(("a", None)));
    let nested = r#"{"type":"x","message":{"id":"n"},"id":"c","parentId":"b"}"#.to_owned();
    assert_eq!(head_ids(nested.as_bytes()), None);
    let mut ancestry = Ancestry::default();
    for line in [
        first.to_owned(),
        message,
        budget("d", "b", Some("delta")),
        nested,
    ] {
        ancestry.learn(line.as_bytes());
    }
    assert_eq!(ancestry.budget_of("d").as_deref(), Some("delta"));
    assert_eq!(ancestry.budget_of("c"), None);
    assert_eq!(ancestry.parents.get("c").map(String::as_str), Some("b"));
}

// A branch from an entry no budget entry is above draws on the default
// once any budget has been named, and on nothing said before then.
#[test]
fn a_branch_from_above_every_budget_entry_draws_on_the_default() {
    let bytes = lines(&[
        HEADER,
        &tree_turn("a", "", "2026-11-07T02:00:00Z"),
        &budget("b", "a", Some("delta")),
        &tree_turn("c", "b", "2026-11-07T02:21:00Z"),
        &tree_turn("d", "a", "2026-11-07T02:22:00Z"),
    ]);
    let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
    assert_eq!(budgets(&events), [None, Some("delta"), Some("")]);
}

// A log without ids is followed in the file's order, and an entry of a
// version this doesn't read is passed over.
#[test]
fn a_budget_entry_of_another_version_is_passed_over() {
    let newer = budget("x", "", Some("delta")).replace("\"v\":1", "\"v\":2");
    let bytes = lines(&[
        HEADER,
        &newer,
        &turn("2026-11-07T02:00:00Z", 0, 1, 0),
        &budget("y", "", Some("aquifer")),
        &turn("2026-11-07T02:01:00Z", 0, 1, 0),
    ]);
    let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
    assert_eq!(budgets(&events), [None, Some("aquifer")]);
}

#[test]
fn a_workspace_s_quest_is_the_directory_under_it() {
    assert_eq!(
        workspace_quest("/w", "/w/QEST-2/a/b").as_deref(),
        Some("QEST-2")
    );
    assert_eq!(
        workspace_quest("/w/", "/w/QEST-2").as_deref(),
        Some("QEST-2")
    );
    assert_eq!(workspace_quest("/w", "/work/QEST-2"), None);
    assert_eq!(workspace_quest("/w", "/w"), None);
    assert_eq!(workspace_quest("/w", "/elsewhere"), None);
}

// A carry kept before it had more to say still reads, so a log under
// way isn't read again from its start.
#[test]
fn an_older_carry_still_reads() {
    let carry: Carry = serde_json::from_str(r#"{"session":"s1","quest":"QEST-1"}"#).unwrap();
    assert_eq!(carry.session.as_deref(), Some("s1"));
    assert_eq!(carry.quest.as_deref(), Some("QEST-1"));
}

#[test]
fn an_extensions_call_is_named() {
    let side = concat!(
        r#"{"type":"custom","customType":"compaction-selection-tags","#,
        r#""timestamp":"2026-10-14T10:00:30Z","data":{"model":"small","#,
        r#""usage":{"input":1000,"output":100,"cacheRead":0,"cacheWrite":0,"#,
        r#""cost":{"total":0.02}}}}"#
    );
    let mut carry = Carry::default();
    let (events, _) = read(&lines(&[HEADER, side]), &mut carry, &refusals());
    let turns = turns(&events);
    assert_eq!(turns[0].kind.name(), "side:compaction-selection-tags");
    assert_eq!(turns[0].model, "small");
}

#[test]
fn a_refusal_is_dated() {
    let refused = concat!(
        r#"{"type":"message","timestamp":"2026-08-01T12:21:12Z","#,
        r#""message":{"role":"assistant","model":"opus","stopReason":"error","#,
        r#""errorMessage":"401 Personal token usage is over the allowed cap.","#,
        r#""usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"cost":{"total":0}}}}"#
    );
    let mut carry = Carry::default();
    let (events, _) = read(&lines(&[HEADER, refused]), &mut carry, &refusals());
    assert!(
        matches!(&events[1], Event::Refusal { model, .. } if model == "opus"),
        "{:?}",
        events[1]
    );
    assert!(turns(&events).is_empty(), "a refusal costs nothing");
}

#[test]
fn carrying_on_reads_the_same_as_reading_once() {
    let bytes = lines(&[
        HEADER,
        &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
        &turn("2026-10-14T10:11:00Z", 0, 52_000, 0),
    ]);
    let mut once = Carry::default();
    let (whole, _) = read(&bytes, &mut once, &refusals());
    let split = HEADER.len() + 1 + turn("2026-10-14T10:00:00Z", 0, 50_000, 0).len() + 1;
    let mut carry = Carry::default();
    let (mut first, _) = read(&bytes[..split], &mut carry, &refusals());
    let saved: Carry = serde_json::from_str(&serde_json::to_string(&carry).unwrap()).unwrap();
    let mut carry = saved;
    let (rest, _) = read(&bytes[split..], &mut carry, &refusals());
    first.extend(rest);
    assert_eq!(first, whole);
    assert_eq!(carry, once);
}

#[test]
fn a_repository_is_named_by_where_it_lives() {
    assert_eq!(
        repo_of("/home/a/src/github.com/me/thing/sub"),
        "github.com/me/thing"
    );
    assert_eq!(
        repo_of("/home/a/big/trees/root/src/areas/tools/x"),
        "big/areas/tools/x"
    );
    assert_eq!(repo_of("/home/a/scratch"), "scratch");
}
