use super::*;

#[test]
fn a_name_gives_way_at_a_whole_word_and_says_so() {
    // A first word past the room is shortened in its middle, so a
    // name still fits and still shows both its ends.
    assert_eq!(
        cut("switch the system now", 18),
        "switch the system\u{2026}"
    );
    assert_eq!(cut("switch the system now", 5), "sw\u{2026}ch");
    assert_eq!(cut("status", 6), "status");
}

#[test]
fn a_cut_never_ends_on_a_small_word() {
    assert_eq!(
        cut("Put Machine, Programs and To-Do on the Terminal Kit", 37),
        "Put Machine, Programs and To-Do…"
    );
    assert_eq!(cut("Map From the Network", 14), "Map…");
    assert_eq!(
        cut("Read Only the Asked Play So a Turn", 30),
        "Read Only the Asked Play…"
    );
    assert_eq!(
        cut("switch the system now", 12),
        "switch…",
        "a small word was left before the cut"
    );
    assert_eq!(cut("The Thing", 6), "The…", "the first word went");
}

#[test]
fn a_long_path_keeps_its_ends_and_parts_at_slashes() {
    let path = "~/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261029-6QB521/tmp/run-48";
    assert_eq!(
        shortened(path, 40),
        "~/\u{2026}/QEST-20261029-6QB521/tmp/run-48"
    );
    assert_eq!(
        shortened(path, 48),
        "~/.cache/pi/\u{2026}/QEST-20261029-6QB521/tmp/run-48"
    );
    assert_eq!(shortened(path, 24), "~/.cache/pi/\u{2026}/tmp/run-48");
    assert_eq!(shortened(path, 20), "~/\u{2026}/tmp/run-48");
    assert_eq!(
        shortened("/var/lib/a-very-long-file-name.zst", 16),
        "/var/li\u{2026}name.zst"
    );
    assert_eq!(shortened(path, 100), path);
    for room in 1..90 {
        assert!(width(&shortened(path, room)) <= room.max(2), "at {room}");
    }
}

#[test]
fn a_word_too_long_for_its_line_is_shortened_not_broken() {
    let words =
        "Cleared ~/.cache/pi/agentic-harness.pi/quest-workspace/QEST-1/tmp/run-48 (16 KiB).";
    assert_eq!(
        wrapped(words, 30),
        [
            "Cleared",
            "~/.cache/\u{2026}/QEST-1/tmp/run-48",
            "(16 KiB)."
        ]
    );
    // With room enough left beside it, the word before keeps it company.
    assert_eq!(
        wrapped(words, 50),
        [
            "Cleared ~/\u{2026}/quest-workspace/QEST-1/tmp/run-48",
            "(16 KiB)."
        ]
    );
}

#[test]
fn a_word_sharing_a_hung_first_line_is_shortened_to_that_line_s_room() {
    let words =
        "Cleared ~/.cache/pi/agentic-harness.pi/quest-workspace/QEST-1/tmp/run-48 (16 KiB).";
    // The first line has 40 cells past its inset and the lines under it
    // 38, so the path beside Cleared has 32 cells to keep, where the
    // hang's measure would leave it 30 and cut a folder more.
    assert_eq!(
        hanging(words, 2, Room::Cells(42)),
        [
            "  Cleared ~/.cache/pi/\u{2026}/QEST-1/tmp/run-48",
            "    (16 KiB)."
        ],
        "the path beside Cleared was cut to the hang's room"
    );
}

#[test]
fn a_word_cut_to_the_hang_still_shares_a_first_line_it_fits() {
    // Ten cells are left beside the first word, under the 24 a word
    // needs to share a line, so it's cut to the ten its own line would
    // give it, and those fit the first line, where it stays.
    assert_eq!(
        hanging("a bbbbbbbbbbbbbbbbbbbbb", 0, Room::Cells(12)),
        ["a bbbb\u{2026}bbbbb"]
    );
}

#[test]
fn a_last_word_is_never_left_alone_on_its_line() {
    assert_eq!(
        wrapped("the guard clears some once idle", 26),
        ["the guard clears some", "once idle"]
    );
    // A figure keeps its unit, so it isn't parted to keep company.
    assert_eq!(
        wrapped("it holds 412 GB now", 15),
        ["it holds 412 GB", "now"]
    );
    // A run's break took its dot, so its items are never run together.
    assert_eq!(
        wrapped("on / \u{b7} filling \u{b7} exhausted", 14),
        ["on / \u{b7} filling", "exhausted"]
    );
}

#[test]
fn a_paragraph_keeps_its_inset_on_every_line() {
    assert_eq!(
        paragraph("one two three four", 4, Room::Cells(14)),
        ["    one two", "    three four"]
    );
    assert_eq!(
        paragraph("one two three four", 4, Room::Unbounded),
        ["    one two three four"]
    );
}

#[test]
fn a_hanging_paragraph_keeps_its_first_line_s_room_and_hangs_the_rest() {
    let words = "one two three four five";
    assert_eq!(
        hanging(words, 2, Room::Cells(15)),
        ["  one two three", "    four five"],
        "the first line gave up the cells only the lines under it hang by"
    );
    assert_eq!(
        hanging(words, 2, Room::Unbounded),
        ["  one two three four five"],
        "into a pipe a hanging paragraph is one line at its inset"
    );
    // An item, a piece and a widowed word each fall, at some room,
    // between the first line's measure and the rest's, so one set to
    // the wrong measure runs a hung line past the room. Below the
    // hang's own floor, a line under the first is a cell of words.
    let cases = [
        (words, 2),
        ("aaaa · bbbbbbbbbbb · cc", 0),
        ("a screen of 1272 × 768 pixels and more", 2),
        ("aa bb cccc dddd", 0),
    ];
    for (said, inset) in cases {
        let longest = said.split_whitespace().map(width).max().unwrap_or(0);
        for room in 0..30 {
            let lines = hanging(said, inset, Room::Cells(room));
            for (at, line) in lines.iter().enumerate() {
                // The first line draws a cell of words past its inset, and
                // each under it a cell past the hang.
                let most = room.max(inset + 1 + if at == 0 { 0 } else { GAP });
                assert!(width(line) <= most, "{line:?} passed {most} cells");
                assert!(
                    !line.trim().is_empty(),
                    "{said:?} drew an empty line at {room}"
                );
            }
            // Where every word fits a hung line, none is cut, lost or
            // said twice. The dot between two items is left out where a
            // line breaks between them, so it isn't counted.
            let words = |text: &str| -> Vec<String> {
                text.split_whitespace()
                    .filter(|word| *word != "·")
                    .map(str::to_owned)
                    .collect()
            };
            if room >= inset + GAP + longest {
                assert_eq!(
                    words(&lines.join(" ")),
                    words(said),
                    "{said:?} lost or changed a word at {room}"
                );
            }
            // A run's dot goes with its break, so no line ends or starts
            // on one, and where no item breaks between its words, each
            // gap between items is either a dot or a line's end.
            for line in &lines {
                let line = line.trim();
                assert!(
                    !line.starts_with('·') && !line.ends_with('·'),
                    "{said:?} left a line on its dot at {room}: {lines:?}"
                );
            }
            let items: Vec<&str> = said.split(ITEMS).collect();
            if items.len() > 1
                && !items.iter().any(|item| item.contains(' '))
                && room >= inset + GAP + longest
            {
                let dots = lines.iter().map(|line| line.matches('·').count()).sum::<usize>();
                assert_eq!(
                    dots + lines.len() - 1,
                    items.len() - 1,
                    "{said:?} lost or doubled a dot at {room}: {lines:?}"
                );
            }
        }
    }
}

#[test]
fn a_figure_parts_from_a_joining_word_after_it() {
    // "6 on" isn't a figure and its unit, so the name before it fits
    // whole and the line breaks after the figure.
    assert_eq!(
        wrapped(
            "tablet 1 connected to Den eero Pro 6 on 5.2 GHz channel 52.",
            38
        ),
        [
            "tablet 1 connected to Den eero Pro 6",
            "on 5.2 GHz channel 52."
        ]
    );
}

#[test]
fn words_wrap_at_items_first_and_hold_a_figure_to_its_unit() {
    assert_eq!(
        wrapped("412 GB free \u{b7} nvme0n1 \u{b7} Samsung", 22),
        ["412 GB free \u{b7} nvme0n1", "Samsung"]
    );
    assert_eq!(
        wrapped("a screen 1272 \u{d7} 768 wide", 12),
        ["a screen", "1272 \u{d7} 768", "wide"]
    );
}

#[test]
fn a_rate_is_held_to_its_figure_as_a_unit_is() {
    // A rate's unit is longer than a unit's, but a line that ends on
    // "10.3" and begins the next on "Mb/s" reads as a bare number.
    assert_eq!(
        wrapped("which let 10.3 Mb/s through", 14),
        ["which let", "10.3 Mb/s", "through"]
    );
    assert_eq!(
        wrapped("read 12 KiB/s here", 8),
        ["read", "12 KiB/s", "here"]
    );
    // As load-proof says a line's rate.
    assert_eq!(
        wrapped("up 9.8 Mbit/s down", 11),
        ["up", "9.8 Mbit/s", "down"]
    );
    // A rate says what it counts, however long its name, as a disk
    // row's requests do.
    assert_eq!(
        wrapped("took 5 requests/s now", 12),
        ["took", "5 requests/s", "now"]
    );
    // A stop after it doesn't hide it.
    assert_eq!(
        wrapped("it let 10.3 Mb/s. Then", 11),
        ["it let", "10.3 Mb/s.", "Then"]
    );
    // Nor does a lone last word take it from its figure.
    assert_eq!(
        wrapped("it let 10.3 Mb/s through", 16),
        ["it let 10.3 Mb/s", "through"]
    );
    // A word as long that isn't a rate still parts from a figure,
    // as does one with anything but letters before its "/s".
    assert_eq!(wrapped("took 5 with it", 7), ["took 5", "with it"]);
    assert_eq!(wrapped("took 5 a.b/s now", 7), ["took 5", "a.b/s", "now"]);
}

#[test]
fn a_figure_held_too_wide_for_its_line_parts_rather_than_shortens() {
    // Shortened to fit, the figure would lose its digits, which are
    // what the line is read for.
    assert_eq!(
        wrapped("up 9.8 Mbit/s down", 6),
        ["up 9.8", "Mbit/s", "down"]
    );
}

#[test]
fn words_cut_wrapped_or_shortened_stay_on_one_line() {
    let name = "Den\neero\tPro\u{2029}6";
    assert_eq!(
        wrapped(name, 80),
        ["Den eero Pro 6"],
        "wrapped words kept a break"
    );
    assert_eq!(cut(name, 80), "Den eero Pro 6", "a cut name kept a break");
    assert_eq!(cut(name, 10), "Den eero\u{2026}", "a cut name kept a break");
    assert_eq!(
        shortened("~/a\nb/c", 80),
        "~/a b/c",
        "a shortened word kept a break"
    );
}

#[test]
fn every_mark_that_reorders_text_is_kept_from_the_terminal() {
    // Unicode's own lists, not `unseen`'s: Bidi_Control, and the
    // Default_Ignorable_Code_Point characters that shape nothing, each
    // range by its ends.
    let marks = [
        '\u{061c}',
        '\u{200e}',
        '\u{200f}',
        '\u{202a}',
        '\u{202b}',
        '\u{202c}',
        '\u{202d}',
        '\u{202e}',
        '\u{2066}',
        '\u{2067}',
        '\u{2068}',
        '\u{2069}',
        '\u{200b}',
        '\u{2060}',
        '\u{2061}',
        '\u{2062}',
        '\u{2063}',
        '\u{2064}',
        '\u{feff}',
        '\u{034f}',
        '\u{17b4}',
        '\u{17b5}',
        '\u{206a}',
        '\u{206f}',
        '\u{3164}',
        '\u{ffa0}',
        '\u{fff0}',
        '\u{fffb}',
        '\u{1bca0}',
        '\u{1bca3}',
        '\u{1d173}',
        '\u{1d17a}',
        '\u{e0000}',
        '\u{e0001}',
        '\u{e0002}',
        '\u{e001f}',
        '\u{e0080}',
        '\u{e00ff}',
        '\u{e01f0}',
        '\u{e0fff}',
    ];
    for mark in marks {
        let words = format!("abc{mark}def");
        assert_eq!(drawn(&words), "abcdef", "{mark:?} reached the terminal");
        assert_eq!(one_line(&words), "abcdef", "{mark:?} reached the terminal");
    }
    assert_eq!(
        drawn("a\u{2028}b\u{2029}c"),
        "a b c",
        "a Unicode separator made a line break"
    );
    assert_eq!(
        drawn("a\u{b}b\u{c}c\u{85}d"),
        "a b c d",
        "a character that parts words ran them together"
    );
    let shaping = "\u{ad}\u{200c}\u{200d}\u{fe0f}\u{e0067}\u{e007f}\u{1160}\u{180b}";
    assert_eq!(
        drawn(shaping),
        shaping,
        "a character that shapes was dropped"
    );
}
