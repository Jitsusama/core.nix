//! The fence list an ask is hidden by, as the daemon reads it: missing,
//! of a version it doesn't know or unreadable, every ask is refused unseen,
//! the journal telling which; and version 2, naming the files it empties,
//! hides as version 1 does.

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
    std::fs::write(
        running.fence_list(),
        FENCE_LIST.replace("\"version\": 1", "\"version\": 3"),
    )
    .unwrap();
    let unknown = "d123456789abcdef0123456789abcdef";
    put_up(&running, unknown, LISTED, &argv);
    let answered = answer(&running, unknown);
    assert_eq!(
        answered["why"], "no fence list to hide by",
        "the answer: {answered}"
    );
    // A list that's there but can't be read is told apart from one
    // missing in the journal: one of a version it doesn't know, one not
    // JSON, one it can't open and one whose patterns don't make.
    let list = running.fence_list();
    let cases = [
        ("e123456789abcdef0123456789abcdef", "is version 3"),
        ("f123456789abcdef0123456789abcdef", "can't read"),
        ("0223456789abcdef0123456789abcdef", "can't open"),
        ("1223456789abcdef0123456789abcdef", "can't make"),
    ];
    for (id, told) in cases {
        match told {
            "can't read" => std::fs::write(&list, "{ not json").unwrap(),
            "can't open" => {
                std::fs::remove_file(&list).unwrap();
                std::fs::create_dir(&list).unwrap();
            }
            "can't make" => {
                std::fs::remove_dir(&list).unwrap();
                let unmade = FENCE_LIST.replacen("\"sshKept\": [", "\"sshKept\": [\"(\", ", 1);
                std::fs::write(&list, unmade).unwrap();
            }
            _ => {}
        }
        put_up(&running, id, LISTED, &argv);
        let answered = answer(&running, id);
        assert_eq!(
            answered["why"], "no fence list to hide by",
            "the answer: {answered}"
        );
        let logged = running.logged();
        assert!(
            logged.contains(told),
            "the journal doesn't say the list {told}: {logged}"
        );
    }
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_fence_list_of_version_2_hides_as_version_1_does() {
    // Version 2 is version 1 naming the files it empties, so a daemon
    // from before emptied files refuses a list that names them, rather
    // than releasing with them whole.
    let running = Running::releasing("asks-fence-two", &[LISTED]);
    std::fs::write(
        running.fence_list(),
        FENCE_LIST.replace("\"version\": 1", "\"version\": 2"),
    )
    .unwrap();
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
}
