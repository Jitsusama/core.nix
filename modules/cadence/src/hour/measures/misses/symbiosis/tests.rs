use super::super::Named;
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

/// An fyi `from` sent to `to` `ago` seconds before `NOW`, answering `re`.
fn answer(ago: i64, from: &str, to: &str, text: &str, re: &Posted) -> Posted {
    Posted {
        re: Some(re.id.clone()),
        ..sent(ago, from, "fyi", &[to], text)
    }
}

#[test]
fn a_finding_counts_against_the_owner_its_opening_names() {
    let cases = [
        // Each as posted on 20 and 21 November.
        (
            "IKQ63G finding for A2VLJN and WCJQSG: the pi host",
            "A2VLJN",
        ),
        ("CIJMZQ, for WCJQSG: optimus doesn't resolve", "WCJQSG"),
        (
            "IKQ63G, for OSPHT4's spawn work (01793639): a host",
            "OSPHT4",
        ),
        ("IKQ63G finding for WCJQSG (mesh owner): a cell", "WCJQSG"),
        // "for the owner of" names none, nor does a "for" past the colon.
        ("IKQ63G, for the owner of the lender lines: x", "P9R1KI"),
        (
            "IKQ63G: since the reset, spawns time out for CWUEGJ",
            "P9R1KI",
        ),
    ];
    for (text, owner) in cases {
        assert_eq!(
            super::owner(&finding(60, "IKQ63G", text), "P9R1KI"),
            owner,
            "{text:?} counted against the wrong owner"
        );
    }
    let to = |from: &str| sent(60, from, "finding", &["@IKQ63G", GROUP], "on the slots");
    assert_eq!(
        super::owner(&to("CIJMZQ"), "P9R1KI"),
        "IKQ63G",
        "a finding naming no owner wasn't counted against the address it went to"
    );
    assert_eq!(
        super::owner(&to("IKQ63G"), "P9R1KI"),
        "P9R1KI",
        "a finding counted against its own sender"
    );
}

#[test]
fn a_finding_is_withdrawn_or_added_to_and_its_landing_notes_kept() {
    let first = finding(9_000, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let more = Posted {
        re: Some(first.id.clone()),
        ..finding(8_000, "CIJMZQ", "CIJMZQ, two more cases")
    };
    let landed = answer(
        7_000,
        "A2VLJN",
        "@WCJQSG",
        "A2VLJN landed harness #1885.",
        &first,
    );
    let talk = answer(
        6_500,
        "WCJQSG",
        GROUP,
        "that's core #5928's, I think",
        &first,
    );
    let gone = finding(6_000, "IKQ63G", "IKQ63G: equilibrius is away");
    let back = answer(
        5_000,
        "IKQ63G",
        GROUP,
        "IKQ63G: withdrawn, it came back",
        &gone,
    );
    // Only its sender withdraws a finding.
    let doubt = answer(
        4_500,
        "CIJMZQ",
        GROUP,
        "CIJMZQ: withdrawn? it's back",
        &first,
    );
    let wrong = finding(4_000, "CIJMZQ", "CIJMZQ: a slip");
    let fixed = Posted {
        supersedes: Some(wrong.id.clone()),
        ..finding(3_000, "CIJMZQ", "CIJMZQ: what I meant")
    };
    let elsewhere = sent(2_000, "IKQ63G", "finding", &["@WCJQSG"], "IKQ63G: not here");
    let posted = [
        first.clone(),
        more,
        landed,
        talk,
        gone,
        back,
        doubt,
        wrong,
        fixed.clone(),
        elsewhere,
    ];
    let open = open_findings(&posted, "P9R1KI");
    let ids: Vec<&str> = open.iter().map(|one| one.posted.id.as_str()).collect();
    assert_eq!(
        ids,
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
    let first = finding(3_600, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    write(&first);
    write(&sent(3_000, "WCJQSG", "fyi", &["@16V6F4"], "elsewhere"));
    write(&finding(WITHIN + 60, "IKQ63G", "a week gone"));
    let cadence = root.path().join("cadence");
    std::fs::create_dir_all(&cadence).expect("cadence's folder");
    let kept = cadence.join("symbiosis.json");
    assert_eq!(
        read(root.path(), NOW, &kept),
        std::slice::from_ref(&first),
        "the group's message wasn't kept, or another, or one past a week, was"
    );
    // A file written later but named for long before the last reading
    // isn't opened again, while an answer since is read beside the kept.
    write(&finding(3_300, "IKQ63G", "named late"));
    let landed = answer(60, "A2VLJN", "@WCJQSG", "landed", &first);
    write(&landed);
    assert_eq!(
        read(root.path(), NOW, &kept),
        [first, landed],
        "the next reading opened files from before the last, or lost what it kept"
    );
}

#[test]
fn a_finding_open_past_a_day_drags_its_owner_until_its_fix_runs() {
    let old = finding(2 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let fixed = finding(2 * DAY - 1, "IKQ63G", "IKQ63G finding for OSPHT4: y");
    let landed = answer(DAY, "OSPHT4", GROUP, "OSPHT4 landed core #6000.", &fixed);
    let unrun = finding(2 * DAY - 2, "CIJMZQ", "CIJMZQ, for KP0L6A: z");
    let merged = answer(DAY, "KP0L6A", GROUP, "KP0L6A landed core #6001.", &unrun);
    let new = finding(3_600, "IKQ63G", "IKQ63G finding for A2VLJN: w");
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
