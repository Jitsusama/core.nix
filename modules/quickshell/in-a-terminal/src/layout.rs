//! What the launcher shows, laid out for a terminal as the launcher lays
//! it on the card: a place's or a row's head, Home's at home, its drawer's
//! facts and words when it has a drawer, and then its rows, its acts left
//! to `launcher do` and `--json`, since a terminal can't press them, each its glyph in its hue, its
//! name, what it holds after it in muted, and its side, right-aligned in one
//! column. Type is the material: only the title is bold, and space alone
//! groups, one empty line between the drawer's groups and between the
//! drawer and the rows. It fits the terminal's width: a row's count and then
//! its side give way by whole items from their ends, and a name is cut
//! only at a whole word.

use std::env;

use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::look::Look;
use crate::words::Words;

/// How a run of items is joined, on a row as in the launcher.
const ITEMS: &str = " \u{b7} ";

/// How a cut name ends.
const CUT: &str = "\u{2026}";

/// How a path's steps are said in its head, as the launcher's field says
/// them.
const CRUMB: &str = " \u{203a} ";

/// The least room between what a row says and its side, and between a
/// fact's label and its value.
const GAP: usize = 2;

/// The widest a drawer's words are set, a reading measure, however wide
/// the terminal.
const MEASURE: usize = 80;

/// How wide the terminal is, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room(pub usize);

impl Room {
    /// The terminal's width: `COLUMNS` where the shell exports it, else
    /// what the terminal says, else eighty.
    pub fn of_stdout() -> Self {
        Self::of(1)
    }

    /// The width failures are set to: stderr's terminal's, as stdout's is.
    pub fn of_stderr() -> Self {
        Self::of(2)
    }

    fn of(fd: i32) -> Self {
        let columns = env::var("COLUMNS")
            .ok()
            .and_then(|columns| columns.parse().ok());
        Self(
            columns
                .or_else(|| terminal_width(fd))
                .filter(|width| *width > 0)
                .unwrap_or(80),
        )
    }
}

/// What the terminal on a file descriptor says its width is.
fn terminal_width(fd: i32) -> Option<usize> {
    #[repr(C)]
    struct WindowSize {
        rows: u16,
        columns: u16,
        across: u16,
        down: u16,
    }
    unsafe extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    const TIOCGWINSZ: u64 = 0x5413;
    let mut size = WindowSize {
        rows: 0,
        columns: 0,
        across: 0,
        down: 0,
    };
    // SAFETY: TIOCGWINSZ writes one winsize, which WindowSize lays out as
    // the kernel does, into memory this frame owns, and reads nothing else.
    let done = unsafe { ioctl(fd, TIOCGWINSZ, &raw mut size) };
    (done == 0 && size.columns > 0).then_some(usize::from(size.columns))
}

/// A string's width in cells.
fn width(words: &str) -> usize {
    UnicodeWidthStr::width(words)
}

/// A field of a JSON object as words, or none.
fn said<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// Everything the shell showed, as lines for a terminal.
pub fn shown(shown: &Value, look: &Look, room: Room, words: &Words) -> String {
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let rows: Vec<&Value> = shown
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default();
    let row = shown.get("row").filter(|row| row.is_object());
    if row.is_none() {
        blocks.push(vec![look.title(words.home())]);
    }
    if let Some(row) = row {
        if let Some(drawer) = row.get("drawer").filter(|drawer| drawer.is_object()) {
            blocks.extend(drawn(drawer, row, look, room, words));
        } else {
            blocks.push(vec![head(shown, look)]);
            // A row holding nothing says itself, its side among it.
            if rows.is_empty() {
                blocks.push(listed(&[row], look, room));
            }
        }
    }
    if !rows.is_empty() {
        blocks.push(listed(&rows, look, room));
    }
    let mut out = blocks
        .into_iter()
        .filter(|block| !block.is_empty())
        .map(|block| block.join("\n"))
        .collect::<Vec<_>>()
        .join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Where a row is, as the launcher's field says it: the steps before it in
/// muted, and its name as the title.
fn head(shown: &Value, look: &Look) -> String {
    let steps: Vec<&str> = shown
        .get("in")
        .and_then(Value::as_array)
        .map(|steps| steps.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    match steps.split_last() {
        None => String::new(),
        Some((last, [])) => look.title(last),
        Some((last, before)) => {
            format!(
                "{}{}",
                look.ink("muted", &format!("{}{CRUMB}", before.join(CRUMB))),
                look.title(last)
            )
        }
    }
}

/// A drawer as blocks: its head (the eyebrow in muted, the title), its
/// facts, its words, and the moment's groups of facts.
fn drawn(drawer: &Value, row: &Value, look: &Look, room: Room, words: &Words) -> Vec<Vec<String>> {
    let mut head = Vec::new();
    let eyebrow = said(drawer, "eyebrow");
    if !eyebrow.is_empty() {
        head.push(look.ink("muted", eyebrow));
    }
    let title = match said(drawer, "title") {
        "" => said(row, "name"),
        title => title,
    };
    head.push(look.title(title));
    let mut blocks = vec![head];
    if let Some(facts) = drawer.get("facts").and_then(Value::as_array) {
        blocks.extend(facts_laid(facts, look, room, words));
    }
    let body = said(drawer, "body");
    if !body.is_empty() {
        let measure = room.0.min(MEASURE);
        let lines = body
            .lines()
            .flat_map(|line| wrapped(line, measure))
            .map(|line| look.ink("text", &line));
        blocks.push(lines.collect());
    }
    for group in drawer
        .get("groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(facts) = group.as_array() {
            blocks.extend(facts_laid(facts, look, room, words));
        }
    }
    blocks
}

/// Facts as blocks, a heading starting a block of its own: each label in
/// muted in one column, its value in text beside it, a value not read yet
/// as the drawer's placeholder, in muted. Where the labels would take
/// more than half the room, each value goes under its label.
fn facts_laid(facts: &[Value], look: &Look, room: Room, words: &Words) -> Vec<Vec<String>> {
    let labels = facts
        .iter()
        .filter(|fact| fact.get("heading").is_none())
        .map(|fact| width(said(fact, "label")))
        .max()
        .unwrap_or(0);
    let beside = labels + GAP <= room.0 / 2;
    let column = if beside { labels + GAP } else { GAP };
    let mut blocks: Vec<Vec<String>> = vec![Vec::new()];
    for fact in facts {
        if let Some(heading) = fact.get("heading").and_then(Value::as_str) {
            if blocks.last().is_some_and(|block| !block.is_empty()) {
                blocks.push(Vec::new());
            }
            push(&mut blocks, look.ink("muted", heading));
            continue;
        }
        let label = said(fact, "label");
        let (value, role) = match fact.get("value").and_then(Value::as_str) {
            Some(value) => (value.to_string(), "text"),
            None => (words.waiting(), "muted"),
        };
        let qualifier = said(fact, "qualifier");
        let mut lines = wrapped(&value, room.0.saturating_sub(column).max(1)).into_iter();
        let first = lines.next().unwrap_or_default();
        let mut first = look.ink(role, &first);
        if !qualifier.is_empty() {
            first = format!("{first} {}", look.ink("muted", qualifier));
        }
        let inset = " ".repeat(column);
        if beside {
            let pad = " ".repeat(column - width(label));
            push(
                &mut blocks,
                format!("{}{pad}{first}", look.ink("muted", label)),
            );
        } else {
            push(&mut blocks, look.ink("muted", label));
            push(&mut blocks, format!("{inset}{first}"));
        }
        for line in lines {
            push(&mut blocks, format!("{inset}{}", look.ink(role, &line)));
        }
    }
    blocks
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect()
}

fn push(blocks: &mut [Vec<String>], line: String) {
    if let Some(block) = blocks.last_mut() {
        block.push(line);
    }
}

/// One row as it's laid: its glyph, name, count and side, before inking.
struct Laid<'a> {
    glyph: String,
    hue: &'a str,
    name: String,
    count: String,
    side: String,
    side_role: &'static str,
}

/// Rows, each a line: the glyph in its hue in a column of its own, the
/// name, the count after it, and the side right-aligned at the widest row
/// that fits the room.
fn listed(rows: &[&Value], look: &Look, room: Room) -> Vec<String> {
    let laid: Vec<Laid> = rows
        .iter()
        .map(|row| Laid {
            glyph: look.mark(said(row, "glyph")),
            hue: said(row, "hue"),
            name: said(row, "name").to_string(),
            count: said(row, "count").to_string(),
            side: said(row, "side").to_string(),
            side_role: if row.get("alerts").and_then(Value::as_bool) == Some(true) {
                "alert"
            } else if row.get("warns").and_then(Value::as_bool) == Some(true) {
                "warning"
            } else {
                "muted"
            },
        })
        .collect();
    let glyphs = laid.iter().map(|row| width(&row.glyph)).max().unwrap_or(0);
    let lead = if glyphs > 0 { glyphs + 1 } else { 0 };
    let natural = |row: &Laid| {
        let left = lead
            + width(&row.name)
            + if row.count.is_empty() {
                0
            } else {
                GAP + width(&row.count)
            };
        left + if row.side.is_empty() {
            0
        } else {
            GAP + width(&row.side)
        }
    };
    let across = laid.iter().map(natural).max().unwrap_or(0).min(room.0);
    laid.into_iter()
        .map(|row| line(row, lead, across, look))
        .collect()
}

/// One row's line, fitted to `across` cells: its count gives way first,
/// then its side, each by whole items from its end, then its name at a
/// whole word.
fn line(row: Laid, lead: usize, across: usize, look: &Look) -> String {
    let Laid {
        glyph,
        hue,
        name,
        count,
        side,
        side_role,
    } = row;
    let mut count = count;
    let mut side = side;
    let mut name = name;
    let needs = |name: &str, count: &str, side: &str| {
        lead + width(name)
            + if count.is_empty() {
                0
            } else {
                GAP + width(count)
            }
            + if side.is_empty() {
                0
            } else {
                GAP + width(side)
            }
    };
    while needs(&name, &count, &side) > across && !count.is_empty() {
        count = fewer(&count);
    }
    while needs(&name, &count, &side) > across && !side.is_empty() {
        side = fewer(&side);
    }
    if needs(&name, &count, &side) > across {
        let room = across.saturating_sub(needs("", &count, &side));
        name = cut(&name, room);
    }
    let mut out = String::new();
    if lead > 0 {
        out.push_str(&look.ink(hue, &glyph));
        out.push_str(&" ".repeat(lead - width(&glyph)));
    }
    out.push_str(&look.ink("text", &name));
    let mut used = lead + width(&name);
    if !count.is_empty() {
        out.push_str(&" ".repeat(GAP));
        out.push_str(&look.ink("muted", &count));
        used += GAP + width(&count);
    }
    if !side.is_empty() {
        out.push_str(&" ".repeat(across.saturating_sub(used + width(&side)).max(GAP)));
        out.push_str(&look.ink(side_role, &side));
    }
    out
}

/// Items less their last: "a · b · c" as "a · b", and one item as none.
fn fewer(items: &str) -> String {
    items
        .rsplit_once(ITEMS)
        .map(|(kept, _)| kept.to_string())
        .unwrap_or_default()
}

/// A name cut to `room` cells at a whole word, ending in an ellipsis. Its
/// first word is kept whole however long, since a name is never cut
/// inside a word.
fn cut(name: &str, room: usize) -> String {
    if width(name) <= room {
        return name.to_string();
    }
    let mut kept = String::new();
    for word in name.split(' ') {
        let longer = if kept.is_empty() {
            word.to_string()
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

/// Words wrapped to `measure` cells at whole words; a word longer than the
/// measure stands on its own line, never broken.
/// Words set after something `lead` cells wide, as a failure's are after
/// its mark: wrapped at whole words to the room, each further line under
/// the first's words, so a name is never cut where the terminal ends.
pub fn hung(words: &str, lead: usize, room: Room) -> String {
    let measure = room.0.saturating_sub(lead).max(1);
    wrapped(words, measure).join(&format!("\n{}", " ".repeat(lead)))
}

fn wrapped(words: &str, measure: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in words.split_whitespace() {
        if !line.is_empty() && width(&line) + 1 + width(word) > measure {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::{Colours, Ink};

    fn plain() -> Look {
        Look::from_value(
            &Value::Null,
            Ink {
                colours: Colours::None,
                console: false,
            },
        )
    }

    fn words() -> Words {
        Words::from_value(
            &serde_json::from_str(r#"{"terminal":{"waiting":"\u2026"},"field":{"home":"Home"}}"#)
                .expect("parses"),
        )
    }

    fn laid(json: &str, across: usize) -> String {
        shown(
            &serde_json::from_str(json).expect("the answer parses"),
            &plain(),
            Room(across),
            &words(),
        )
    }

    const HOME: &str = r#"{"in":[],"path":[],"row":null,"rows":[
        {"id":"on","name":"Now","glyph":"\u25c8","hue":"voice","count":"2 need you"},
        {"id":"computer","name":"Machine","glyph":"\u25c6","hue":"machine",
         "count":"Disk \u00b7 Power \u00b7 Sound"},
        {"id":"disk","name":"Disk","glyph":"\u25c6","side":"412 GB free \u00b7 nvme0n1"}]}"#;

    #[test]
    fn home_lists_its_rows_their_sides_in_one_column() {
        assert_eq!(
            laid(HOME, 100),
            "Home\n\n\
             \u{25c8} Now  2 need you\n\
             \u{25c6} Machine  Disk \u{b7} Power \u{b7} Sound\n\
             \u{25c6} Disk    412 GB free \u{b7} nvme0n1\n"
        );
    }

    #[test]
    fn a_count_gives_way_first_then_a_side_by_whole_items() {
        assert_eq!(
            laid(HOME, 24),
            "Home\n\n\
             \u{25c8} Now  2 need you\n\
             \u{25c6} Machine  Disk \u{b7} Power\n\
             \u{25c6} Disk       412 GB free\n"
        );
    }

    #[test]
    fn a_name_is_cut_only_at_a_whole_word() {
        assert_eq!(cut("Sessions to Reopen", 14), "Sessions to\u{2026}");
        assert_eq!(cut("Sessions to Reopen", 5), "Sessions\u{2026}");
        assert_eq!(cut("Disk", 5), "Disk");
    }

    #[test]
    fn a_place_with_no_drawer_heads_its_rows_with_where_it_is() {
        let place = r#"{"in":["Machine","Disk"],"path":["computer","disk"],
            "row":{"id":"computer\u203adisk","name":"Disk","side":"412 GB free"},
            "rows":[{"id":"s","name":"Space","side":"412 GB free"}]}"#;
        assert_eq!(
            laid(place, 80),
            "Machine \u{203a} Disk\n\nSpace  412 GB free\n"
        );
    }

    #[test]
    fn a_place_leaves_its_acts_to_launcher_do() {
        let place = r#"{"in":["Machine","Disk"],"path":["computer","disk"],
            "row":{"id":"computer\u203adisk","name":"Disk"},
            "rows":[{"id":"s","name":"Space","kind":"figure","side":"412 GB free"}],
            "acts":[{"id":"c","name":"Copy","kind":"action"}]}"#;
        assert_eq!(
            laid(place, 80),
            "Machine \u{203a} Disk\n\nSpace  412 GB free\n"
        );
    }

    #[test]
    fn a_row_holding_nothing_says_itself() {
        let row = r#"{"in":["Machine","Uptime"],"path":["a","b"],
            "row":{"id":"a\u203ab","name":"Uptime","side":"2 hours"},"rows":[]}"#;
        assert_eq!(
            laid(row, 80),
            "Machine \u{203a} Uptime\n\nUptime  2 hours\n"
        );
    }

    #[test]
    fn a_drawer_heads_with_its_eyebrow_and_title_then_its_facts() {
        let thing = r#"{"in":["Machine","Disk","nvme0n1"],"path":[],
            "row":{"id":"x","name":"nvme0n1","drawer":{"eyebrow":"Disk","title":"Samsung PM9A1",
              "facts":[{"label":"Size","value":"1 TB"},{"label":"Temperature","value":null},
                {"heading":"Health"},{"label":"Wear","value":"3","qualifier":"%"}],
              "body":"Read and written since it was made."}},
            "rows":[{"id":"y","name":"Copy"}]}"#;
        assert_eq!(
            laid(thing, 80),
            "Disk\nSamsung PM9A1\n\n\
             Size         1 TB\n\
             Temperature  \u{2026}\n\n\
             Health\n\
             Wear         3 %\n\n\
             Read and written since it was made.\n\n\
             Copy\n"
        );
    }

    #[test]
    fn a_long_value_wraps_under_itself() {
        let thing = r#"{"in":["A"],"path":[],"row":{"id":"a","name":"A","drawer":{"title":"A",
            "facts":[{"label":"Says","value":"one two three four"}]}},"rows":[]}"#;
        assert_eq!(laid(thing, 16), "A\n\nSays  one two\n      three four\n");
    }

    #[test]
    fn labels_too_wide_for_the_room_set_values_under_them() {
        let thing = r#"{"in":["A"],"path":[],"row":{"id":"a","name":"A","drawer":{"title":"A",
            "facts":[{"label":"A long label","value":"1"}]}},"rows":[]}"#;
        assert_eq!(laid(thing, 20), "A\n\nA long label\n  1\n");
    }

    #[test]
    fn words_wrap_at_whole_words_and_never_break_one() {
        assert_eq!(wrapped("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrapped("unbreakable", 4), vec!["unbreakable"]);
        assert_eq!(wrapped("", 4), vec![""]);
    }

    #[test]
    fn a_failure_wraps_under_its_own_words() {
        let said = "nothing called mahcine \u{b7} did you mean Machine?";
        assert_eq!(
            hung(said, 2, Room(42)),
            "nothing called mahcine \u{b7} did you mean\n  Machine?"
        );
        assert_eq!(hung(said, 2, Room(80)), said);
    }
}
