use super::super::Named;
use super::*;

/// 12:00 UTC on Sat 21 November 2026, in seconds.
const NOW: i64 = 1_795_262_400;

/// A message sent `ago` seconds before `NOW`, tagged `tag`.
fn sent(ago: i64, tag: &str, from: &str, intent: &str, to: &[&str], text: &str) -> Posted {
    Posted {
        id: format!("{}-{from}-1-{tag}", (NOW - ago) * 1000),
        from: from.to_owned(),
        to: to.iter().map(|to| (*to).to_owned()).collect(),
        intent: intent.to_owned(),
        text: text.to_owned(),
        ..Posted::default()
    }
}

/// `one`, answering the message `re`.
fn answering(one: Posted, re: &Posted) -> Posted {
    Posted {
        re: Some(re.id.clone()),
        ..one
    }
}

#[test]
fn a_finding_counts_against_the_owner_its_opening_names() {
    let finding = |text: &str, to: &[&str]| sent(60, "a", "IKQ63G", "finding", to, text);
    let cases = [
        // Each as posted on 20 and 21 November.
        (
            "IKQ63G finding for A2VLJN and WCJQSG: the background pi host",
            "A2VLJN",
        ),
        (
            "CIJMZQ, for WCJQSG: optimus doesn't resolve from equilibrius",
            "WCJQSG",
        ),
        (
            "IKQ63G, for OSPHT4's spawn work (01793639): a background",
            "OSPHT4",
        ),
        (
            "IKQ63G finding for WCJQSG (mesh owner): a cell on equilibrius",
            "WCJQSG",
        ),
        // "for the owner of" names none, nor does a "for" past the colon.
        (
            "IKQ63G, for the owner of the VM lender lines (80ECO2 is ended): x",
            "P9R1KI",
        ),
        (
            "IKQ63G: since the reset, spawns time out for CWUEGJ",
            "P9R1KI",
        ),
    ];
    for (text, owner) in cases {
        assert_eq!(
            super::owner(&finding(text, &[GROUP]), "P9R1KI"),
            owner,
            "{text:?} counted against the wrong owner"
        );
    }
    assert_eq!(
        super::owner(
            &finding("CIJMZQ, on the upload slots", &["@IKQ63G", GROUP]),
            "P9R1KI"
        ),
        "IKQ63G",
        "a finding naming no owner wasn't counted against the address it went to"
    );
    assert_eq!(
        super::owner(
            &finding("IKQ63G, on the slots", &["@IKQ63G", GROUP]),
            "P9R1KI"
        ),
        "P9R1KI",
        "a finding counted against its own sender"
    );
}

#[test]
fn a_finding_is_withdrawn_or_added_to_and_its_landing_notes_kept() {
    let first = sent(
        9_000,
        "a",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G finding for A2VLJN: x",
    );
    let more = answering(
        sent(
            8_000,
            "b",
            "CIJMZQ",
            "finding",
            &[GROUP],
            "CIJMZQ, two more cases",
        ),
        &first,
    );
    let landed = answering(
        sent(
            7_000,
            "c",
            "A2VLJN",
            "fyi",
            &["@WCJQSG"],
            "A2VLJN landed harness #1885.",
        ),
        &first,
    );
    let talk = answering(
        sent(
            6_500,
            "d",
            "WCJQSG",
            "fyi",
            &[GROUP],
            "that's core #5928's, I think",
        ),
        &first,
    );
    let gone = sent(
        6_000,
        "e",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G: equilibrius is away",
    );
    let back = answering(
        sent(
            5_000,
            "f",
            "IKQ63G",
            "fyi",
            &[GROUP],
            "IKQ63G: withdrawn, it came back",
        ),
        &gone,
    );
    let wrong = sent(4_000, "g", "CIJMZQ", "finding", &[GROUP], "CIJMZQ: a slip");
    let fixed = Posted {
        supersedes: Some(wrong.id.clone()),
        ..sent(
            3_000,
            "h",
            "CIJMZQ",
            "finding",
            &[GROUP],
            "CIJMZQ: what I meant",
        )
    };
    let elsewhere = sent(
        2_000,
        "i",
        "IKQ63G",
        "finding",
        &["@WCJQSG"],
        "IKQ63G: not here",
    );
    let posted = [
        first.clone(),
        more,
        landed,
        talk,
        gone,
        back,
        wrong,
        fixed.clone(),
        elsewhere,
    ];
    let open = open_findings(&posted, "P9R1KI");
    assert_eq!(
        open.iter()
            .map(|finding| finding.posted.id.as_str())
            .collect::<Vec<_>>(),
        [first.id.as_str(), fixed.id.as_str()],
        "an added case counted apart, a withdrawn or superseded finding counted, \
         or one sent outside the group was read"
    );
    assert_eq!(
        open[0].landed, "A2VLJN landed harness #1885.",
        "only the landing notes answering it are its check"
    );
    assert_eq!(open[0].owner, "A2VLJN");
}

#[test]
fn a_reading_keeps_the_group_and_its_answers_and_opens_only_what_is_new() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let folder = root.path().join("mesh-workflow/messages");
    std::fs::create_dir_all(&folder).expect("the messages' folder");
    let write = |one: &Posted| {
        let json = serde_json::to_string(one).expect("a message");
        std::fs::write(folder.join(format!("{}.json", one.id)), json).expect("written");
    };
    let finding = sent(
        3_600,
        "a",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G finding for A2VLJN: x",
    );
    let other = sent(3_000, "b", "WCJQSG", "fyi", &["@16V6F4"], "elsewhere");
    let old = sent(
        WITHIN + 60,
        "c",
        "IKQ63G",
        "finding",
        &[GROUP],
        "a week gone",
    );
    for one in [&finding, &other, &old] {
        write(one);
    }
    let kept = root.path().join("cadence/symbiosis.json");
    assert_eq!(
        read(root.path(), NOW, &kept),
        [finding.clone()],
        "the group's message wasn't kept, or another, or one past a week, was"
    );
    // A file written later but named for an hour before the last reading
    // isn't opened again, while an answer since is read beside the kept.
    write(&sent(
        3_300,
        "d",
        "IKQ63G",
        "finding",
        &[GROUP],
        "named late",
    ));
    let landed = answering(
        sent(60, "e", "A2VLJN", "fyi", &["@WCJQSG"], "landed"),
        &finding,
    );
    write(&landed);
    assert_eq!(
        read(root.path(), NOW, &kept),
        [finding, landed],
        "the next reading opened files from before the last, or lost what it kept"
    );
}

#[test]
fn a_finding_open_past_a_day_drags_its_owner_until_its_fix_runs() {
    let old = sent(
        2 * DAY,
        "a",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G finding for A2VLJN: x",
    );
    let fixed = sent(
        2 * DAY,
        "b",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G finding for OSPHT4: y",
    );
    let landed = answering(
        sent(
            DAY,
            "c",
            "OSPHT4",
            "fyi",
            &[GROUP],
            "OSPHT4 landed core #6000.",
        ),
        &fixed,
    );
    let unrun = sent(
        2 * DAY,
        "d",
        "CIJMZQ",
        "finding",
        &[GROUP],
        "CIJMZQ, for KP0L6A: z",
    );
    let merged = answering(
        sent(
            DAY,
            "e",
            "KP0L6A",
            "fyi",
            &[GROUP],
            "KP0L6A landed core #6001.",
        ),
        &unrun,
    );
    let new = sent(
        3_600,
        "f",
        "IKQ63G",
        "finding",
        &[GROUP],
        "IKQ63G finding for A2VLJN: w",
    );
    let posted = [old, fixed, landed, unrun, merged, new];
    let findings = open_findings(&posted, "P9R1KI");
    let text = as_misses(&findings);
    let repos = Checkouts::from([("core".to_owned(), "/nowhere".into())]);
    let core = |number| Named {
        repo: "core".to_owned(),
        number,
    };
    let judged = Judged {
        merged: &BTreeSet::from([core(6000), core(6001)]),
        unread: &BTreeSet::new(),
        unrun: &BTreeMap::from([(core(6001), "the last switch doesn't carry it")]),
        here: "here",
    };
    let measure = measured(&findings, &text, &repos, &judged, NOW);
    assert_eq!(
        measure.now, 2.0,
        "a fix running here didn't close its finding, a merged one not yet running did, \
         or one open under a day dragged"
    );
    assert_eq!(measure.target, Some(0.0));
    assert_eq!(
        measure.owner, "A2VLJN",
        "the oldest open finding's owner isn't asked"
    );
    assert_eq!(
        measure.behind[..2],
        [
            "A2VLJN: 2 open, 1 past a day",
            "KP0L6A: 1 open, 1 past a day"
        ],
        "each owner's open count wasn't shown first"
    );
    assert_eq!(measure.behind.len(), 5, "each open finding wasn't listed");
}
