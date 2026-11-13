use disk_guard::clock::Awake;
use disk_guard::pass::Learned;
use disk_guard::plan::{Candidate, Class};
use disk_guard::state::{Memory, load, private, save, share};
use disk_guard::surveyed::{self, Offered, Surveyed};
use std::fs;

fn remembered() -> Memory {
    let mut learned = Learned::default();
    learned.learn(Class::Target, 100.0, 60.0);
    Memory {
        awake: Awake::default(),
        learned,
        upkept: Some(1_760_000_000.0),
        ..Memory::default()
    }
}

#[test]
fn what_is_saved_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    save(&path, &remembered()).unwrap();
    let loaded: Memory = load(&path).unwrap().unwrap();
    assert_eq!(loaded, remembered());
    crate::assert_exactly(loaded.learned.ratio(Class::Target), 0.6);
}

#[test]
fn nothing_saved_loads_as_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        load::<Memory>(&dir.path().join("memory.json")).unwrap(),
        None
    );
}

// Written in a folder of its own beside it, so a watcher of the file's
// folder hears only the rename, and nothing is left in either.
#[test]
fn a_save_replaces_the_file_whole_and_leaves_nothing_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    fs::write(&path, "x".repeat(100_000)).unwrap();
    save(&path, &remembered()).unwrap();
    let mut names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, [".writing", "memory.json"]);
    assert_eq!(
        fs::read_dir(dir.path().join(".writing")).unwrap().count(),
        0
    );
    assert!(load::<Memory>(&path).unwrap().is_some());
}

// The machine's guard keeps its state in a folder any account may pass
// through to read its journal, so only what's meant to be read is.
#[test]
fn what_is_saved_is_the_owners_alone_and_what_is_shared_anyones() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (kept, shared) = (
        dir.path().join("memory.json"),
        dir.path().join("status.json"),
    );
    save(&kept, &remembered()).unwrap();
    share(&shared, &remembered()).unwrap();
    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode(&kept),
        0o600,
        "a saved file should be its owner's alone"
    );
    assert_eq!(
        mode(&shared),
        0o644,
        "a shared file should be readable by anyone"
    );
}

// What an earlier guard saved under the umask, which a folder of root's
// alone hid, is made its owner's alone before the folder opens: only the
// journal stays readable.
#[test]
fn what_an_earlier_guard_left_readable_is_made_private_but_the_journal() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let open = |path: &std::path::Path, mode: u32| {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    };
    for name in ["memory.json", "journal.jsonl", "journal.jsonl.1"] {
        fs::write(dir.path().join(name), "{}").unwrap();
        open(&dir.path().join(name), 0o644);
    }
    fs::create_dir(dir.path().join(".writing")).unwrap();
    open(&dir.path().join(".writing"), 0o755);
    private(dir.path()).unwrap();
    let mode = |name: &str| {
        fs::metadata(dir.path().join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(
        mode("memory.json"),
        0o600,
        "an old memory file should be root's alone"
    );
    assert_eq!(
        mode(".writing"),
        0o700,
        "the folder saves are made in should be root's alone"
    );
    assert_eq!(
        mode("journal.jsonl"),
        0o644,
        "the journal should stay readable"
    );
    assert_eq!(
        mode("journal.jsonl.1"),
        0o644,
        "the earlier journal should stay readable"
    );
}

#[test]
fn fields_a_newer_guard_wrote_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    save(&path, &remembered()).unwrap();
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["version"], 1);
    json["version"] = 2.into();
    json["fill_rates"] = serde_json::json!({ "root": 3 });
    fs::write(&path, json.to_string()).unwrap();
    let loaded: Memory = load(&path).unwrap().unwrap();
    save(&path, &loaded).unwrap();
    let again: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(again["fill_rates"]["root"], 3);
    assert_eq!(again["version"], 2, "a newer file stays as new as it was");
}

// On 10 November a producer that had never run was offered as idle
// forever, which JSON writes as null, so the whole survey of 29,000
// candidates couldn't be read back and each plan walked for minutes.
#[test]
fn a_survey_with_a_candidate_idle_forever_is_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let never = Candidate {
        path: "/cache#producer".into(),
        class: Class::Target,
        idle: f64::INFINITY,
        size: 0,
        occupied: false,
        spent: false,
        checkout: None,
    };
    let survey = Surveyed {
        version: 1,
        at: 1.0,
        offered: vec![Offered {
            device: 1,
            candidate: never,
        }],
    };
    surveyed::save(dir.path(), &survey).unwrap();
    assert_eq!(surveyed::load(dir.path()), Some(survey));
}

#[test]
fn a_damaged_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    fs::write(&path, "{ \"version\": 1, \"learned\": ").unwrap();
    assert!(load::<Memory>(&path).is_err());
}
