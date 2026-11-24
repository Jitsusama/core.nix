//! Threads from the mesh of 17 November and the shapes around them: who
//! takes a pain up, which landing closes it, and what's read to know.

use super::*;

fn heard(entry: &str) -> Heard {
    serde_json::from_value(serde_json::json!({
        "key": format!("s/{entry}"), "session": "s", "entry": entry, "at": 0,
        "words": "i can't show the launcher", "kinds": ["corrected"], "hits": 1,
        "carried": false
    }))
    .expect("a pain")
}

fn message(id: &str, from: &str, to: &str, re: Option<&str>, text: &str) -> Message {
    Message {
        id: id.into(),
        from: from.into(),
        to: vec![to.into()],
        re: re.map(Into::into),
        text: text.into(),
    }
}

/// The Tuple pain of 17 November, as the mesh carried it to its fix: the
/// pivot replies with the question's whole id, as the mesh writes it.
fn tuple() -> Vec<Message> {
    vec![
        message(
            "1-6LU6ZM-1-186eeb39",
            "6LU6ZM",
            "@WCJQSG",
            None,
            "Joel hit something in mycelium:cell/LR3WMN#e1: \"i am not able\"",
        ),
        message(
            "2-WCJQSG-1-1adf6d68",
            "WCJQSG",
            "@Y2YBTS",
            Some("1-6LU6ZM-1-186eeb39"),
            "Show the launcher in a shared screen",
        ),
        message(
            "3-X-1-aaaaaaaa",
            "X",
            "#orchestration/wcjqsg",
            None,
            "1adf6d68 and more",
        ),
        message(
            "4-Y-1-bbbbbbbb",
            "Y",
            "#orchestration/wcjqsg",
            Some("aaaaaaaa"),
            "Y landed core #1, merged at 99999999abcd",
        ),
        message(
            "5-Y2YBTS-1-a85417d8",
            "Y2YBTS",
            "#orchestration/wcjqsg",
            None,
            "Y2YBTS landed core #5342, merged at 255494bb414c (12:38), for WCJQSG's pivot \
             1adf6d68",
        ),
    ]
}

fn stands(entry: &str, held: &[&str], messages: &[Message], on: bool) -> Standing {
    standing(&heard(entry), "6LU6ZM", held, messages, &mut |_| on)
}

fn taken(by: &str) -> Standing {
    Standing {
        taken_by: Some(by.into()),
        closed_by: None,
        closed_at: None,
    }
}

#[test]
fn a_pain_closes_once_a_switch_carries_the_landing_its_thread_led_to() {
    let mut asked = Vec::new();
    let mut on = |commit: &str| {
        asked.push(commit.to_owned());
        commit == "255494bb414c"
    };
    assert_eq!(
        standing(&heard("e1"), "6LU6ZM", &["e1"], &tuple(), &mut on),
        Standing {
            taken_by: Some("WCJQSG".into()),
            closed_by: Some("255494bb414c".into())
        },
        "the pivot took it up and its landing closed it"
    );
    assert_eq!(
        asked,
        ["255494bb414c"],
        "a landing re a note to a group is no landing of this pain"
    );
}

#[test]
fn a_landing_no_switch_carries_leaves_it_taken() {
    assert_eq!(
        stands("e1", &["e1"], &tuple(), false),
        taken("WCJQSG"),
        "taken, not closed"
    );
    assert_eq!(
        standing(&heard("e1"), "6lu6zm", &["e1"], &tuple(), &mut |_| false),
        taken("WCJQSG"),
        "the asker's address in any case"
    );
}

#[test]
fn an_unanswered_question_or_the_keepers_note_takes_nothing_up() {
    assert_eq!(
        stands("e1", &["e1"], &tuple()[..1], true),
        Standing::default(),
        "nobody answered"
    );
    let mut noted = tuple();
    noted[0].text = format!(
        "1 new pain heard from Joel, {FOR_THE_KEEPER}: {}",
        noted[0].text
    );
    assert_eq!(
        stands("e1", &["e1"], &noted, true),
        Standing::default(),
        "the keeper's note asks nothing of anyone"
    );
    assert_eq!(
        stands("e9", &["e9"], &tuple(), true),
        Standing::default(),
        "a pain never asked of anyone"
    );
}

#[test]
fn a_question_holding_several_pains_is_taken_up_but_closed_by_no_landing() {
    let mut shared = tuple();
    shared[0].text.push_str(" and mycelium:cell/OMNOJW#e2");
    assert_eq!(
        stands("e1", &["e1", "e2"], &shared, true),
        taken("WCJQSG"),
        "the answer took both up, and one landing fixed neither for sure"
    );
}

#[test]
fn a_note_to_a_group_takes_nothing_up_and_a_mention_grows_no_thread() {
    let messages = [
        tuple().remove(0),
        message(
            "2-Z-1-cccccccc",
            "Z",
            "#orchestration/wcjqsg",
            None,
            "where things stand: 186eeb39, 5b05e833 and more",
        ),
        message("3-W-1-dddddddd", "W", "@V", None, "V: please take 186eeb39"),
        message(
            "4-V-1-eeeeeeee",
            "V",
            "@W",
            Some("3-W-1-dddddddd"),
            "V landed core #9, merged at 99999999abcd",
        ),
    ];
    assert_eq!(
        stands("e1", &["e1"], &messages, true),
        taken("W"),
        "named to a peer, taken; its reply is no reply to the question"
    );
}

#[test]
fn a_status_note_naming_several_messages_closes_no_pain() {
    let mut messages = tuple();
    messages.insert(
        1,
        message("1-6LU6ZM-1-22222222", "6LU6ZM", "@WCJQSG", None, "in #e2"),
    );
    let status = "Z landed core #9, merged at 255494bb414c; on 186eeb39 and 22222222";
    messages[5] = message("5-Z-1-cccccccc", "Z", "#orchestration/wcjqsg", None, status);
    assert_eq!(
        stands("e1", &["e1", "e2"], &messages, true),
        taken("WCJQSG"),
        "which of the two it answers isn't said"
    );
    messages[5].text = "Z landed core #9, merged at 255494bb414c, for 186eeb39".into();
    assert_eq!(
        stands("e1", &["e1", "e2"], &messages, true)
            .closed_by
            .as_deref(),
        Some("255494bb414c"),
        "one named, it answers that one"
    );
}

#[test]
fn a_pain_asked_again_is_followed_from_either_question() {
    let mut messages = tuple();
    messages.insert(
        1,
        message(
            "1-6LU6ZM-2-33333333",
            "6LU6ZM",
            "@WCJQSG",
            None,
            "again: #e1",
        ),
    );
    messages[2].re = Some("33333333".into());
    assert_eq!(
        stands("e1", &["e1"], &messages, false),
        taken("WCJQSG"),
        "the answer to the second ask"
    );
}

#[test]
fn an_entry_is_named_whole() {
    let mut messages = tuple();
    messages[0].text = "Joel hit something in mycelium:cell/LR3WMN#e10".into();
    assert_eq!(
        stands("e1", &["e1", "e10"], &messages, true),
        Standing::default(),
        "#e10 isn't #e1"
    );
    assert!(names("in #e1, and", "e1"), "before a comma");
    assert!(names("in #e1", "e1"), "at the end");
}

#[test]
fn a_note_of_two_landings_closes_nothing() {
    let mut two = tuple();
    two[4].text.push_str(", and #5343 merged at 3333333abcd");
    assert_eq!(
        stands("e1", &["e1"], &two, true),
        taken("WCJQSG"),
        "which of the two fixed it isn't said"
    );
}

#[test]
fn an_id_out_of_shape_links_nothing() {
    assert_eq!(short("1-6LU6ZM-1-186eeb39"), Some("186eeb39"), "the mesh's");
    assert_eq!(short("1-6LU6ZM-1-"), None, "an empty short form");
    assert_eq!(short("1-6LU6ZM-1-a"), None, "one found in every message");
    let mut torn = tuple();
    torn[0].id = "1-6LU6ZM-1-".into();
    assert_eq!(
        stands("e1", &["e1"], &torn, true),
        Standing::default(),
        "a question whose id is out of shape has no thread"
    );
}

#[test]
fn a_landing_names_the_commits_it_merged_at() {
    assert_eq!(
        merged_at("landed core #5486, merged at 5b05e833e36c (06:45)"),
        ["5b05e833e36c"],
        "the commit"
    );
    assert_eq!(
        merged_at("merged at 20:28, as merged at 5b05e833e36c"),
        ["5b05e833e36c"],
        "a time is no commit"
    );
    assert!(
        merged_at("merged at abc").is_empty(),
        "too short to be a commit"
    );
    assert!(merged_at("landed it").is_empty(), "no commit");
}

#[test]
fn the_core_on_is_the_last_switchs_shaped_as_a_commit() {
    let switches = concat!(
        r#"{"at":"a","generation":1,"revs":{"core":"aaaaaaaa"}}"#,
        "\n",
        r#"{"at":"b","generation":2,"revs":{"core":"bbbbbbbb","world":"w"}}"#,
        "\n",
        r#"{"at":"c","generation":3,"revs":{"core":"--output=x"}}"#,
        "\nnot json\n"
    );
    assert_eq!(core_on(switches).as_deref(), Some("bbbbbbbb"), "the newest");
    assert_eq!(core_on(""), None, "no switch yet");
}

#[test]
fn messages_are_read_from_the_folder_and_its_days_in_order_sent() {
    let dir = std::env::temp_dir().join(format!("landed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let day = dir.join("2026-11-17");
    let old = dir.join("2026-11-01");
    fs::create_dir_all(&day).expect("a day folder");
    fs::create_dir_all(&old).expect("an old day folder");
    let write = |path: std::path::PathBuf, id: &str| {
        let text = format!(r#"{{"id":"{id}","from":"A","to":["@B"],"text":"t"}}"#);
        fs::write(path, text).expect("a message");
    };
    write(day.join("20-A-1-bbbbbbbb.json"), "20-A-1-bbbbbbbb");
    write(dir.join("30-A-1-cccccccc.json"), "30-A-1-cccccccc");
    write(dir.join("5-A-1-aaaaaaaa.json"), "5-A-1-aaaaaaaa");
    write(dir.join(".read-A.json"), "x");
    write(old.join("40-A-1-dddddddd.json"), "40-A-1-dddddddd");
    fs::File::open(&old)
        .and_then(|folder| folder.set_modified(UNIX_EPOCH))
        .expect("an old folder's time");
    std::os::unix::fs::symlink(
        dir.join("30-A-1-cccccccc.json"),
        dir.join("35-A-1-eeeeeeee.json"),
    )
    .expect("a link");
    // A message in every way but its size, so only the limit keeps it out.
    let big = format!(
        r#"{{"id":"36-A-1-ffffffff","from":"A","text":"{}"}}"#,
        "t".repeat(usize::try_from(READ_AT_MOST).expect("a size"))
    );
    fs::write(dir.join("36-A-1-ffffffff.json"), big).expect("a big file");
    let read: Vec<(i64, String)> = messages(&dir, 10)
        .into_iter()
        .map(|(sent, m)| (sent, m.id))
        .collect();
    fs::remove_dir_all(&dir).expect("cleaned up");
    assert_eq!(
        read,
        [
            (20, String::from("20-A-1-bbbbbbbb")),
            (30, String::from("30-A-1-cccccccc"))
        ],
        "since 10, in order, with no hidden file, old day, link or big file"
    );
}

#[test]
fn a_landing_naming_the_entry_closes_it_whatever_its_thread() {
    let mut shared = tuple();
    shared[0].text.push_str(" and mycelium:cell/OMNOJW#e2");
    shared.push(message(
        "6-Z-1-cccccccc",
        "Z",
        "#orchestration/wcjqsg",
        None,
        "Z landed core #7, merged at 3333333abcd, for #e1",
    ));
    assert_eq!(
        stands("e1", &["e1", "e2"], &shared, true)
            .closed_by
            .as_deref(),
        Some("3333333abcd"),
        "named outright, a shared question's pain closes"
    );
    assert_eq!(
        stands("e2", &["e1", "e2"], &shared, true),
        taken("WCJQSG"),
        "the other it held stays taken"
    );
    let unasked = [shared.remove(5)];
    assert_eq!(
        stands("e1", &["e1"], &unasked, true).closed_by.as_deref(),
        Some("3333333abcd"),
        "a pain whose question was never seen"
    );
}

#[test]
fn a_reading_ends_at_its_newest_and_keeps_what_may_come_late() {
    let late = |at: i64, short: &str| {
        let id = format!("{at}-A-1-{short}");
        (at, message(&id, "A", "@B", None, ""))
    };
    let batch = [
        late(1000, "aaaaaaaa"),
        late(LATE_AT_MOST_MS + 5000, "bbbbbbbb"),
    ];
    let read = read_after(None, 0, LATE_AT_MOST_MS + 10_000, &batch);
    assert_eq!(read.at, LATE_AT_MOST_MS + 5000, "at the newest");
    assert_eq!(
        read.seen,
        [format!("{}-A-1-bbbbbbbb", LATE_AT_MOST_MS + 5000)],
        "only what may yet have another come before it"
    );
    let quiet = read_after(Some(&read), read.at, 10 * LATE_AT_MOST_MS, &[]);
    assert_eq!(
        (quiet.at, quiet.seen.len()),
        (9 * LATE_AT_MOST_MS, 0),
        "a quiet mesh moves the start on"
    );
    let ahead = [late(20 * LATE_AT_MOST_MS, "dddddddd")];
    let held = read_after(Some(&quiet), quiet.at, 10 * LATE_AT_MOST_MS, &ahead);
    assert_eq!(
        (held.at, held.seen.len()),
        (9 * LATE_AT_MOST_MS, 1),
        "one named as sent ahead is read once and moves nothing"
    );
}

#[test]
fn every_message_kept_is_listed_by_its_short_form() {
    let folder = std::env::temp_dir().join(format!("listed-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("2026-11-16")).expect("a day folder");
    fs::create_dir_all(folder.join("stale")).expect("another folder");
    for name in [
        "30-A-1-cccccccc.json",
        "2026-11-16/20-A-1-bbbbbbbb.json",
        "stale/10-A-1-aaaaaaaa.json",
        "40-A-1-dddddddd.part",
    ] {
        fs::write(folder.join(name), "{}").expect("a file");
    }
    let listed: Vec<String> = listed_ids(&folder).into_iter().collect();
    assert_eq!(
        listed,
        ["bbbbbbbb", "cccccccc"],
        "its own and its days', and nothing else"
    );
}
