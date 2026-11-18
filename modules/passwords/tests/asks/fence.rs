//! The fence list an ask is hidden by, as the daemon reads it: missing,
//! of a version it doesn't know or unreadable, every ask is refused unseen,
//! the answer saying whether it was there and the journal why; and the
//! files it empties named only at version 2, so a daemon from before them
//! refuses rather than ignoring them.

use std::path::Path;

use super::{LISTED, answer, holds, put_up, released, released_with, saying, stored_bash};
use crate::common::{FENCE_LIST, Running};

#[test]
fn an_ask_is_refused_unseen_without_the_fences_list_or_with_one_it_cant_read() {
    let running = Running::releasing("asks-unfenced", &[LISTED]);
    let bash = stored_bash();
    let argv = saying(&bash);
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    std::fs::remove_file(running.fence_list()).unwrap();
    let missing = "c123456789abcdef0123456789abcdef";
    put_up(&running, missing, LISTED, &argv);
    let answered = answer(&running, missing);
    assert_eq!(answered["state"], "refused", "the answer: {answered}");
    assert_eq!(
        answered["why"], "no fence list to hide by",
        "the answer: {answered}"
    );
    let logged = running.logged();
    assert!(
        !logged.contains("passwords: can't"),
        "a missing list was said to be unread: {logged}"
    );
    // A list that's there but can't be read is told apart from one
    // missing, in the answer and in the journal: one of a version it
    // doesn't know, said in a shape this can't read, one not JSON, one
    // not that version's form, one it can't read at all and one whose
    // patterns don't make. Each reads only what the journal says after
    // its own ask, as the list's own line, since the refusal's says
    // "can't read" too.
    let list = running.fence_list();
    let unmade = FENCE_LIST.replacen("\"sshKept\": [", "\"sshKept\": [\"(\", ", 1);
    let cases = [
        (
            "d123456789abcdef0123456789abcdef",
            "is version 3",
            r#"{"version": 3, "home": [{}]}"#,
        ),
        (
            "e123456789abcdef0123456789abcdef",
            "isn't a fence list",
            "{ not json",
        ),
        (
            "f123456789abcdef0123456789abcdef",
            "isn't version 2's form",
            r#"{"version": 2}"#,
        ),
        ("0223456789abcdef0123456789abcdef", "can't be read", ""),
        (
            "1223456789abcdef0123456789abcdef",
            "can't make",
            unmade.as_str(),
        ),
    ];
    for (id, told, text) in cases {
        if text.is_empty() {
            std::fs::remove_file(&list).unwrap();
            std::fs::create_dir(&list).unwrap();
        } else {
            let _ = std::fs::remove_dir(&list);
            std::fs::write(&list, text).unwrap();
        }
        let before = running.logged().len();
        put_up(&running, id, LISTED, &argv);
        let answered = answer(&running, id);
        assert_eq!(answered["why"], UNREAD, "the answer: {answered}");
        let logged = running.logged();
        let said = format!("passwords: {} {told}", list.display());
        assert!(
            logged[before..].contains(&said),
            "the journal doesn't say the list {told}: {logged}"
        );
    }
    assert!(!released(&running), "the manager was asked to release");
}

/// Why an ask is refused while the fence's list is there but unread.
const UNREAD: &str = "a fence list this daemon can't read";

#[test]
fn a_fence_list_of_version_2_hides_what_version_1_does_and_empties() {
    let running = Running::releasing("asks-fence-two", &[LISTED]);
    let args = released_with(&running, "2223456789abcdef0123456789abcdef");
    let docker = std::fs::canonicalize(running.folder.join("home/.docker/config.json")).unwrap();
    holds(
        &args,
        &[
            Path::new("--ro-bind"),
            &running.folder.join("runtime/passwords/emptied.json"),
            &docker,
        ],
    );
    let secret = std::fs::canonicalize(running.folder.join("home/.secret-file")).unwrap();
    holds(
        &args,
        &[Path::new("--ro-bind"), Path::new("/dev/null"), &secret],
    );
}

#[test]
fn a_list_naming_emptied_files_by_the_wrong_version_is_refused() {
    // A daemon from before emptied files reads version 1 and ignores what
    // it doesn't know, so a version 1 list naming them would release with
    // them whole there; and a version 2 list naming none is a list gone
    // wrong, not one with nothing to empty.
    let running = Running::releasing("asks-fence-wrong", &[LISTED]);
    let bash = stored_bash();
    let argv = saying(&bash);
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    let mut unnamed: serde_json::Value = serde_json::from_str(FENCE_LIST).unwrap();
    unnamed.as_object_mut().unwrap().remove("emptied");
    let cases = [
        (
            "3223456789abcdef0123456789abcdef",
            FENCE_LIST.replace("\"version\": 2", "\"version\": 1"),
            "names emptied files at version 1",
        ),
        (
            "4223456789abcdef0123456789abcdef",
            unnamed.to_string(),
            "names no emptied files at version 2",
        ),
    ];
    for (id, list, told) in cases {
        std::fs::write(running.fence_list(), list).unwrap();
        put_up(&running, id, LISTED, &argv);
        let answered = answer(&running, id);
        assert_eq!(answered["why"], UNREAD, "the answer: {answered}");
        let logged = running.logged();
        assert!(
            logged.contains(told),
            "the journal doesn't say the list {told}: {logged}"
        );
    }
    assert!(!released(&running), "the manager was asked to release");
}
