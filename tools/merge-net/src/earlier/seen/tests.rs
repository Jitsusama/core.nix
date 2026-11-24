//! The tests of whether a round read a head.

use serde_json::json;

use super::*;

/// A full commit id of the letter `letter`, as a round names one.
fn id(letter: char) -> String {
    letter.to_string().repeat(40)
}

fn round(extra: &Value) -> Value {
    let mut round = json!({
        "id": "council-1",
        "round": "council",
        "startedAt": "2026-11-19T17:00:00.000Z",
        "witness": id('a'),
        "outcomes": [{"participantId": "a", "findingIds": [1], "answerPath": "/x"}],
    });
    for (key, value) in extra.as_object().unwrap() {
        round[key] = value.clone();
    }
    round
}

fn read(rounds: &[Value]) -> Vec<String> {
    read_in(&json!({"change": {}, "runs": rounds}).to_string())
        .into_iter()
        .map(|read| read.commit)
        .collect()
}

fn change_read(commit: char) -> Read {
    Read {
        commit: id(commit),
        from: None,
    }
}

/// A folder of its own for a test named `name`, empty.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("merge-net-seen-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// Writes the round file `name` in `folder` holding `change`, as
/// written `ago` seconds before now.
fn written(folder: &Path, name: &str, change: &Value, ago: u64) {
    let path = folder.join(name);
    fs::write(&path, change.to_string()).unwrap();
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(ago);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(when)
        .unwrap();
}

#[test]
fn a_collected_round_reads_its_witness() {
    assert_eq!(read(&[round(&json!({}))]), [id('a')]);
    assert_eq!(
        read(&[round(
            &json!({"round": "stack", "witness": null, "witnesses": [id('c'), id('d')]})
        )]),
        [id('c'), id('d')]
    );
    assert_eq!(
        read(&[round(&json!({"outcomes": [
            {"participantId": "a", "failure": "timed out"},
            {"participantId": "b", "stopped": true, "findingIds": [2]},
        ]}))]),
        [id('a')]
    );
}

// A witness that isn't a full commit id, which git would read as
// whatever it names now, stands for no read.
#[test]
fn a_witness_that_is_no_commit_id_reads_nothing() {
    for witness in [
        "main",
        "HEAD",
        "aaa",
        &id('A'),
        &id('g'),
        &format!("{}a", id('a')),
    ] {
        assert!(
            read(&[round(&json!({"witness": witness}))]).is_empty(),
            "{witness}"
        );
    }
}

// A round whose every reviewer was stopped before it filed anything
// speaks for no head. One of four reviewers all cancelled, with no
// failure written, once passed as read.
#[test]
fn a_round_whose_reviewers_were_all_stopped_unheard_reads_nothing() {
    let cancelled = json!({"limit": "cancelled", "detail": "sent SIGTERM."});
    let outcomes: Vec<Value> = ["correctness", "test-skeptic", "operational-safety"]
        .into_iter()
        .map(|who| json!({"participantId": who, "findingIds": [], "stopped": cancelled}))
        .chain([json!({"participantId": "security-attacker", "stopped": cancelled})])
        .collect();
    assert!(
        read(&[round(&json!({"outcomes": outcomes}))]).is_empty(),
        "a round whose reviewers were all cancelled was counted as reading its head"
    );
    let walled = json!({"participantId": "a", "findingIds": [],
        "stopped": {"limit": "wall-clock"}});
    assert!(
        read(&[round(&json!({"outcomes": [walled]}))]).is_empty(),
        "a reviewer stopped at its wall with nothing filed was counted as answering"
    );
    let filed = json!({"participantId": "a", "findingIds": [3],
        "stopped": {"limit": "soft-deadline"}});
    assert_eq!(
        read(&[round(&json!({"outcomes": [walled, filed]}))]),
        [id('a')],
        "a reviewer stopped after filing a finding wasn't counted as answering"
    );
    let clean = json!({"participantId": "a", "findingIds": [], "answerPath": "/x"});
    assert_eq!(
        read(&[round(&json!({"outcomes": [clean]}))]),
        [id('a')],
        "a reviewer that finished finding nothing wasn't counted as answering"
    );
    for silent in [
        json!({"participantId": "a"}),
        json!({"participantId": "a", "findingIds": []}),
        json!({"participantId": "a", "findingIds": [], "answerPath": null}),
        json!({"participantId": "a", "findingIds": [], "skipped": true}),
    ] {
        assert!(
            read(&[round(&json!({"outcomes": [silent]}))]).is_empty(),
            "an outcome leaving no answer, {silent}, was counted as answering"
        );
    }
}

// A judge or critique, which read findings rather than code, a round
// still running, given up on with nothing behind it, read from another
// tree than its witness, or with every reviewer failed speaks for no
// head.
#[test]
fn a_round_that_didnt_read_its_witness_in_full_reads_nothing() {
    for extra in [
        json!({"round": "judge"}),
        json!({"round": "critique"}),
        json!({"open": true}),
        json!({"closed": true}),
        json!({"unpinned": {"read": "checkout"}}),
        json!({"outcomes": [{"participantId": "a", "failure": "timed out"}]}),
        json!({"outcomes": []}),
        json!({"startedAt": null}),
    ] {
        assert!(read(&[round(&extra)]).is_empty(), "{extra}");
    }
    assert_eq!(read_in("not json"), Vec::<Read>::new());
}

// A change's rounds read its commit; a range's read it from the
// range's base, which the net checks is on main.
#[test]
fn every_change_s_rounds_are_read_from_the_folder() {
    let folder = folder("read");
    written(
        &folder,
        "github_github_o_r_1.json",
        &json!({
            "change": {"provider": "github", "id": "o/r#1"},
            "runs": [round(&json!({"from": id('8')}))],
        }),
        0,
    );
    written(
        &folder,
        "local-range_github_o_r_main..c.json",
        &json!({
            "change": {"provider": "local-range", "id": "origin/main..c"},
            "runs": [round(&json!({"witness": id('c'), "from": id('8')}))],
        }),
        0,
    );
    written(
        &folder,
        "local-range_github_o_r_main..b.json",
        &json!({
            "change": {"provider": "local-range", "id": "origin/main..b"},
            "runs": [round(&json!({"witness": id('b')}))],
        }),
        0,
    );
    written(
        &folder,
        "notes.txt",
        &json!({"change": {}, "runs": [round(&json!({"witness": id('d')}))]}),
        0,
    );
    let seen = witnessed(&folder).unwrap();
    fs::remove_dir_all(&folder).unwrap();
    let ranged = Read {
        commit: id('b'),
        from: Some("origin/main".to_owned()),
    };
    let recorded = Read {
        commit: id('c'),
        from: Some(id('8')),
    };
    // A change's round keeps no base even when it records one, since
    // the host keeps a change's base.
    assert_eq!(
        seen,
        HashSet::from([change_read('a'), ranged, recorded]),
        "{seen:?}"
    );
}

// A range round reads from the commit it recorded, whatever the range
// is named; a recorded base that's no commit (a ref, null, a number) is
// none, so the round counts for no head; and only a round recording
// nothing falls back to the range's name.
#[test]
fn a_range_round_reads_from_the_base_it_records() {
    let folder = folder("recorded");
    let cases = [
        ("origin/main", json!(id('8')), Some(id('8'))),
        (id('9').as_str(), json!(id('8')), Some(id('8'))),
        ("origin/main", json!("origin/main"), Some(String::new())),
        (id('9').as_str(), json!("origin/main"), Some(String::new())),
        ("origin/main", json!(null), Some(String::new())),
        (id('9').as_str(), json!(null), Some(String::new())),
        (id('9').as_str(), json!(8), Some(String::new())),
        (
            id('9').as_str(),
            json!(id('e').to_uppercase()),
            Some(String::new()),
        ),
    ]
    .map(|(base, from, read)| (base.to_owned(), Some(from), read));
    let absent = [
        ("origin/main", Some("origin/main".to_owned())),
        (id('9').as_str(), Some(id('9'))),
    ]
    .map(|(base, read)| (base.to_owned(), None, read));
    for (base, from, read) in cases.into_iter().chain(absent) {
        let mut witness = json!({"witness": id('c')});
        if let Some(from) = &from {
            witness["from"] = from.clone();
        }
        written(
            &folder,
            "local-range_github_o_r_x.json",
            &json!({
                "change": {"provider": "local-range", "id": format!("{base}..c")},
                "runs": [round(&witness)],
            }),
            0,
        );
        let seen = witnessed(&folder).unwrap();
        assert_eq!(
            seen,
            HashSet::from([Read {
                commit: id('c'),
                from: read.clone()
            }]),
            "a round on {base}..c recording {from:?} read from the wrong base"
        );
    }
    fs::remove_dir_all(&folder).unwrap();
}

// A folder not made yet holds no round, as before the first is asked;
// one that can't be read is said so.
#[test]
fn a_folder_not_made_yet_holds_no_round_and_an_unreadable_one_is_said_so() {
    let folder = folder("gone");
    assert_eq!(witnessed(&folder.join("gone")), Ok(HashSet::new()));
    fs::write(folder.join("file"), "").unwrap();
    let said = witnessed(&folder.join("file")).unwrap_err();
    fs::remove_dir_all(&folder).unwrap();
    assert!(said.contains("can't be read"), "{said}");
}

// Each node's rounds are read, a link to a folder's too, in name order
// past a stray file; a root not made yet mirrors nothing.
#[test]
fn every_mirrored_node_is_read_in_name_order() {
    let root = folder("mirrors");
    let elsewhere = folder("mirrored-elsewhere");
    let change =
        |commit: char| json!({"change": {}, "runs": [round(&json!({"witness": id(commit)}))]});
    let range = |base: &str, commit: char| {
        json!({
            "change": {"provider": "local-range", "id": format!("{base}..x")},
            "runs": [round(&json!({"witness": id(commit)}))],
        })
    };
    fs::create_dir(root.join("zebra")).unwrap();
    written(&root.join("zebra"), "one.json", &change('b'), 0);
    written(&elsewhere, "two.json", &change('c'), 0);
    std::os::unix::fs::symlink(&elsewhere, root.join("aardvark")).unwrap();
    fs::write(root.join("stray"), "").unwrap();
    fs::create_dir(root.join(".sync-tool")).unwrap();
    written(&root.join(".sync-tool"), "three.json", &change('d'), 0);
    fs::create_dir(root.join("ranges")).unwrap();
    written(
        &root.join("ranges"),
        "by-ref.json",
        &range("origin/main", 'e'),
        0,
    );
    written(
        &root.join("ranges"),
        "by-commit.json",
        &range(&id('9'), 'f'),
        0,
    );
    // A range named by ref whose round recorded the commit it read from
    // counts from that commit, though the name alone would be blanked.
    written(
        &root.join("ranges"),
        "by-ref-recorded.json",
        &json!({
            "change": {"provider": "local-range", "id": "origin/main..x"},
            "runs": [round(&json!({"witness": id('7'), "from": id('8')}))],
        }),
        0,
    );
    let mirrored = mirrored(&root);
    let gone = super::mirrored(&root.join("gone"));
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(&elsewhere).unwrap();
    assert_eq!(
        mirrored.asked,
        ["aardvark", "ranges", "zebra"],
        "the nodes asked, a hidden one left out"
    );
    let by_ref = Read {
        commit: id('e'),
        from: Some(String::new()),
    };
    let by_commit = Read {
        commit: id('f'),
        from: Some(id('9')),
    };
    let recorded = Read {
        commit: id('7'),
        from: Some(id('8')),
    };
    assert_eq!(
        mirrored.rounds,
        HashSet::from([
            change_read('b'),
            change_read('c'),
            by_ref,
            by_commit,
            recorded,
        ]),
        "the rounds read, a range's base kept only when it names a commit"
    );
    assert!(mirrored.trouble.is_empty(), "{:?}", mirrored.trouble);
    assert!(
        gone.asked.is_empty() && gone.trouble.is_empty() && gone.rounds.is_empty(),
        "a root not made yet mirrored something: {gone:?}"
    );
}

// A node that can't be read is said and left out, so the rest still
// count; one holding a runs folder is said to be mirrored too high.
#[test]
fn a_mirror_that_cant_be_read_is_said_and_the_rest_count() {
    let root = folder("mirrors-trouble");
    std::os::unix::fs::symlink(root.join("nowhere"), root.join("dangling")).unwrap();
    fs::create_dir_all(root.join("high/runs")).unwrap();
    fs::create_dir(root.join("good")).unwrap();
    written(
        &root.join("good"),
        "one.json",
        &json!({"change": {}, "runs": [round(&json!({}))]}),
        0,
    );
    fs::create_dir(root.join("newer")).unwrap();
    for each in 0..SHAPE_JUDGED_BY {
        written(
            &root.join("newer"),
            &format!("new-{each}.json"),
            &json!({"change": {}, "rounds": [round(&json!({}))]}),
            0,
        );
    }
    let mirrored = mirrored(&root);
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        mirrored.asked,
        ["good"],
        "the nodes asked, one mirrored too high left out"
    );
    assert_eq!(
        mirrored.rounds,
        HashSet::from([change_read('a')]),
        "the good node's rounds"
    );
    assert!(
        mirrored.trouble.len() == 3
            && mirrored.trouble[0].starts_with("the mirror from dangling can't be read")
            && mirrored.trouble[1].contains("from high holds a runs folder")
            && mirrored.trouble[2].contains("mirrored from newer hold no round of the shape")
            && !mirrored
                .trouble
                .iter()
                .any(|said| said.contains("review/runs")),
        "a mirror's trouble wasn't said as a mirror's: {:?}",
        mirrored.trouble
    );
}

// The mirrors sit beside the rounds folder, wherever that is set.
#[test]
fn the_mirrors_sit_beside_the_rounds() {
    assert_eq!(
        mirrors_beside(Path::new("/state/pi/agentic-harness.pi/review/runs")),
        Path::new("/state/pi/agentic-harness.pi/review/runs-from"),
        "the mirrors folder isn't beside the rounds folder"
    );
}

// Newest round files of another shape, as a change to the harness's
// records would write them, are said so rather than read as no head
// ever reviewed, whatever older files hold; files of the shape whose
// rounds haven't been collected are just unread.
#[test]
fn newest_round_files_of_another_shape_are_said_so() {
    let folder = folder("shape");
    assert_eq!(witnessed(&folder), Ok(HashSet::new()), "an empty folder");
    written(
        &folder,
        "old.json",
        &json!({"change": {}, "runs": [round(&json!({}))]}),
        86_400,
    );
    written(
        &folder,
        "open.json",
        &json!({"change": {}, "runs": [round(&json!({"witness": id('b'), "open": true}))]}),
        60,
    );
    assert_eq!(
        witnessed(&folder),
        Ok(HashSet::from([change_read('a')])),
        "a round still open isn't another shape"
    );
    for each in 0..SHAPE_JUDGED_BY {
        written(
            &folder,
            &format!("new-{each}.json"),
            &json!({"change": {}, "rounds": [round(&json!({}))]}),
            0,
        );
    }
    let said = witnessed(&folder).unwrap_err();
    fs::remove_dir_all(&folder).unwrap();
    assert!(said.contains("record shape"), "{said}");
}
