//! `cadence hour` over a stand-in account holding one of Joel's misses,
//! whose check names a repository the run was given no checkout of: only
//! the run that sends the card may keep the hour or owe anyone its drag,
//! since a look by hand, missing the timer's checkouts, reads differently.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

/// One miss whose check merged in World, which a run without World's
/// checkout can't see, so it reads the miss as open.
const MISS: &str = r#"{"at": "2026-11-14T11:15:00-05:00", "words": "the notice stayed up", "what": "the backup notice stayed up after later runs succeeded", "owner": "88LQC0", "check": "world #2450001"}"#;

/// `cadence hour` with `args` over the account in `folder`, with nothing it
/// reads by command able to run, so only the files there count.
fn hour(folder: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cadence"))
        .arg("hour")
        .args(args)
        .env("HOME", folder)
        .env("XDG_STATE_HOME", folder.join("state"))
        .env("XDG_DATA_HOME", folder.join("data"))
        .env("PATH", "")
        .output()
        .expect("cadence runs")
}

fn account() -> tempfile::TempDir {
    let account = tempfile::tempdir().expect("a stand-in account");
    let mission = account.path().join("state/mission");
    fs::create_dir_all(&mission).expect("a mission folder");
    fs::write(mission.join("misses.jsonl"), format!("{MISS}\n")).expect("the misses");
    fs::create_dir_all(
        account
            .path()
            .join("state/pi/agentic-harness.pi/mesh-workflow/messages"),
    )
    .expect("the mesh's messages folder");
    account
}

/// Every file under `folder`, by its path inside it.
fn files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(at) = folders.pop() {
        for entry in fs::read_dir(&at).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if let Ok(inside) = path.strip_prefix(folder) {
                found.push(inside.display().to_string());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_look_by_hand_prints_the_card_and_keeps_and_tells_nothing() {
    let account = account();
    let before = files(account.path());

    let ran = hour(account.path(), &[]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let card = String::from_utf8_lossy(&ran.stdout);
    assert!(
        card.contains("the backup notice stayed up after later runs succeeded · 88LQC0"),
        "the look didn't print the open miss: {card}"
    );
    assert_eq!(
        files(account.path()),
        before,
        "a look by hand kept the hour, owed its drag or told someone, which only the \
         run sending the card may"
    );
}

/// A miss renamed in the ledger, or cut from it, leaves the misses unjudged
/// the hour it goes, since the hour can't say whether it was fixed; the
/// hour after reads the ledger as it now stands, so one rename doesn't
/// leave them unjudged for good.
#[test]
fn the_card_counts_the_pains_the_hearer_summed_up() {
    let account = account();
    let hearer = account.path().join("state/pain-points");
    fs::create_dir_all(&hearer).expect("the hearer's folder");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock past 1970")
        .as_millis();
    fs::write(
        hearer.join("hour.json"),
        format!(r#"{{"at": {now}, "open": 3, "worst": "the bar hides", "troubles": []}}"#),
    )
    .expect("the hearer's summary");

    let ran = hour(account.path(), &[]);
    let card = String::from_utf8_lossy(&ran.stdout);
    assert!(
        card.contains(
            "His pains:\n  ✕ pains Joel said in the last day that nobody has taken up: 3"
        ),
        "the card didn't read the hearer's summary into its own row: {card}"
    );
}

#[test]
fn a_miss_gone_from_the_ledger_leaves_one_hour_unjudged() {
    let account = account();
    let kept = account.path().join("state/cadence");
    fs::create_dir_all(&kept).expect("cadence's folder");
    fs::write(
        kept.join("hour.json"),
        r#"{"open": ["the notice stayed up, as first written · 88LQC0",
                     "the backup notice stayed up after later runs succeeded · 88LQC0"]}"#,
    )
    .expect("last hour");

    let mut cards = Vec::new();
    for _ in 0..2 {
        let ran = hour(account.path(), &["--to", "WCJQSG"]);
        let said = String::from_utf8_lossy(&ran.stderr);
        assert!(ran.status.success(), "cadence failed: {said}");
        cards.push(String::from_utf8_lossy(&ran.stdout).into_owned());
        // Only the gone miss leaves what's carried; one still held stays.
        let carried = fs::read_to_string(kept.join("hour.json")).expect("the hour kept");
        assert!(
            carried.contains("after later runs succeeded"),
            "a miss the ledger still holds wasn't carried: {carried}"
        );
    }
    assert!(
        cards[0].contains("wasn't read whole"),
        "the hour a miss left the ledger judged the misses: {}",
        cards[0]
    );
    assert!(
        !cards[1].contains("wasn't read whole"),
        "the hour after still couldn't judge the misses: {}",
        cards[1]
    );
    assert!(
        !cards.concat().contains("as first written"),
        "the gone miss was told fixed or kept open: {cards:?}"
    );
}

/// A ledger torn mid-write names no miss gone, so the hour carries every
/// one open last hour, and the one that closes later is still told fixed.
#[test]
fn a_torn_ledger_leaves_the_misses_unjudged_and_carried() {
    let account = account();
    let kept = account.path().join("state/cadence");
    fs::create_dir_all(&kept).expect("cadence's folder");
    fs::write(
        kept.join("hour.json"),
        r#"{"open": ["the notice stayed up, as first written · 88LQC0"]}"#,
    )
    .expect("last hour");
    fs::write(
        account.path().join("state/mission/misses.jsonl"),
        format!("{MISS}\n{{\"what\": \"the notice st"),
    )
    .expect("a torn ledger");

    let ran = hour(account.path(), &["--to", "WCJQSG"]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let card = String::from_utf8_lossy(&ran.stdout);
    assert!(
        card.contains("wasn't read whole") && !card.contains("For Joel"),
        "a torn ledger was judged: {card}"
    );
    let carried = fs::read_to_string(kept.join("hour.json")).expect("the hour kept");
    assert!(
        carried.contains("as first written"),
        "a torn ledger dropped a miss open last hour: {carried}"
    );
}

/// A card that can't be sent leaves last hour kept as it was, so the
/// "fixed" lines it held are told again next hour rather than never, and
/// leaves the findings ledger as it was, so no owner's finding is lost.
#[test]
fn a_card_that_cant_be_sent_keeps_last_hour_as_it_was() {
    let account = account();
    let kept = account.path().join("state/cadence");
    fs::create_dir_all(&kept).expect("cadence's folder");
    let last = r#"{"open": ["the backup ran twice · 88LQC0"]}"#;
    fs::write(kept.join("hour.json"), last).expect("last hour");
    let messages = account
        .path()
        .join("state/pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&messages).expect("the mesh's folder");
    fs::set_permissions(&messages, fs::Permissions::from_mode(0o555))
        .expect("a mesh nothing can be sent into");

    let ran = hour(account.path(), &["--to", "WCJQSG"]);
    assert!(!ran.status.success(), "a send that couldn't go was said to");
    let after = fs::read_to_string(kept.join("hour.json")).expect("the hour kept");
    assert_eq!(after, last, "an unsent card's hour was kept as told");
    assert!(
        !kept.join("findings.jsonl").exists(),
        "an unsent hour's findings were kept as told"
    );
}

/// One of the work system's findings, two days open and sent to its
/// owner, counts on the card beside Joel's misses rather than among them,
/// and its owner is owed its drag, whatever else leads the hour.
#[test]
fn a_finding_two_days_open_counts_apart_and_is_owed_to_its_owner() {
    let account = account();
    let sent = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 24 * 3600);
    let ms = sent
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock past 1970")
        .as_millis();
    let id = format!("{ms}-IKQ63G-1-ab12");
    let file = account
        .path()
        .join("state/pi/agentic-harness.pi/mesh-workflow/messages")
        .join(format!("{id}.json"));
    fs::write(
        &file,
        format!(
            r##"{{"id": "{id}", "from": "IKQ63G", "to": ["#v2-symbiosis", "@A2VLJN"],
                "intent": "finding", "text": "IKQ63G: the slots stay held"}}"##
        ),
    )
    .expect("a finding");
    fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|open| open.set_modified(sent))
        .expect("written two days ago");

    let ran = hour(account.path(), &["--to", "WCJQSG"]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let card = String::from_utf8_lossy(&ran.stdout);
    assert!(
        card.contains("open: A2VLJN 1 (1 past a day): 1, at most 0"),
        "the finding wasn't counted against its owner: {card}"
    );
    assert!(
        card.contains("to catch the next: 1, at most 0"),
        "the finding was counted among Joel's misses, or the miss was lost: {card}"
    );
    let kept = account.path().join("state/cadence");
    assert!(
        kept.join("symbiosis.json").exists(),
        "the reading of the mesh wasn't kept"
    );
    let ledger = fs::read_to_string(kept.join("findings.jsonl")).expect("the ledger");
    assert!(
        ledger.contains("\"owner\":\"A2VLJN\""),
        "the finding's owner wasn't owed its drag: {ledger}"
    );
}

#[test]
fn the_run_sending_the_card_keeps_the_hour_and_owes_its_drag() {
    let account = account();

    let ran = hour(account.path(), &["--to", "WCJQSG"]);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    let kept = files(account.path());
    for wanted in ["state/cadence/hour.json", "state/cadence/findings.jsonl"] {
        assert!(
            kept.iter().any(|file| file == wanted),
            "the run sending the card didn't keep {wanted}: {kept:?}"
        );
    }
    let ledger = fs::read_to_string(account.path().join("state/cadence/findings.jsonl"))
        .expect("the ledger");
    assert!(
        ledger.contains("\"owner\":\"88LQC0\""),
        "the drag wasn't owed to the miss's owner: {ledger}"
    );
    assert!(
        kept.iter()
            .any(|file| file.starts_with("state/pi/agentic-harness.pi/mesh-workflow/messages/")),
        "the card and the owner's finding weren't sent: {kept:?}"
    );
}
