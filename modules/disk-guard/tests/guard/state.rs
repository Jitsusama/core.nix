use disk_guard::clock::Awake;
use disk_guard::pass::Learned;
use disk_guard::plan::Class;
use disk_guard::state::{Memory, load, save};
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

#[test]
fn a_save_replaces_the_file_whole_and_leaves_nothing_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    fs::write(&path, "x".repeat(100_000)).unwrap();
    save(&path, &remembered()).unwrap();
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["memory.json"]);
    assert!(load::<Memory>(&path).unwrap().is_some());
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

#[test]
fn a_damaged_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    fs::write(&path, "{ \"version\": 1, \"learned\": ").unwrap();
    assert!(load::<Memory>(&path).is_err());
}
