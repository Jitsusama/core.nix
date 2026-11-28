use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::super::flaky::{Blame, Red};
use super::{keep, recent, record};
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
    let checkout = super::super::folder(&root).join("checkout");
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
    assert_eq!(recent(&checkout), vec![CANDIDATE.to_owned()]);
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
    let folder = super::super::folder(&root).join("excused");
    fs::create_dir_all(&folder).unwrap();
    let old = folder.join(OLDER);
    fs::write(&old, "{\"dir\":null,\"run\":[\"--base\",\"x\"]}\n").unwrap();
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
