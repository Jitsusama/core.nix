use disk_guard::band::Band;
use disk_guard::journal::{Journal, Line};
use disk_guard::pass::Outcome;
use std::fs;
use std::path::{Path, PathBuf};

fn action(path: &str, outcome: Outcome) -> Line {
    Line::Action {
        at: 1_760_000_000.0,
        band: Band::Pressure,
        path: PathBuf::from(path),
        kind: "target".to_owned(),
        size: 4096,
        outcome,
    }
}

fn reclaimed(path: &str) -> Line {
    action(path, Outcome::Reclaimed)
}

fn lines(path: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn each_line_is_one_json_record() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    let mut journal = Journal::new(&path, 1 << 20);
    journal.write(&reclaimed("/home/u/src/app/target")).unwrap();
    journal
        .write(&action(
            "/home/u/src/web/target",
            Outcome::Kept("cargo holds it".to_owned()),
        ))
        .unwrap();
    journal
        .write(&Line::Pass {
            at: 1_760_000_010.0,
            band: Band::Pressure,
            end: "reached".to_owned(),
            freed: 1 << 30,
        })
        .unwrap();
    let read = lines(&path);
    assert_eq!(read.len(), 3);
    assert_eq!(read[0]["event"], "action");
    assert_eq!(read[0]["path"], "/home/u/src/app/target");
    assert_eq!(read[0]["band"], "pressure");
    assert_eq!(read[0]["outcome"], "reclaimed");
    assert_eq!(read[1]["outcome"], "kept");
    assert_eq!(read[1]["reason"], "cargo holds it");
    assert_eq!(read[2]["event"], "pass");
    assert_eq!(read[2]["freed"], 1u64 << 30);
}

#[test]
fn a_journal_past_its_limit_starts_again_keeping_one_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    let mut journal = Journal::new(&path, 500);
    for n in 0..30 {
        journal.write(&reclaimed(&format!("/home/u/{n}"))).unwrap();
    }
    let current = lines(&path);
    let previous = lines(&dir.path().join("journal.jsonl.1"));
    assert!(fs::metadata(&path).unwrap().len() <= 500 + 200);
    assert_eq!(current.last().unwrap()["path"], "/home/u/29");
    // Whole lines only, and nothing lost between the two.
    let last_before = previous.last().unwrap()["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let first_now = current[0]["path"].as_str().unwrap();
    let number = |path: &str| path.rsplit('/').next().unwrap().parse::<u32>().unwrap();
    assert_eq!(number(&last_before) + 1, number(first_now));
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        names.len(),
        2,
        "only one earlier journal is kept: {names:?}"
    );
}

#[test]
fn a_journal_carries_on_from_what_an_earlier_run_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    Journal::new(&path, 1 << 20)
        .write(&reclaimed("/a"))
        .unwrap();
    Journal::new(&path, 1 << 20)
        .write(&reclaimed("/b"))
        .unwrap();
    assert_eq!(lines(&path).len(), 2);
}
