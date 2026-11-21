use std::sync::mpsc;

use super::*;

fn call(folder: &Path, key: &str, started: u64) -> Record {
    Record {
        key: key.to_owned(),
        title: "Planning".to_owned(),
        started,
        ended: None,
        event: Some("e1".to_owned()),
        person: None,
        account: None,
        people: Vec::new(),
        known: None,
        folder: folder.join(key),
        transcript: None,
        opening: None,
        summary: None,
        transcribing: true,
    }
}

fn ask(book: &mut Book, line: &str) -> (Value, mpsc::Receiver<String>) {
    let (told, heard) = mpsc::channel();
    (book.answer(line, 1, &told), heard)
}

fn present(name: &str, joined: u64, left: Option<u64>) -> Present {
    Present {
        name: name.to_owned(),
        joined,
        left,
    }
}

#[test]
fn who_was_in_a_meet_is_its_newest_look_and_whoever_that_leaves_out() {
    let mut book = Book::default();
    book.present(
        "call:e1",
        &[present("Ada", 10, Some(50)), present("Bo", 20, None)],
    );
    book.present(
        "call:e1",
        &[
            present("Ada", 10, None),
            present("Cy", 70, None),
            present("Cy", 80, Some(90)),
        ],
    );
    assert_eq!(
        book.presence("call:e1"),
        [
            present("Ada", 10, None),
            present("Cy", 70, None),
            present("Cy", 80, Some(90)),
            present("Bo", 20, None),
        ],
        "a rejoin kept the old leave, one person's two devices became one, or one left out was lost"
    );
    assert!(
        book.presence("call:e2").is_empty(),
        "a call whose Meet wasn't seen was given someone's"
    );
}

#[test]
fn a_watch_hears_each_call_as_it_changes_and_lists_are_newest_first() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    let (told, heard) = mpsc::channel();
    let held = book.answer(r#"{"id": 4, "ask": "watch", "kind": "call"}"#, 1, &told);
    assert_eq!(held, json!({"id": 4, "records": []}));
    book.put(call(folder.path(), "call:e1", 10));
    book.put(call(folder.path(), "call:e2", 20));
    let changed: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
    assert_eq!(changed["id"], 4);
    assert_eq!(changed["changed"][0]["key"], "call:e1");
    assert_eq!(changed["changed"][0]["transcribing"], true);
    let (listed, _) = ask(&mut book, r#"{"id": 5, "ask": "list", "limit": 1}"#);
    assert_eq!(listed["records"].as_array().unwrap().len(), 1);
    assert_eq!(listed["records"][0]["key"], "call:e2");
}

#[test]
fn a_call_is_read_back_from_its_folder_and_one_cut_short_has_ended() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    let mut record = call(folder.path(), "call:e1", 10);
    book.put(record.clone());
    std::fs::write(
        record.folder.join(TRANSCRIPT),
        "# Transcript\n\n**Joel**, 0:00: Can we move it?\n",
    )
    .unwrap();
    let read = Book::read(folder.path());
    let back = read.record("call:e1").unwrap();
    assert!(back.ended.is_some(), "the daemon stopped during it");
    assert!(!back.transcribing, "a call cut short is still transcribing");
    assert_eq!(back.transcript, Some(record.folder.join(TRANSCRIPT)));
    assert_eq!(
        back.opening.as_deref(),
        Some("**Joel**, 0:00: Can we move it?")
    );
    record.ended = Some(99);
    book.put(record);
    assert_eq!(
        Book::read(folder.path()).record("call:e1").unwrap().ended,
        Some(99)
    );
}

#[test]
fn the_call_recorded_now_is_served_as_recording_and_only_while_it_is() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    book.put(call(folder.path(), "call:e1", 10));
    book.put(call(folder.path(), "call:e2", 20));
    let (told, heard) = mpsc::channel();
    book.answer(r#"{"id": 4, "ask": "watch", "kind": "call"}"#, 1, &told);
    let changed = || -> Value { serde_json::from_str(&heard.try_recv().unwrap()).unwrap() };
    let recording = |book: &mut Book| -> Vec<Value> {
        let (listed, _) = ask(book, r#"{"id": 5, "ask": "list"}"#);
        listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["recording"].clone())
            .collect()
    };
    assert_eq!(recording(&mut book), [false, false], "nothing recorded yet");

    book.recording(Some("call:e1"));
    let said = changed();
    assert_eq!(said["changed"][0]["key"], "call:e1");
    assert_eq!(
        said["changed"][0]["recording"], true,
        "its watch isn't told"
    );
    assert_eq!(recording(&mut book), [false, true], "e2 is listed first");
    book.recording(Some("call:e1"));
    assert!(heard.try_recv().is_err(), "said again, a watch is told");
    book.put(call(folder.path(), "call:e1", 10));
    assert_eq!(
        changed()["changed"][0]["recording"],
        true,
        "a call's record kept again while it's recorded says it isn't"
    );

    book.recording(Some("call:e2"));
    let (was, now) = (changed(), changed());
    assert_eq!(
        (&was["changed"][0]["key"], &was["changed"][0]["recording"]),
        (&json!("call:e1"), &json!(false)),
        "the call it was isn't told it's no longer recorded"
    );
    assert_eq!(now["changed"][0]["key"], "call:e2");
    assert_eq!(now["changed"][0]["recording"], true);

    book.recording(None);
    assert_eq!(
        changed()["changed"][0]["recording"],
        false,
        "its end isn't told"
    );
    assert_eq!(
        recording(&mut book),
        [false, false],
        "a call ended still records"
    );
}

#[test]
fn a_call_read_back_after_the_daemon_stopped_mid_call_is_not_recording() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    let record = call(folder.path(), "call:e1", 10);
    book.put(record.clone());
    book.recording(Some("call:e1"));
    // Kept again while recorded, as hear keeps it with each line.
    book.put(record.clone());
    let meta = std::fs::read_to_string(record.folder.join(META)).unwrap();
    let meta: Value = serde_json::from_str(&meta).unwrap();
    assert!(
        meta.get("recording").is_none(),
        "being recorded was written beside the call: {meta}"
    );

    // Stopped mid-call, it starts again from the folder alone.
    let mut read = Book::read(folder.path());
    let (listed, _) = ask(&mut read, r#"{"id": 6, "ask": "list"}"#);
    assert_eq!(listed["records"][0]["key"], "call:e1");
    assert_eq!(
        listed["records"][0]["recording"], false,
        "a call read back says it's still being recorded"
    );
}

#[test]
fn a_calls_summary_is_read_from_its_one_file_or_from_the_summary_md_it_had_before() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    let said = "# Transcript\n\n**Joel**, 0:00: Can we move it?\n";
    let one = call(folder.path(), "call:e1", 10);
    let before = call(folder.path(), "call:e2", 20);
    let pending = call(folder.path(), "call:e3", 30);
    let rejoined = call(folder.path(), "call:e4", 40);
    for record in [&one, &before, &pending, &rejoined] {
        book.put(record.clone());
    }
    // A summary paragraph led as a line said is, so only the Transcript
    // section gives the opening.
    let summed = "# Planning\n\n**Ada**, 0:09: picked\n\n- Joel: \"move it\"";
    let file = filed::whole("Planning", Some(summed), said);
    std::fs::write(one.folder.join(TRANSCRIPT), file).unwrap();
    std::fs::write(before.folder.join(TRANSCRIPT), said).unwrap();
    std::fs::write(before.folder.join(SUMMARY), "# Planning\n\n- kept\n").unwrap();
    let file = filed::whole("Planning", None, said);
    std::fs::write(pending.folder.join(TRANSCRIPT), &file).unwrap();
    std::fs::write(rejoined.folder.join(TRANSCRIPT), &file).unwrap();
    std::fs::write(rejoined.folder.join(SUMMARY), "# Planning\n\n- first leg\n").unwrap();
    let read = Book::read(folder.path());
    let summary = |key| read.record(key).unwrap().summary.clone();
    let opening = |key| read.record(key).unwrap().opening.clone();
    assert_eq!(
        summary("call:e1").as_deref(),
        Some("**Ada**, 0:09: picked\n\n- Joel: \"move it\""),
        "one file"
    );
    assert_eq!(
        opening("call:e1").as_deref(),
        Some("**Joel**, 0:00: Can we move it?"),
        "its opening is the first line said"
    );
    assert_eq!(
        summary("call:e2").as_deref(),
        Some("- kept"),
        "before, kept without its title as the one file's is"
    );
    assert_eq!(
        summary("call:e3"),
        None,
        "a summary still to come is none yet"
    );
    assert_eq!(
        summary("call:e4"),
        None,
        "rejoined since it was summed up, its summary.md is of its first leg alone"
    );
}

#[test]
fn the_one_other_person_names_their_side_and_the_note_says_how_they_re_known() {
    let folder = tempfile::tempdir().unwrap();
    let invited = Record {
        people: vec!["Ada Lovelace".to_owned()],
        known: Some(Known::Invitation),
        ..call(folder.path(), "call:e1", 10)
    };
    assert_eq!(invited.them("Joel").as_deref(), Some("Ada Lovelace"));
    assert_eq!(
        invited.note("Joel").as_deref(),
        Some("Their side is Ada Lovelace, the one other person invited.")
    );
    let tuple = Record {
        known: Some(Known::Tuple),
        ..invited.clone()
    };
    assert_eq!(
        tuple.note("Joel").as_deref(),
        Some("Their side is Ada Lovelace, the one other person on the Tuple call.")
    );
    let meet = Record {
        known: Some(Known::Meet),
        ..invited.clone()
    };
    assert_eq!(
        meet.note("Joel").as_deref(),
        Some("Their side is Ada Lovelace, the one other person in the Meet."),
        "a Meet's one other person wasn't said to be from the Meet"
    );
    let mut book = Book::default();
    book.put(meet.clone());
    let back = Book::read(folder.path());
    assert_eq!(
        back.record("call:e1").and_then(|back| back.known),
        Some(Known::Meet),
        "a Meet's people weren't read back as the Meet's"
    );
    book.put(invited.clone());
    let back = Book::read(folder.path());
    let back = back.record("call:e1").unwrap();
    assert_eq!(back.people, invited.people, "its people are kept beside it");
    assert_eq!(back.known, Some(Known::Invitation));
}

#[test]
fn several_others_name_no_side_and_the_note_lists_them() {
    let folder = tempfile::tempdir().unwrap();
    let names = |count: usize| (1..=count).map(|n| format!("P{n}")).collect::<Vec<_>>();
    let two = Record {
        people: vec!["Ada".to_owned(), "Bo".to_owned()],
        known: Some(Known::Invitation),
        ..call(folder.path(), "call:e1", 10)
    };
    assert_eq!(two.them("Joel"), None);
    assert_eq!(two.note("Joel").as_deref(), Some("Invited: Ada and Bo."));
    let many = Record {
        people: names(11),
        known: Some(Known::Tuple),
        ..two.clone()
    };
    assert_eq!(
        many.note("Joel").as_deref(),
        Some("On the Tuple call: P1, P2, P3, P4, P5, P6, P7, P8 and 3 more.")
    );
    let meet = Record {
        known: Some(Known::Meet),
        ..two.clone()
    };
    assert_eq!(
        meet.note("Joel").as_deref(),
        Some("In the Meet: Ada and Bo."),
        "a Meet's people weren't listed as in it"
    );
    let namesake = Record {
        people: vec!["Joel".to_owned()],
        ..two.clone()
    };
    assert_eq!(
        namesake.them("Joel"),
        None,
        "called what Joel is, so Others"
    );
    assert_eq!(namesake.note("Joel").as_deref(), Some("Invited: Joel."));
    let nobody = call(folder.path(), "call:e2", 20);
    assert_eq!((nobody.them("Joel"), nobody.note("Joel")), (None, None));
}

#[test]
fn a_tuple_call_kept_before_its_people_were_is_named_by_its_title_when_that_is_one() {
    let folder = tempfile::tempdir().unwrap();
    let before = Record {
        title: "Ada *Love*\nlace".to_owned(),
        person: Some("ada@example.com".to_owned()),
        ..call(folder.path(), "call:person:ada@example.com:10", 10)
    };
    assert_eq!(
        before.them("Joel").as_deref(),
        Some("Ada Love lace"),
        "named by its title, cleaned as a speaker"
    );
    assert_eq!(before.note("Joel"), None, "how they're known wasn't kept");
    let several = Record {
        title: "Ada and 1 more".to_owned(),
        ..before.clone()
    };
    assert_eq!(several.them("Joel"), None, "several, so Others");
    let known = Record {
        known: Some(Known::Tuple),
        ..before.clone()
    };
    assert_eq!(known.them("Joel"), None, "kept since, its people say who");
}

#[test]
fn a_call_s_account_is_kept_beside_it_and_never_served() {
    let folder = tempfile::tempdir().unwrap();
    let mut book = Book::default();
    let record = Record {
        account: Some("jsama@example.com".to_owned()),
        ..call(folder.path(), "call:e1", 10)
    };
    book.put(record.clone());
    let read = Book::read(folder.path());
    assert_eq!(
        read.record("call:e1").unwrap().account.as_deref(),
        Some("jsama@example.com"),
        "the account kept with the call wasn't read back"
    );
    assert!(
        !book.served(&record).to_string().contains("jsama"),
        "the shell was served the call's account"
    );
    book.put(call(folder.path(), "call:e2", 20));
    assert_eq!(
        Book::read(folder.path()).record("call:e2").unwrap().account,
        None,
        "a call kept with no account read back as one's"
    );
}

#[test]
fn an_opening_is_the_first_paragraphs_cut_at_a_word() {
    assert_eq!(opening("# Transcript\n"), None);
    let said =
        "# Transcript\n\nInvited: Ada and Bo.\n\n**Joel**, 0:00: Hi.\n\n**Others**, 0:02: Hello.\n";
    assert_eq!(
        opening(said).as_deref(),
        Some("**Joel**, 0:00: Hi.\n\n**Others**, 0:02: Hello.")
    );
    let many: Vec<String> = (0..9)
        .map(|n| format!("**Joel**, 0:0{n}: Line {n}."))
        .collect();
    assert_eq!(
        opening(&format!("# Transcript\n\n{}", many.join("\n\n")))
            .unwrap()
            .matches("**Joel**")
            .count(),
        OPENING_PARAGRAPHS
    );
    let long = format!("# Transcript\n\n**Others**, 0:00: {}", "word ".repeat(200));
    let cut = opening(&long).unwrap();
    assert!(cut.chars().count() <= OPENING_CHARACTERS + 1, "{cut}");
    assert!(cut.ends_with("word\u{2026}"), "{cut}");
}

fn planted(calls: &Path, name: &str, meta: &[u8]) {
    let folder = calls.join(name);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join(META), meta).unwrap();
}

#[test]
fn a_record_too_large_or_malformed_is_skipped_and_counted_as_failing() {
    let calls = tempfile::tempdir().unwrap();
    let good = json!({"key": "call:at:1", "started": 1, "ended": 2, "title": ""});
    planted(calls.path(), "good", good.to_string().as_bytes());
    assert!(
        !Book::read(calls.path()).failing(),
        "a sound record reads as failing"
    );
    let large = json!({
        "key": "call:at:2", "started": 2, "ended": 3, "title": "a".repeat(70 * 1024),
    });
    planted(calls.path(), "large", large.to_string().as_bytes());
    let read = Book::read(calls.path());
    assert!(
        read.record("call:at:2").is_none(),
        "a record past 64 KiB is read"
    );
    assert!(read.failing(), "a record past 64 KiB isn't counted failing");
    std::fs::remove_dir_all(calls.path().join("large")).unwrap();
    for (name, meta) in [
        ("cut", &b"{\"key\": \"call:at:3\", \"start"[..]),
        ("worded", br#"{"key": "call:at:4", "started": "soon"}"#),
        ("keyless", br#"{"started": 4}"#),
        ("listed", b"[1, 2, 3]"),
        ("bytes", &[0xff, 0xfe, 0x00, 0x7b][..]),
    ] {
        planted(calls.path(), name, meta);
        let read = Book::read(calls.path());
        assert!(read.failing(), "{name} reads as sound");
        assert_eq!(read.starts().collect::<Vec<_>>(), [1], "{name}");
        std::fs::remove_dir_all(calls.path().join(name)).unwrap();
    }
}

#[test]
fn a_record_linked_from_elsewhere_or_a_pipe_is_skipped_at_once_and_failing() {
    let calls = tempfile::tempdir().unwrap();
    let elsewhere = calls.path().join("elsewhere.json");
    let sound = json!({"key": "call:at:5", "started": 5, "ended": 6, "title": ""});
    std::fs::write(&elsewhere, sound.to_string()).unwrap();
    std::fs::create_dir_all(calls.path().join("linked")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, calls.path().join("linked").join(META)).unwrap();
    std::fs::create_dir_all(calls.path().join("piped")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(calls.path().join("piped").join(META))
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo made no pipe: {made}");
    // Read on its own thread, so a read that waits on the pipe fails
    // here rather than hanging the run.
    let (told, heard) = std::sync::mpsc::channel();
    let folder = calls.path().to_owned();
    std::thread::spawn(move || {
        let read = Book::read(&folder);
        let _ = told.send((read.failing(), read.starts().collect::<Vec<_>>()));
    });
    let read = heard.recv_timeout(std::time::Duration::from_secs(10));
    assert_eq!(
        read,
        Ok((true, vec![])),
        "a linked record or a pipe was read, or waited on"
    );
}

#[test]
fn a_folder_with_no_record_or_no_calls_folder_is_no_failure() {
    let calls = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(calls.path().join("elsewhere")).unwrap();
    std::fs::write(calls.path().join("notes.md"), "# Notes\n").unwrap();
    assert!(
        !Book::read(calls.path()).failing(),
        "a folder with no record, or a file beside them, reads as failing"
    );
    assert!(
        !Book::read(&calls.path().join("none yet")).failing(),
        "no calls folder yet reads as failing"
    );
}

#[test]
fn a_calls_folder_that_cant_be_listed_is_failing() {
    let calls = tempfile::tempdir().unwrap();
    let file = calls.path().join("a file");
    std::fs::write(&file, "").unwrap();
    assert!(
        Book::read(&file).failing(),
        "a calls folder that can't be listed reads as sound"
    );
}

#[test]
fn another_kind_or_ask_is_refused_by_its_code() {
    let mut book = Book::default();
    let (kind, _) = ask(&mut book, r#"{"id": 1, "ask": "list", "kind": "person"}"#);
    assert_eq!(kind["error"]["code"], "unknown-kind");
    let (other, _) = ask(&mut book, r#"{"id": 2, "ask": "shout"}"#);
    assert_eq!(other["error"]["code"], "unknown-ask");
    let (hello, _) = ask(&mut book, r#"{"id": 3, "ask": "hello", "version": 1}"#);
    assert_eq!(hello["domain"], "calls");
}
