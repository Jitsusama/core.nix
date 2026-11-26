//! The tests of the clipboard's history: what's read of an offer and kept,
//! a picture's size from its header, the asks the shell sends, niri's
//! answers to a paste, and what's served.

use super::*;
use std::os::unix::fs::PermissionsExt;

use crate::history::{
    Content, Copy, Heard, History, Picture, Served, dimensions, offered, prepared,
};
use crate::paste::{Focus, Keys, active, keys_of, let_go, pasted, under};
use crate::rows::words;

fn types(names: &[&str]) -> Vec<String> {
    names.iter().map(ToString::to_string).collect()
}

#[test]
fn a_secret_is_never_read() {
    let offered = types(&["text/plain;charset=utf-8", SECRET]);
    assert_eq!(readable(&offered, true), None);
    assert_eq!(readable(&types(&["image/png", SECRET]), true), None);
}

#[test]
fn text_is_read_as_utf8_first() {
    let offered = types(&["TEXT", "text/plain", "text/plain;charset=utf-8"]);
    assert_eq!(
        readable(&offered, true),
        Some(Readable::Text("text/plain;charset=utf-8"))
    );
}

#[test]
fn text_is_read_before_a_picture_offered_beside_it() {
    let offered = types(&["image/png", "text/plain"]);
    assert_eq!(readable(&offered, true), Some(Readable::Text("text/plain")));
}

#[test]
fn a_picture_is_read_as_png_first() {
    let offered = types(&["image/gif", "image/jpeg", "image/png"]);
    assert_eq!(
        readable(&offered, true),
        Some(Readable::Picture("image/png"))
    );
}

#[test]
fn a_picture_is_not_read_when_pictures_arent_kept() {
    assert_eq!(readable(&types(&["image/png"]), false), None);
}

#[test]
fn highlighted_text_is_given_whole_up_to_its_limit() {
    assert_eq!(selected(b"highlighted words".to_vec()), "highlighted words");
    let long = "a".repeat(SELECTED + 10);
    assert_eq!(selected(long.into_bytes()).len(), SELECTED);
}

#[test]
fn a_character_cut_at_the_limit_is_left_out() {
    let mut text = "a".repeat(SELECTED - 1);
    text.push('é');
    let given = selected(text.into_bytes());
    assert_eq!(given, "a".repeat(SELECTED - 1), "no half of a character");
}

#[test]
fn highlighted_text_not_utf8_is_read_as_near_as_it_can_be() {
    assert_eq!(selected(vec![b'o', 0xff, b'k']), "o\u{fffd}k");
}

#[test]
fn highlighted_text_is_answered_as_a_selection_record() {
    assert_eq!(
        selection("words", 7),
        json!({"key": "selection:primary", "kind": "selection", "text": "words", "at": 7})
    );
}

#[test]
fn a_picture_the_shell_cant_draw_is_not_read() {
    assert_eq!(readable(&types(&["image/tiff"]), true), None);
}

#[test]
fn its_own_offer_is_never_read() {
    assert_eq!(readable(&types(&["text/plain", OURS]), true), None);
}

/// A directory of its own for one test's pictures.
fn directory(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("clipboard-history-{}-{name}", std::process::id()));
    prepared(&dir).unwrap();
    dir
}

fn history(keep: usize) -> History {
    History::new(keep, None, 0)
}

/// The keys of the rows it serves, in their order.
fn keys(history: &History) -> Vec<String> {
    let (rows, _) = history.records(&words());
    rows.iter()
        .map(|row| row["key"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "it's what copied takes, as Wayland gives it"
)]
fn picture(bytes: &[u8]) -> Option<Heard> {
    Some(Heard::Picture {
        kind: "image/png",
        bytes: bytes.to_vec(),
    })
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "it's what copied takes, as Wayland gives it"
)]
fn text(said: &str) -> Option<Heard> {
    Some(Heard::Text(said.to_string()))
}

#[test]
fn a_kept_copy_is_what_the_clipboard_holds() {
    let mut kept = history(10);
    kept.copied(text("one"));
    assert_eq!(kept.current, Some(Content::Text("one".to_string())));
}

#[test]
fn a_copy_not_kept_leaves_nothing_to_give_back() {
    let mut kept = history(10);
    kept.copied(text("one"));
    // A secret or something too large reads as none.
    assert_eq!(kept.copied(None), None);
    assert_eq!(kept.current, None);
    kept.copied(text("two"));
    kept.copied(text("  "));
    assert_eq!(kept.current, None);
}

#[test]
fn a_picture_is_kept_in_a_file_of_its_own() {
    let dir = directory("kept");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    let copy = kept.copied(picture(b"a picture")).unwrap();
    let Content::Picture(made) = &copy.content else {
        panic!("not a picture: {copy:?}");
    };
    assert_eq!(fs::read(&made.path).unwrap(), b"a picture");
    assert_eq!(made.path.parent(), Some(dir.as_path()));
    let permissions = fs::metadata(&made.path).unwrap().permissions().mode();
    assert_eq!(permissions & 0o777, 0o600);
    assert_eq!(kept.current, Some(copy.content.clone()));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_picture_never_handed_over_is_not_kept() {
    let dir = directory("empty");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    kept.copied(text("one"));
    assert_eq!(kept.copied(picture(b"")), None);
    assert_eq!(kept.current, None);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_picture_is_not_kept_without_a_directory() {
    let mut kept = history(10);
    assert_eq!(kept.copied(picture(b"a picture")), None);
    assert_eq!(kept.current, None);
}

#[test]
fn a_picture_copied_again_is_the_one_kept() {
    let dir = directory("again");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    let first = kept.copied(picture(b"a picture")).unwrap();
    kept.copied(text("between"));
    let again = kept.copied(picture(b"a picture")).unwrap();
    assert_eq!(first.content, again.content);
    assert_eq!(kept.copies.len(), 2);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_oldest_pictures_go_once_they_take_too_much_room() {
    let dir = directory("room");
    let mut kept = History::new(10, Some(dir.clone()), 10);
    let oldest = kept.copied(picture(b"aaaa")).unwrap();
    kept.copied(text("words take no room"));
    kept.copied(picture(b"bbbb"));
    kept.copied(picture(b"cccc"));
    assert_eq!(kept.copies.len(), 3);
    let Content::Picture(oldest) = oldest.content else {
        unreachable!()
    };
    assert!(!oldest.path.exists(), "the oldest picture's file is kept");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
    // Its row goes with it.
    assert_eq!(
        keys(&kept),
        ["row:copies:4", "row:copies:3", "row:copies:2"]
    );
    assert_eq!(oldest.id, 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_picture_past_what_it_keeps_leaves_its_row() {
    let dir = directory("told");
    let mut kept = History::new(1, Some(dir.clone()), 100);
    kept.copied(picture(b"a picture"));
    assert_eq!(keys(&kept), ["row:copies:1"]);
    kept.copied(text("one"));
    assert_eq!(keys(&kept), ["row:copies:2"]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_picture_past_what_it_keeps_goes_with_its_file() {
    let dir = directory("keep");
    let mut kept = History::new(1, Some(dir.clone()), 100);
    kept.copied(picture(b"a picture"));
    kept.copied(text("one"));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_picture_chosen_moves_to_the_front_as_what_the_clipboard_holds() {
    let dir = directory("chosen");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    let made = kept.copied(picture(b"a picture")).unwrap();
    kept.copied(text("since"));
    let Content::Picture(picture) = &made.content else {
        unreachable!()
    };
    let chosen = kept.choose(picture.id).unwrap();
    assert_eq!(chosen.content, made.content);
    assert_eq!(kept.copies[0].content, made.content);
    assert_eq!(kept.current, Some(made.content.clone()));
    // A number no copy has, text and pictures sharing one count.
    assert_eq!(kept.choose(99), None);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn forgetting_forgets_what_the_clipboard_holds_too() {
    let dir = directory("forget");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    kept.copied(text("one"));
    kept.copied(picture(b"a picture"));
    kept.forget();
    assert_eq!(kept.copies.len(), 0, "a copy outlived forgetting");
    assert_eq!(kept.current, None);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_earlier_runs_pictures_are_cleared() {
    let dir = directory("earlier");
    fs::write(dir.join("1.png"), b"left behind").unwrap();
    prepared(&dir).unwrap();
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    let mode = fs::metadata(&dir).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    fs::remove_dir_all(dir).unwrap();
}

fn copy(at: u128, text: &str) -> Copy {
    Copy {
        at,
        id: 0,
        content: Content::Text(text.to_string()),
    }
}

#[test]
fn text_is_numbered_from_the_count_pictures_take_theirs_from() {
    let dir = directory("numbered");
    let mut kept = History::new(10, Some(dir.clone()), 100);
    let one = kept.copied(text("one")).unwrap();
    let picture = kept.copied(picture(b"a picture")).unwrap();
    let two = kept.copied(text("two")).unwrap();
    assert_eq!((one.id, picture.id, two.id), (1, 2, 3));
    let Content::Picture(made) = &picture.content else {
        unreachable!()
    };
    assert_eq!(made.id, picture.id);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn text_copied_again_keeps_its_number() {
    let mut kept = history(10);
    let first = kept.copied(text("one")).unwrap();
    kept.copied(text("two"));
    let again = kept.copied(text("one")).unwrap();
    assert_eq!(again.id, first.id);
    assert_eq!(kept.copied(text("three")).unwrap().id, 3);
}

#[test]
fn text_chosen_moves_to_the_front_as_what_the_clipboard_holds() {
    let mut kept = history(10);
    let one = kept.copied(text("one")).unwrap();
    kept.copied(text("two"));
    let chosen = kept.choose(one.id).unwrap();
    assert_eq!(chosen.content, one.content);
    assert_eq!(kept.copies[0].content, one.content);
    assert_eq!(kept.current, Some(one.content));
}

#[test]
fn the_shell_asks_for_a_copy_by_its_number_or_its_text() {
    assert_eq!(
        wanted(&json!({ "ask": "copy", "copy": 12, "id": "ask-1" })),
        Some(Wanted::Copy(Which::Kept(12)))
    );
    assert_eq!(
        wanted(&json!({ "ask": "copy", "text": "\u{1f600} \"hi\"\n" })),
        Some(Wanted::Copy(Which::Text("\u{1f600} \"hi\"\n".to_string())))
    );
    assert_eq!(wanted(&json!({ "ask": "clear" })), Some(Wanted::Clear));
}

#[test]
fn the_shell_asks_for_a_paste_with_the_keys_its_window_takes() {
    assert_eq!(
        wanted(&json!({ "ask": "paste", "copy": 3, "into": "terminal" })),
        Some(Wanted::Paste(Which::Kept(3), Some(Keys::Terminal)))
    );
    assert_eq!(
        wanted(&json!({ "ask": "paste", "text": "a b", "into": "plain" })),
        Some(Wanted::Paste(
            Which::Text("a b".to_string()),
            Some(Keys::Plain)
        ))
    );
}

#[test]
fn a_paste_that_names_no_keys_takes_its_windows() {
    assert_eq!(
        wanted(&json!({ "ask": "paste", "copy": 3 })),
        Some(Wanted::Paste(Which::Kept(3), None))
    );
}

#[test]
fn the_keys_go_back_to_the_focused_workspaces_active_window() {
    let workspaces = json!({ "Ok": { "Workspaces": [
        { "id": 1, "is_focused": false, "active_window_id": 3 },
        { "id": 2, "is_focused": true, "active_window_id": 7 }
    ] } });
    assert_eq!(active(&workspaces.to_string()), Some(Focus::Window(7)));
    let empty = json!({ "Ok": { "Workspaces": [
        { "id": 1, "is_focused": false, "active_window_id": 3 },
        { "id": 2, "is_focused": true, "active_window_id": null }
    ] } });
    assert_eq!(active(&empty.to_string()), Some(Focus::Nowhere));
    assert_eq!(active("{\"Ok\":{\"Layers\":[]}}"), None);
    assert_eq!(under(None), Some(Keys::Plain));
}

#[test]
fn a_window_says_its_keys_by_its_program() {
    let windows = json!({ "Ok": { "Windows": [
        { "id": 3, "app_id": "firefox" },
        { "id": 7, "app_id": "foot" },
        { "id": 8, "app_id": "com.mitchellh.ghostty" }
    ] } })
    .to_string();
    assert_eq!(keys_of(&windows, 7), Some(Keys::Terminal));
    assert_eq!(keys_of(&windows, 8), Some(Keys::Terminal));
    assert_eq!(keys_of(&windows, 3), Some(Keys::Plain));
    assert_eq!(keys_of(&windows, 9), None, "a window niri doesn't list");
}

#[test]
fn an_ask_the_shell_never_sends_wants_nothing() {
    for said in [
        json!({ "ask": "copy" }),
        json!({ "ask": "copy", "copy": "x" }),
        json!({ "ask": "copy", "id": 3 }),
        json!({ "ask": "paste", "copy": 3, "into": "loud" }),
        json!({ "ask": "paste", "copy": 3, "into": 1 }),
        json!({ "ask": "press" }),
        json!({}),
    ] {
        assert_eq!(wanted(&said), None, "{said}");
    }
}

#[test]
fn an_ask_is_handed_to_the_wayland_side_and_answered_as_it_answers() {
    let (asked, handed) = mpsc::channel();
    let (mut woken, wake) = io::pipe().unwrap();
    let own = own(asked, wake, search::Index::of(&[]));
    let answering = thread::spawn(move || {
        let Ok(Asked::Wanted(wanted, reply)) = handed.recv() else {
            panic!("nothing was handed over");
        };
        assert_eq!(wanted, Wanted::Copy(Which::Kept(3)));
        reply.send(json!({ "copied": true })).unwrap();
    });
    let answer = own(&json!({ "ask": "copy", "copy": 3 }));
    answering.join().unwrap();
    assert_eq!(answer, Some(json!({ "copied": true })));
    let mut wakes = [0; 1];
    woken.read_exact(&mut wakes).unwrap();
    assert_eq!(&wakes, b"!");
    // One it doesn't know is the interface's to refuse.
    assert_eq!(own(&json!({ "ask": "press" })), None);
}

#[test]
fn an_ask_with_no_wayland_side_is_refused_as_not_answering() {
    let (asked, handed) = mpsc::channel();
    let (_woken, wake) = io::pipe().unwrap();
    drop(handed);
    let answer = own(asked, wake, search::Index::of(&[]))(&json!({ "ask": "clear" })).unwrap();
    assert_eq!(answer["error"]["code"], "not-answering");
}

#[test]
fn a_search_is_answered_at_once_with_no_wayland_side() {
    let (asked, handed) = mpsc::channel();
    let (_woken, wake) = io::pipe().unwrap();
    drop(handed);
    let list = "[\"🎉\", \"party popper\", \"tada\", \"Activities\"]";
    let index = search::Index::of(&emoji::records(list, &rows::words()));
    let answer =
        own(asked, wake, index)(&json!({ "ask": "search", "place": "emoji", "words": "tada" }))
            .unwrap();
    assert_eq!(answer["keys"], json!(["row:emoji:0"]), "{answer}");
}

#[test]
fn a_paste_says_how_long_the_launcher_and_the_keys_took() {
    assert_eq!(
        pasted(Duration::from_micros(114_040), Duration::from_micros(8_060)),
        "Pasted: the launcher let go of the keys 114.0 ms after the ask, and they were \
         pressed 8.1 ms after that."
    );
}

#[test]
fn the_keys_are_let_go_once_the_launchers_layer_takes_none() {
    let layers = |keys: &str| {
        format!(
            "{{\"Ok\":{{\"Layers\":[{{\"namespace\":\"launcher\",\"output\":\"DP-4\",\
             \"layer\":\"Overlay\",\"keyboard_interactivity\":\"{keys}\"}},\
             {{\"namespace\":\"notices\",\"keyboard_interactivity\":\"OnDemand\"}}]}}}}"
        )
    };
    assert_eq!(let_go(&layers("None")), Some(true));
    assert_eq!(let_go(&layers("OnDemand")), Some(false));
    assert_eq!(let_go(&layers("Exclusive")), Some(false));
    assert_eq!(let_go("{\"Ok\":{\"Layers\":[]}}"), Some(true));
    assert_eq!(let_go("{\"Err\":\"no\"}"), None);
}

#[test]
fn a_terminal_takes_shift_besides() {
    assert_eq!(Keys::Plain.modifiers(), CONTROL);
    assert_eq!(Keys::Terminal.modifiers(), CONTROL | SHIFT);
}

#[test]
fn a_copy_made_again_moves_to_the_front() {
    let mut kept = history(10);
    kept.add(copy(1, "one"));
    kept.add(copy(2, "two"));
    kept.add(copy(3, "one"));
    assert_eq!(kept.copies, [copy(3, "one"), copy(2, "two")]);
}

#[test]
fn the_oldest_goes_first() {
    let mut kept = history(2);
    for (at, text) in ["one", "two", "three"].into_iter().enumerate() {
        kept.add(copy(at as u128, text));
    }
    assert_eq!(kept.copies, [copy(2, "three"), copy(1, "two")]);
}

#[test]
fn a_blank_copy_is_not_kept() {
    let mut kept = history(10);
    assert!(!kept.add(copy(1, " \n\t")), "a blank copy was said kept");
    assert_eq!(kept.copies.len(), 0, "a blank copy was kept");
}

/// What the clipboard holds, as it's served.
fn holding(types: &[&str], text: &str) -> Value {
    json!({ "key": "now:clipboard", "kind": "now", "now": { "types": types, "text": text } })
}

#[test]
fn a_copy_is_said_by_its_types_and_its_text() {
    let said = offered(&types(&["text/plain", "image/png"]), text("a\nb").as_ref());
    assert_eq!(said, holding(&["text/plain", "image/png"], "a\nb"));
}

#[test]
fn a_copy_read_as_no_text_says_none() {
    let said = offered(&types(&["image/png"]), picture(b"a picture").as_ref());
    assert_eq!(said, holding(&["image/png"], ""));
    // Text too large to read, or a program that never handed it over.
    assert_eq!(
        offered(&types(&["text/plain"]), None),
        holding(&["text/plain"], "")
    );
}

#[test]
fn a_secret_is_said_as_nothing() {
    let said = offered(&types(&["text/plain", SECRET]), text("hunter2").as_ref());
    assert_eq!(said, holding(&[], ""));
}

#[test]
fn what_the_clipboard_holds_changes_once_each_time_it_changes() {
    let mut history = history(5);
    let one = offered(&types(&["text/plain"]), text("one").as_ref());
    history.holds(one.clone());
    history.holds(one);
    assert_eq!(history.changes, 1);
    history.holds(offered(&types(&["text/plain"]), text("two").as_ref()));
    assert_eq!(history.changes, 2);
    assert_eq!(history.now, holding(&["text/plain"], "two"));
}

#[test]
fn the_copy_it_gives_back_is_said_as_it_offers_it() {
    let mut history = history(5);
    history.copied(text("one"));
    history.holds_current();
    assert_eq!(history.now, holding(&TEXT, "one"));
}

#[test]
fn forgetting_leaves_the_clipboard_holding_nothing() {
    let mut history = history(5);
    history.copied(text("one"));
    history.holds_current();
    history.forget();
    assert_eq!(history.now, holding(&[], ""));
    assert_eq!(keys(&history), Vec::<String>::new());
}

fn served() -> Served {
    Served {
        shared: Shared::new(DOMAIN, Box::new(|_| None)),
        kept: Kept::default(),
        words: words(),
        emoji: Vec::new(),
        changes: None,
    }
}

#[test]
fn the_copies_are_served_newest_first_beside_their_count_and_the_clipboard() {
    let mut served = served();
    let mut history = history(5);
    served.send(&history);
    assert_eq!(
        served.kept.get("summary:copies").unwrap()["summary"]["said"],
        "empty"
    );
    assert_eq!(
        served.kept.get("now:clipboard").unwrap()["now"]["types"],
        json!([])
    );
    history.copied(text("one"));
    history.copied(text("two"));
    history.holds_current();
    served.send(&history);
    let order = |key: &str| served.kept.get(key).unwrap()["row"]["order"].as_i64();
    assert!(
        order("row:copies:2") < order("row:copies:1"),
        "the newest copy is served first"
    );
    assert_eq!(
        served.kept.get("summary:copies").unwrap()["summary"]["said"],
        "2 copies"
    );
    assert_eq!(
        served.kept.get("now:clipboard").unwrap()["now"]["text"],
        "two"
    );
}

#[test]
fn nothing_is_made_again_while_the_history_stands() {
    let mut served = served();
    let mut history = history(5);
    history.copied(text("one"));
    served.send(&history);
    // Made again only as the history changes, which these rows outlast.
    served.kept = Kept::default();
    served.send(&history);
    assert_eq!(served.kept.all().count(), 0);
}

#[test]
fn keeping_none_keeps_nothing_to_give_back() {
    let mut history = history(0);
    assert_eq!(history.copied(text("one")), None);
    assert_eq!(history.copies.len(), 0);
    assert_eq!(history.current, None);
}

#[test]
fn a_texts_row_holds_it_as_it_was_copied() {
    let row = copy(5, "say \"hi\"\n\\").row(&words());
    assert_eq!(row["key"], "row:copies:0");
    assert_eq!(row["held"]["text"], "say \"hi\"\n\\");
    assert_eq!(row["side"][1], json!({ "at": 5, "as": "came" }));
}

#[test]
fn a_pictures_row_holds_where_it_is_and_its_size() {
    let row = Copy {
        at: 5,
        id: 3,
        content: Content::Picture(Picture {
            id: 3,
            kind: "image/png",
            path: PathBuf::from("/run/user/1000/pictures/3.png"),
            bytes: 42,
            hash: 0,
            size: Some((640, 480)),
        }),
    }
    .row(&words());
    assert_eq!(row["key"], "row:copies:3");
    assert_eq!(
        row["held"],
        json!({ "id": 3, "picture": "/run/user/1000/pictures/3.png",
                "across": 640, "down": 480 })
    );
    assert_eq!(row["side"][1]["said"], "42 B");
}

#[test]
fn a_png_says_its_size() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    png.extend(640u32.to_be_bytes());
    png.extend(480u32.to_be_bytes());
    assert_eq!(dimensions("image/png", &png), Some((640, 480)));
    assert_eq!(dimensions("image/png", &png[..20]), None);
}

#[test]
fn a_gif_says_its_size() {
    let gif = [b"GIF89a".as_slice(), &[0x80, 0x02, 0xe0, 0x01]].concat();
    assert_eq!(dimensions("image/gif", &gif), Some((640, 480)));
}

#[test]
fn a_jpeg_says_its_size_past_its_other_segments() {
    let jpeg = [
        [0xff, 0xd8].as_slice(),
        // An application segment of four bytes, then a Huffman table,
        // which shares the start of frame's range.
        &[0xff, 0xe0, 0x00, 0x04, 0x00, 0x00],
        &[0xff, 0xc4, 0x00, 0x03, 0x00],
        &[0xff, 0xc0, 0x00, 0x11, 0x08, 0x01, 0xe0, 0x02, 0x80],
    ]
    .concat();
    assert_eq!(dimensions("image/jpeg", &jpeg), Some((640, 480)));
    assert_eq!(dimensions("image/jpeg", &jpeg[..12]), None);
}

fn webp(chunk: &[u8], body: &[u8]) -> Vec<u8> {
    [b"RIFF\0\0\0\0WEBP".as_slice(), chunk, &[0; 4], body].concat()
}

#[test]
fn a_webp_says_its_size_whichever_way_it_was_made() {
    let extended = webp(b"VP8X", &[0, 0, 0, 0, 0x7f, 0x02, 0x00, 0xdf, 0x01, 0x00]);
    assert_eq!(dimensions("image/webp", &extended), Some((640, 480)));
    // The width and height less one, as WebP keeps them.
    let bits: u32 = (640 - 1) | ((480 - 1) << 14);
    let lossless = webp(b"VP8L", &[[0x2f].as_slice(), &bits.to_le_bytes()].concat());
    assert_eq!(dimensions("image/webp", &lossless), Some((640, 480)));
    let lossy = webp(
        b"VP8 ",
        &[0, 0, 0, 0x9d, 0x01, 0x2a, 0x80, 0x02, 0xe0, 0x01],
    );
    assert_eq!(dimensions("image/webp", &lossy), Some((640, 480)));
}

#[test]
fn a_picture_whose_header_says_nothing_has_no_size() {
    assert_eq!(dimensions("image/png", b"not a picture"), None);
    assert_eq!(dimensions("image/tiff", b"II*\0"), None);
}
