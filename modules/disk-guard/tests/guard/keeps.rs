//! The keeps a machine's configuration declares, beside those made by hand.

use disk_guard::config::Config;
use disk_guard::keeps::{declare, keep, load, unkeep};
use std::collections::BTreeMap;
use std::fs;

#[test]
fn a_declared_keep_follows_the_configuration_and_leaves_a_hand_keep_be() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let (image, photos) = (dir.path().join("image"), dir.path().join("photos"));
    fs::create_dir_all(&image).unwrap();
    fs::create_dir_all(&photos).unwrap();
    let (image, photos) = (
        image.canonicalize().unwrap(),
        photos.canonicalize().unwrap(),
    );
    fs::create_dir_all(&state).unwrap();
    keep(&state, &photos, "the only copies", 1.0).unwrap();
    let declared = BTreeMap::from([
        (image.clone(), "the restore image".to_owned()),
        (photos.clone(), "what the machine says".to_owned()),
    ]);
    declare(&state, &declared, 2.0).unwrap();
    let keeps = load(&state).unwrap();
    assert_eq!(
        (keeps[&image].reason.as_str(), keeps[&image].declared),
        ("the restore image", true),
        "the configuration's keep is written"
    );
    assert_eq!(
        (keeps[&photos].reason.as_str(), keeps[&photos].declared),
        ("the only copies", false),
        "Joel's own reason stands"
    );
    let refused = unkeep(&state, &image).unwrap_err();
    assert!(
        refused.to_string().contains("configuration keeps it"),
        "{refused}"
    );
    declare(&state, &BTreeMap::new(), 3.0).unwrap();
    let keeps = load(&state).unwrap();
    assert!(!keeps.contains_key(&image), "named no longer, it goes");
    assert!(
        keeps.contains_key(&photos),
        "a hand keep never goes with it"
    );
}

#[test]
fn a_declared_keep_needs_a_plain_path_and_a_reason() {
    let base = r#"{ "version": 1, "scope": "user", "sources": {}, "keep": "#;
    for (kept, refusal) in [
        (
            r#"{ "backups": "the restore image" }"#,
            "not a plain absolute path",
        ),
        (r#"{ "/home/u/backups": " " }"#, "kept without a reason"),
    ] {
        let refused = Config::parse(&format!("{base}{kept} }}")).unwrap_err();
        assert!(refused.contains(refusal), "{refused}");
    }
}
