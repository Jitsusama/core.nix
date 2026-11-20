//! What the state keeps, case by case, in folders of the tests' own.

use super::*;

/// A state of the test's own. The tests run side by side, so a number
/// counted for each keeps two that chose the same name from emptying
/// each other's folder.
fn state(name: &str) -> State {
    static MADE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let made = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("merge-net-{name}-{}-{made}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let state = State::at(root);
    state.make().expect("made");
    state
}

#[test]
fn what_is_said_over_several_lines_is_one_line_of_the_log() {
    let state = state("said-once");
    state.say("couldn't revert abc: git failed: error: could not revert\nhint: after resolving\n");
    let log = fs::read_to_string(state.root.join("logs/net.log")).expect("the log");
    assert_eq!(
        log.lines().count(),
        1,
        "one thing said took several lines: {log:?}"
    );
    assert!(
        log.ends_with("could not revert / hint: after resolving\n"),
        "its lines weren't joined on: {log:?}"
    );
}

#[test]
fn logs_past_three_days_go_and_the_nets_own_and_newer_ones_stay() {
    let state = state("aged");
    let now = SystemTime::now();
    let written = |name: &str, age: Duration| {
        let path = state.root.join("logs").join(name);
        let file = fs::File::create(&path).expect("a log");
        file.set_modified(now - age).expect("its time");
        path
    };
    let day = Duration::from_hours(24);
    let old = written("aaaa-vm-a-1.log", 4 * day);
    let new = written("aaaa-vm-a-2.log", 2 * day);
    let own = written("net.log", 9 * day);
    assert_eq!(
        state.age_out_logs(now, LOGS_BUDGET),
        Aged { old: 1, over: 0 },
        "not one log went for its age alone"
    );
    assert!(!old.exists(), "a log four days old stayed");
    assert!(new.exists(), "a log two days old went");
    assert!(own.exists(), "the net's own log went");
}

#[test]
fn the_oldest_logs_past_the_budget_go_and_the_nets_own_stays() {
    let state = state("budget");
    let now = SystemTime::now();
    let written = |name: &str, hours: u64, bytes: usize| {
        log_aged(&state, now, name, Duration::from_hours(hours), bytes)
    };
    // The names sort otherwise than their ages, so a budget kept by
    // name keeps the wrong ones; the net's log is the newest and
    // largest, so counting it takes logs that fit.
    let newest = written("dddd-vm-a-1.log", 25, 40);
    let newer = written("aaaa-vm-b-1.log", 26, 40);
    let older = written("cccc-vm-c-1.log", 27, 40);
    let oldest = written("bbbb-vm-d-1.log", 28, 40);
    let aged = written("eeee-vm-e-1.log", 4 * 24, 1000);
    let own = written("net.log", 0, 400);
    assert_eq!(
        state.age_out_logs(now, 80),
        Aged { old: 1, over: 2 },
        "not the one log past three days and the two past a budget of 80 bytes went"
    );
    assert!(
        newest.exists() && newer.exists(),
        "a log within the budget went, or the budget counted the net's log or an aged one"
    );
    assert!(
        !older.exists() && !oldest.exists() && !aged.exists(),
        "a log past the budget or past three days stayed"
    );
    assert!(own.exists(), "the net's own log went for the budget");
}

#[test]
fn the_budget_leaves_the_logs_the_net_still_reads() {
    let state = state("still-read");
    let now = SystemTime::now();
    let (passed, seen, round) = ("1".repeat(40), "2".repeat(40), "3".repeat(40));
    write_json(
        &state.root.join("state.json"),
        &json!({"passed": passed, "seen": seen, "round": round}),
    )
    .expect("the heads");
    let written =
        |name: &str, minutes: u64| log_aged(&state, now, name, Duration::from_mins(minutes), 40);
    let day = 24 * 60;
    let base = written("111111111111-vm-a-1.log", 2 * day);
    let looked = written("222222222222-vm-b-1.log", 2 * day);
    let under_way = written("333333333333-vm-c-1.log", 2 * day);
    let young = written("444444444444-vm-d-1.log", day - 10);
    let other = written("555555555555-vm-e-1.log", day + 10);
    let newer = written("666666666666-vm-f-1.log", 2 * day);
    assert_eq!(
        state.age_out_logs(now, 0),
        Aged { old: 0, over: 2 },
        "not just the two logs the net no longer reads went for the budget"
    );
    assert!(
        base.exists() && looked.exists() && under_way.exists(),
        "a log of a head the net holds went for the budget"
    );
    assert!(young.exists(), "a log under a day old went for the budget");
    assert!(
        !other.exists() && !newer.exists(),
        "a log the net no longer reads stayed past the budget"
    );
}

/// A log `name` of `bytes` bytes, last written `age` before `now`.
fn log_aged(state: &State, now: SystemTime, name: &str, age: Duration, bytes: usize) -> PathBuf {
    let path = state.root.join("logs").join(name);
    fs::write(&path, vec![b'x'; bytes]).expect("a log");
    let file = fs::File::options()
        .write(true)
        .open(&path)
        .expect("the log");
    file.set_modified(now - age).expect("its time");
    path
}

#[test]
fn a_check_lasts_as_long_as_its_newest_pass() {
    let state = state("lasted");
    let log = state.log("aaaa", "vm-a", 1);
    let runs = [
        ("aaaa", "vm-a", Ran::Passed, Some(0), 600),
        ("aaaa", "vm-a", Ran::Failed, Some(1), 30),
        ("bbbb", "vm-a", Ran::Passed, Some(0), 500),
        ("bbbb", "vm-b", Ran::Stalled, None, 3000),
    ];
    for (commit, check, ran, code, seconds) in runs {
        state
            .record(commit, check, ran, code, &log, seconds)
            .unwrap();
    }
    let lasted = state.lasted();
    assert_eq!(
        lasted.get("vm-a"),
        Some(&500),
        "vm-a's newest pass took 500 seconds: {lasted:?}"
    );
    assert_eq!(
        lasted.get("vm-b"),
        None,
        "a stalled run says nothing of how long vm-b takes"
    );
}

// A failure a quarantine excuses counts as passed for blame, but it ran no
// further than its failure, and a count of passes in a row read from the
// record took 7 such runs of one call test as passes from 14 November.
#[test]
fn an_excused_failure_is_kept_apart_from_a_pass() {
    let state = state("excused");
    let log = state.log("aaaa", "vm-a", 1);
    state
        .record("aaaa", "vm-a", Ran::Passed, Some(0), &log, 600)
        .unwrap();
    state
        .record("bbbb", "vm-a", Ran::Passed, Some(1), &log, 40)
        .unwrap();
    state
        .record("cccc", "vm-a", Ran::Failed, Some(1), &log, 30)
        .unwrap();
    state
        .record("dddd", "vm-a", Ran::Stalled, None, &log, 3000)
        .unwrap();
    assert_eq!(
        state.lasted().get("vm-a"),
        Some(&600),
        "an excused failure says nothing of how long vm-a takes"
    );
    let record = fs::read_to_string(state.root.join("results.jsonl")).unwrap();
    let excused: Vec<bool> = record
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()["excused"] == true)
        .collect();
    assert_eq!(
        excused,
        [false, true, false, false],
        "only the pass that failed was excused, not a failure or a stall: {record}"
    );
    assert_eq!(
        state.runs("bbbb", "vm-a"),
        vec![Ran::Passed],
        "blame still takes it as passed"
    );
    fs::remove_dir_all(&state.root).unwrap();
}

#[test]
fn a_run_is_kept_by_its_commit_and_check() {
    let state = state("runs");
    let log = state.log("aaaa", "vm-a", 1);
    state
        .record("aaaa", "vm-a", Ran::Failed, Some(1), &log, 0)
        .unwrap();
    state
        .record("aaaa", "vm-a", Ran::Passed, Some(0), &log, 0)
        .unwrap();
    state
        .record("bbbb", "vm-a", Ran::Passed, Some(0), &log, 0)
        .unwrap();
    assert_eq!(state.runs("aaaa", "vm-a"), vec![Ran::Failed, Ran::Passed]);
    state
        .record("cccc", "vm-a", Ran::Stalled, None, &log, 0)
        .unwrap();
    assert_eq!(state.runs("cccc", "vm-a"), vec![Ran::Stalled]);
    assert_eq!(state.runs("aaaa", "vm-b"), vec![]);
    fs::remove_dir_all(&state.root).unwrap();
}

#[test]
fn runs_ending_together_are_each_kept_whole() {
    let state = state("together");
    let checks = ["vm-a", "vm-b", "vm-c", "vm-d"];
    std::thread::scope(|scope| {
        for check in checks {
            let state = &state;
            scope.spawn(move || {
                let log = state.log("aaaa", check, 1);
                for _ in 0..200 {
                    state
                        .record("aaaa", check, Ran::Passed, Some(0), &log, 0)
                        .unwrap();
                }
            });
        }
    });
    for check in checks {
        assert_eq!(state.runs("aaaa", check).len(), 200, "{check}'s runs");
    }
    fs::remove_dir_all(&state.root).unwrap();
}

#[test]
fn the_heads_and_the_hold_outlast_the_program() {
    let state = state("heads");
    assert_eq!(state.heads(), (None, None));
    state.set_heads("aaaa", "bbbb").unwrap();
    assert_eq!(
        State::at(state.root.clone()).heads(),
        (Some("aaaa".to_owned()), Some("bbbb".to_owned()))
    );
    state.set_hold(&json!({ "bad": "cccc" })).unwrap();
    assert_eq!(state.hold().unwrap()["bad"], "cccc");
    state
        .set_hold(&json!({ "bad": "cccc", "already_runs": true }))
        .unwrap();
    assert!(
        !state.hold_file().exists(),
        "a switch reads no hold on a merge the machine runs"
    );
    assert_eq!(state.hold().unwrap()["already_runs"], true);
    state.set_hold(&json!({ "bad": "dddd" })).unwrap();
    assert!(
        !state.already_runs_file().exists(),
        "one hold stands at a time"
    );
    state.clear_hold().unwrap();
    assert!(state.hold().is_none(), "a cleared hold still stands");
    state.clear_hold().unwrap();
    fs::remove_dir_all(&state.root).unwrap();
}

// cadence reads `since` to say when a hold has stopped the net for half an
// hour while main moves, as the conclude and Slack holds of 19 November
// did for over four hours, so it stays put while the net stays held,
// whatever holds it, and starts again after a gap.
#[test]
fn a_hold_keeps_when_it_began_while_it_stands() {
    let first = held_since(None, 1_000);
    assert_eq!(first, 1_000, "a hold newly seen begins now");
    let still = json!({ "at": 1_060, "why": "a reboot", "since": first });
    assert_eq!(
        held_since(Some(&still), 1_120),
        1_000,
        "a stop seen a minute on, by this hold or its rewording, began when it was first seen"
    );
    assert_eq!(
        held_since(Some(&still), 1_060 + HELD_GAP_S + 1),
        1_060 + HELD_GAP_S + 1,
        "a hold seen again after a gap begins again"
    );
    let older = json!({ "at": 1_060, "why": "a reboot" });
    assert_eq!(
        held_since(Some(&older), 1_120),
        1_120,
        "a held.json written before `since` begins the hold now"
    );
    let state = state("held-since");
    state.held("a reboot").unwrap();
    let written = read_json(&state.root.join("held.json")).unwrap();
    assert_eq!(
        written["since"], written["at"],
        "held.json lacks since: {written}"
    );
    fs::remove_dir_all(&state.root).unwrap();
}
