//! How an answer is laid out in cells (docs/commands.md, How It Reads):
//! lists of rows, a mark's two cells, a name and its words, the side flush
//! right at the widest row; facts, a label in one column and its value
//! beside it; tables aligned by space under muted labels; and words
//! wrapped at whole words, which `words` sets. Into a terminal it fits the
//! width, giving way least needed first; into a pipe nothing gives way. The
//! launcher in a terminal set these first
//! (modules/quickshell/in-a-terminal/src/layout.rs).

use unicode_width::UnicodeWidthStr;

use crate::environment::Environment;
use crate::ink::Pen;

mod words;

pub use words::{cut, hung, paragraph, shortened, wrapped};

/// How a run of items is joined on one line.
pub const ITEMS: &str = " \u{b7} ";

/// How a cut name ends, where the theme's `more` mark isn't to hand.
const CUT: &str = "\u{2026}";

/// The least room between columns, and between a row's text and its side.
pub const GAP: usize = 2;

/// The cells a mark and its space take.
pub const MARK: usize = 2;

/// The widest prose is set, a reading measure, however wide the terminal.
pub const MEASURE: usize = 80;

/// How wide a stream is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    /// A terminal this many cells wide.
    Cells(usize),
    /// A pipe, where nothing gives way.
    Unbounded,
}

impl Room {
    /// Stdout's width: `COLUMNS`, then its terminal's, then eighty; into a
    /// pipe, unbounded. `COLUMNS` comes first because neither zsh nor bash
    /// exports it, so it's in a command's environment only when someone
    /// asked for that width, as git and man read it.
    pub fn of_stdout(environment: &Environment) -> Self {
        Self::of(1, environment.stdout, environment)
    }

    /// Stderr's width, as stdout's is found.
    pub fn of_stderr(environment: &Environment) -> Self {
        Self::of(2, environment.stderr, environment)
    }

    fn of(fd: i32, terminal: bool, environment: &Environment) -> Self {
        if !terminal {
            return Self::Unbounded;
        }
        let columns = environment
            .var("COLUMNS")
            .and_then(|columns| columns.parse().ok())
            .filter(|width: &usize| *width > 0);
        Self::Cells(columns.or_else(|| terminal_width(fd)).unwrap_or(80))
    }

    /// The cells there are, where they're counted.
    pub fn cells(self) -> Option<usize> {
        match self {
            Self::Cells(cells) => Some(cells),
            Self::Unbounded => None,
        }
    }

    /// The measure prose is set to: the room, at most eighty, and eighty
    /// into a pipe, since help is read by a person wherever it goes.
    pub fn measure(self) -> usize {
        self.cells().map_or(MEASURE, |cells| cells.min(MEASURE))
    }

    /// The room prose is set in: a terminal's at most the measure, and a
    /// pipe's unbounded, so a line read by `grep` is never parted.
    #[must_use]
    pub fn for_prose(self) -> Self {
        match self {
            Self::Cells(cells) => Self::Cells(cells.min(MEASURE)),
            Self::Unbounded => Self::Unbounded,
        }
    }

    /// The room left inside an inset of `cells`, as a nested block has.
    #[must_use]
    pub fn less(self, cells: usize) -> Self {
        match self {
            Self::Cells(room) => Self::Cells(room.saturating_sub(cells).max(1)),
            Self::Unbounded => Self::Unbounded,
        }
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
        fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
    }
    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x4008_7468;
    #[cfg(not(target_os = "macos"))]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
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

/// Words' width in cells, by Unicode's width.
pub fn width(words: &str) -> usize {
    UnicodeWidthStr::width(words)
}

/// Words padded with spaces to `cells`, on the right or, for a figure,
/// on the left.
fn padded(words: &str, inked: String, cells: usize, right: bool) -> String {
    let pad = " ".repeat(cells.saturating_sub(width(words)));
    if right { pad + &inked } else { inked + &pad }
}

/// Items less their last: "a · b · c" as "a · b", and one item as none.
pub fn fewer(items: &str) -> String {
    items
        .rsplit_once(ITEMS)
        .map(|(kept, _)| kept.to_owned())
        .unwrap_or_default()
}

/// One row of a list: the mark it wears and in what role, its name, the
/// words it holds after its name, and its side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    /// A mark by what it means, and the role it's drawn in.
    pub mark: Option<(String, String)>,
    pub name: String,
    /// What it holds, in muted after its name; the first to give way.
    pub words: String,
    /// Its side, flush right, the next to give way.
    pub side: String,
    /// The side's role: muted, or alert or warning where it says so.
    pub side_role: String,
}

impl Row {
    /// A row by its name alone.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            side_role: "muted".to_owned(),
            ..Self::default()
        }
    }

    /// The same, wearing a mark in a role.
    #[must_use]
    pub fn marked(mut self, mark: &str, role: &str) -> Self {
        self.mark = Some((mark.to_owned(), role.to_owned()));
        self
    }

    /// The same, holding words after its name.
    #[must_use]
    pub fn holding(mut self, words: impl Into<String>) -> Self {
        self.words = words.into();
        self
    }

    /// The same, with a side in muted.
    #[must_use]
    pub fn side(mut self, side: impl Into<String>) -> Self {
        self.side = side.into();
        self
    }

    /// The same, its side in a role of the state it says.
    #[must_use]
    pub fn side_in(mut self, role: &str) -> Self {
        role.clone_into(&mut self.side_role);
        self
    }

    fn needs(lead: usize, name: &str, words: &str, side: &str) -> usize {
        let then = |words: &str| {
            if words.is_empty() {
                0
            } else {
                GAP + width(words)
            }
        };
        lead + width(name) + then(words) + then(side)
    }
}

/// Rows, each a line: its mark in its role in two cells of their own,
/// held blank on a row with none where another wears one; its name; its
/// words in muted after it; and its side flush right at the widest row,
/// in one column for the list. Into a terminal a row's words give way
/// first, then its side, each by whole items from its end, then its name
/// at a whole word.
pub fn list(rows: &[Row], pen: &Pen, room: Room) -> Vec<String> {
    let lead = if rows.iter().any(|row| row.mark.is_some()) {
        MARK
    } else {
        0
    };
    let natural = rows
        .iter()
        .map(|row| Row::needs(lead, &row.name, &row.words, &row.side))
        .max()
        .unwrap_or(0);
    let across = room.cells().map_or(natural, |cells| natural.min(cells));
    rows.iter()
        .map(|row| line(row, lead, across, pen))
        .collect()
}

fn line(row: &Row, lead: usize, across: usize, pen: &Pen) -> String {
    let mut words = row.words.clone();
    let mut side = row.side.clone();
    let mut name = row.name.clone();
    while Row::needs(lead, &name, &words, &side) > across && !words.is_empty() {
        words = fewer(&words);
    }
    while Row::needs(lead, &name, &words, &side) > across && !side.is_empty() {
        side = fewer(&side);
    }
    if Row::needs(lead, &name, &words, &side) > across {
        name = cut(
            &name,
            across.saturating_sub(Row::needs(lead, "", &words, &side)),
        );
    }
    let mut out = String::new();
    if lead > 0 {
        out.push_str(&match &row.mark {
            Some((mark, role)) => pen.marked(mark, role),
            None => " ".repeat(lead),
        });
    }
    out.push_str(&pen.role("text", &name));
    let mut used = lead + width(&name);
    if !words.is_empty() {
        out.push_str(&" ".repeat(GAP));
        out.push_str(&pen.role("muted", &words));
        used += GAP + width(&words);
    }
    if !side.is_empty() {
        out.push_str(&" ".repeat(across.saturating_sub(used + width(&side)).max(GAP)));
        out.push_str(&pen.role(&row.side_role, &side));
    }
    out
}

/// Facts, each a label in muted in one column and its value in text
/// beside it, a value it can't know said as `unknown` in muted. Where the
/// labels would take more than half the room, each value goes under its
/// label, two cells in. A long value wraps under itself.
pub fn facts(facts: &[(&str, Option<&str>)], unknown: &str, pen: &Pen, room: Room) -> Vec<String> {
    let labels = facts
        .iter()
        .map(|(label, _)| width(label))
        .max()
        .unwrap_or(0);
    let beside = room.cells().is_none_or(|cells| labels + GAP <= cells / 2);
    let column = if beside { labels + GAP } else { GAP };
    let mut lines = Vec::new();
    for (label, value) in facts {
        let (value, role) = value.map_or((unknown, "muted"), |value| (value, "text"));
        let inset = " ".repeat(column);
        let hung = hung(value, column, room);
        let mut values = hung
            .split('\n')
            .map(|line| pen.role(role, line.trim_start()));
        let first = values.next().unwrap_or_default();
        if beside {
            lines.push(padded(label, pen.role("muted", label), column, false) + &first);
        } else {
            let label = room
                .cells()
                .map_or_else(|| (*label).to_owned(), |cells| shortened(label, cells));
            lines.push(pen.role("muted", &label));
            lines.push(format!("{inset}{first}"));
        }
        lines.extend(values.map(|line| format!("{inset}{line}")));
    }
    lines
}

/// How a table's column is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Words, flush left.
    Left,
    /// Figures, flush right, so their places line up.
    Right,
}

/// A table: a label in muted capitals over each column, set apart by two
/// cells at least, never ruled or boxed, figures flush right. Every line
/// names its record in its first column, so a line read through `grep`
/// stands alone; into a pipe the labels still head it, and nothing is
/// cut.
pub fn table(columns: &[(&str, Align)], rows: &[Vec<String>], pen: &Pen) -> Vec<String> {
    table_in(columns, rows, pen, Room::Unbounded, &[])
}

/// A table fitted to the room: where it's too wide, its columns go in
/// the order `gives_way` names them by place, least useful first, and
/// what's still too wide has its last column's words cut at a whole word.
/// The first column names each record, so it never goes. Into a pipe
/// nothing goes.
pub fn table_in(
    columns: &[(&str, Align)],
    rows: &[Vec<String>],
    pen: &Pen,
    room: Room,
    gives_way: &[usize],
) -> Vec<String> {
    let Some(cells) = room.cells() else {
        return set_table(columns, rows, pen);
    };
    let mut kept: Vec<usize> = (0..columns.len()).collect();
    let across = |kept: &[usize]| {
        kept.iter()
            .map(|at| {
                rows.iter()
                    .filter_map(|row| row.get(*at))
                    .map(|cell| width(cell))
                    .chain([width(columns[*at].0)])
                    .max()
                    .unwrap_or(0)
            })
            .sum::<usize>()
            + GAP * kept.len().saturating_sub(1)
    };
    for going in gives_way.iter().filter(|at| **at > 0) {
        if across(&kept) <= cells {
            break;
        }
        kept.retain(|at| at != going);
    }
    let columns_kept: Vec<(&str, Align)> = kept.iter().map(|at| columns[*at]).collect();
    let mut rows_kept: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            kept.iter()
                .map(|at| row.get(*at).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    let over = across(&kept).saturating_sub(cells);
    if over > 0 && kept.len() > 1 {
        let last = kept.len() - 1;
        let before = across(&kept[..last]) + GAP;
        for row in &mut rows_kept {
            row[last] = cut(&row[last], cells.saturating_sub(before).max(1));
        }
    }
    set_table(&columns_kept, &rows_kept, pen)
}

fn set_table(columns: &[(&str, Align)], rows: &[Vec<String>], pen: &Pen) -> Vec<String> {
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(at, (label, _))| {
            rows.iter()
                .filter_map(|row| row.get(at))
                .map(|cell| width(cell))
                .chain([width(label)])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let set = |cells: Vec<(String, String)>| {
        let last = cells.len().saturating_sub(1);
        let mut line = String::new();
        for (at, (plain, inked)) in cells.into_iter().enumerate() {
            let (_, align) = columns[at];
            if at > 0 {
                line.push_str(&" ".repeat(GAP));
            }
            // The last column on the left isn't padded, so no line ends in
            // spaces.
            if at == last && align == Align::Left {
                line.push_str(&inked);
            } else {
                line.push_str(&padded(&plain, inked, widths[at], align == Align::Right));
            }
        }
        line
    };
    let head = columns
        .iter()
        .map(|(label, _)| (label.to_uppercase(), pen.label(label)))
        .collect();
    let mut lines = vec![set(head)];
    for row in rows {
        let cells = columns
            .iter()
            .enumerate()
            .map(|(at, _)| {
                let cell = row.get(at).cloned().unwrap_or_default();
                let inked = pen.role("text", &cell);
                (cell, inked)
            })
            .collect();
        lines.push(set(cells));
    }
    lines
}

/// Blocks as one answer: one empty line between each, ending with a
/// newline where there's anything at all.
pub fn joined(blocks: Vec<Vec<String>>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn facts_set_their_values_in_one_column() {
        let facts_ = [("Size", Some("1 TB")), ("Temperature", None)];
        assert_eq!(
            facts(&facts_, "\u{2026}", &Pen::plain(), Room::Cells(80)),
            ["Size         1 TB", "Temperature  \u{2026}"]
        );
        assert_eq!(
            facts(&facts_, "\u{2026}", &Pen::plain(), Room::Cells(20)),
            ["Size", "  1 TB", "Temperature", "  \u{2026}"]
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
}
