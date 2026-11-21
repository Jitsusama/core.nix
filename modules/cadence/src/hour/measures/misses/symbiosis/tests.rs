use std::time::{Duration, UNIX_EPOCH};

use super::super::{Named, named_in};
use super::*;

/// 12:00 UTC on Sat 21 November 2026, in seconds.
const NOW: i64 = 1_795_262_400;

/// A message `from` sent `ago` seconds before `NOW` to `to`, as `intent`.
fn sent(ago: i64, from: &str, intent: &str, to: &[&str], text: &str) -> Posted {
    Posted {
        id: format!("{}-{from}-1-{ago}", (NOW - ago) * 1000),
        from: from.to_owned(),
        to: to.iter().map(|to| (*to).to_owned()).collect(),
        intent: intent.to_owned(),
        text: text.to_owned(),
        ..Posted::default()
    }
}

/// A finding `from` sent to the group `ago` seconds before `NOW`.
fn finding(ago: i64, from: &str, text: &str) -> Posted {
    sent(ago, from, "finding", &[GROUP], text)
}

/// `one`, answering `re`.
fn answering(one: Posted, re: &Posted) -> Posted {
    Posted {
        re: Some(re.id.clone()),
        ..one
    }
}

/// An fyi `from` sent to the group `ago` seconds before `NOW`, answering `re`.
fn answer(ago: i64, from: &str, text: &str, re: &Posted) -> Posted {
    answering(sent(ago, from, "fyi", &[GROUP], text), re)
}

/// Every address the tests' messages are sent from.
fn known() -> BTreeSet<String> {
    ["IKQ63G", "CIJMZQ", "A2VLJN", "WCJQSG", "OSPHT4", "KP0L6A"]
        .map(str::to_owned)
        .into()
}

#[test]
fn a_finding_counts_against_the_owner_its_opening_names() {
    let cases = [
        // Each as posted on 20 and 21 November.
        (
            "IKQ63G finding for A2VLJN and WCJQSG: the pi host",
            "A2VLJN",
        ),
        ("CIJMZQ, for WCJQSG: the laptop doesn't resolve", "WCJQSG"),
        (
            "IKQ63G, for OSPHT4's spawn work (01793639): a host",
            "OSPHT4",
        ),
        ("IKQ63G finding for WCJQSG (mesh owner): a cell", "WCJQSG"),
        ("For KP0L6A: the node sleeps unsaid", "KP0L6A"),
        // None is named by "for the owner of", a word no cell sends as,
        // a "for" past the colon, or an opening with no colon at all.
        ("IKQ63G, for the owner of the lender lines: x", "P9R1KI"),
        ("IKQ63G finding for GITHUB auth: tokens lapse", "P9R1KI"),
        (
            "IKQ63G: since the reset, spawns time out for CWUEGJ",
            "P9R1KI",
        ),
        ("spawns time out for CIJMZQ since the reset", "P9R1KI"),
    ];
    for (text, owner) in cases {
        assert_eq!(
            super::owner(&finding(60, "IKQ63G", text), "P9R1KI", &known()),
            owner,
            "{text:?} counted against the wrong owner"
        );
    }
    let owner_of = |from: &str, to: &[&str]| {
        super::owner(
            &sent(60, from, "finding", to, "on the slots"),
            "P9R1KI",
            &known(),
        )
    };
    assert_eq!(
        owner_of("CIJMZQ", &["@IKQ63G", GROUP]),
        "IKQ63G",
        "a finding naming no owner wasn't counted against the address it went to"
    );
    assert_eq!(
        owner_of("IKQ63G", &["@IKQ63G", GROUP]),
        "P9R1KI",
        "a finding counted against its own sender"
    );
    assert_eq!(
        owner_of("CIJMZQ", &["@not one", GROUP]),
        "P9R1KI",
        "a finding counted against what isn't an address"
    );
}

#[test]
fn a_follow_up_adds_to_an_open_finding_and_its_latest_landing_note_is_its_check() {
    let first = finding(9_000, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let more = answering(finding(8_900, "CIJMZQ", "CIJMZQ, two more cases"), &first);
    let talk = answer(8_800, "WCJQSG", "that's core #5928's, I think", &first);
    let caused = answer(8_700, "CIJMZQ", "this broke when core #5900 landed", &first);
    let landed = answer(8_600, "A2VLJN", "A2VLJN landed core #6000.", &more);
    let again = answer(
        8_500,
        "A2VLJN",
        "Landed core #6010 instead.\nIt replaces it.",
        &first,
    );
    let back = answering(finding(8_400, "CIJMZQ", "CIJMZQ: it's back"), &first);
    let note = sent(8_300, "WCJQSG", "fyi", &[GROUP], "rolled out harness 1.2");
    let on_note = answering(finding(8_200, "KP0L6A", "KP0L6A: 1.2 broke spawns"), &note);
    let elsewhere = sent(8_100, "IKQ63G", "finding", &["@WCJQSG"], "IKQ63G: not here");
    let posted = [
        first.clone(),
        more,
        talk,
        caused,
        landed,
        again,
        back.clone(),
        note,
        on_note.clone(),
        elsewhere,
    ];
    let found = open_findings(&posted, "P9R1KI", &known());
    let ids: Vec<&str> = found.iter().map(|one| one.posted.id.as_str()).collect();
    assert_eq!(
        ids,
        [first.id.as_str(), back.id.as_str(), on_note.id.as_str()],
        "a follow-up to an open finding counted apart, one after its landing note or \
         answering an fyi was folded away, or one sent outside the group was read"
    );
    assert_eq!(
        found[0].landed.as_deref(),
        Some("Landed core #6010 instead."),
        "the latest landing note in its thread, in any case, wasn't its check, or talk of \
         a landing was taken for one"
    );
    assert_eq!(found[1].landed, None);
}

#[test]
fn only_its_sender_withdraws_a_finding_and_only_by_saying_so() {
    let kept = finding(9_000, "IKQ63G", "IKQ63G: the other laptop is away");
    let refused = answer(
        8_900,
        "IKQ63G",
        "IKQ63G: not withdrawing, still away",
        &kept,
    );
    let doubted = answer(8_800, "CIJMZQ", "CIJMZQ: withdrawn? it's back", &kept);
    let gone = finding(8_700, "IKQ63G", "IKQ63G: a slip");
    let withdrawn = answer(8_600, "IKQ63G", "IKQ63G: withdrawn, it came back", &gone);
    let after = answering(finding(8_500, "CIJMZQ", "CIJMZQ: still seeing it"), &gone);
    let wrong = finding(8_400, "CIJMZQ", "CIJMZQ: wrong");
    let taken = Posted {
        supersedes: Some(wrong.id.clone()),
        ..sent(8_300, "A2VLJN", "fyi", &["@P9R1KI"], "ok")
    };
    let slip = finding(8_200, "CIJMZQ", "CIJMZQ: a slip of mine");
    let meant = Posted {
        supersedes: Some(slip.id.clone()),
        ..finding(8_100, "CIJMZQ", "CIJMZQ: what I meant")
    };
    let posted = [
        kept.clone(),
        refused,
        doubted,
        gone,
        withdrawn,
        after.clone(),
        wrong.clone(),
        taken,
        slip,
        meant.clone(),
    ];
    let found = open_findings(&posted, "P9R1KI", &known());
    let ids: Vec<&str> = found.iter().map(|one| one.posted.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            kept.id.as_str(),
            after.id.as_str(),
            wrong.id.as_str(),
            meant.id.as_str()
        ],
        "a sender's refusal or another's doubt withdrew a finding, another's supersedes \
         took it away, or a withdrawal or the sender's own supersedes didn't"
    );
}

/// Writes `one` into `folder`, as the file it names, written `ago` seconds
/// before `NOW`.
fn put(folder: &Path, one: &Posted, ago: i64) {
    put_as(
        folder,
        &format!("{}.json", one.id),
        &serde_json::to_string(one).expect("json"),
        ago,
    );
}

/// Writes `text` into `folder` as `name`, written `ago` seconds before `NOW`.
fn put_as(folder: &Path, name: &str, text: &str, ago: i64) {
    std::fs::create_dir_all(folder).expect("a folder");
    let file = folder.join(name);
    std::fs::write(&file, text).expect("written");
    let at = UNIX_EPOCH + Duration::from_secs(u64::try_from(NOW - ago).expect("after 1970"));
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|open| open.set_modified(at))
        .expect("its time set");
}

#[test]
fn a_reading_opens_only_what_this_machine_wrote_since_the_last() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let first = finding(3_600, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    put(&messages, &first, 3_600);
    put(
        &messages,
        &sent(3_000, "WCJQSG", "fyi", &["@16V6F4"], "elsewhere"),
        3_000,
    );
    let forged = Posted {
        from: "CIJMZQ".to_owned(),
        ..finding(2_950, "IKQ63G", "IKQ63G: in another's name")
    };
    put(&messages, &forged, 2_950);
    put_as(
        &messages,
        "1795259000000-IKQ63G-1-x.json",
        &serde_json::to_string(&first).expect("json"),
        2_900,
    );
    let kept = root.path().join("symbiosis.json");
    // A reading of another shape, however far it says it got, is read anew.
    std::fs::write(
        &kept,
        r#"{"v":0,"through_ms":99999999999999,"posted":[],"known":[]}"#,
    )
    .expect("an old reading");
    let read_now = |keeps| {
        read(&messages, NOW, &kept, keeps)
            .expect("a reading")
            .posted
    };
    assert_eq!(
        read_now(false),
        std::slice::from_ref(&first),
        "the group's message wasn't read, an old reading's mark was followed, or another \
         message, a forged one or one copied under another name was"
    );
    assert!(
        std::fs::read_to_string(&kept).is_ok_and(|text| text.contains("\"v\":0")),
        "a look by hand kept its reading"
    );
    assert_eq!(read_now(true), std::slice::from_ref(&first));
    // Written before the last reading's mark, a file isn't opened again,
    // however it's named; written since, it is, even named days back and
    // filed in an old day's folder, as one relayed late would be.
    put(
        &messages,
        &finding(7_200, "IKQ63G", "IKQ63G: written long ago"),
        7_200,
    );
    let late = finding(3 * DAY, "CIJMZQ", "CIJMZQ: relayed late");
    put(&messages.join("2026-11-18"), &late, 60);
    let landed = answer(30, "A2VLJN", "A2VLJN landed core #6000.", &first);
    put(&messages, &landed, 30);
    assert_eq!(
        read_now(true),
        [late, first, landed],
        "the next reading opened files from before the last, missed one written since, \
         or lost what it kept"
    );
}

#[test]
fn a_reading_that_cant_read_a_file_says_so_and_reads_it_again() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let kept = root.path().join("symbiosis.json");
    assert!(
        read(&messages, NOW, &kept, true).is_err(),
        "a mesh that can't be listed read as one with no findings"
    );
    let torn = finding(120, "CIJMZQ", "CIJMZQ: torn as it was read");
    let name = format!("{}.json", torn.id);
    put_as(&messages, &name, "{\"id\":", 100);
    let first = finding(50, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    put(&messages, &first, 50);
    let read_now = || read(&messages, NOW, &kept, true).expect("a reading");
    let once = read_now();
    assert_eq!(
        (once.posted, once.unread),
        (vec![first.clone()], 1),
        "a file that couldn't be read went unsaid"
    );
    put(&messages, &torn, 100);
    let again = read_now();
    assert_eq!(
        (again.posted, again.unread),
        (vec![torn, first], 0),
        "a file that couldn't be read wasn't read again once whole"
    );
}

#[test]
fn a_finding_is_kept_past_the_mesh_while_its_messages_age_out() {
    let old = finding(10 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let answered = answer(9 * DAY, "A2VLJN", "A2VLJN: looking", &old);
    let chatter = sent(8 * DAY, "WCJQSG", "fyi", &[GROUP], "a week gone");
    let ancient = finding(30 * DAY, "IKQ63G", "IKQ63G: a month gone");
    let ahead = finding(-2 * 3_600, "IKQ63G", "IKQ63G: from a clock two hours fast");
    let kept = vec![ancient, old.clone(), answered.clone(), chatter];
    assert_eq!(
        kept_beside(kept, vec![ahead], NOW * 1000),
        [old, answered],
        "an open finding left with the mesh's week, or chatter stayed past it, or what \
         is past four weeks or stamped hours ahead was kept"
    );
}

#[test]
fn a_finding_open_past_a_day_is_filed_with_its_owner_until_its_fix_runs_here() {
    let old = finding(2 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let fixed = finding(2 * DAY - 1, "IKQ63G", "IKQ63G finding for OSPHT4: y");
    let landed = answer(DAY, "OSPHT4", "OSPHT4 landed core #6000.", &fixed);
    let unrun = finding(2 * DAY - 2, "CIJMZQ", "CIJMZQ, for KP0L6A: z");
    let merged = answer(DAY, "KP0L6A", "KP0L6A landed core #6001.", &unrun);
    let new = finding(3_600, "IKQ63G", "IKQ63G finding for A2VLJN: w");
    let posted = [old, fixed, unrun, landed, merged, new];
    let found = open_findings(&posted, "P9R1KI", &known());
    let repos = Checkouts::from([("core".to_owned(), "/nowhere".into())]);
    let core = |number| Named {
        repo: "core".to_owned(),
        number,
    };
    let text = as_misses(&found);
    let named: Vec<Named> = named_in(&text, &repos)
        .into_iter()
        .map(|(named, _)| named)
        .collect();
    assert_eq!(
        named,
        [core(6000), core(6001)],
        "a landing note's changes weren't read as its finding's check"
    );
    let findings = Findings {
        found,
        text,
        unread: None,
        orchestrator: "P9R1KI".to_owned(),
    };
    let judged = Judged {
        merged: &BTreeSet::from([core(6000), core(6001)]),
        unread: &BTreeSet::new(),
        unrun: &BTreeMap::from([(core(6001), "the last switch doesn't carry it")]),
        here: "here",
    };
    let measure = findings.measured(&repos, &judged, NOW);
    assert_eq!(
        (measure.now, measure.target),
        (2.0, Some(0.0)),
        "a fix running here didn't close its finding, a merged one not yet running did, \
         or one open under a day dragged"
    );
    assert!(
        measure.behind.len() == 2
            && measure.behind[0].ends_with(" · A2VLJN")
            && measure.behind[1].contains("doesn't carry it")
            && measure.behind[1].ends_with(" · KP0L6A"),
        "each finding past a day wasn't filed with its own owner, saying why: {:?}",
        measure.behind
    );
    assert!(
        measure
            .says
            .ends_with("open: A2VLJN 2 (1 past a day), KP0L6A 1 (1 past a day)"),
        "each owner's open count wasn't said: {}",
        measure.says
    );
    let unread = Findings {
        unread: Some("the mesh's messages can't be listed".to_owned()),
        ..findings
    };
    assert_eq!(
        unread.measured(&repos, &judged, NOW).target,
        None,
        "a mesh that couldn't be read was judged"
    );
}
