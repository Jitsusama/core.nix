use super::list::{GROUPS, Row, named};
use super::*;

fn rows_of(group: &str) -> Vec<Row> {
    listing()
        .into_iter()
        .find_map(|(g, rows)| (g == group).then_some(rows))
        .unwrap_or_else(|| panic!("{group} isn't listed"))
}

#[test]
fn the_list_gives_each_thing_one_row_with_every_way_to_type_it() {
    let moving = rows_of("Moving");
    let down = moving
        .iter()
        .find(|(_, does)| *does == "Down a line")
        .expect("a line down");
    assert_eq!(down.0, ["j"], "the letter alone, no arrow named");
    assert!(
        moving.iter().any(|(ways, _)| ways == &["Ctrl+E"]),
        "Moving lists no row typed Ctrl+E"
    );
    assert!(
        moving.iter().any(|(ways, _)| ways == &["Ctrl+Y"]),
        "Moving lists no row typed Ctrl+Y"
    );
    assert!(
        rows_of("Jumping").iter().any(|(ways, _)| ways == &["gg"]),
        "Jumping lists no row typed gg"
    );
    assert!(
        rows_of("In View").iter().any(|(ways, _)| ways == &["zz"]),
        "In View lists no row typed zz"
    );
    assert!(
        rows_of("Links and Files")
            .iter()
            .any(|(ways, _)| ways == &["]b"]),
        "Links and Files lists no row typed ]b"
    );
    let page = rows_of("The Page");
    assert!(
        page.iter().any(|(ways, _)| ways == &["ZZ", "Space w"]),
        "The Page lists no row typed ZZ and Space w together"
    );
    assert!(
        page.iter()
            .any(|(ways, _)| ways == &["Ctrl+Equal", "Ctrl+Plus"]),
        "The Page lists no row typed Ctrl+Equal and Ctrl+Plus together"
    );
    assert!(
        listing().iter().all(|(_, rows)| !rows.is_empty()),
        "a group of the key list holds no row"
    );
}

#[test]
fn the_list_says_selecting_text_objects_and_copying_by_letters() {
    let selecting = rows_of("Selecting");
    assert!(
        selecting.iter().any(|(ways, _)| ways == &["h"]),
        "Selecting lists no row typed h"
    );
    assert!(
        selecting.iter().any(|(ways, _)| ways == &["l"]),
        "Selecting lists no row typed l"
    );
    assert!(
        selecting.iter().any(|(ways, _)| ways == &["v"]),
        "Selecting lists no row typed v"
    );
    assert!(
        selecting.iter().any(|(ways, _)| ways == &["V"]),
        "Selecting lists no row typed V"
    );
    let objects = rows_of("Text Objects");
    assert_eq!(objects.len(), 8);
    assert!(
        objects.iter().any(|(ways, _)| ways == &["i`"]),
        "Text Objects lists no row typed i`"
    );
    assert!(
        objects.iter().any(|(ways, _)| ways == &["a`"]),
        "Text Objects lists no row typed a`"
    );
    let copying = rows_of("Copying");
    assert!(
        copying.iter().any(|(ways, _)| ways == &["y", "Ctrl+C"]),
        "Copying lists no row typed y and Ctrl+C together"
    );
    assert!(
        copying.iter().any(|(ways, _)| ways == &["yy"]),
        "Copying lists no row typed yy"
    );
    assert!(
        copying.iter().any(|(ways, _)| ways == &["yip"]),
        "Copying lists no row typed yip"
    );
    assert!(
        copying.iter().any(|(ways, _)| ways == &["yap"]),
        "Copying lists no row typed yap"
    );
    assert!(
        copying.iter().any(|(ways, _)| ways == &["y}"]),
        "Copying lists no row typed y}}"
    );
}

#[test]
fn every_act_the_reader_speaks_is_the_languages_and_every_key_of_it_is_taken() {
    // Reading the language panics on an act it doesn't name or a key the
    // reader can't be given.
    let table = read(LANGUAGE);
    for &(name, act) in SPOKEN {
        assert!(
            table.sequences.iter().any(|&(_, taken, _)| taken == act),
            "{name} has no keys"
        );
    }
    assert_eq!(table.slots.len(), 3, "m, ' and the backtick take a letter");
}

#[test]
fn keys_are_read_in_neovims_notation() {
    assert_eq!(parse("gg"), Some(vec![Char('g'), Char('g')]));
    assert_eq!(parse("<C-d>"), Some(vec![Ctrl('d')]));
    assert_eq!(parse("<Space>?"), Some(vec![Char(' '), Char('?')]));
    assert_eq!(parse("<PageDown>"), Some(vec![Key::PageDown]));
    assert_eq!(parse("<lt>"), Some(vec![Char('<')]));
    assert_eq!(parse("<"), Some(vec![Char('<')]));
    assert_eq!(parse("<S-Tab>"), Some(vec![Key::BackTab]));
    assert_eq!(parse("<S-Left>"), None);
}

#[test]
fn the_key_map_spells_a_key_of_two_words_as_one() {
    let chords = chords();
    let screen = |chord: &str| {
        chords
            .iter()
            .any(|c| c.chord == chord && c.act == "screen-down")
    };
    assert!(
        screen("Page_Down") && screen("Ctrl+F"),
        "Page_Down and Ctrl+F don't both move a screen down"
    );
    assert!(
        chords.iter().any(|c| c.chord == "g g" && c.act == "first"),
        "g g isn't listed as going to the first line"
    );
    assert!(
        chords
            .iter()
            .any(|c| c.chord == "`" && c.act == "go-to-mark"),
        "` isn't listed as going to a mark"
    );
    assert!(
        chords
            .iter()
            .any(|c| c.chord == "Ctrl+Equal" && c.act.is_empty()),
        "Ctrl+Equal isn't listed as the reader's own key"
    );
}

#[test]
fn the_lines_take_their_keys_from_the_language() {
    assert_eq!(line_key(Key::Return, true), Some(LineKey::Run));
    assert_eq!(line_key(Key::Return, false), Some(LineKey::Open));
    assert_eq!(line_key(Key::Escape, false), Some(LineKey::Back));
    assert_eq!(line_key(Ctrl('c'), true), Some(LineKey::Leave));
    assert_eq!(line_key(Ctrl('h'), true), Some(LineKey::DeleteLetter));
    assert_eq!(line_key(Key::Backspace, false), Some(LineKey::DeleteLetter));
    assert_eq!(line_key(Ctrl('w'), true), Some(LineKey::DeleteWord));
    assert_eq!(line_key(Ctrl('u'), false), Some(LineKey::DeleteToStart));
    assert_eq!(line_key(Char('a'), true), None);
    let commands: Vec<String> = chords()
        .into_iter()
        .filter(|c| c.group == "commands")
        .map(|c| format!("{} {}", c.chord, c.act))
        .collect();
    assert!(commands.contains(&"Return run".to_string()), "{commands:?}");
    assert!(
        commands.contains(&"Ctrl+H delete-letter".to_string()),
        "{commands:?}"
    );
    let (_, rows) = listing()
        .into_iter()
        .find(|(group, _)| *group == "Command Line")
        .expect("the lines' group");
    assert_eq!(rows.len(), 12, "@:, then Backspace and Ctrl+H on one row");
    assert!(
        chords()
            .iter()
            .any(|c| c.group == "commands" && c.chord == "@ :" && c.act == "again-command"),
        "@ : isn't listed as running the last command again"
    );
    assert_eq!(line_key(Key::Tab, true), Some(LineKey::NextRow));
    assert_eq!(line_key(Key::BackTab, true), Some(LineKey::PreviousRow));
    assert_eq!(line_key(Ctrl('y'), true), Some(LineKey::TakeName));
    assert_eq!(line_key(Ctrl('e'), true), Some(LineKey::MenuAway));
    assert_eq!(
        line_key(Ctrl('e'), false),
        None,
        "the find line has no menu to put away"
    );
}

#[test]
fn every_listed_act_has_a_group_and_no_two_groups_share_a_name() {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for &(_, act, _) in &table().sequences {
        if let Some((group, name)) = named(act) {
            assert!(
                GROUPS.contains(&group),
                "{act:?} is in {group}, which isn't shown"
            );
            if let Some(other) = seen.iter().find(|(g, n)| *n == name && *g != group) {
                panic!("{name} is in both {} and {group}", other.0);
            }
            seen.push((group, name));
        }
    }
}

fn typed(machine: &mut Machine, text: &str) -> Vec<(Act, Option<u32>)> {
    typed_in(machine, text, false)
}

fn typed_in(machine: &mut Machine, text: &str, selecting: bool) -> Vec<(Act, Option<u32>)> {
    text.chars()
        .filter_map(|c| machine.press(Char(c), selecting))
        .collect()
}

#[test]
fn a_held_key_goes_on_only_when_it_moves() {
    let mut machine = Machine::default();
    assert_eq!(machine.held(Char('j'), false), Some((Act::Down, None)));
    assert_eq!(machine.held(Char('w'), true), Some((Act::WordOn, None)));
    assert_eq!(typed(&mut machine, "yy"), [(Act::CopyPiece, None)]);
    assert_eq!(machine.held(Char('y'), false), None, "a held y copies once");
    assert_eq!(
        typed(&mut machine, "j"),
        [(Act::Down, None)],
        "and leaves nothing waiting"
    );
    assert_eq!(
        typed(&mut machine, "g"),
        [] as [(crate::keys::Act, std::option::Option<u32>); 0]
    );
    assert_eq!(machine.held(Char('g'), false), None, "nor ends a sequence");
    assert_eq!(typed(&mut machine, "g"), [(Act::First, None)]);
    assert_eq!(
        typed(&mut machine, "d"),
        [] as [(crate::keys::Act, std::option::Option<u32>); 0]
    );
    assert_eq!(machine.held(Char('d'), false), None, "nor ends an operator");
    assert_eq!(
        typed(&mut machine, "j"),
        [(Act::Change(Edit::DeleteMotion), None)]
    );
    assert_eq!(machine.held(Char('v'), false), None);
    assert_eq!(machine.held(Char('5'), false), None);
    assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
}

#[test]
fn a_letter_moves_at_once() {
    let mut machine = Machine::default();
    assert_eq!(
        typed(&mut machine, "jk"),
        [(Act::Down, None), (Act::Up, None)]
    );
}

#[test]
fn no_list_names_an_arrow() {
    let named = |chord: &str| chords().iter().any(|c| c.chord == chord);
    for arrow in ["Up", "Down", "Left", "Right"] {
        assert!(!named(arrow), "the key map names {arrow}");
    }
    let shown = listing()
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .flat_map(|(ways, _)| ways)
        .collect::<Vec<_>>();
    for arrow in ["Up", "Down", "Left", "Right"] {
        assert!(
            !shown.iter().any(|way| way == arrow),
            "Space ? shows {arrow}"
        );
    }
}

#[test]
fn the_arrows_move_as_the_letters_do() {
    let mut machine = Machine::default();
    assert_eq!(machine.press(Key::Down, false), Some((Act::Down, None)));
    assert!(typed(&mut machine, "3").is_empty(), "a count waits");
    assert_eq!(machine.press(Key::Down, false), Some((Act::Down, Some(3))));
    assert_eq!(
        machine.press(Key::PageUp, false),
        Some((Act::ScreenUp, None))
    );
    assert_eq!(machine.press(Key::Left, true), Some((Act::Left, None)));
    assert_eq!(machine.press(Key::Right, true), Some((Act::Right, None)));
    assert_eq!(machine.press(Key::Up, true), Some((Act::Up, None)));
}

#[test]
fn a_count_goes_with_the_motion_after_it() {
    let mut machine = Machine::default();
    assert_eq!(typed(&mut machine, "12j"), [(Act::Down, Some(12))]);
    assert_eq!(typed(&mut machine, "3G"), [(Act::Last, Some(3))]);
    assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
    assert_eq!(typed_in(&mut machine, "3w", true), [(Act::WordOn, Some(3))]);
}

#[test]
fn a_zero_alone_starts_no_count_and_goes_to_the_lines_start_while_selecting() {
    let mut machine = Machine::default();
    assert_eq!(typed(&mut machine, "0j"), [(Act::Down, None)]);
    assert_eq!(typed(&mut machine, "10j"), [(Act::Down, Some(10))]);
    assert_eq!(typed_in(&mut machine, "0", true), [(Act::LineStart, None)]);
    assert_eq!(
        typed_in(&mut machine, "20l", true),
        [(Act::Right, Some(20))]
    );
}

#[test]
fn two_keys_make_one_act() {
    let mut machine = Machine::default();
    assert_eq!(typed(&mut machine, "g"), []);
    assert!(machine.busy(), "g doesn't wait for its second key");
    assert_eq!(typed(&mut machine, "g"), [(Act::First, None)]);
    assert_eq!(typed(&mut machine, "ZZ"), [(Act::Close, None)]);
    assert_eq!(typed(&mut machine, " w"), [(Act::Close, None)]);
    assert_eq!(typed(&mut machine, " ?"), [(Act::ShowKeys, None)]);
}

#[test]
fn y_copies_at_once_while_selecting_and_waits_for_its_object_at_rest() {
    let mut machine = Machine::default();
    assert_eq!(typed_in(&mut machine, "y", true), [(Act::Copy, None)]);
    assert_eq!(machine.press(Ctrl('c'), true), Some((Act::Copy, None)));
    assert_eq!(
        machine.press(Ctrl('c'), false),
        None,
        "nothing to copy at rest"
    );
    assert_eq!(typed(&mut machine, "y"), []);
    assert_eq!(typed(&mut machine, "y"), [(Act::CopyPiece, None)]);
    assert_eq!(typed(&mut machine, "3yy"), [(Act::CopyPiece, Some(3))]);
    assert_eq!(typed(&mut machine, "yip"), [(Act::CopyParagraph, None)]);
    assert_eq!(typed(&mut machine, "yap"), [(Act::CopyAParagraph, None)]);
    assert_eq!(
        typed(&mut machine, "2y}"),
        [(Act::CopyToParagraphEnd, Some(2))]
    );
    assert_eq!(
        typed_in(&mut machine, "y}", true),
        [(Act::Copy, None), (Act::ParagraphOn, None)],
        "while selecting, y copies at once"
    );
}

#[test]
fn text_objects_and_motions_work_only_while_selecting() {
    let mut machine = Machine::default();
    assert_eq!(
        typed_in(&mut machine, "iwapi`a`o$e{}", true),
        [
            (Act::Object(Object::InWord), None),
            (Act::Object(Object::AParagraph), None),
            (Act::Object(Object::InCode), None),
            (Act::Object(Object::ACode), None),
            (Act::OtherEnd, None),
            (Act::LineEnd, None),
            (Act::WordEnd, None),
            (Act::ParagraphBack, None),
            (Act::ParagraphOn, None),
        ]
    );
    assert_eq!(typed(&mut machine, "iwob"), [], "at rest they mean nothing");
    assert_eq!(typed(&mut machine, "v"), [(Act::Select, None)]);
    assert_eq!(
        typed_in(&mut machine, "V", true),
        [(Act::SelectPieces, None)]
    );
}

#[test]
fn vis_paragraph_heading_and_view_keys_take_counts() {
    let mut machine = Machine::default();
    assert_eq!(
        typed(&mut machine, "}{2]][["),
        [
            (Act::ParagraphOn, None),
            (Act::ParagraphBack, None),
            (Act::HeadingOn, Some(2)),
            (Act::HeadingBack, None),
        ]
    );
    assert_eq!(
        typed(&mut machine, "H3MLztzzzb"),
        [
            (Act::ViewTop, None),
            (Act::ViewMiddle, Some(3)),
            (Act::ViewBottom, None),
            (Act::PlaceTop, None),
            (Act::PlaceMiddle, None),
            (Act::PlaceBottom, None),
        ]
    );
    assert_eq!(
        typed(&mut machine, "%50%*#"),
        [
            (Act::Bracket, None),
            (Act::Bracket, Some(50)),
            (Act::SameWordOn, None),
            (Act::SameWordBack, None),
        ]
    );
    // Only { and } of these move a selection's cursor; the rest are a
    // page's at rest.
    assert_eq!(
        typed_in(&mut machine, "{}H", true),
        [(Act::ParagraphBack, None), (Act::ParagraphOn, None)]
    );
}

#[test]
fn files_links_the_outline_and_the_command_line_have_their_keys() {
    let mut machine = Machine::default();
    assert_eq!(
        typed(&mut machine, "]b2[bgfgO:"),
        [
            (Act::FileOn, None),
            (Act::FileBack, Some(2)),
            (Act::FollowFile, None),
            (Act::Outline, None),
            (Act::Command, None),
        ]
    );
    assert_eq!(
        machine.press(Ctrl(']'), false),
        Some((Act::FollowLink, None))
    );
    // ] waits for its second key, as g does, and a wrong one drops it.
    assert_eq!(typed(&mut machine, "]x"), []);
    assert!(
        !machine.busy(),
        "] x still waits, where a wrong second key drops it"
    );
}

#[test]
fn a_wrong_second_key_drops_the_sequence_as_vi_does() {
    let mut machine = Machine::default();
    assert_eq!(typed(&mut machine, "gq"), []);
    assert!(
        !machine.busy(),
        "g q still waits, where a wrong second key drops it"
    );
    assert_eq!(typed(&mut machine, "gx"), [(Act::Links, None)]);
    assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
}

#[test]
fn marks_take_the_letter_after_them() {
    let mut machine = Machine::default();
    assert_eq!(typed(&mut machine, "ma"), [(Act::SetMark('a'), None)]);
    assert_eq!(typed(&mut machine, "'a"), [(Act::GoToMark('a'), None)]);
    assert_eq!(typed(&mut machine, "`b"), [(Act::GoToMark('b'), None)]);
    assert_eq!(typed(&mut machine, "''"), [(Act::LastJump, None)]);
}

#[test]
fn a_mark_is_made_while_selecting_as_vim_makes_it() {
    let mut machine = Machine::default();
    assert_eq!(
        typed_in(&mut machine, "ma", true),
        [(Act::SetMark('a'), None)]
    );
}

#[test]
fn the_jumps_go_back_and_forward() {
    let mut machine = Machine::default();
    assert_eq!(machine.press(Ctrl('o'), false), Some((Act::JumpBack, None)));
    assert_eq!(
        machine.press(Ctrl('i'), false),
        Some((Act::JumpForward, None))
    );
}

#[test]
fn escape_cancels_what_is_in_progress() {
    let mut machine = Machine::default();
    typed(&mut machine, "5g");
    assert_eq!(machine.press(Key::Escape, false), None);
    assert!(!machine.busy(), "Escape leaves the count and the g waiting");
    assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
}

#[test]
fn escape_at_rest_says_so_and_closes_nothing() {
    let mut machine = Machine::default();
    assert_eq!(machine.press(Key::Escape, false), Some((Act::Rest, None)));
    assert_eq!(machine.press(Key::Escape, true), Some((Act::Rest, None)));
}

#[test]
fn q_does_nothing_and_search_starts_both_ways() {
    let mut machine = Machine::default();
    assert_eq!(
        typed(&mut machine, "q/?nN"),
        [
            (Act::SearchForward, None),
            (Act::SearchBackward, None),
            (Act::Next, None),
            (Act::Previous, None),
        ]
    );
}

#[test]
fn a_change_is_refused_once_it_is_whole() {
    use Edit::{ChangeLine, ChangeMotion, DeleteCharacter, DeleteMotion, Ending, PutAfter};
    let mut machine = Machine::default();
    let refused = |change| (Act::Change(change), None);
    assert_eq!(typed(&mut machine, "x"), [refused(DeleteCharacter)]);
    assert_eq!(typed(&mut machine, "p"), [refused(PutAfter)]);
    assert_eq!(typed(&mut machine, "S"), [refused(ChangeLine)]);
    assert_eq!(typed(&mut machine, "dd"), [refused(Ending)]);
    assert_eq!(typed(&mut machine, "cc"), [refused(ChangeLine)]);
    // d and c wait for their motion, whatever it is, and say nothing
    // until it's whole, as vi does.
    assert_eq!(typed(&mut machine, "d"), []);
    assert!(machine.busy(), "d doesn't wait for its motion");
    assert_eq!(typed(&mut machine, "j"), [refused(DeleteMotion)]);
    assert_eq!(
        typed(&mut machine, "d3j"),
        [(Act::Change(DeleteMotion), Some(3))]
    );
    for motion in ["dip", "d'a", "dgg", "d}", "cw", "c]]"] {
        assert_eq!(typed(&mut machine, motion).len(), 1, "{motion}");
        assert!(!machine.busy(), "{motion}");
    }
    assert_eq!(typed(&mut machine, "cgg"), [refused(ChangeMotion)]);
    // A key that's no motion drops the operator, and Escape cancels it.
    assert_eq!(typed(&mut machine, "dqj"), [(Act::Down, None)]);
    typed(&mut machine, "c");
    assert_eq!(machine.press(Key::Escape, false), None);
    assert_eq!(typed(&mut machine, "j"), [(Act::Down, None)]);
}

#[test]
fn a_selection_refuses_d_c_and_p_at_once() {
    use Edit::{ChangeMotion, DeleteMotion, PutAfter};
    let mut machine = Machine::default();
    assert_eq!(
        typed_in(&mut machine, "dcp", true),
        [
            (Act::Change(DeleteMotion), None),
            (Act::Change(ChangeMotion), None),
            (Act::Change(PutAfter), None),
        ]
    );
}

#[test]
fn every_sequence_in_the_table_is_reachable_and_none_twice() {
    for selecting in [false, true] {
        let here: Vec<&[Key]> = table()
            .sequences
            .iter()
            .filter(|(_, _, held)| held.holds(selecting))
            .map(|(keys, _, _)| keys.as_slice())
            .collect();
        for (i, keys) in here.iter().enumerate() {
            assert!(!here[..i].contains(keys), "{keys:?} is in the table twice");
            // No whole sequence is the start of a longer one, or the
            // longer could never be typed.
            assert!(
                !here
                    .iter()
                    .any(|other| other.len() > keys.len() && other.starts_with(keys)),
                "{keys:?} hides a longer sequence"
            );
        }
    }
}
