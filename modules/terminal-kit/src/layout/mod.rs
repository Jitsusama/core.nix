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

pub(crate) use words::unseen;
pub use words::{cut, drawn, hung, one_line, paragraph, shortened, wrapped};

/// Whether `words` hold anything the kit keeps from the terminal, so a
/// layout copies only the rare cell that has to be held to one line.
fn holds_unseen(words: &str) -> bool {
    words.chars().any(unseen)
}

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

/// The first of a text's lines wider than the room, if any; in a pipe, none.
/// Every view's tests ask this of it at each width, so the rule lives here
/// once rather than in each program's tests.
pub fn overrun(text: &str, room: Room) -> Option<&str> {
    let cells = room.cells()?;
    text.lines().find(|line| width(line) > cells)
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

    /// Whether its name, words or side holds something the pen never
    /// draws, so a list copies only the rows that need holding.
    fn holds_unseen(&self) -> bool {
        [&self.name, &self.words, &self.side]
            .into_iter()
            .any(|words| holds_unseen(words))
    }

    /// The same, its name, words and side each held to one line, so a
    /// list measures what it draws.
    fn on_one_line(&self) -> Self {
        Self {
            name: one_line(&self.name).into_owned(),
            words: one_line(&self.words).into_owned(),
            side: one_line(&self.side).into_owned(),
            ..self.clone()
        }
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
/// at a whole word. Each row is held to one line, as `one_line` says.
pub fn list(rows: &[Row], pen: &Pen, room: Room) -> Vec<String> {
    let held: Vec<Row>;
    let rows = if rows.iter().any(Row::holds_unseen) {
        held = rows.iter().map(Row::on_one_line).collect();
        held.as_slice()
    } else {
        rows
    };
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
/// beside it, a value it can't know said as `unknown` in muted. The column
/// is set by the longest label whose fact fits beside it, and each fact is
/// judged on its own: one that can't fit beside the column, whole on its
/// line, or wrapping in no more than half the room without cutting a word
/// of it, as an identifier, that would fit whole under, goes under its
/// label, two cells in. A long value wraps under itself, and is held to
/// one line first, as `one_line` says.
pub fn facts(facts: &[(&str, Option<&str>)], unknown: &str, pen: &Pen, room: Room) -> Vec<String> {
    fn said<'a>(value: Option<&'a str>, unknown: &'a str) -> (&'a str, &'static str) {
        value.map_or((unknown, "muted"), |value| (value, "text"))
    }
    let held: Vec<(&str, Option<String>)>;
    let flat: Vec<(&str, Option<&str>)>;
    let facts = if facts
        .iter()
        .any(|(_, value)| value.is_some_and(holds_unseen))
    {
        held = facts
            .iter()
            .map(|(label, value)| (*label, value.map(|value| one_line(value).into_owned())))
            .collect();
        flat = held
            .iter()
            .map(|(label, value)| (*label, value.as_deref()))
            .collect();
        flat.as_slice()
    } else {
        facts
    };
    let fits = |column: usize, value: &str| {
        room.cells().is_none_or(|cells| {
            let longest = value.split_whitespace().map(width).max().unwrap_or(0);
            let whole = column + longest <= cells || longest + GAP > cells;
            column + width(value) <= cells || (column <= cells / 2 && whole)
        })
    };
    let column = facts
        .iter()
        .filter(|(label, value)| fits(width(label) + GAP, said(*value, unknown).0))
        .map(|(label, _)| width(label) + GAP)
        .max()
        .unwrap_or(GAP);
    let mut lines = Vec::new();
    for (label, value) in facts {
        let (value, role) = said(*value, unknown);
        let beside = width(label) + GAP <= column && fits(column, value);
        let column = if beside { column } else { GAP };
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

/// The fewest cells a last column of words is counted at when the
/// others are weighed for the room: enough for a path shortened in its
/// middle to still name its folder and itself. Counted at its full width,
/// a long path would send every other column away before it was
/// shortened at all.
pub const SHORTENED: usize = 24;

/// A table fitted to the room: where it's too wide, its columns go in
/// the order `gives_way` names them by place, least useful first, and
/// what's still too wide has its last column's words cut at a whole word.
/// A last column of words is weighed at `SHORTENED` cells at most, since
/// it can be cut and the columns before it can't. The first column names
/// each record, so it never goes. Into a pipe nothing goes. Each cell is
/// held to one line first, as `one_line` says.
pub fn table_in(
    columns: &[(&str, Align)],
    rows: &[Vec<String>],
    pen: &Pen,
    room: Room,
    gives_way: &[usize],
) -> Vec<String> {
    let held: Vec<Vec<String>>;
    let rows = if rows.iter().flatten().any(|cell| holds_unseen(cell)) {
        held = rows
            .iter()
            .map(|row| row.iter().map(|cell| one_line(cell).into_owned()).collect())
            .collect();
        held.as_slice()
    } else {
        rows
    };
    let Some(cells) = room.cells() else {
        return set_table(columns, rows, pen);
    };
    let mut kept: Vec<usize> = (0..columns.len()).collect();
    let widest = |at: usize| {
        rows.iter()
            .filter_map(|row| row.get(at))
            .map(|cell| width(cell))
            .chain([width(columns[at].0)])
            .max()
            .unwrap_or(0)
    };
    let across = |kept: &[usize]| {
        kept.iter().map(|at| widest(*at)).sum::<usize>() + GAP * kept.len().saturating_sub(1)
    };
    let needed = |kept: &[usize]| match kept.split_last() {
        Some((last, before)) if !before.is_empty() && columns[*last].1 == Align::Left => {
            across(before) + GAP + widest(*last).min(SHORTENED)
        }
        _ => across(kept),
    };
    for going in gives_way.iter().filter(|at| **at > 0) {
        if needed(&kept) <= cells {
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
        // A line ends at its last cell that says something, and that cell
        // isn't padded when its words sit on the left, so no line ends in
        // spaces, even where a record leaves its last columns empty. A row
        // that says nothing keeps its first column's room, so it never
        // reads as the empty line that parts two blocks.
        let said = cells
            .iter()
            .rposition(|(plain, _)| !plain.is_empty())
            .map_or(1, |at| at + 1);
        let last = said.saturating_sub(1);
        let mut line = String::new();
        for (at, (plain, inked)) in cells.into_iter().take(said).enumerate() {
            let (_, align) = columns[at];
            if at > 0 {
                line.push_str(&" ".repeat(GAP));
            }
            if at == last && align == Align::Left && !plain.is_empty() {
                line.push_str(&inked);
            } else {
                line.push_str(&padded(&plain, inked, widths[at], align == Align::Right));
            }
        }
        line
    };
    let head = columns
        .iter()
        .map(|(label, _)| (crate::ink::capitals(label), pen.label(label)))
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
mod tests;
