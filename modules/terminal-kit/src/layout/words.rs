//! How words fit the room they're given (docs/commands.md, Width and What
//! Gives Way): prose wrapped at whole words keeping its inset, a run giving
//! way at its items, a figure held to its unit, no last word left alone,
//! and a word too long for its line shortened in its middle, never broken.
//! Lists, facts and tables set their cells with these, once each is
//! drawn as the terminal may be given it. Each holds its words to one
//! line first, as `one_line` says, since a break of their own would
//! draw past the inset or the measure they keep. A paragraph and hung
//! words do too: the kit can't tell a break a command wrote from one in
//! a name it was given, and one in a name would draw a line of its own
//! at the inset, where it could pass for a failure's try line. A command
//! with several lines to say sets each apart.

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

use super::{CUT, GAP, ITEMS, Room, width};

/// The small words a cut never ends on, so it reads as a cut rather than
/// a sentence missing its end (VKCF3Z, 2026-11-15), as the password
/// prompt's titles are cut too.
const SMALL: [&str; 12] = [
    "a", "an", "the", "of", "for", "with", "to", "in", "on", "and", "so", "from",
];

/// Words as the kit draws them: every control character dropped but the
/// line break, a tab and a Unicode line or paragraph separator drawn as a
/// space, so no break is made that a filter for line breaks upstream
/// never saw, and the marks that reorder text right to
/// left dropped too, since one can draw a name backwards over what's
/// beside it. A name, a path or a message
/// comes from anyone, an eero device's name or a Wi-Fi network's among
/// them, and one holding the terminal's own escapes would retitle the
/// window, clear the screen or draw over a line as it's shown; only the
/// kit spells escapes (docs/commands.md, Colour and Marks). The words are
/// borrowed as they are where they hold none, as nearly all do. The
/// vertical tab, form feed and next line part words as a tab does, so
/// they're drawn as a space too; a return is dropped, since nearly every
/// one ends a line before its break.
pub fn drawn(words: &str) -> Cow<'_, str> {
    kept(words, |c| match c {
        '\n' => Some('\n'),
        c if parts(c) => Some(' '),
        _ => None,
    })
}

/// Words from outside on one line, each run of space closed to one, so
/// none can wrap into a line reading as a failure's try line, whose `try`
/// two spaces mark. Drawn first, so a control dropped joins what stood
/// either side of it rather than parting them. A blank that isn't a space
/// to Unicode, braille's or the Hangul filler the pen keeps to shape a
/// syllable, still draws as one, so it closes with the rest. A word that
/// draws nothing, a joiner the pen keeps standing alone, goes, or the
/// blanks either side of it would draw as two.
pub fn spaced(words: &str) -> String {
    one_line(words)
        .split(|c: char| c.is_whitespace() || matches!(c, '\u{115f}' | '\u{2800}'))
        .filter(|word| super::width(word) > 0)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A command to type, when it draws safely just as it was written: on one
/// line, with nothing the pen drops, and each blank between its words a
/// single one a cell wide. Nothing otherwise, since changing a command
/// to draw it safely would have it name something it didn't, a device
/// whose name holds two spaces as another without them, and drawing it
/// as written could pad it to where the terminal's own wrap starts a row
/// reading as a try. A character that draws nothing just after a blank
/// counts as a blank of its own, since the two would draw as two.
pub fn typed(words: &str) -> Option<&str> {
    if one_line(words) != words {
        return None;
    }
    let blank = |c: char| c.is_whitespace() || matches!(c, '\u{115f}' | '\u{2800}');
    // At the start, as after a blank, so a blank leading it is refused.
    let mut after_blank = true;
    for c in words.chars() {
        let cells = c.width();
        if blank(c) {
            if after_blank || cells != Some(1) {
                return None;
            }
            after_blank = true;
        } else if after_blank && cells.unwrap_or(0) == 0 {
            return None;
        } else {
            after_blank = false;
        }
    }
    (!after_blank).then_some(words)
}

/// Whether a character parts words without a line break of its own.
fn parts(c: char) -> bool {
    matches!(
        c,
        '\t' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// The same, held to one line, as a table's cell, a list's row or a fact
/// is: a line break is a space too, so no field draws a record of its
/// own, and every line read through `grep` still names its record.
pub fn one_line(words: &str) -> Cow<'_, str> {
    kept(words, |c| (c == '\n' || parts(c)).then_some(' '))
}

/// Whether the kit keeps `c` from the terminal: a control character, a
/// Unicode line or paragraph separator, or a mark that sets, embeds,
/// overrides or isolates a run of text's direction, the implicit ones
/// (left-to-right, right-to-left and Arabic letter marks) among them, or
/// one of Unicode's default-ignorable characters that draws nothing and
/// shapes nothing: the zero-width space, the combining grapheme joiner,
/// Khmer's unwritten vowels, the word joiner and the format controls
/// after it, the byte-order mark, the blank Hangul fillers that stand
/// alone, the interlinear marks, shorthand and music formatting, and the
/// language tag with the unassigned rest of its block. Those would hide
/// a difference between two names drawn alike. The soft hyphen, the
/// joiners and fillers that shape a script, variation selectors and the
/// tags that spell a flag stay, since an emoji or a word is drawn wrong
/// without them; a command that compares names folds those away itself.
pub(crate) fn unseen(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{034f}'
                | '\u{061c}'
                | '\u{17b4}'
                | '\u{17b5}'
                | '\u{200b}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{3164}'
                | '\u{feff}'
                | '\u{ffa0}'
                | '\u{fff0}'..='\u{fffb}'
                | '\u{1bca0}'..='\u{1bca3}'
                | '\u{1d173}'..='\u{1d17a}'
                | '\u{e0000}'..='\u{e001f}'
                | '\u{e0080}'..='\u{e00ff}'
                | '\u{e01f0}'..='\u{e0fff}'
        )
}

/// Words with each character `unseen` keeps given to `instead`, which
/// draws it as another character or drops it.
fn kept(words: &str, instead: impl Fn(char) -> Option<char>) -> Cow<'_, str> {
    if !words.chars().any(unseen) {
        return Cow::Borrowed(words);
    }
    Cow::Owned(
        words
            .chars()
            .filter_map(|c| if unseen(c) { instead(c) } else { Some(c) })
            .collect(),
    )
}

/// A name cut to `room` cells at a whole word, ending in an ellipsis, and
/// never after a small word, though its first word is kept whatever. A
/// first word too long for the room alone is shortened in its middle, as
/// a path is, since a name is never cut at its end inside a word.
pub fn cut(name: &str, room: usize) -> String {
    let flat = one_line(name);
    let name = &*flat;
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
    while let Some((before, last)) = kept.rsplit_once(' ')
        && SMALL.contains(&last.to_lowercase().as_str())
    {
        kept.truncate(before.len());
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
    let flat = one_line(word);
    let word = &*flat;
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
/// indent wherever it wraps. The words are held to one line first, as
/// the module says; into a pipe that one line is the paragraph.
pub fn paragraph(words: &str, inset: usize, room: Room) -> Vec<String> {
    let pad = " ".repeat(inset);
    set(words, inset, room)
        .into_iter()
        .map(|line| format!("{pad}{line}"))
        .collect()
}

/// Words set after something `lead` cells wide: wrapped at whole words
/// to the room, each further line under the first's words. The words are
/// held to one line first, as the module says; into a pipe they stay on
/// it.
pub fn hung(words: &str, lead: usize, room: Room) -> String {
    set(words, lead, room).join(&format!("\n{}", " ".repeat(lead)))
}

/// Words set as a hanging paragraph: the first line `inset` cells in,
/// each further line `GAP` cells further, so where one entry of a list
/// ends and the next begins reads at a glance. Each line is wrapped to
/// the room its own inset leaves, so the first keeps the cells the hang
/// gives up. Where `hung` sets words after something already drawn `lead`
/// cells wide and hands back the lines without the first's lead, this
/// draws every line's inset itself, one line to an entry. The words are
/// held to one line first, as the module says; into a pipe that one line
/// is the paragraph. A room too narrow for the hang still draws a cell
/// of words a line, so below `inset + GAP + 1` cells the lines under the
/// first run past it, and at `inset` cells or fewer the first does too,
/// each word wider than its cell drawn as its ellipsis alone, as nothing
/// narrower could read.
pub fn hanging(words: &str, inset: usize, room: Room) -> Vec<String> {
    let lines = match room.cells() {
        None => vec![one_line(words).trim().to_owned()],
        Some(cells) => {
            let first = cells.saturating_sub(inset).max(1);
            measured(words, first, first.saturating_sub(GAP).max(1))
        }
    };
    lines
        .into_iter()
        .enumerate()
        .map(|(at, line)| {
            let lead = if at == 0 { inset } else { inset + GAP };
            format!("{}{line}", " ".repeat(lead))
        })
        .collect()
}

/// The words held to one line, wrapped to the room left after `inset`
/// cells, ready to set at the inset; into a pipe, where nothing wraps,
/// the one line without the space a trailing break left.
fn set(words: &str, inset: usize, room: Room) -> Vec<String> {
    match room.cells() {
        None => vec![one_line(words).trim().to_owned()],
        Some(cells) => wrapped(words, cells.saturating_sub(inset).max(1)),
    }
}

/// Words wrapped to `measure` cells, every line to the same measure, the
/// first among them; `hanging` gives its first line more room than the
/// rest. A run gives way at its items first, the dot going with the
/// break, so no line ends or starts on one, and only an item too long for
/// a line of its own breaks between its words, never parting what `held`
/// keeps together. A held piece wider than the measure gives way at its
/// words before any is shortened, since a figure cut to fit its unit
/// loses the digits it's read for. A word longer than the measure is
/// shortened in its middle, never broken across two: to the room left on
/// its line where that's at least `SHARES` cells, so it doesn't leave the
/// word before it alone, and otherwise on a line of its own.
pub fn wrapped(words: &str, measure: usize) -> Vec<String> {
    measured(words, measure, measure)
}

/// Words wrapped as `wrapped` says, the first line to `first` cells and
/// each after it to `rest`, which is never more than `first`, so an item
/// too wide for the first line is too wide for the next too. Where the
/// first is wider, a word left under `SHARES` cells beside the words
/// before it is shortened to the rest's measure, as a line of its own
/// would cut it, and still stays on the first line where it fits there,
/// since moving it down would cut it no less.
fn measured(words: &str, first: usize, rest: usize) -> Vec<String> {
    let flat = one_line(words);
    let words = &*flat;
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let measure_of = |at: usize| if at == 0 { first } else { rest.min(first) };
    for item in words.split(ITEMS).map(str::trim) {
        if width(&parted(&line, item, ITEMS)) <= measure_of(lines.len()) {
            line = parted(&line, item, ITEMS);
        } else if !line.is_empty() && width(item) <= measure_of(lines.len() + 1) {
            lines.push(std::mem::replace(&mut line, item.to_owned()));
        } else {
            // An item's first piece is still parted from the line by its
            // dot; the rest of its pieces by spaces.
            let mut between = ITEMS;
            let pieces = held(item).into_iter().flat_map(|piece| {
                if width(&piece) > rest.min(first) && piece.contains(' ') {
                    piece.split(' ').map(str::to_owned).collect()
                } else {
                    vec![piece]
                }
            });
            for piece in pieces {
                // The line it's on, or where it goes on one of its own.
                let here = measure_of(lines.len());
                let own = measure_of(lines.len() + usize::from(!line.is_empty()));
                let left = here.saturating_sub(width(&line) + width(between));
                let piece = if width(&piece) > own && !line.is_empty() && left >= SHARES {
                    shortened(&piece, left)
                } else {
                    shortened(&piece, own)
                };
                if !line.is_empty() && width(&parted(&line, &piece, between)) > here {
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
    // The last line is never the first, so it keeps to the rest's measure.
    unwidowed(lines, rest.min(first))
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

/// What a break mustn't part: a figure and its unit (`412 GB`) or its
/// rate (`10.3 Mb/s`), and `\u{d7}` and what it joins (`1272 \u{d7} 768`),
/// each as one piece. A bracket before a figure or a stop after its unit
/// (`(16 KiB).`) doesn't hide either.
fn held(words: &str) -> Vec<String> {
    let figure = |word: &str| {
        word.trim_start_matches(['(', '[', '~', '\u{2212}', '-', '+'])
            .starts_with(|first: char| first.is_ascii_digit())
    };
    let unit = |word: &str| {
        let bare = word.trim_end_matches(|last: char| last.is_ascii_punctuation());
        // A rate is a unit a second, however long its name (`Mb/s`,
        // `requests/s`), and as much the figure's. A bare `/s` is short
        // enough to be a unit.
        let rate = bare
            .strip_suffix("/s")
            .is_some_and(|per| per.chars().all(char::is_alphabetic));
        !figure(word) && (width(bare) <= UNIT || rate) && !JOINING.contains(&bare)
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
mod tests;
