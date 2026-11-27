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

/// `m` as this machine wrote it, as the sender stamped it.
fn heard(m: Message) -> (Message, i64) {
    let written = stamped_at(&m.id).expect("a stamped id");
    (m, written)
}

/// Writes `m` into `messages` as the harness does, under its id, dated as
/// this machine wrote it `ago` seconds before `NOW`.
fn write(messages: &Path, m: &Message, ago: i64) {
    let path = messages.join(format!("{}.json", m.id));
    fs::write(&path, serde_json::to_vec(m).expect("a message serialised")).expect("written");
    let at = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(u64::try_from(NOW - ago).expect("a time after 1970"));
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(at))
        .expect("its time written set");
}

fn case(class: &str, change: &str, ago: i64) -> Case {
    Case {
        class: class.to_owned(),
        said: class.to_owned(),
        change: change.to_owned(),
        at: NOW - ago,
        said_in: None,
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
        not a cause: here | change: core.nix#1\n\
        re: x | cause: test passes undone | change: core.nix#2";
    let cases = said(&[heard(sent(text, 60, "@1JQPPC"))]);
    let read: Vec<(&str, &str)> = cases
        .iter()
        .map(|c| (c.class.as_str(), c.change.as_str()))
        .collect();
    assert_eq!(
        read,
        [
            (TEST_PASSES_UNDONE, "core.nix#6310"),
            (TEST_PASSES_UNDONE, "core.nix#6311"),
            (folds::WORDS_DIFFER, "agentic-harness.pi#2123"),
        ],
        "a cause line wasn't read as a case per change, or a line that isn't one was"
    );
    let id = format!("{}-SENDER-1-0000000a", (NOW - 60) * 1000);
    assert!(
        cases
            .iter()
            .all(|c| c.at == NOW - 60 && c.said_in.as_deref() == Some(id.as_str())),
        "a case isn't dated when its message was, or doesn't name it: {cases:?}"
    );
    assert!(
        said(&[heard(sent(text, 60, "@SOMEONE"))]).is_empty(),
        "a cause line sent to someone other than the keeper was counted"
    );
    let ahead = sent("cause: racy | change: core.nix#3", -DAY * 60, "@1JQPPC");
    let ahead = said(&[(ahead, (NOW - 60) * 1000)]);
    assert_eq!(
        ahead.iter().map(|c| c.at).collect::<Vec<_>>(),
        [NOW - 60],
        "a line stamped months ahead wasn't dated when it was written here"
    );
}

#[test]
fn a_class_by_another_name_is_counted_as_one() {
    assert_eq!(
        class_named(&plain(
            "  A fix with no test that fails  when it is undone. "
        )),
        TEST_PASSES_UNDONE,
        "an alias wasn't folded to the name it counts under"
    );
    let long = "word ".repeat(60);
    assert_eq!(
        class_named(&plain(&long)).chars().count(),
        folds::UNFOLDED.len() + UNFOLDED_AT_MOST - 1,
        "an unfolded class kept a sender's words at any length"
    );
    assert_eq!(
        changes("harness#2123 and core#6310, #6311"),
        ["agentic-harness.pi#2123", "core.nix#6310", "core.nix#6311"],
        "a short repository name or a bare number wasn't read as its change"
    );
    assert_eq!(
        changes("core #6310 and #6311, agentic-harness#7 and Jitsusama/mycelium #8"),
        [
            "core.nix#6310",
            "core.nix#6311",
            "agentic-harness.pi#7",
            "mycelium#8"
        ],
        "a repository written apart from its number, or by another name, wasn't read"
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
fn a_class_owned_apart_by_repository_goes_to_each_repositorys_owner() {
    let cases = [
        case(TEST_PASSES_UNDONE, "core.nix#1", 100),
        case(TEST_PASSES_UNDONE, "core.nix#2", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#3", 100),
        case(TEST_PASSES_UNDONE, "mycelium#4", 100),
        case("a pinned copy lags the tree", "core.nix#5", 100),
        case("a pinned copy lags the tree", "core.nix#6", 100),
        case("a pinned copy lags the tree", "mycelium#7", 100),
    ];
    let m = recurring(&cases, NOW);
    assert_eq!(
        m.now, 2.0,
        "a class split by repository counted more than once"
    );
    let owners: Vec<(&str, &str)> = m
        .behind
        .iter()
        .filter_map(|line| line.rsplit_once(" · "))
        .map(|(line, owner)| (class_of(line), owner))
        .collect();
    let half = |repo: &str| format!("{TEST_PASSES_UNDONE} in {repo}");
    let (harness, core, mycelium) = (
        half("agentic-harness.pi"),
        half("core.nix"),
        half("mycelium"),
    );
    assert_eq!(
        owners,
        [
            (harness.as_str(), "85PKB2"),
            (core.as_str(), "80ECO2"),
            (mycelium.as_str(), "1JQPPC"),
            (PIN_LAGS, "16V6F4"),
        ],
        "a class wasn't split by repository to each owner, or one owned whole was split"
    );
    assert!(
        m.behind[1].contains("on 2 of its 4 changes in a week, core.nix#1, core.nix#2 ·"),
        "a repository's half didn't name its own changes: {}",
        m.behind[1]
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
                class: "(unfolded) racy test".to_owned(),
                said: "racy test".to_owned(),
                change: "core.nix#7".to_owned(),
                at: 1_795_530_369,
                said_in: None,
            },
            Case {
                class: "(unfolded) racy test".to_owned(),
                said: "racy test".to_owned(),
                change: "core.nix#8".to_owned(),
                at: 1_795_532_400,
                said_in: None,
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
        write(&messages, &m, 600);
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
    let third = format!("{}-SENDER-1-00000003", (NOW - 600) * 1000);
    let mut stranger = sent("not mine to withdraw", -3650, "@1JQPPC");
    stranger.from = "STRANGER".to_owned();
    stranger.id = format!("{}-STRANGER-1-0000000b", (NOW + 3650) * 1000);
    stranger.supersedes = Some(third.clone());
    write(&messages, &stranger, -3650);
    // A file named for one message whose body claims the sender's own.
    let mut forged = sent("not mine either", -3650, "@1JQPPC");
    forged.id = format!("{}-SENDER-1-0000000c", (NOW + 3650) * 1000);
    forged.supersedes = Some(third.clone());
    let named = messages.join(format!("{}-STRANGER-1-0000000c.json", (NOW + 3650) * 1000));
    fs::write(named, serde_json::to_vec(&forged).expect("serialised")).expect("written");
    let by_stranger = read(home.path(), folder.path(), NOW + 3660, false);
    assert_eq!(
        by_stranger.now, 1.0,
        "another sender's word, or a file naming another message, withdrew a line: {:?}",
        by_stranger.behind
    );
    let mut withdraws = sent("no cause here after all", -3700, "@1JQPPC");
    withdraws.supersedes = Some(third);
    write(&messages, &withdraws, -3700);
    let after = read(home.path(), folder.path(), NOW + 7200, true);
    assert_eq!(
        after.now, 0.0,
        "a line its sender withdrew still counted: {:?}",
        after.behind
    );
    fs::remove_dir_all(&messages).expect("the mesh lets the withdrawal go");
    fs::create_dir_all(&messages).expect("an empty messages folder");
    let next = read(home.path(), folder.path(), NOW + 3 * 3600, true);
    assert_eq!(
        next.now, 0.0,
        "a withdrawn line came back once its withdrawal was let go: {:?}",
        next.behind
    );
}

/// A cell whose clock runs slow, or a message relayed late, writes a
/// line here stamped before the last reading, which still counts.
#[test]
fn a_line_written_here_after_the_last_reading_counts_whatever_its_stamp() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let first = read(home.path(), folder.path(), NOW, true);
    assert_eq!(first.now, 0.0, "an empty mesh read as a cause recurring");
    for n in 1..=3 {
        let line = format!("cause: flaky gate | change: core.nix#{n}");
        let mut m = sent(&line, 3 * 3600, "@1JQPPC");
        m.id = format!("{}-SENDER-1-0000000{n}", (NOW - 3 * 3600) * 1000);
        write(&messages, &m, -600);
    }
    let later = read(home.path(), folder.path(), NOW + 3600, true);
    assert_eq!(
        later.now, 1.0,
        "lines stamped before the last reading but written after it were lost: {:?}",
        later.behind
    );
}

/// A case kept under one fold is counted under the class its wording
/// folds into now, so a pattern changed never splits one class in two.
#[test]
fn a_kept_case_is_folded_again_by_its_wording() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let cases: Vec<Case> = (1..=3)
        .map(|n| Case {
            class: "(unfolded) a flaky gate".to_owned(),
            ..case("a flaky gate", &format!("core.nix#{n}"), 600)
        })
        .collect();
    let kept = Kept {
        v: VERSION,
        through_ms: (NOW - 600) * 1000,
        cases,
    };
    let text = serde_json::to_string(&kept).expect("serialised");
    fs::write(folder.path().join("causes.json"), text).expect("kept");
    let m = read(home.path(), folder.path(), NOW, false);
    assert_eq!(
        m.behind.iter().map(|l| class_of(l)).collect::<Vec<_>>(),
        [TIMING],
        "a kept case was counted under the fold it was kept with"
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
