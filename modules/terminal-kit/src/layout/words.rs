//! How words fit the room they're given (docs/commands.md, Width and What
//! Gives Way): prose wrapped at whole words keeping its inset, a run giving
//! way at its items, a figure held to its unit, no last word left alone,
//! and a word too long for its line shortened in its middle, never broken.
//! Lists, facts and tables set their cells with these.

use unicode_width::UnicodeWidthChar;

use super::{CUT, ITEMS, Room, width};

/// A name cut to `room` cells at a whole word, ending in an ellipsis. A
/// first word too long for the room alone is shortened in its middle, as
/// a path is, since a name is never cut at its end inside a word.
pub fn cut(name: &str, room: usize) -> String {
    if width(name) <= room {
        return name.to_owned();
    }
    let first = name.split(' ').next().unwrap_or_default();
    if width(first) + width(CUT) > room {
        return shortened(first, room);
    }
    let mut kept = String::new();
    for word in name.split(' ') {
        let longer = if kept.is_empty() {
            word.to_owned()
        } else {
            format!("{kept} {word}")
        };
        if !kept.is_empty() && width(&longer) + width(CUT) > room {
            break;
        }
        kept = longer;
    }
    format!("{kept}{CUT}")
}

/// A word too long for `room` cells, shortened in its middle so both its
/// ends still say what it is. A path keeps its first part and as many of
/// its last as fit, parted at slashes (`~/.cache/pi/…/tmp/run-48`),
/// its last part first, since that names the thing; any other word, or
/// a path whose last part alone is too long, keeps its first and last
/// letters either side of the cut.
pub fn shortened(word: &str, room: usize) -> String {
    if width(word) <= room {
        return word.to_owned();
    }
    let parts: Vec<&str> = word.split('/').collect();
    if parts.len() > 2 {
        let set = |head: usize, tail: usize| {
            format!(
                "{}/{CUT}/{}",
                parts[..head].join("/"),
                parts[parts.len() - tail..].join("/")
            )
        };
        // Something is always left out between the two ends.
        let most = parts.len() - 1;
        let mut tail = 1;
        while tail + 1 < most && width(&set(1, tail + 1)) <= room {
            tail += 1;
        }
        let mut head = 1;
        while head + tail < most && width(&set(head + 1, tail)) <= room {
            head += 1;
        }
        if width(&set(head, tail)) <= room {
            return set(head, tail);
        }
    }
    let letters: Vec<char> = word.chars().collect();
    let fits = |letters: &[char]| {
        letters
            .iter()
            .map(|letter| letter.width().unwrap_or(0))
            .sum::<usize>()
    };
    let room = room.saturating_sub(width(CUT));
    let (mut head, mut tail) = (0, 0);
    // The end gets the odd cell, since a word's end tells most apart.
    while head + tail < letters.len() {
        let longer_tail = fits(&letters[letters.len() - tail - 1..]) + fits(&letters[..head]);
        let longer_head = fits(&letters[letters.len() - tail..]) + fits(&letters[..=head]);
        if tail <= head && longer_tail <= room {
            tail += 1;
        } else if longer_head <= room {
            head += 1;
        } else if longer_tail <= room {
            tail += 1;
        } else {
            break;
        }
    }
    let head: String = letters[..head].iter().collect();
    let tail: String = letters[letters.len() - tail..].iter().collect();
    format!("{head}{CUT}{tail}")
}

/// Words set as a paragraph `inset` cells in: wrapped at whole words to
/// the room, every line at the same inset, so a nested level keeps its
/// indent wherever it wraps. Into a pipe it's one line.
pub fn paragraph(words: &str, inset: usize, room: Room) -> Vec<String> {
    let pad = " ".repeat(inset);
    match room.cells() {
        None => vec![format!("{pad}{words}")],
        Some(cells) => wrapped(words, cells.saturating_sub(inset).max(1))
            .into_iter()
            .map(|line| format!("{pad}{line}"))
            .collect(),
    }
}

/// Words set after something `lead` cells wide: wrapped at whole words
/// to the room, each further line under the first's words. Into a pipe
/// they stay one line.
pub fn hung(words: &str, lead: usize, room: Room) -> String {
    match room.cells() {
        None => words.to_owned(),
        Some(cells) => wrapped(words, cells.saturating_sub(lead).max(1))
            .join(&format!("\n{}", " ".repeat(lead))),
    }
}

/// Words wrapped to `measure` cells: a run gives way at its items first,
/// the dot going with the break, so no line ends or starts on one, and
/// only an item too long for a line of its own breaks between its words,
/// never parting what `held` keeps together. A piece longer than the
/// measure is shortened in its middle, never broken across two: to the
/// room left on its line where that's at least `SHARES` cells, so it
/// doesn't leave the word before it alone, and otherwise on a line of
/// its own.
pub fn wrapped(words: &str, measure: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for item in words.split(ITEMS).map(str::trim) {
        if width(&parted(&line, item, ITEMS)) <= measure {
            line = parted(&line, item, ITEMS);
        } else if width(item) <= measure {
            lines.push(std::mem::replace(&mut line, item.to_owned()));
        } else {
            // An item's first piece is still parted from the line by its
            // dot; the rest of its pieces by spaces.
            let mut between = ITEMS;
            for piece in held(item) {
                let left = measure.saturating_sub(width(&line) + width(between));
                let piece = if width(&piece) > measure && !line.is_empty() && left >= SHARES {
                    shortened(&piece, left)
                } else {
                    shortened(&piece, measure)
                };
                if !line.is_empty() && width(&parted(&line, &piece, between)) > measure {
                    lines.push(std::mem::take(&mut line));
                }
                line = parted(&line, &piece, between);
                between = " ";
            }
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    if words.contains(ITEMS) {
        return lines;
    }
    unwidowed(lines, measure)
}

/// Lines with no last word left alone on its line: the word before it
/// comes down too, where both fit, the line it leaves keeps two, and it
/// doesn't part what `held` keeps together. Only for words with no run in
/// them, whose breaks are all spaces.
fn unwidowed(mut lines: Vec<String>, measure: usize) -> Vec<String> {
    let [.., before, last] = lines.as_mut_slice() else {
        return lines;
    };
    if last.contains(' ') {
        return lines;
    }
    let Some(at) = before.rfind(' ') else {
        return lines;
    };
    let (kept, word) = (before[..at].to_owned(), before[at + 1..].to_owned());
    let parts_a_figure = held(&before[before[..at].rfind(' ').map_or(0, |at| at + 1)..]).len() == 1;
    // The line it leaves keeps two words at least, or it'd only move the
    // lone word up a line.
    if !kept.contains(' ') || parts_a_figure || width(&word) + 1 + width(last) > measure {
        return lines;
    }
    *last = format!("{word} {last}");
    *before = kept;
    lines
}

/// The fewest cells a shortened word is given on a line it shares; with
/// less, it takes a line of its own, where it keeps more of itself.
const SHARES: usize = 24;

/// `line` with `more` after it, `between` them where both say something.
fn parted(line: &str, more: &str, between: &str) -> String {
    match (line.is_empty(), more.is_empty()) {
        (true, _) => more.to_owned(),
        (false, true) => line.to_owned(),
        (false, false) => format!("{line}{between}{more}"),
    }
}

/// The longest a unit after a figure is, as `GB` and `ms` are, so a word
/// after a figure that isn't its unit can part from it.
const UNIT: usize = 3;

/// The short words English joins a sentence with, which follow a figure
/// as often as a unit does (`Pro 6 on 5 GHz`) but never measure it, so a
/// line may part a figure from them.
const JOINING: [&str; 22] = [
    "a", "an", "and", "are", "as", "at", "but", "by", "for", "if", "in", "is", "it", "nor", "of",
    "on", "or", "so", "the", "to", "was", "yet",
];

/// What a break mustn't part: a figure and its unit (`412 GB`), and
/// `\u{d7}` and what it joins (`1272 \u{d7} 768`), each as one piece. A
/// bracket before a figure or a stop after its unit (`(16 KiB).`) doesn't
/// hide either.
fn held(words: &str) -> Vec<String> {
    let figure = |word: &str| {
        word.trim_start_matches(['(', '[', '~', '\u{2212}', '-', '+'])
            .starts_with(|first: char| first.is_ascii_digit())
    };
    let unit = |word: &str| {
        let bare = word.trim_end_matches(|last: char| last.is_ascii_punctuation());
        !figure(word) && width(bare) <= UNIT && !JOINING.contains(&bare)
    };
    let mut pieces: Vec<String> = Vec::new();
    let mut was: Option<&str> = None;
    for word in words.split_whitespace() {
        let joins = word == "\u{d7}"
            || was.is_some_and(|was| was == "\u{d7}" || (figure(was) && unit(word)));
        match pieces.last_mut() {
            Some(last) if joins => {
                last.push(' ');
                last.push_str(word);
            }
            _ => pieces.push(word.to_owned()),
        }
        was = Some(word);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_gives_way_at_a_whole_word_and_says_so() {
        // A first word past the room is shortened in its middle, so a
        // name still fits and still shows both its ends.
        assert_eq!(cut("switch the system now", 12), "switch the\u{2026}");
        assert_eq!(cut("switch the system now", 5), "sw\u{2026}ch");
        assert_eq!(cut("status", 6), "status");
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
}
