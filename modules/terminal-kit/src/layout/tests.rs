//! The layout's cases: rows, facts and tables laid out in a terminal and
//! into a pipe, and words from outside laid out as they're drawn. Kept
//! beside the layout rather than in it, so the layout reads on its own.

use super::*;

#[test]
fn spaced_words_close_every_blank_to_one_space() {
    assert_eq!(
        spaced(" try \u{2800}\u{115f}rm\u{2800}-rf\t\n~ \u{7}"),
        "try rm -rf ~",
        "a run of blanks stayed open, so words could read as a try line"
    );
    assert_eq!(
        spaced("lo\rst"),
        "lost",
        "a dropped control parted the words it stood in"
    );
}

#[test]
fn a_table_cell_from_outside_stays_on_its_line_and_in_its_column() {
    let pen = Pen::plain();
    let rows = vec![
        vec!["hall\n2  forged".to_owned(), "on\u{1b}[2J".to_owned()],
        vec!["den".to_owned(), "off".to_owned()],
    ];
    let columns = [("Name", Align::Left), ("State", Align::Left)];
    assert_eq!(
        table(&columns, &rows, &pen),
        [
            "NAME            STATE",
            "hall 2  forged  on[2J",
            "den             off"
        ]
        .map(str::to_owned),
        "a line break in a cell drew a record of its own, or a control moved a column"
    );
}

#[test]
fn a_list_row_from_outside_stays_on_its_line_and_its_side_in_place() {
    let rows = [
        Row::named("hall\nforged\u{7}").side("on"),
        Row::named("den").side("off"),
    ];
    assert_eq!(
        list(&rows, &Pen::plain(), Room::Unbounded),
        ["hall forged  on", "den         off"].map(str::to_owned),
        "a row's line break or bell moved what's beside it"
    );
}

#[test]
fn a_layout_measures_outside_words_as_it_draws_them() {
    for room in [Room::Cells(24), Room::Unbounded] {
        laid_out_as_drawn(room);
    }
}

fn laid_out_as_drawn(room: Room) {
    let pen = Pen::plain();
    let rows = |name: &str, words: &str, side: &str| {
        [
            Row::named(name).holding(words).side(side),
            Row::named("den").holding("quiet").side("off"),
        ]
    };
    assert_eq!(
        list(
            &rows("hall\u{7}", "forged\nnow and\tthen", "o\u{1b}n"),
            &pen,
            room
        ),
        list(&rows("hall", "forged now and then", "on"), &pen, room),
        "a row's words or side were measured with what they never draw"
    );
    assert_eq!(
        list(&rows("hall", "quiet", "o\nn"), &pen, room),
        list(&rows("hall", "quiet", "o n"), &pen, room),
        "a break in a row's side drew a line of its own"
    );
    let cells = |first: &str, second: &str| {
        vec![
            vec![first.to_owned(), second.to_owned()],
            vec!["den".to_owned(), "quiet".to_owned()],
        ]
    };
    let columns = [("Name", Align::Left), ("Says", Align::Left)];
    assert_eq!(
        table_in(
            &columns,
            &cells("hall\u{8}", "forged\nnow and then"),
            &pen,
            room,
            &[]
        ),
        table_in(
            &columns,
            &cells("hall", "forged now and then"),
            &pen,
            room,
            &[]
        ),
        "a cell was measured with what it never draws"
    );
    let fact = |value| [("Name", Some(value))];
    assert_eq!(
        facts(&fact("a long\nname\u{7} for a hall"), "unknown", &pen, room),
        facts(&fact("a long name for a hall"), "unknown", &pen, room),
        "a fact was measured with what it never draws"
    );
}

#[test]
fn a_fact_from_outside_stays_beside_its_label() {
    let facts_ = [("Name", Some("hall\nPassword  hunter2\u{1b}[8m"))];
    assert_eq!(
        facts(&facts_, "unknown", &Pen::plain(), Room::Unbounded),
        ["Name  hall Password  hunter2[8m"].map(str::to_owned),
        "a fact's value drew a fact of its own"
    );
    let labelled = [("Den\nWAN", Some("up")), ("Hall", Some("up"))];
    for room in [Room::Unbounded, Room::Cells(80)] {
        assert_eq!(
            facts(&labelled, "unknown", &Pen::plain(), room),
            ["Den WAN  up", "Hall     up"].map(str::to_owned),
            "a fact's label drew a fact of its own in {room:?}"
        );
    }
}

#[test]
fn a_paragraph_and_hung_words_hold_a_break_to_one_line() {
    let said = "Den\n  try  rm -rf ~\u{2028}c\n";
    let flat = "Den   try  rm -rf ~ c";
    for room in [Room::Unbounded, Room::Cells(80)] {
        assert_eq!(
            hung(said, 2, room),
            flat,
            "a break in hung words drew a line of its own, or a trailing one a blank, in {room:?}"
        );
        assert_eq!(
            paragraph(said, 4, room),
            [format!("    {flat}")],
            "a break in a paragraph drew a line of its own, or a trailing one a blank, in {room:?}"
        );
    }
    assert_eq!(
        hung("one two\nthree", 2, Room::Cells(7)),
        "one\n  two\n  three",
        "hung words wrapped past the room, or left the inset as they wrapped"
    );
}

#[test]
fn overrun_names_the_first_line_past_the_room_and_none_in_a_pipe() {
    let text = "short\nfar too long\nalso too long";
    assert_eq!(overrun(text, Room::Cells(5)), Some("far too long"));
    assert_eq!(overrun(text, Room::Cells(13)), None);
    assert_eq!(overrun(text, Room::Unbounded), None);
}

fn rows() -> Vec<Row> {
    vec![
        Row::named("Now")
            .marked("active", "voice")
            .holding("2 need you"),
        Row::named("Machine")
            .marked("done", "success")
            .holding("Disk \u{b7} Power \u{b7} Sound"),
        Row::named("Disk").side("412 GB free \u{b7} nvme0n1"),
    ]
}

#[test]
fn a_list_holds_a_marks_cells_on_every_row_and_its_side_flush_right() {
    assert_eq!(
        list(&rows(), &Pen::plain(), Room::Unbounded),
        [
            "\u{25c8} Now  2 need you",
            "\u{2713} Machine  Disk \u{b7} Power \u{b7} Sound",
            "  Disk    412 GB free \u{b7} nvme0n1",
        ]
    );
}

#[test]
fn its_words_give_way_first_then_its_side_by_whole_items() {
    assert_eq!(
        list(&rows(), &Pen::plain(), Room::Cells(24)),
        [
            "\u{25c8} Now  2 need you",
            "\u{2713} Machine  Disk \u{b7} Power",
            "  Disk       412 GB free",
        ]
    );
}

#[test]
fn a_pipe_cuts_nothing() {
    let long = [Row::named("A name far longer than any terminal is wide")];
    assert_eq!(
        list(&long, &Pen::plain(), Room::Unbounded),
        ["A name far longer than any terminal is wide"]
    );
    assert_eq!(
        list(&long, &Pen::plain(), Room::Cells(14)),
        ["A name far\u{2026}"]
    );
}

#[test]
fn a_narrow_table_gives_way_by_column_least_useful_first() {
    let columns = [
        ("Machine", Align::Left),
        ("Builds", Align::Right),
        ("State", Align::Left),
    ];
    let rows = [vec![
        "second-one".to_owned(),
        "120".to_owned(),
        "building now".to_owned(),
    ]];
    let pen = Pen::plain();
    assert_eq!(
        table_in(&columns, &rows, &pen, Room::Cells(80), &[1]),
        [
            "MACHINE     BUILDS  STATE",
            "second-one     120  building now"
        ]
    );
    assert_eq!(
        table_in(&columns, &rows, &pen, Room::Cells(24), &[1]),
        ["MACHINE     STATE", "second-one  building now"]
    );
    assert_eq!(
        table_in(&columns, &rows, &pen, Room::Cells(22), &[1]),
        ["MACHINE     STATE", "second-one  building\u{2026}"]
    );
    assert_eq!(
        table_in(&columns, &rows, &pen, Room::Unbounded, &[1]),
        table(&columns, &rows, &pen)
    );
}

#[test]
fn a_long_last_column_is_shortened_before_the_others_give_way() {
    let columns = [
        ("Kind", Align::Left),
        ("Size", Align::Right),
        ("Path", Align::Left),
    ];
    let rows = [vec![
        "cache".to_owned(),
        "1 GiB".to_owned(),
        "/home/someone/.cache/a/very/deep/folder/of/runs/run-48".to_owned(),
    ]];
    let pen = Pen::plain();
    assert_eq!(
        table_in(&columns, &rows, &pen, Room::Cells(40), &[1]),
        [
            "KIND    SIZE  PATH",
            "cache  1 GiB  /…/folder/of/runs/run-48"
        ],
        "a path that can be shortened should leave the size in place"
    );
    let narrow = table_in(&columns, &rows, &pen, Room::Cells(28), &[1]);
    assert!(
        narrow[0] == "KIND   PATH",
        "with less room than a shortened path needs, the size should go: {narrow:?}"
    );
}

#[test]
fn facts_set_their_values_in_one_column() {
    let facts_ = [("Size", Some("1 TB")), ("Temperature", None)];
    assert_eq!(
        facts(&facts_, "\u{2026}", &Pen::plain(), Room::Cells(80)),
        ["Size         1 TB", "Temperature  \u{2026}"]
    );
    assert_eq!(
        facts(&facts_, "\u{2026}", &Pen::plain(), Room::Cells(20)),
        ["Size         1 TB", "Temperature  \u{2026}"],
        "facts that fit beside their labels went under them"
    );
    assert_eq!(
        facts(&facts_, "\u{2026}", &Pen::plain(), Room::Cells(12)),
        ["Size  1 TB", "Temperature", "  \u{2026}"],
        "only the fact that can't fit beside should go under its label"
    );
}

#[test]
fn a_word_too_long_beside_its_label_goes_under_it_whole() {
    let urn = "urn:schemas-upnp-org:device:MediaRenderer:1";
    let facts_ = [("Kind", Some(urn)), ("Name", Some("Stereo"))];
    assert_eq!(
        facts(&facts_, "", &Pen::plain(), Room::Cells(46)),
        ["Kind", &format!("  {urn}"), "Name  Stereo"],
        "an identifier was cut beside its label when it fits whole under it"
    );
    assert_eq!(
        facts(&facts_, "", &Pen::plain(), Room::Cells(80))[0],
        format!("Kind  {urn}"),
        "an identifier that fits beside its label left it"
    );
}

#[test]
fn a_long_value_wraps_under_itself() {
    let facts_ = [("Note", Some("one two three four"))];
    assert_eq!(
        facts(&facts_, "", &Pen::plain(), Room::Cells(14)),
        ["Note  one two", "      three", "      four"]
    );
}

#[test]
fn a_table_aligns_by_space_and_sets_figures_flush_right() {
    let columns = [
        ("Machine", Align::Left),
        ("Builds", Align::Right),
        ("State", Align::Left),
    ];
    let rows = [
        vec!["first".to_owned(), "3".to_owned(), "idle".to_owned()],
        vec![
            "second-one".to_owned(),
            "120".to_owned(),
            "building".to_owned(),
        ],
    ];
    assert_eq!(
        table(&columns, &rows, &Pen::plain()),
        [
            "MACHINE     BUILDS  STATE",
            "first            3  idle",
            "second-one     120  building",
        ]
    );
}

#[test]
fn a_row_ends_at_its_last_cell_that_says_something() {
    let columns = [
        ("Port", Align::Left),
        ("Link", Align::Left),
        ("Speed", Align::Right),
        ("Duplex", Align::Left),
    ];
    let rows = [
        vec!["1".into(), "up".into(), "1000".into(), "full".into()],
        vec!["2".into(), "down".into(), String::new(), String::new()],
        vec!["3".into(), String::new(), String::new(), String::new()],
        // A figure last on its line still sits flush right, under its
        // column's others.
        vec!["4".into(), "up".into(), "10".into(), String::new()],
        // An empty cell before one that says something keeps its room.
        vec!["5".into(), String::new(), "100".into(), String::new()],
        // A row that says nothing isn't an empty line.
        vec![String::new(), String::new(), String::new(), String::new()],
    ];
    assert_eq!(
        table(&columns, &rows, &Pen::plain()),
        [
            "PORT  LINK  SPEED  DUPLEX",
            "1     up     1000  full",
            "2     down",
            "3",
            "4     up       10",
            "5             100",
            "    ",
        ],
        "a row doesn't end at its last word, or a figure left its column"
    );
}

#[test]
fn blocks_part_by_one_empty_line() {
    assert_eq!(
        joined(vec![vec!["a".into()], vec![], vec!["b".into(), "c".into()]]),
        "a\n\nb\nc\n"
    );
    assert_eq!(joined(vec![]), "");
}

#[test]
fn a_pipe_has_no_width_to_fit() {
    let environment = Environment::of([false; 3], &[("COLUMNS", "40")]);
    assert_eq!(Room::of_stdout(&environment), Room::Unbounded);
    assert_eq!(Room::Unbounded.measure(), MEASURE);
    assert_eq!(Room::Cells(50).measure(), 50);
}
