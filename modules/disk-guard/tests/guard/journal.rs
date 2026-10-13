use disk_guard::band::Band;
use disk_guard::journal::{Journal, Journaled, Line};
use disk_guard::pass::{Outcome, Reclaimer};
use disk_guard::plan::{Candidate, Class};
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
            seconds: 42.5,
        })
        .unwrap();
    let read = lines(&path);
    assert_eq!(read.len(), 3);
    assert_eq!(read[2]["seconds"], 42.5);
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

fn begin(path: &str, root: &str) -> Line {
    Line::Begin {
        at: 1_760_000_000.0,
        path: PathBuf::from(path),
        root: PathBuf::from(root),
    }
}

#[test]
fn what_was_begun_and_never_ended_is_found_across_both_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    let mut journal = Journal::new(&path, 400);
    journal.write(&begin("/t/a", "/t")).unwrap();
    journal.write(&reclaimed("/t/a")).unwrap();
    journal.write(&begin("/t/b", "/t")).unwrap();
    journal.write(&reclaimed("/t/x")).unwrap();
    journal.write(&begin("/t/c", "/t")).unwrap();
    journal.write(&reclaimed("/t/c")).unwrap();
    journal.write(&begin("/u/d", "/u")).unwrap();
    assert!(
        dir.path().join("journal.jsonl.1").exists(),
        "the lines span both files"
    );
    assert_eq!(
        journal.unfinished().unwrap(),
        [
            (PathBuf::from("/t/b"), PathBuf::from("/t")),
            (PathBuf::from("/u/d"), PathBuf::from("/u")),
        ]
    );
    let read = lines(&path);
    let begun = read
        .iter()
        .find(|line| line["event"] == "begin" && line["path"] == "/u/d")
        .unwrap();
    assert_eq!(begun["root"], "/u");
}

/// Reads the journal from inside each reclaim, as a guard dying there
/// would leave it.
struct Peeking {
    journal: PathBuf,
    seen: Vec<Vec<(PathBuf, PathBuf)>>,
}

impl Reclaimer for Peeking {
    fn reclaim(&mut self, _: &Candidate) -> Outcome {
        self.seen
            .push(Journal::new(&self.journal, 1 << 20).unfinished().unwrap());
        Outcome::Reclaimed
    }
}

fn candidate(path: &Path) -> Candidate {
    Candidate {
        path: path.to_owned(),
        class: Class::Scratch { healthy: None },
        idle: 0.0,
        size: 5,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

#[test]
fn a_directory_is_begun_before_it_is_reclaimed_and_ended_after() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    let (run, file) = (dir.path().join("run"), dir.path().join("out.log"));
    fs::create_dir(&run).unwrap();
    fs::write(&file, "x").unwrap();
    let mut journal = Journal::new(&path, 1 << 20);
    let mut journaled = Journaled {
        inner: Peeking {
            journal: path.clone(),
            seen: Vec::new(),
        },
        journal: &mut journal,
        roots: [&run, &file]
            .into_iter()
            .map(|p| (p.clone(), dir.path().to_owned()))
            .collect(),
        at: 1.0,
        band: Band::Pressure,
    };
    assert_eq!(journaled.reclaim(&candidate(&run)), Outcome::Reclaimed);
    assert_eq!(journaled.reclaim(&candidate(&file)), Outcome::Reclaimed);
    assert_eq!(
        journaled.inner.seen,
        [vec![(run.clone(), dir.path().to_owned())], vec![]],
        "a directory is begun before it's touched; a file, unlinked at once, isn't"
    );
    assert!(journal.unfinished().unwrap().is_empty(), "both ended");
    let read = lines(&path);
    assert_eq!(read.len(), 3);
    assert_eq!(read[2]["kind"], "scratch");
}

#[test]
fn nothing_journalled_has_nothing_unfinished() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::new(&dir.path().join("journal.jsonl"), 400);
    assert!(journal.unfinished().unwrap().is_empty());
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
