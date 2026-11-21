//! The cases of the work system's findings count.
//!
//! Long because each way the mesh can move under a reading, and each way
//! a thread's findings, follow-ups and landing notes can come, is a case
//! of its own.

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use super::super::{Named, named_in};
use super::*;

/// 12:00 UTC on Sat 21 November 2026, in seconds.
const NOW: i64 = 1_795_262_400;

/// A message `from` sent `ago` seconds before `NOW` to `to`, as `intent`,
/// written here as it was sent.
fn sent(ago: i64, from: &str, intent: &str, to: &[&str], text: &str) -> Posted {
    Posted {
        id: format!("{}-{from}-1-{ago}", (NOW - ago) * 1000),
        from: from.to_owned(),
        to: to.iter().map(|to| (*to).to_owned()).collect(),
        intent: intent.to_owned(),
        text: text.to_owned(),
        written: (NOW - ago) * 1000,
        ..Posted::default()
    }
}

/// `one`, as written here `ago` seconds before `NOW`.
fn written(one: &Posted, ago: i64) -> Posted {
    Posted {
        written: (NOW - ago) * 1000,
        ..one.clone()
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

/// Whom `one` counts against, P9R1KI where it names none.
fn owned(one: &Posted) -> String {
    super::owner(one, &known()).unwrap_or_else(|| "P9R1KI".to_owned())
}

/// The ids of the findings `posted` holds.
fn ids_of(posted: &[Posted]) -> Vec<String> {
    open_findings(posted, "P9R1KI", &known())
        .into_iter()
        .map(|one| one.posted.id)
        .collect()
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
        (
            "IKQ63G finding for A2VLJN/WCJQSG: the load is 167",
            "A2VLJN",
        ),
        // Named before the first colon, which here is a time's.
        (
            "IKQ63G, for WCJQSG. At 07:29, one push had waited",
            "WCJQSG",
        ),
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
        ("IKQ63G, for WCJQSG. At 7am, one push had waited", "P9R1KI"),
    ];
    for (text, owner) in cases {
        assert_eq!(
            owned(&finding(60, "IKQ63G", text)),
            owner,
            "{text:?} counted against the wrong owner"
        );
    }
    let owner_of = |from: &str, to: &[&str]| owned(&sent(60, from, "finding", to, "on the slots"));
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
fn a_follow_up_adds_to_an_open_finding_and_its_landing_notes_are_its_check() {
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
    // On the first, it lands for the regression beneath it too, and adds
    // to the first's fix rather than replacing it.
    let part = answer(8_350, "A2VLJN", "Landed core #6020.", &first);
    let asked = answer(
        8_340,
        "CIJMZQ",
        "CIJMZQ: Landed core #6030? It still fails here.",
        &first,
    );
    let doubted = answer(8_330, "CIJMZQ", "Landed? Not that I saw.", &first);
    let note = sent(8_300, "WCJQSG", "fyi", &[GROUP], "rolled out harness 1.2");
    let on_note = answering(finding(8_200, "KP0L6A", "KP0L6A: 1.2 broke spawns"), &note);
    let elsewhere = sent(8_100, "IKQ63G", "finding", &["@WCJQSG"], "IKQ63G: not here");
    // After the note's thread holds a finding, a note there lands on it
    // alone, never on the first's.
    let late = answer(8_000, "KP0L6A", "Landed harness #1950.", &on_note);
    let posted = [
        first.clone(),
        more,
        talk,
        caused,
        landed,
        again,
        back.clone(),
        part,
        asked,
        doubted,
        note,
        on_note.clone(),
        elsewhere,
        late,
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
        found[0].landed,
        ["Landed core #6010 instead.", "Landed core #6020."],
        "a note saying instead didn't replace the notes before it, a later part didn't add \
         to them, or talk of a landing, or a question, was taken for one"
    );
    assert_eq!(
        found[0].landed_at,
        Some((NOW - 8_500) * 1000),
        "the fix wasn't dated from its earliest note still counted"
    );
    assert_eq!(
        found[1].landed,
        ["Landed core #6020."],
        "a note on the first finding didn't land for the regression beneath it"
    );
    assert_eq!(
        found[2].landed,
        ["Landed harness #1950."],
        "a landing note on a finding's thread didn't land for it"
    );
    assert!(
        !found[0].landed.iter().any(|line| line.contains("harness")),
        "a landing note in another thread landed here"
    );
    // A note answering a follow-up folded into its finding lands for it.
    let root = finding(500, "IKQ63G", "IKQ63G finding for A2VLJN: q");
    let folded = answering(finding(400, "CIJMZQ", "CIJMZQ: one more"), &root);
    let on_folded = answer(300, "A2VLJN", "Landed core #6040.", &folded);
    let found = open_findings(&[root, folded, on_folded], "P9R1KI", &known());
    assert_eq!(
        found[0].landed,
        ["Landed core #6040."],
        "a landing note on a folded follow-up didn't land for its finding"
    );
}

#[test]
fn a_follow_up_after_a_landing_note_waits_on_that_fix_and_counts_once_it_runs() {
    let first = finding(2 * DAY + 900, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let landed = answer(2 * DAY + 800, "A2VLJN", "Landed core #6000.", &first);
    let more = answering(
        finding(2 * DAY + 700, "CIJMZQ", "CIJMZQ: still seeing it"),
        &first,
    );
    let again = answering(
        finding(2 * DAY + 600, "CIJMZQ", "CIJMZQ: and again"),
        &first,
    );
    // Sent as a finding, a landing note still lands.
    let sent_as = answering(
        finding(2 * DAY + 500, "A2VLJN", "Landed core #6010 too."),
        &first,
    );
    let found = open_findings(
        &[first, landed, more.clone(), again, sent_as],
        "P9R1KI",
        &known(),
    );
    let ids: Vec<&str> = found.iter().map(|one| one.posted.id.as_str()).collect();
    assert_eq!(
        ids[1..],
        [more.id.as_str()],
        "a second follow-up counted apart from the first, or a landing note sent as a \
         finding counted as one"
    );
    assert_eq!(
        (found[1].owner.as_str(), found[1].follows),
        ("A2VLJN", Some(0)),
        "a follow-up naming no owner wasn't owned as its thread's first, or didn't follow it"
    );
    assert_eq!(
        found[0].landed,
        ["Landed core #6000.", "Landed core #6010 too."],
        "a landing note sent as a finding didn't add to the fix"
    );
    let repos = Checkouts::from([("core".to_owned(), "/nowhere".into())]);
    let core = |number| Named {
        repo: "core".to_owned(),
        number,
    };
    let findings = Findings {
        text: as_misses(&found, NOW),
        found,
        unread: None,
        orchestrator: "P9R1KI".to_owned(),
    };
    let merged = BTreeSet::from([core(6000), core(6010)]);
    let waiting = BTreeMap::from([(core(6010), "the last switch doesn't carry it")]);
    let none = BTreeSet::new();
    let judged = |unrun| Judged {
        merged: &merged,
        unread: &none,
        unrun,
        here: "here",
    };
    let waits = findings.measured(&repos, &judged(&waiting), NOW).behind;
    assert!(
        waits.len() == 1 && waits[0].contains(": x,"),
        "a follow-up counted while the fix it follows didn't run yet: {waits:?}"
    );
    let ran = findings
        .measured(&repos, &judged(&BTreeMap::new()), NOW)
        .behind;
    assert!(
        ran.len() == 1 && ran[0].contains("still seeing it"),
        "a follow-up didn't count once the fix it follows ran, or the fixed one did: {ran:?}"
    );
}

#[test]
fn a_landing_note_that_doubts_itself_or_was_cut_lands_nothing() {
    let first = finding(9_000, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let doubts = answer(
        8_900,
        "CIJMZQ",
        "Landed core #6000 but it still fails here.",
        &first,
    );
    let cut = Posted {
        cut: true,
        ..answer(
            8_800,
            "A2VLJN",
            "Landed core #6001, core #6002 and core #60",
            &first,
        )
    };
    let landed = answer(8_700, "A2VLJN", "Landed core #6020.", &first);
    let kept = answer(
        8_600,
        "A2VLJN",
        "Landed core #6021 as well, not instead of #6020.",
        &first,
    );
    let found = open_findings(&[first, doubts, cut, landed, kept], "P9R1KI", &known());
    assert_eq!(
        found[0].landed,
        [
            "Landed core #6020.",
            "Landed core #6021 as well, not instead of #6020."
        ],
        "a note doubting itself or cut short landed, or one saying not instead replaced"
    );
    let mut long = format!("Landed {}", "core #6000, ".repeat(400));
    assert!(
        cut_to_opening(&mut long) && long.len() <= FIRST_LINE,
        "a first line past what's kept wasn't marked cut"
    );
    let mut whole = format!("Landed {}.\n{}", "core #6000, ".repeat(60), "x".repeat(900));
    let first_line = whole.lines().next().unwrap_or_default().to_owned();
    assert!(
        !cut_to_opening(&mut whole) && whole == first_line,
        "a first line past the opening but within what's kept was cut"
    );
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
    let asked = answer(8_750, "IKQ63G", "Withdrawn? No, still seeing it", &kept);
    let gone = finding(8_700, "IKQ63G", "IKQ63G: a slip");
    // Sent as a finding, as one was on 21 November.
    let withdrawn = answering(
        finding(8_600, "IKQ63G", "IKQ63G: withdrawn, it came back"),
        &gone,
    );
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
        asked,
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
        "a sender's refusal or question, or another's doubt, withdrew a finding, another's \
         supersedes took it away, or a withdrawal or the sender's own supersedes didn't"
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

/// What a reading of `messages` at `NOW` kept at `kept` holds.
fn read_at(messages: &Path, kept: &Path, keeps: bool) -> Read {
    read(messages, NOW, kept, keeps).expect("a reading")
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
    assert_eq!(
        read_at(&messages, &kept, false).posted,
        std::slice::from_ref(&first),
        "the group's message wasn't read, an old reading's mark was followed, or another \
         message, a forged one or one copied under another name was"
    );
    assert!(
        std::fs::read_to_string(&kept).is_ok_and(|text| text.contains("\"v\":0")),
        "a look by hand kept its reading"
    );
    assert_eq!(
        read_at(&messages, &kept, true).posted,
        std::slice::from_ref(&first)
    );
    // Written before the last reading's mark, a file isn't opened again,
    // however it's named; written since, it is, even named days back and
    // filed in an old day's folder, as one relayed late would be, or
    // stamped hours ahead by a fast clock. Each is aged and ordered by
    // when it was written here, never by its sender's stamp.
    put(
        &messages,
        &finding(7_200, "IKQ63G", "IKQ63G: written long ago"),
        7_200,
    );
    let late = finding(3 * DAY, "CIJMZQ", "CIJMZQ: relayed late");
    put(&messages.join("2026-11-18"), &late, 60);
    let landed = answer(30, "A2VLJN", "A2VLJN landed core #6000.", &first);
    put(&messages, &landed, 30);
    let ahead = finding(-2 * 3_600, "IKQ63G", "IKQ63G: from a clock two hours fast");
    put(&messages, &ahead, 20);
    assert_eq!(
        read_at(&messages, &kept, true).posted,
        [first, written(&late, 60), landed, written(&ahead, 20)],
        "the next reading opened files from before the last, missed one written since, \
         ordered them by their senders' clocks, or lost what it kept"
    );
}

#[test]
fn a_file_written_just_behind_the_last_mark_is_still_read() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let kept = root.path().join("symbiosis.json");
    let first = finding(50, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    put(&messages, &first, 50);
    read_at(&messages, &kept, true);
    // Written half a minute before the mark, but listed only after it.
    let just = finding(80, "CIJMZQ", "CIJMZQ: listed after the reading");
    put(&messages, &just, 80);
    assert_eq!(
        read_at(&messages, &kept, true).posted,
        [just, first],
        "a file written as the last reading listed was never read"
    );
}

#[test]
fn a_mark_past_now_reads_the_week_again_and_a_file_ahead_moves_it_no_further_than_now() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let kept = root.path().join("symbiosis.json");
    // Kept as this machine's clock ran five minutes fast, then stepped back.
    let fast = Kept {
        v: VERSION,
        through_ms: (NOW + 300) * 1000,
        ..Kept::default()
    };
    std::fs::write(&kept, serde_json::to_string(&fast).expect("json")).expect("a reading");
    let stepped = finding(200, "IKQ63G", "IKQ63G: written in the step back");
    put(&messages, &stepped, 200);
    let ahead = finding(30, "CIJMZQ", "CIJMZQ: a file dated hours ahead");
    put(&messages, &ahead, -2 * 3_600);
    assert_eq!(
        read_at(&messages, &kept, true).posted,
        [stepped, written(&ahead, 0)],
        "a file written while the clock stood behind the mark went unread, or one dated \
         ahead was taken as written after now"
    );
    let mark = serde_json::from_slice::<Kept>(&std::fs::read(&kept).expect("kept"))
        .expect("a reading")
        .through_ms;
    assert!(
        mark <= NOW * 1000,
        "a file dated ahead moved the mark past now, hiding what's written before it"
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
    // Torn well over the slack before the newest, so only holding the mark
    // back to it reads it again.
    let torn = finding(200, "CIJMZQ", "CIJMZQ: torn as it was read");
    put_as(&messages, &format!("{}.json", torn.id), "{\"id\":", 200);
    let first = finding(50, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    put(&messages, &first, 50);
    // Broken an hour since, it's no message, and holds nothing back.
    put_as(&messages, "1795258800000-CIJMZQ-1-old.json", "{}", 3_600);
    let once = read_at(&messages, &kept, true);
    assert_eq!(
        (once.posted, once.unread),
        (vec![first.clone()], 1),
        "a file that couldn't be read went unsaid, or one long broken was counted"
    );
    put(&messages, &torn, 200);
    let again = read_at(&messages, &kept, true);
    assert_eq!(
        (again.posted, again.unread),
        (vec![torn, first], 0),
        "a file that couldn't be read wasn't read again once whole, or one long broken \
         still held the count"
    );
}

#[test]
fn a_days_folder_that_cant_be_listed_is_an_error() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let kept = root.path().join("symbiosis.json");
    let day = messages.join("2026-11-20");
    put(&day, &finding(60, "IKQ63G", "IKQ63G: filed in a day"), 60);
    std::fs::set_permissions(&day, std::fs::Permissions::from_mode(0o000)).expect("shut");
    let reading = read(&messages, NOW, &kept, true);
    std::fs::set_permissions(&day, std::fs::Permissions::from_mode(0o755)).expect("opened");
    assert!(
        reading.is_err(),
        "a day's folder that couldn't be listed read as holding no findings"
    );
    assert!(
        !kept.exists(),
        "a reading that failed kept a mark past what it missed"
    );
}

#[test]
fn a_finding_is_kept_past_the_mesh_while_its_messages_age_out() {
    let old = finding(10 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let answered = answer(9 * DAY, "A2VLJN", "A2VLJN: looking", &old);
    let chatter = sent(8 * DAY, "WCJQSG", "fyi", &[GROUP], "a week gone");
    let ancient = finding(30 * DAY, "IKQ63G", "IKQ63G: a month gone");
    // Withdrawn by supersedes alone, which answers nothing by its re.
    let slip = finding(10 * DAY - 1, "CIJMZQ", "CIJMZQ: a slip");
    let taken = Posted {
        supersedes: Some(slip.id.clone()),
        ..sent(9 * DAY, "CIJMZQ", "fyi", &["@P9R1KI"], "CIJMZQ: never mind")
    };
    let kept = vec![
        ancient,
        old.clone(),
        answered.clone(),
        chatter,
        slip.clone(),
        taken.clone(),
    ];
    let posted = kept_beside(kept, Vec::new(), NOW * 1000);
    assert_eq!(
        posted,
        [old.clone(), slip, answered, taken],
        "an open finding or its withdrawal left with the mesh's week, chatter stayed past \
         it, or what is past four weeks was kept"
    );
    let ids: Vec<String> = open_findings(&posted, "P9R1KI", &known())
        .into_iter()
        .map(|one| one.posted.id)
        .collect();
    assert_eq!(ids, [old.id], "a finding withdrawn a week ago came back");
}

#[test]
fn a_finding_open_past_a_day_is_filed_with_its_owner_until_its_fix_runs_here() {
    let old = finding(2 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let fixed = finding(2 * DAY - 1, "IKQ63G", "IKQ63G finding for OSPHT4: y");
    let unrun = finding(2 * DAY - 2, "CIJMZQ", "CIJMZQ, for KP0L6A: z");
    let unseen = finding(2 * DAY - 3, "CIJMZQ", "CIJMZQ, for OSPHT4: v");
    let apart = finding(2 * DAY - 4, "IKQ63G", "IKQ63G finding for KP0L6A: u");
    let notes = [
        answer(DAY, "OSPHT4", "OSPHT4 landed core #6000.", &fixed),
        answer(DAY, "KP0L6A", "KP0L6A landed core #6001.", &unrun),
        answer(DAY, "OSPHT4", "OSPHT4 landed core #6002.", &unseen),
        answer(DAY, "KP0L6A", "KP0L6A landed pi-packages #7.", &apart),
    ];
    // Past a day exactly, and a second short of one.
    let edge = finding(DAY, "IKQ63G", "IKQ63G finding for WCJQSG: a day");
    let young = finding(DAY - 1, "IKQ63G", "IKQ63G finding for WCJQSG: not quite");
    let new = finding(3_600, "IKQ63G", "IKQ63G finding for A2VLJN: w");
    let mut posted = vec![old, fixed, unrun, unseen, apart];
    posted.extend(notes);
    posted.extend([edge, young, new]);
    let found = open_findings(&posted, "P9R1KI", &known());
    let repos = Checkouts::from([
        ("core".to_owned(), "/nowhere".into()),
        ("pi-packages".to_owned(), "/nowhere".into()),
    ]);
    let change = |repo: &str, number| Named {
        repo: repo.to_owned(),
        number,
    };
    let core = |number| change("core", number);
    let text = as_misses(&found, NOW);
    let named = named_in(&text, &repos);
    assert_eq!(
        named,
        [
            (core(6000), NOW - DAY),
            (core(6001), NOW - DAY),
            (core(6002), NOW - DAY),
            (change("pi-packages", 7), NOW - DAY)
        ],
        "a landing note's changes weren't read as its finding's check, from when it came"
    );
    let findings = Findings {
        found,
        text,
        unread: None,
        orchestrator: "P9R1KI".to_owned(),
    };
    let judged = Judged {
        merged: &BTreeSet::from([core(6000), core(6001), core(6002), change("pi-packages", 7)]),
        unread: &BTreeSet::new(),
        unrun: &BTreeMap::from([
            (core(6001), "the last switch doesn't carry it"),
            (core(6002), "cadence can't see what runs here"),
        ]),
        here: "here",
    };
    let measure = findings.measured(&repos, &judged, NOW);
    assert_eq!(
        (measure.now, measure.target),
        (3.0, Some(0.0)),
        "a fix running here didn't close its finding, a merged one not yet running did, \
         one open under a day dragged, or one held by the hour's blindness counted"
    );
    assert!(
        measure.behind.len() == 4
            && measure.behind[0].ends_with(" · A2VLJN")
            && measure.behind[1].contains("doesn't carry it")
            && measure.behind[1].ends_with(" · KP0L6A")
            && held_unseen(&measure.behind[2])
            && measure.behind[2].ends_with(" · OSPHT4")
            && measure.behind[3].contains(": a day · WCJQSG"),
        "each finding past a day wasn't filed with its own owner, saying why, or one held \
         by what cadence can't see left the card or wasn't marked held: {:?}",
        measure.behind
    );
    assert!(
        measure.says.ends_with(
            "; 2 more held where cadence can't see it run; open: A2VLJN 2 (1 past a day), \
             KP0L6A 1 (1 past a day), WCJQSG 2 (1 past a day)"
        ),
        "each owner's open count wasn't said, or the findings held unseen weren't: {}",
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

#[test]
fn a_finding_held_by_a_main_unread_or_a_change_unreadable_is_kept_apart() {
    let unread = finding(2 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let elsewhere = finding(2 * DAY - 1, "IKQ63G", "IKQ63G finding for OSPHT4: y");
    let numberless = finding(2 * DAY - 2, "IKQ63G", "IKQ63G finding for KP0L6A: z");
    let posted = [
        unread.clone(),
        elsewhere.clone(),
        numberless.clone(),
        answer(DAY, "A2VLJN", "Landed core #6000.", &unread),
        answer(DAY, "OSPHT4", "Landed mycelial-node #12.", &elsewhere),
        answer(DAY, "KP0L6A", "Landed the fix.", &numberless),
    ];
    let found = open_findings(&posted, "P9R1KI", &known());
    let findings = Findings {
        text: as_misses(&found, NOW),
        found,
        unread: None,
        orchestrator: "P9R1KI".to_owned(),
    };
    let repos = Checkouts::from([("core".to_owned(), "/nowhere".into())]);
    let judged = Judged {
        merged: &BTreeSet::new(),
        unread: &BTreeSet::from(["core".to_owned()]),
        unrun: &BTreeMap::new(),
        here: "here",
    };
    let measure = findings.measured(&repos, &judged, NOW);
    assert!(
        measure.now == 0.0
            && measure.behind.len() == 3
            && measure.behind.iter().all(|line| held_unseen(line))
            && measure
                .says
                .contains("3 more held where cadence can't see it run"),
        "a finding held by a main cadence couldn't read, or by a landing note naming no \
         change it reads, was counted, or left the card: {} {:?}",
        measure.says,
        measure.behind
    );
}

#[test]
fn a_findings_landing_note_is_walked_for_and_held_while_its_main_is_unread() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let first = finding(2 * DAY, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let posted = [
        first.clone(),
        answer(DAY, "A2VLJN", "Landed core #6000.", &first),
    ];
    let found = open_findings(&posted, "P9R1KI", &known());
    let findings = Findings {
        text: as_misses(&found, NOW),
        found,
        unread: None,
        orchestrator: "P9R1KI".to_owned(),
    };
    // No core mirror is kept here, so the walk the note asks for fails.
    let (_, measure) = super::super::checked_beside("", root.path(), &[], "", Some(&findings), NOW)
        .expect("judged");
    let measure = measure.expect("the findings measured");
    assert!(
        measure.now == 0.0
            && measure.behind.len() == 1
            && measure.behind[0].contains("cadence couldn't read core's main"),
        "a finding's landing note wasn't walked for, so its finding counted as unlanded: {:?}",
        measure.behind
    );
}

#[test]
fn an_answer_off_the_group_written_before_its_finding_still_counts() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let kept = root.path().join("symbiosis.json");
    let first = finding(600, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let slip = finding(590, "CIJMZQ", "CIJMZQ: a slip");
    // Each written here before the finding it settles, as one relayed late.
    let landed = answering(
        sent(500, "A2VLJN", "fyi", &["@IKQ63G"], "Landed core #6000."),
        &first,
    );
    let taken = Posted {
        supersedes: Some(slip.id.clone()),
        ..sent(500, "CIJMZQ", "fyi", &["@P9R1KI"], "CIJMZQ: never mind")
    };
    put(&messages, &landed, 300);
    put(&messages, &taken, 300);
    read_at(&messages, &kept, true);
    put(&messages, &first, 100);
    put(&messages, &slip, 100);
    let posted = read_at(&messages, &kept, true).posted;
    let found = open_findings(&posted, "P9R1KI", &known());
    assert!(
        found.len() == 1 && found[0].landed == ["Landed core #6000."],
        "an answer off the group written before its finding was lost: {found:?}"
    );
}

#[test]
fn a_file_moved_away_holds_the_mark_and_one_unreadable_holds_the_count() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let gone = root.path().join("1795262340000-IKQ63G-1-x.json");
    let at = (NOW - 60) * 1000;
    let moved = opened(
        vec![(gone, "1795262340000-IKQ63G-1-x.json".to_owned(), at)],
        NOW * 1000,
        at - 1000,
    );
    assert_eq!(
        (moved.through_ms, moved.unread),
        (at, 0),
        "a file moved since it was listed was counted unread, or let the mark pass it"
    );
    let expired = opened(
        vec![(
            root.path().join("1794657600000-IKQ63G-1-y.json"),
            "1794657600000-IKQ63G-1-y.json".to_owned(),
            (NOW - WITHIN + 60) * 1000,
        )],
        NOW * 1000,
        0,
    );
    assert_eq!(
        expired.through_ms,
        (NOW - WITHIN + 60) * 1000,
        "a file expired at the week's end held the next reading back to it"
    );
    let one = finding(30, "IKQ63G", "IKQ63G: shut");
    put(root.path(), &one, 30);
    let path = root.path().join(format!("{}.json", one.id));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("shut");
    let shut = opened(
        vec![(path, format!("{}.json", one.id), (NOW - 30) * 1000)],
        NOW * 1000,
        0,
    );
    assert_eq!(
        (shut.unread, shut.through_ms),
        (1, (NOW - 30) * 1000),
        "a file that can't be opened wasn't counted unread and held to"
    );
}

#[test]
fn a_file_dated_ahead_ages_from_when_it_was_first_written_here() {
    let one = finding(0, "IKQ63G", "IKQ63G finding for A2VLJN: x");
    let kept = vec![written(&one, 3_600)];
    let posted = kept_beside(kept, vec![written(&one, 0)], NOW * 1000);
    assert_eq!(
        posted,
        [written(&one, 3_600)],
        "a file opened again moved its finding's age to now"
    );
}

#[test]
fn a_kept_reading_that_cant_be_read_is_an_error() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    put(&messages, &finding(60, "IKQ63G", "IKQ63G: x"), 60);
    let kept = root.path().join("symbiosis.json");
    std::fs::create_dir(&kept).expect("a folder where the reading goes");
    assert!(
        read(&messages, NOW, &kept, true).is_err(),
        "a kept reading that couldn't be read was taken for none"
    );
}

#[test]
fn only_plain_files_are_read() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let messages = root.path().join("messages");
    let first = finding(60, "IKQ63G", "IKQ63G: x");
    put(&messages, &first, 60);
    std::os::unix::fs::symlink("/dev/zero", messages.join("1795262390000-CIJMZQ-1-z.json"))
        .expect("a link");
    std::fs::create_dir(messages.join("1795262390000-CIJMZQ-1-d.json")).expect("a folder");
    let kept = root.path().join("symbiosis.json");
    let read = read_at(&messages, &kept, true);
    assert_eq!(
        (read.posted, read.unread),
        (vec![first], 0),
        "a link or a folder named as a message was read"
    );
}

#[test]
fn a_look_reads_the_mesh_only_where_the_hour_keeps_it() {
    let account = tempfile::tempdir().expect("a scratch folder");
    let root = account.path().join("harness");
    let folder = account.path().join("cadence");
    let messages = root.join("mesh-workflow/messages");
    let one = finding(60, "IKQ63G", "IKQ63G: x");
    put(&messages, &one, 60);
    std::fs::create_dir_all(&folder).expect("a folder");
    let kept = folder.join("symbiosis.json");
    let now = jiff::Timestamp::now().as_second();
    assert!(
        findings(&root, &folder, now, None).is_none(),
        "a look with no reading kept read the mesh"
    );
    put_as(&folder, "symbiosis.json", "{}", NOW - now + 3 * 3_600);
    assert!(
        findings(&root, &folder, now, None).is_none(),
        "a look read the mesh past a reading no hour keeps"
    );
    std::fs::write(&kept, "{}").expect("kept now");
    let look = findings(&root, &folder, now, None).expect("a look at a kept reading");
    assert_eq!(
        look.orchestrator, "the orchestrator",
        "a look said a finding naming no owner as nobody's"
    );
}
