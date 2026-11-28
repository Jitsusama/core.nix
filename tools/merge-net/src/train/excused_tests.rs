use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::super::flaky::{Blame, Red};
use super::{beside, keep, recent, record, unread};
use crate::ready::built_there;

const BASE: &str = "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0";
const CANDIDATE: &str = "c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1";
const OLDER: &str = "d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2";

/// A fresh state folder for `name`, and the train's checkout in it, where
/// `ride_from` keeps it.
fn state(name: &str) -> (PathBuf, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("merge-net-excused-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let checkout = super::super::checkout_in(&root);
    (root, checkout)
}

fn red<'a>(checkout: &'a Path, log: &'a Path) -> Red<'a> {
    Red {
        checkout,
        base: BASE,
        candidate: CANDIDATE,
        log,
    }
}

#[test]
fn an_excused_red_reads_beside_the_checkout_as_a_pass_on_its_base() {
    let (root, checkout) = state("pass");
    let log = root.join("gate.log");
    let judged = [
        ("vm-pi".to_owned(), Blame::Flaky),
        ("every-long-file-says-why".to_owned(), Blame::Main),
    ];
    keep(&root, &red(&checkout, &log), &judged);
    assert_eq!(
        recent(&checkout),
        vec![CANDIDATE.to_owned()],
        "the record kept under the state folder wasn't listed beside the train's checkout"
    );
    let kept = record(&checkout, CANDIDATE);
    assert!(
        built_there(&kept, &|base| base == BASE),
        "an excused red's record wasn't read as a run that built on its base: {kept}"
    );
    assert!(
        !built_there(&kept, &|_| false),
        "an excused red's record spoke for a main that doesn't hold its base: {kept}"
    );
    assert!(
        kept.contains("\"vm-pi\"") && kept.contains("\"Flaky\"") && kept.contains("\"Main\""),
        "the record doesn't name the checks it excused, and why: {kept}"
    );
    assert_eq!(
        record(&checkout, "c1c1c1c1"),
        "",
        "a short name was read as a record"
    );
    assert_eq!(
        record(&checkout, OLDER),
        "",
        "a commit never excused had a record"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_record_past_its_day_is_neither_read_nor_kept() {
    let (root, checkout) = state("day");
    let folder = beside(&checkout);
    fs::create_dir_all(&folder).unwrap();
    let old = folder.join(OLDER);
    fs::write(
        &old,
        format!("{{\"dir\":null,\"run\":[\"--base\",\"{BASE}\"]}}\n"),
    )
    .unwrap();
    let day_and_an_hour = Duration::from_hours(25);
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(SystemTime::now() - day_and_an_hour)
        .unwrap();
    assert_eq!(
        recent(&checkout),
        Vec::<String>::new(),
        "a record past its day was read"
    );
    assert_eq!(
        record(&checkout, OLDER),
        "",
        "a record past its day was read as a pass, though no excuse kept since removed it"
    );
    let log = root.join("gate.log");
    keep(
        &root,
        &red(&checkout, &log),
        &[("vm-pi".to_owned(), Blame::Flaky)],
    );
    assert!(!old.exists(), "a record past its day was kept");
    assert_eq!(recent(&checkout), vec![CANDIDATE.to_owned()]);
    fs::remove_dir_all(&root).unwrap();
}

/// A candidate never excused has no record, and is said nothing of; one
/// whose record can't be read reads as none too, so it speaks for no
/// recut, and the train says it couldn't read it.
#[test]
fn a_missing_or_unreadable_record_reads_as_none_and_only_the_unreadable_is_said() {
    let (root, checkout) = state("unreadable");
    assert_eq!(
        record(&checkout, CANDIDATE),
        "",
        "a candidate never excused had a record"
    );
    let missing = fs::read_to_string(beside(&checkout).join(CANDIDATE)).unwrap_err();
    assert_eq!(
        unread(CANDIDATE, &missing),
        None,
        "a candidate with no record was said to have one that couldn't be read"
    );
    // A folder where the record would be is a record that exists but can't
    // be read as text.
    let unreadable = beside(&checkout).join(CANDIDATE);
    fs::create_dir_all(&unreadable).unwrap();
    assert_eq!(
        recent(&checkout),
        vec![CANDIDATE.to_owned()],
        "the unreadable record wasn't listed, so this doesn't test reading it"
    );
    assert_eq!(
        record(&checkout, CANDIDATE),
        "",
        "a record that couldn't be read was read as something"
    );
    let failed = fs::read_to_string(&unreadable).unwrap_err();
    let said = unread(CANDIDATE, &failed).unwrap_or_default();
    assert!(
        said.starts_with("merge-net: the excuse of ") && said.contains(&failed.to_string()),
        "a record that exists but couldn't be read wasn't said, with why: {said:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}
