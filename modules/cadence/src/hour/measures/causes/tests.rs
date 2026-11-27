//! The causes measure: which cause lines and review records it reads,
//! how it folds a class's names, and when a class is over.

use std::fs;

use super::*;

mod withdrawals;

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
    write_as(&messages.join(format!("{}.json", m.id)), m, ago);
}

/// Writes `m` at `path`, dated written `ago` seconds before `NOW`, so
/// what reads by write time lists it whatever the clock says.
fn write_as(path: &Path, m: &Message, ago: i64) {
    fs::write(path, serde_json::to_vec(m).expect("a message serialised")).expect("written");
    let at = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(u64::try_from(NOW - ago).expect("a time after 1970"));
    fs::File::options()
        .write(true)
        .open(path)
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
}

#[test]
fn an_unfolded_class_keeps_only_its_senders_first_hundred_letters() {
    let long = "word ".repeat(60);
    assert_eq!(
        class_named(&plain(&long)).chars().count(),
        folds::UNFOLDED.len() + UNFOLDED_AT_MOST - 1,
        "an unfolded class kept a sender's words at any length"
    );
}

/// One line can't keep a cause of any length, nor count changes without
/// end: a case keeps its first `SAID_AT_MOST` letters, and a line its
/// first `CHANGES_PER_LINE` changes.
#[test]
fn a_line_keeps_only_so_much_of_its_cause_and_so_many_changes() {
    let named: Vec<String> = (1..=CHANGES_PER_LINE + 5)
        .map(|n| format!("core.nix#{n}"))
        .collect();
    let line = format!(
        "cause: {} | change: {}",
        "word ".repeat(SAID_AT_MOST),
        named.join(", ")
    );
    let cases = cause_line(&line, NOW);
    assert_eq!(
        cases
            .iter()
            .map(|case| case.change.as_str())
            .collect::<Vec<_>>(),
        named[..CHANGES_PER_LINE],
        "a line counted changes past its cap, or not its first ones"
    );
    assert!(
        cases
            .iter()
            .all(|case| case.said.chars().count() <= SAID_AT_MOST),
        "a case kept its cause at any length: {} letters",
        cases[0].said.chars().count()
    );
}

#[test]
fn a_change_is_read_by_its_repository_however_it_is_written() {
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
    assert_eq!(
        changes("core.nix#1, mycelium #8, plus #9"),
        ["core.nix#1"],
        "a number after a repository unknown was counted in the one before it"
    );
    let long = format!("{}#3", "r".repeat(REPO_AT_MOST + 1));
    assert_eq!(
        changes(&format!(
            "x`y\"z#1, a/b:c#2, {long}, core.nix#4\tcore.nix#5"
        )),
        ["core.nix#4", "core.nix#5"],
        "a change in a repository no host could name was counted, or a tab didn't part two"
    );
}

/// A class is written plain: its quotes and backticks, which would close
/// the quotes around it in an ask or end the span its command is in, are
/// apostrophes, which the command quotes for the shell.
#[test]
fn a_class_is_written_plain_with_its_quotes_made_apostrophes() {
    assert_eq!(
        plain("x \"y\" `z` \u{2019}w"),
        "x 'y' 'z' 'w",
        "a quote or backtick in a sender's words was kept as written"
    );
}

/// One message of many junk wordings, each on three changes, counts only
/// its first few unfolded classes, and an hour names only its first
/// several classes, saying how many more are over to the keeper alone.
#[test]
fn one_message_or_one_hour_puts_only_so_many_classes_to_their_owners() {
    let text: String = (0..UNFOLDED_PER_MESSAGE + 3)
        .map(|n| format!("cause: junk {n} | change: core.nix#1, #2, #3"))
        .collect::<Vec<_>>()
        .join("\n");
    let cases = said(&[heard(sent(&text, 60, "@1JQPPC"))]);
    let classes: BTreeSet<&str> = cases.iter().map(|c| c.class.as_str()).collect();
    assert_eq!(
        (classes.len(), cases.len()),
        (UNFOLDED_PER_MESSAGE, UNFOLDED_PER_MESSAGE * 3),
        "one message counted unfolded classes past its share, or one let in lost a change: \
         {classes:?}"
    );
    let cases: Vec<Case> = (0..NAMED_AT_MOST + 2)
        .flat_map(|n| (1..=3).map(move |c| case(&format!("class {n:02}"), &format!("r#{c}"), 60)))
        .collect();
    let m = recurring(&cases, NOW);
    assert_eq!(
        m.now,
        count(NAMED_AT_MOST + 2),
        "the classes an hour didn't name weren't counted"
    );
    let named: Vec<&str> = m.behind.iter().map(|l| class_of(l)).collect();
    assert!(
        named.len() == NAMED_AT_MOST + 1
            && named.first() == Some(&MORE)
            && names_no_class(&m.behind[0])
            && m.behind[0].contains("2 more classes"),
        "an hour named classes past its share, or didn't say the rest: {:?}",
        m.behind
    );
}

/// A burst of unfolded wordings, which any cell can make, can't push a
/// vetted class out of those an hour names, though theirs sort first.
#[test]
fn a_vetted_class_is_named_ahead_of_unfolded_ones() {
    let vetted = "a vetted class";
    let mut cases: Vec<Case> = (0..=NAMED_AT_MOST)
        .flat_map(|n| {
            (1..=3).map(move |c| {
                case(
                    &format!("{}junk {n:02}", folds::UNFOLDED),
                    &format!("r#{c}"),
                    60,
                )
            })
        })
        .collect();
    cases.extend((1..=3).map(|c| case(vetted, &format!("r#{c}"), 60)));
    let m = recurring(&cases, NOW);
    let named: Vec<&str> = m.behind.iter().map(|l| class_of(l)).collect();
    assert!(
        named.get(1) == Some(&vetted) && named.len() == NAMED_AT_MOST + 1,
        "unfolded wordings pushed a vetted class out of those named, or past its place: {:?}",
        m.behind
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

/// A class of cause sorts before how fast things ship and how clean the
/// code is, so only its filter keeps it from leading the hour, though it
/// is filed with its own owners apart.
#[test]
fn a_cause_found_again_and_again_never_leads_what_comes_after_it() {
    let cases: Vec<Case> = (1..=3)
        .map(|c| case("stale docs", &format!("core.nix#{c}"), 60))
        .collect();
    let far = Measure {
        key: "far".to_owned(),
        class: Class::Clean,
        says: "far".to_owned(),
        now: 30.0,
        target: Some(10.0),
        owner: "OWNER".to_owned(),
        behind: Vec::new(),
    };
    let measures = [recurring(&cases, NOW), far];
    assert_eq!(
        super::super::drag(&measures).map(|m| m.key.as_str()),
        Some("far"),
        "a cause filed with its own owners apart led the hour"
    );
}

#[test]
fn a_class_owned_apart_by_repository_goes_to_each_repositorys_owner() {
    let cases = [
        case(TEST_PASSES_UNDONE, "core.nix#1", 100),
        case(TEST_PASSES_UNDONE, "core.nix#2", 100),
        case(TEST_PASSES_UNDONE, "core.nix#8", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#3", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#9", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#10", 100),
        case(TEST_PASSES_UNDONE, "mycelium#4", 100),
        case(TEST_PASSES_UNDONE, "world#11", 100),
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
    let (harness, core, rest) = (
        half("agentic-harness.pi"),
        half("core.nix"),
        half("other repositories"),
    );
    assert_eq!(
        owners,
        [
            (harness.as_str(), "85PKB2"),
            (core.as_str(), "80ECO2"),
            (rest.as_str(), "1JQPPC"),
            (PIN_LAGS, "16V6F4"),
        ],
        "a class wasn't split by repository to each owner over its bar, the rest to the \
         class's owner, or one owned whole was split"
    );
    assert!(
        m.behind[1]
            .contains("on 3 of its 8 changes in a week, core.nix#1, core.nix#2, core.nix#8 ·"),
        "a repository's part didn't name its own changes: {}",
        m.behind[1]
    );
    assert!(
        m.behind[2].contains("on 2 of its 8 changes in a week, mycelium#4, world#11 ·"),
        "the repositories under the bar weren't named together: {}",
        m.behind[2]
    );
}

/// On 27 November world was asked apart on 1 of a class's 55 changes. A
/// repository under the bar is asked of only with the rest, and a class
/// none of whose repositories is over it is asked of whole, so three
/// repositories on two changes each still reach the class's owner.
#[test]
fn a_repository_under_the_bar_is_asked_of_with_the_rest_not_apart() {
    let cases = [
        case(TEST_PASSES_UNDONE, "core.nix#1", 100),
        case(TEST_PASSES_UNDONE, "core.nix#2", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#3", 100),
        case(TEST_PASSES_UNDONE, "agentic-harness.pi#4", 100),
        case(TEST_PASSES_UNDONE, "world#5", 100),
        case(TEST_PASSES_UNDONE, "world#6", 100),
    ];
    let m = recurring(&cases, NOW);
    assert_eq!(
        m.behind,
        [format!(
            "{TEST_PASSES_UNDONE}: on 6 changes in a week, agentic-harness.pi#3, \
             agentic-harness.pi#4, core.nix#1, core.nix#2, world#5, world#6 · 1JQPPC"
        )],
        "a class none of whose repositories is over the bar wasn't asked of whole"
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
    // A file named for one message whose body claims the sender's own: a
    // check that the file agrees with itself, since one that agrees with
    // itself passes whoever wrote it.
    let mut forged = sent("not mine either", -3650, "@1JQPPC");
    forged.id = format!("{}-SENDER-1-0000000c", (NOW + 3650) * 1000);
    forged.supersedes = Some(third.clone());
    let named = messages.join(format!("{}-STRANGER-1-0000000c.json", (NOW + 3650) * 1000));
    write_as(&named, &forged, -3650);
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
        withdrawn: BTreeSet::new(),
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

/// An hour that can't keep what it read says so on the card, naming no
/// class, and first, above the classes over the bar, since the card
/// shows only its first few lines; so the week read again each hour
/// isn't silent.
#[test]
fn what_couldnt_be_kept_is_said_and_names_no_class() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let line = sent(
        "cause: flaky gate | change: core.nix#1, #2, #3",
        600,
        "@1JQPPC",
    );
    write(&messages, &line, 600);
    let gone = folder.path().join("gone");
    let m = read(home.path(), &gone, NOW, true);
    assert!(
        m.past().is_some()
            && m.behind.len() == 2
            && class_of(&m.behind[0]) == UNKEPT
            && names_no_class(&m.behind[0])
            && !names_no_class(&m.behind[1]),
        "a reading that couldn't be kept went unsaid, or below a class, or was counted as \
         one: {:?}",
        m.behind
    );
    let kept = read(home.path(), folder.path(), NOW, true);
    assert!(
        kept.behind.len() == 1 && !names_no_class(&kept.behind[0]),
        "a reading kept whole said it wasn't: {:?}",
        kept.behind
    );
}

/// A kept reading that doesn't parse is unread, as symbiosis's is, since
/// read as none the cases only it holds would be lost; one of another
/// version reads the mesh's week anew.
#[test]
fn a_kept_reading_that_doesnt_parse_is_unread_and_another_version_is_read_anew() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let messages = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the messages folder");
    let kept_at = folder.path().join("causes.json");
    fs::write(&kept_at, "{not json").expect("a torn reading");
    let torn = read(home.path(), folder.path(), NOW, true);
    assert!(
        torn.past().is_some() && torn.behind.len() == 1 && class_of(&torn.behind[0]) == UNREAD,
        "a kept reading that doesn't parse passed as none: {:?}",
        torn.behind
    );
    assert!(
        torn.behind[0].contains("moved aside, the week would be read anew, forgetting"),
        "a torn reading didn't say what moving it aside would cost: {}",
        torn.behind[0]
    );
    // An old reading over the bar on its own, so one read as kept would
    // count it; read anew, the empty mesh counts nothing.
    let old_cases: Vec<String> = (1..=AT_LEAST)
        .map(|n| {
            format!(
                r#"{{"class":"stale docs","said":"stale docs","change":"core.nix#{n}","at":{}}}"#,
                NOW - 100
            )
        })
        .collect();
    let old_reading = format!(
        r#"{{"v":1,"through_ms":{},"cases":[{}]}}"#,
        (NOW - 100) * 1000,
        old_cases.join(",")
    );
    fs::write(&kept_at, old_reading).expect("an old reading");
    let old = read(home.path(), folder.path(), NOW, true);
    assert!(
        old.past().is_none() && old.behind.is_empty(),
        "a kept reading of another version was counted rather than read anew: {:?}",
        old.behind
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

/// A review record that can't be read is unread, not empty, or the classes
/// only it holds up would close and be asked again the next hour; one not
/// written yet is no review causes.
#[test]
fn a_review_record_that_cant_be_read_is_unread_and_a_missing_one_is_none() {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = tempfile::tempdir().expect("a temporary folder");
    let root = home.path().join("pi/agentic-harness.pi");
    fs::create_dir_all(root.join("mesh-workflow/messages")).expect("the messages folder");
    let missing = read(home.path(), folder.path(), NOW, false);
    assert!(
        missing.past().is_none() && missing.behind.is_empty(),
        "no review record yet read as unread: {:?}",
        missing.behind
    );
    // A folder where the record goes can't be read as a file.
    fs::create_dir_all(root.join("review/causes.jsonl")).expect("a folder in its place");
    let m = read(home.path(), folder.path(), NOW, false);
    assert!(
        m.past().is_some() && m.behind.len() == 1 && class_of(&m.behind[0]) == "unread",
        "a review record that couldn't be read passed as no review causes: {:?}",
        m.behind
    );
}
