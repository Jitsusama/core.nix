//! The causes measure: which cause lines and review records it reads,
//! how it folds a class's names, and when a class is over.

use std::fs;

use super::*;

const NOW: i64 = 1_795_700_000;

fn sent(text: &str, ago: i64, to: &str) -> Message {
    Message {
        v: 1,
        id: format!("{}-SENDER-1-0000000a", (NOW - ago) * 1000),
        at: String::new(),
        from: "SENDER".to_owned(),
        to: vec![to.to_owned()],
        intent: "finding".to_owned(),
        text: text.to_owned(),
        pid: 1,
        re: None,
        supersedes: None,
        ends_hold: false,
    }
}

fn case(class: &str, change: &str, ago: i64) -> Case {
    Case {
        class: class.to_owned(),
        change: change.to_owned(),
        at: NOW - ago,
    }
}

/// On 26 November cells sent the loop's keeper lines like these, several
/// in one finding, one naming two changes.
#[test]
fn each_cause_line_sent_to_the_keeper_is_a_case_for_each_change_it_names() {
    let text = "Three causes from today's rounds:\n\
        - cause: test passes with the fix undone | change: core.nix#6310, #6311 | run: \
          council-x | severity: medium | state: fixed in #6312\n\
        cause: Docs Claim More Than The Code. | change: Jitsusama/agentic-harness.pi#2123 | \
          run: none | severity: minor | state: open\n\
        not a cause: here | change: core.nix#1";
    let cases = said(&[sent(text, 60, "@1JQPPC")]);
    let read: Vec<(&str, &str)> = cases
        .iter()
        .map(|c| (c.class.as_str(), c.change.as_str()))
        .collect();
    assert_eq!(
        read,
        [
            ("test passes with the fix undone", "core.nix#6310"),
            ("test passes with the fix undone", "core.nix#6311"),
            ("docs claim more than the code", "agentic-harness.pi#2123"),
        ],
        "a cause line wasn't read as a case per change, or a line that isn't one was"
    );
    assert!(
        cases.iter().all(|c| c.at == NOW - 60),
        "a case isn't dated when its message was: {cases:?}"
    );
    assert!(
        said(&[sent(text, 60, "@SOMEONE")]).is_empty(),
        "a cause line sent to someone other than the keeper was counted"
    );
}

#[test]
fn a_class_by_another_name_is_counted_as_one() {
    assert_eq!(
        folded("  A fix with no test that fails  when it is undone. "),
        "test passes with the fix undone",
        "an alias wasn't folded to the name it counts under"
    );
    assert_eq!(
        changes("harness#2123 and core#6310, #6311"),
        ["agentic-harness.pi#2123", "core.nix#6310", "core.nix#6311"],
        "a short repository name or a bare number wasn't read as its change"
    );
    assert!(
        changes("#6310").is_empty(),
        "a number with no repository before it was read as a change"
    );
}

#[test]
fn a_class_on_three_changes_in_a_week_is_over_and_names_its_owner() {
    let cases = [
        case("stale docs", "core.nix#1", 100),
        case("stale docs", "core.nix#1", 200),
        case("stale docs", "core.nix#2", 300),
        case("stale docs", "agentic-harness.pi#3", 400),
        case("racy test", "core.nix#4", 100),
        case("racy test", "core.nix#5", 100),
        case("racy test", "core.nix#6", WITHIN + 60),
    ];
    let m = recurring(&cases, NOW);
    assert_eq!(
        (m.now, m.target),
        (1.0, Some(0.0)),
        "one class on three changes wasn't counted once, or one with a case past the week was"
    );
    assert_eq!(
        m.behind,
        [concat!(
            "stale docs: on 3 changes in a week, ",
            "agentic-harness.pi#3, core.nix#1, core.nix#2 · 1JQPPC"
        )],
        "the class over wasn't named with its changes and the keeper as its owner"
    );
    assert_eq!(
        class_of(&m.behind[0]),
        "stale docs",
        "its line didn't give back the class it is keyed by"
    );
}

#[test]
fn a_review_record_is_dated_by_its_run() {
    let text = [
        r#"{"cause":"racy test","change":"Jitsusama/core.nix#7","#,
        r#""run":"council-20261124T142609723-000001","severity":"minor","#,
        r#""findings":["F1"],"machine":"m"}"#,
        "\n",
        r#"{"cause":"racy test","change":"core.nix#8","#,
        r#""run":["judge-20261124T150000000-000002"]}"#,
        "\n",
        r#"{"cause":"racy test","change":"core.nix#9"}"#,
        "\nnot json",
    ]
    .concat();
    let text = text.as_str();
    let cases = reviewed(text);
    assert_eq!(
        cases,
        [
            Case {
                class: "racy test".to_owned(),
                change: "core.nix#7".to_owned(),
                at: 1_795_530_369,
            },
            Case {
                class: "racy test".to_owned(),
                change: "core.nix#8".to_owned(),
                at: 1_795_532_400,
            },
        ],
        "a record wasn't dated by its run's id, or one with no time was kept"
    );
}

#[test]
fn what_the_mesh_said_is_kept_for_the_next_hour_and_a_look_keeps_nothing() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    for n in 1..=3 {
        let line = format!("cause: flaky gate | change: core.nix#{n} | run: none");
        let mut m = sent(&line, 600, "@1JQPPC");
        m.id = format!("{}-SENDER-1-0000000{n}", (NOW - 600) * 1000);
        let body = serde_json::to_vec(&m).expect("a message serialised");
        fs::write(messages.join(format!("{}.json", m.id)), body).expect("a message written");
    }
    let look = read(home.path(), folder.path(), NOW, false);
    assert!(
        !folder.path().join("causes.json").exists(),
        "a look by hand kept a reading"
    );
    assert_eq!(
        look.now, 1.0,
        "the look didn't read the mesh: {:?}",
        look.behind
    );
    let hour = read(home.path(), folder.path(), NOW, true);
    assert_eq!(
        hour.now, 1.0,
        "the hour didn't read the mesh: {:?}",
        hour.behind
    );
    fs::remove_dir_all(&messages).expect("the mesh lets its messages go");
    fs::create_dir_all(&messages).expect("an empty messages folder");
    let later = read(home.path(), folder.path(), NOW + 3600, true);
    assert_eq!(
        later.now, 1.0,
        "the cases read before were lost once the mesh let their messages go"
    );
}

#[test]
fn with_no_mesh_to_read_it_is_past_its_target_for_the_keeper() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let m = read(home.path(), folder.path(), NOW, true);
    assert!(
        m.past().is_some() && class_of(&m.behind[0]) == "unread" && m.behind[0].ends_with("1JQPPC"),
        "no mesh to read passed as no cause recurring: {:?}",
        m.behind
    );
}
