//! Text as the views set it: runs of words each in a role, lines of runs,
//! facts and tables whose columns line up, so a view says what each run
//! means and the ink decides how a terminal shows it. A page is laid out
//! only as it's drawn, against the room the terminal kit finds: into a
//! pipe every line is whole, and into a terminal each fits its width,
//! as docs/commands.md's Width and What Gives Way sets it (`fit.rs`).
//!
//! Long because a table's layout is one piece: its columns' widths, what
//! gives way to a narrow terminal, and the columns it shares with the
//! tables under it on a page are each read by the others, and its tests
//! sit beside them.

use std::fmt::Write as _;

use terminal_kit::Room;

use super::fit;

/// What a run of text is, which the ink turns into the terminal's colour
/// for it, or nothing off a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// What Joel reads.
    Text,
    /// A heading, a name that leads its lines.
    Strong,
    /// Labels, units, separators and graphs: context, never the figure.
    Muted,
    /// Where a graph would be, drawn under it so its ends show.
    Track,
    /// Something wrong now: offline, unreachable, a failed heartbeat.
    Alert,
    /// Something that will want him: an update waiting, an old reading.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub role: Role,
}

pub fn run(text: impl Into<String>, role: Role) -> Run {
    Run {
        text: text.into(),
        role,
    }
}

pub fn plain(text: impl Into<String>) -> Run {
    run(text, Role::Text)
}

pub fn muted(text: impl Into<String>) -> Run {
    run(text, Role::Muted)
}

pub fn strong(text: impl Into<String>) -> Run {
    run(text, Role::Strong)
}

/// A line is its runs, set one after another.
pub type Line = Vec<Run>;

/// The least room a value or a wrapped column is given beside what leads
/// it; with less, a fact's value goes under its label.
const FEWEST: usize = 24;

/// What a page holds, laid out as it's drawn.
#[derive(Debug)]
enum Block {
    /// A line, which wraps under the cells its mark or inset leads with.
    Line(Line),
    /// Labels and their values, `indent` cells in.
    Facts(usize, Vec<(String, Line)>),
    /// A table, `indent` cells in, and whether it shares its leading
    /// columns with the page's other tables that do.
    Table(usize, Table, bool),
}

/// A view's text: its lines, facts and tables, top to bottom.
#[derive(Debug, Default)]
pub struct Page {
    blocks: Vec<Block>,
}

impl Page {
    pub fn line(&mut self, line: Line) {
        self.blocks.push(Block::Line(line));
    }

    pub fn blank(&mut self) {
        self.line(Vec::new());
    }

    /// Labels and their values, the labels in one muted column and every
    /// value starting at one edge.
    pub fn facts(&mut self, indent: usize, facts: Vec<(&str, Line)>) {
        let facts = facts
            .into_iter()
            .map(|(label, value)| (label.to_owned(), value))
            .collect();
        self.blocks.push(Block::Facts(indent, facts));
    }

    /// A table, set `indent` cells in.
    pub fn table(&mut self, indent: usize, table: Table) {
        self.blocks.push(Block::Table(indent, table, false));
    }

    /// A table, set `indent` cells in, whose leading columns sit where the
    /// same columns of the page's other shared tables do, so a column
    /// reads down through each: the columns under the same headings, in
    /// the same order, from the first.
    pub fn shared_table(&mut self, indent: usize, table: Table) {
        self.blocks.push(Block::Table(indent, table, true));
    }

    /// The page's lines as `room` lays them out.
    pub fn lines(self, room: Room) -> Vec<Line> {
        let shared = Shared::of(&self.blocks, room);
        let mut lines = Vec::new();
        for block in self.blocks {
            match block {
                Block::Line(line) => {
                    let lead = leads(&line);
                    lines.extend(hung(line, lead, room));
                }
                Block::Facts(indent, facts) => lines.extend(set_facts(indent, facts, room)),
                Block::Table(indent, table, shares) => {
                    lines.extend(table.lines(indent, room, shares.then_some(&shared)));
                }
            }
        }
        lines
    }
}

/// What a page's shared tables set alike in their leading columns: each
/// as wide as the widest of them, figures ending at one place, and gone
/// from every table, or shortened in every one, where any gives it way.
#[derive(Debug, Default)]
struct Shared {
    /// How many leading columns the tables share.
    columns: usize,
    wide: Vec<usize>,
    before: Vec<usize>,
    after: Vec<usize>,
    gone: Vec<bool>,
    set: Vec<usize>,
}

impl Shared {
    fn of(blocks: &[Block], room: Room) -> Self {
        let tables: Vec<(usize, &Table)> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Table(indent, table, true) => Some((*indent, table)),
                _ => None,
            })
            .collect();
        let Some(((_, first), rest)) = tables.split_first() else {
            return Self::default();
        };
        let columns = rest
            .iter()
            .fold(first.headings.len(), |columns, (_, table)| {
                let alike = first
                    .headings
                    .iter()
                    .zip(&first.aligns)
                    .zip(table.headings.iter().zip(&table.aligns))
                    .take_while(|(ours, theirs)| ours == theirs)
                    .count();
                columns.min(alike)
            });
        let mut shared = Self {
            columns,
            wide: vec![0; columns],
            before: vec![0; columns],
            after: vec![0; columns],
            gone: vec![false; columns],
            set: vec![usize::MAX; columns],
        };
        for (_, table) in &tables {
            let (wide, before, after) = table.widths();
            for at in 0..columns {
                shared.wide[at] = shared.wide[at].max(wide[at]);
                shared.before[at] = shared.before[at].max(before[at]);
                shared.after[at] = shared.after[at].max(after[at]);
            }
        }
        // Each table is placed in the room at the shared widths; a column
        // any of them gives way goes from all, and one any shortens is
        // shortened alike, which only ever leaves each more room.
        let (mut gone, mut set) = (vec![false; columns], vec![usize::MAX; columns]);
        for (indent, table) in &tables {
            let placed = table.placed(*indent, room, Some(&shared));
            for at in 0..columns {
                if placed.kept.contains(&at) {
                    set[at] = set[at].min(placed.set[at]);
                } else {
                    gone[at] = true;
                }
            }
        }
        shared.gone = gone;
        shared.set = set;
        shared
    }
}

/// The cells a line leads with, which its wrapped lines sit under: its
/// inset, and a mark and its space after it.
fn leads(line: &[Run]) -> usize {
    let text: String = line.iter().map(|run| run.text.as_str()).collect();
    let inset = text.chars().take_while(|letter| *letter == ' ').count();
    let mut rest = text.chars().skip(inset);
    match (rest.next(), rest.next()) {
        (Some(mark), Some(' ')) if !mark.is_alphanumeric() => inset + 2,
        _ => inset,
    }
}

/// A line as the room sets it: whole where it fits or into a pipe, and
/// otherwise wrapped, every line after the first under its first `lead`
/// cells.
fn hung(line: Line, lead: usize, room: Room) -> Vec<Line> {
    let Some(cells) = room.cells() else {
        return vec![line];
    };
    if line_width(&line) <= cells || lead >= cells {
        return vec![line];
    }
    let (head, rest) = split(&line, lead);
    let mut lines = fit::wrapped(&rest, cells - lead).into_iter();
    let mut first = head;
    first.extend(lines.next().unwrap_or_default());
    let mut out = vec![first];
    out.extend(lines.map(|line| {
        let mut set = vec![plain(" ".repeat(lead))];
        set.extend(line);
        set
    }));
    out
}

/// A line parted after its first `cells` cells.
fn split(line: &[Run], cells: usize) -> (Line, Line) {
    let (mut head, mut tail) = (Vec::new(), Vec::new());
    let mut left = cells;
    for each in line {
        let size = width(&each.text);
        if left >= size {
            head.push(each.clone());
            left -= size;
        } else if left == 0 {
            tail.push(each.clone());
        } else {
            let at = each
                .text
                .char_indices()
                .nth(left)
                .map_or(each.text.len(), |(at, _)| at);
            head.push(run(&each.text[..at], each.role));
            tail.push(run(&each.text[at..], each.role));
            left = 0;
        }
    }
    (head, tail)
}

/// Labels and their values: each value beside its label, wrapping under
/// itself, while the labels leave it room; past that each value goes
/// under its label, two cells in.
fn set_facts(indent: usize, facts: Vec<(String, Line)>, room: Room) -> Vec<Line> {
    let widest = facts
        .iter()
        .map(|(label, _)| width(label))
        .max()
        .unwrap_or(0);
    let column = indent + widest + 2;
    let beside = room
        .cells()
        .is_none_or(|cells| column + FEWEST <= cells || column * 2 <= cells);
    let mut lines = Vec::new();
    for (label, value) in facts {
        if beside || value.is_empty() {
            let mut line = vec![plain(" ".repeat(indent)), muted(&label)];
            if !value.is_empty() {
                line.push(plain(" ".repeat(widest - width(&label) + 2)));
                line.extend(value);
            }
            lines.extend(hung(line, column, room));
        } else {
            lines.push(vec![plain(" ".repeat(indent)), muted(&label)]);
            let mut line = vec![plain(" ".repeat(indent + 2))];
            line.extend(value);
            lines.extend(hung(line, indent + 2, room));
        }
    }
    lines
}

/// How many cells a run of text takes, by Unicode's width, as the
/// terminal kit counts the names it shortens and the cells it cuts: a
/// device's name may hold an emoji or a CJK letter two cells wide.
pub fn width(text: &str) -> usize {
    terminal_kit::layout::width(text)
}

fn line_width(runs: &[Run]) -> usize {
    runs.iter().map(|run| width(&run.text)).sum()
}

/// How a column lines up its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    /// Figures, each ending where the column's figures end, whole numbers
    /// and a missing figure's dash as much as tenths, so the units after
    /// them start at one place.
    Point,
}

/// What a table's last column does when a terminal is too narrow for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    /// Its words are cut at a whole word.
    Cut,
    /// Its words wrap under it, given at least this many cells.
    Wraps(usize),
}

/// Columns of cells, each column as wide as its widest cell and two
/// spaces from the next, the header muted. Into a terminal too narrow for
/// it, whole columns give way in the order the view names them, least
/// needed first; then the column of names is shortened in its middle, and
/// the last column's words are cut or wrapped. The first column names
/// each row, so it never goes.
#[derive(Debug)]
pub struct Table {
    aligns: Vec<Align>,
    headings: Vec<String>,
    rows: Vec<Row>,
    gives_way: Vec<Vec<usize>>,
    shortens: Option<(usize, usize)>,
    last: Last,
}

#[derive(Debug)]
enum Row {
    Header(Vec<Line>),
    Cells(Vec<Line>),
    /// A line of its own across the table, such as a group's heading, which
    /// takes no part in the columns' widths.
    Across(Line),
}

/// Two cells between columns.
const GAP: usize = 2;

/// The fewest cells a last column is cut to; in fewer, a cell says
/// nothing but its ellipsis.
const SHORTEST: usize = 6;

/// Where a table's columns are set: those kept, by their places, each
/// one's width, and the widest unit after its figures.
struct Placed {
    kept: Vec<usize>,
    set: Vec<usize>,
    after: Vec<usize>,
}

impl Table {
    pub fn new(aligns: &[Align]) -> Self {
        Self {
            aligns: aligns.to_vec(),
            headings: Vec::new(),
            rows: Vec::new(),
            gives_way: Vec::new(),
            shortens: None,
            last: Last::Cut,
        }
    }

    /// The header, each heading muted, set over its column the way the
    /// column's figures are.
    pub fn header(&mut self, headings: &[&str]) {
        self.headings = headings
            .iter()
            .map(|heading| (*heading).to_owned())
            .collect();
        self.rows.push(Row::Header(
            headings
                .iter()
                .map(|heading| vec![muted(*heading)])
                .collect(),
        ));
    }

    /// Every column under a heading, by its place.
    pub fn under(&self, heading: &str) -> Vec<usize> {
        self.headings
            .iter()
            .enumerate()
            .filter(|(_, said)| *said == heading)
            .map(|(at, _)| at)
            .collect()
    }

    /// The columns that give way to a narrow terminal, least needed first,
    /// each group by the places of its columns, which go together.
    pub fn gives_way(&mut self, groups: Vec<Vec<usize>>) {
        self.gives_way = groups;
    }

    /// The columns under each heading give way, least needed first.
    pub fn gives_way_by(&mut self, headings: &[&str]) {
        self.gives_way = headings.iter().map(|heading| self.under(heading)).collect();
    }

    /// The column of names that's shortened in its middle before the last
    /// column gives way, to no fewer than `least` cells.
    pub fn shortens(&mut self, at: usize, least: usize) {
        self.shortens = Some((at, least));
    }

    /// The last column's words wrap under it rather than being cut, given
    /// at least `least` cells, as prose and runs of facts do.
    pub fn wraps(&mut self, least: usize) {
        self.last = Last::Wraps(least);
    }

    pub fn row(&mut self, cells: Vec<Line>) {
        self.rows.push(Row::Cells(cells));
    }

    pub fn across(&mut self, line: Line) {
        self.rows.push(Row::Across(line));
    }

    /// The part of a cell its figure takes, its first run, and the part
    /// after it, its unit.
    fn split(cell: &[Run]) -> (usize, usize) {
        let all = line_width(cell);
        let Some(first) = cell.first() else {
            return (0, 0);
        };
        let figure = width(first.text.trim_end()).min(all);
        (figure, all - figure)
    }

    /// Each column's width, and its figures' widths and their units'.
    fn widths(&self) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let columns = self.aligns.len();
        let mut wide = vec![0; columns];
        let mut before = vec![0; columns];
        let mut after = vec![0; columns];
        for row in &self.rows {
            let (Row::Header(cells) | Row::Cells(cells)) = row else {
                continue;
            };
            for (at, cell) in cells.iter().enumerate() {
                wide[at] = wide[at].max(line_width(cell));
                if let Row::Cells(_) = row {
                    let (b, a) = Self::split(cell);
                    before[at] = before[at].max(b);
                    after[at] = after[at].max(a);
                }
            }
        }
        for at in 0..columns {
            if self.aligns[at] == Align::Point {
                wide[at] = wide[at].max(before[at] + after[at]);
            }
        }
        (wide, before, after)
    }

    /// The columns kept in `cells`, and the width each is set at: whole
    /// columns go first, then the names shorten, then the last column. Only
    /// the columns `from` are set, the rest having gone already.
    fn kept(&self, from: Vec<usize>, wide: &[usize], cells: usize) -> (Vec<usize>, Vec<usize>) {
        let mut kept = from;
        let across = |kept: &[usize], at: &dyn Fn(usize) -> usize| {
            kept.iter().map(|column| at(*column)).sum::<usize>()
                + GAP * kept.len().saturating_sub(1)
        };
        // What the kept columns need at the least: a last column of words
        // at its fewest. The names are counted whole, since a name that
        // reads whole says more than a column kept beside it, so they're
        // shortened only once every column that gives way has gone.
        let least = |kept: &[usize]| {
            let last = kept.last().copied();
            across(kept, &|column| {
                let mut size = wide[column];
                if Some(column) == last && column > 0 && self.aligns[column] == Align::Left {
                    size = size.min(match self.last {
                        Last::Cut => terminal_kit::layout::SHORTENED,
                        Last::Wraps(fewest) => fewest,
                    });
                }
                size
            })
        };
        for group in &self.gives_way {
            if least(&kept) <= cells {
                break;
            }
            kept.retain(|column| *column == 0 || !group.contains(column));
        }
        let mut set: Vec<usize> = wide.to_vec();
        let mut over = across(&kept, &|column| set[column]).saturating_sub(cells);
        if let Some((names, fewest)) = self.shortens
            && over > 0
            && kept.contains(&names)
        {
            let less = over.min(set[names].saturating_sub(fewest));
            set[names] -= less;
            over -= less;
        }
        // The last column of words is cut to what's left, but one left
        // fewer cells than say anything goes rather than show its ellipsis,
        // as does a last column of figures with no room, and the one before
        // it is weighed in its place, so no row runs past the terminal.
        while let Some(last) = kept.last().copied()
            && over > 0
            && last > 0
        {
            if self.aligns[last] == Align::Left && set[last].saturating_sub(over) >= SHORTEST {
                set[last] -= over;
                break;
            }
            kept.pop();
            over = across(&kept, &|column| set[column]).saturating_sub(cells);
        }
        (kept, set)
    }

    /// The columns kept in `room` and each one's width, with its units'
    /// widths, set alike with the tables it shares columns with.
    fn placed(&self, indent: usize, room: Room, shared: Option<&Shared>) -> Placed {
        let (mut wide, mut before, mut after) = self.widths();
        if let Some(shared) = shared {
            for at in 0..shared.columns {
                before[at] = before[at].max(shared.before[at]);
                after[at] = after[at].max(shared.after[at]);
                wide[at] = wide[at].max(shared.wide[at]);
                if self.aligns[at] == Align::Point {
                    wide[at] = wide[at].max(before[at] + after[at]);
                }
            }
        }
        // A shared column another table gave way goes before this one is
        // set, so what it leaves is room for the rest.
        let from: Vec<usize> = (0..self.aligns.len())
            .filter(|at| shared.is_none_or(|shared| *at >= shared.columns || !shared.gone[*at]))
            .collect();
        let (kept, mut set) = match room.cells() {
            Some(cells) => self.kept(from, &wide, cells.saturating_sub(indent)),
            None => (from, wide),
        };
        if let Some(shared) = shared {
            for (width, alike) in set.iter_mut().zip(&shared.set) {
                *width = (*width).min(*alike);
            }
        }
        Placed { kept, set, after }
    }

    fn lines(self, indent: usize, room: Room, shared: Option<&Shared>) -> Vec<Line> {
        let Placed { kept, set, after } = self.placed(indent, room, shared);
        let names = self.shortens.map(|(at, _)| at);
        let last = kept.last().copied();
        let pad = plain(" ".repeat(indent));
        let mut lines = Vec::new();
        for row in self.rows {
            let (cells, header) = match row {
                Row::Across(line) => {
                    let mut out = vec![pad.clone()];
                    out.extend(line);
                    lines.extend(hung(out, indent, room));
                    continue;
                }
                Row::Header(cells) => (cells, true),
                Row::Cells(cells) => (cells, false),
            };
            let mut out = vec![pad.clone()];
            let mut beneath: Vec<Line> = Vec::new();
            let mut start = indent;
            let shown: Vec<(usize, Line)> = cells
                .into_iter()
                .enumerate()
                .filter(|(at, _)| kept.contains(at))
                .collect();
            let final_at = shown.len().saturating_sub(1);
            for (place, (at, mut cell)) in shown.into_iter().enumerate() {
                if line_width(&cell) > set[at] {
                    if Some(at) == names {
                        cell = fit::shorter(&cell, set[at]);
                    } else if Some(at) == last
                        && !header
                        && let Last::Wraps(_) = self.last
                    {
                        let mut wrapped = fit::wrapped(&cell, set[at]).into_iter();
                        cell = wrapped.next().unwrap_or_default();
                        beneath.extend(wrapped);
                    } else {
                        cell = fit::cut_to(&cell, set[at]);
                    }
                }
                let size = line_width(&cell);
                let room_for = set[at].max(size);
                let (left, right) = match self.aligns[at] {
                    Align::Left => (0, room_for - size),
                    Align::Right => (room_for - size, 0),
                    // A heading over figures sits flush right, over their units.
                    Align::Point if header => (room_for - size, 0),
                    // Its figure ends where the widest figure does, so its
                    // unit starts where every other unit starts.
                    Align::Point => {
                        let (figure, _) = Self::split(&cell);
                        let lead = room_for
                            .saturating_sub(after[at])
                            .saturating_sub(figure)
                            .min(room_for - size);
                        (lead, room_for - size - lead)
                    }
                };
                if place > 0 {
                    out.push(plain("  "));
                    start += GAP;
                }
                if place == final_at {
                    beneath = beneath
                        .into_iter()
                        .map(|line| {
                            let mut set = vec![plain(" ".repeat(start))];
                            set.extend(line);
                            set
                        })
                        .collect();
                }
                out.push(plain(" ".repeat(left)));
                out.extend(cell);
                if place < final_at {
                    out.push(plain(" ".repeat(right)));
                }
                start += room_for;
            }
            lines.push(out);
            lines.extend(beneath);
        }
        lines
    }
}

/// The SGR numbers a terminal is given for each role, or none for a role
/// drawn in its own foreground.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub text: Option<String>,
    pub strong: Option<String>,
    pub muted: Option<String>,
    pub track: Option<String>,
    pub alert: Option<String>,
    pub warning: Option<String>,
}

impl Palette {
    fn of(&self, role: Role) -> Option<&str> {
        match role {
            Role::Text => self.text.as_deref(),
            Role::Strong => self.strong.as_deref(),
            Role::Muted => self.muted.as_deref(),
            Role::Track => self.track.as_deref(),
            Role::Alert => self.alert.as_deref(),
            Role::Warning => self.warning.as_deref(),
        }
    }
}

/// How a page reaches its reader: in the terminal's colours, or as plain
/// text when it isn't a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ink {
    pub palette: Option<Palette>,
}

impl Ink {
    pub fn plain() -> Self {
        Self { palette: None }
    }

    pub fn coloured(&self) -> bool {
        self.palette.is_some()
    }

    /// A page laid out in `room` and inked.
    pub fn page(&self, page: Page, room: Room) -> String {
        let mut out = String::new();
        for line in page.lines(room) {
            let mut set = String::new();
            for run in &line {
                match self
                    .palette
                    .as_ref()
                    .and_then(|palette| palette.of(run.role))
                {
                    Some(sgr) if !run.text.trim().is_empty() => {
                        // Writing to a String can't fail.
                        _ = write!(set, "\x1b[{sgr}m{}\x1b[0m", run.text);
                    }
                    _ => set.push_str(&run.text),
                }
            }
            out.push_str(set.trim_end());
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figure(value: &str, unit: &str) -> Line {
        vec![plain(value), muted(format!(" {unit}"))]
    }

    fn speeds() -> Table {
        let mut table = Table::new(&[Align::Left, Align::Left, Align::Point]);
        table.header(&["device", "kind", "speed"]);
        table.row(vec![
            vec![plain("laptop computer 1")],
            vec![plain("laptop")],
            figure("11.5", "Mbit/s"),
        ]);
        table.row(vec![
            vec![plain("printer")],
            vec![plain("printer")],
            figure("98", "bit/s"),
        ]);
        table
    }

    fn set(page: Page, room: Room) -> String {
        Ink::plain().page(page, room)
    }

    #[test]
    fn figures_end_together_so_their_units_line_up() {
        let mut table = Table::new(&[Align::Left, Align::Point]);
        table.header(&["device", "speed"]);
        table.row(vec![vec![plain("laptop")], figure("1.20", "Gb/s")]);
        table.row(vec![vec![plain("printer")], figure("433", "Mb/s")]);
        table.row(vec![vec![plain("tablet")], figure("72.2", "Mb/s")]);
        table.row(vec![vec![plain("phone")], vec![muted("–")]]);
        let mut page = Page::default();
        page.table(0, table);
        assert_eq!(
            set(page, Room::Unbounded),
            "device       speed\n\
             laptop   1.20 Gb/s\n\
             printer   433 Mb/s\n\
             tablet   72.2 Mb/s\n\
             phone       –\n",
            "a whole number and a missing figure's dash end where tenths do"
        );
    }

    #[test]
    fn shared_tables_set_their_leading_columns_alike() {
        let mut on = Table::new(&[Align::Left, Align::Left, Align::Point]);
        on.header(&["device", "kind", "speed"]);
        on.gives_way_by(&["kind"]);
        on.row(vec![
            vec![plain("laptop computer 1")],
            vec![plain("laptop")],
            figure("11.5", "Mb/s"),
        ]);
        let mut off = Table::new(&[Align::Left, Align::Left, Align::Left]);
        off.header(&["device", "kind", "last on"]);
        off.row(vec![
            vec![plain("printer")],
            vec![plain("office printer")],
            vec![plain("2 d ago")],
        ]);
        let page = |on, off| {
            let mut page = Page::default();
            page.shared_table(0, on);
            page.blank();
            page.shared_table(0, off);
            page
        };
        assert_eq!(
            set(page(on, off), Room::Unbounded),
            "device             kind                speed\n\
             laptop computer 1  laptop          11.5 Mb/s\n\
             \n\
             device             kind            last on\n\
             printer            office printer  2 d ago\n",
            "each table's device and kind start where the other's do"
        );
    }

    #[test]
    fn a_column_one_shared_table_gives_way_goes_from_both() {
        let mut on = Table::new(&[Align::Left, Align::Left, Align::Point]);
        on.header(&["device", "kind", "speed"]);
        on.gives_way_by(&["kind"]);
        on.row(vec![
            vec![plain("laptop computer 1")],
            vec![plain("laptop")],
            figure("11.5", "Mb/s"),
        ]);
        let mut off = Table::new(&[Align::Left, Align::Left, Align::Left]);
        off.header(&["device", "kind", "last on"]);
        off.row(vec![
            vec![plain("printer")],
            vec![plain("printer")],
            vec![plain("2 d ago")],
        ]);
        let mut page = Page::default();
        page.shared_table(0, on);
        page.shared_table(0, off);
        assert_eq!(
            set(page, Room::Cells(32)),
            "device                 speed\n\
             laptop computer 1  11.5 Mb/s\n\
             device             last on\n\
             printer            2 d ago\n",
            "the kind the first table gave way to 32 cells goes from the second too"
        );
    }

    #[test]
    fn a_narrow_table_gives_way_by_whole_columns_then_shortens_its_names() {
        let mut table = speeds();
        table.gives_way_by(&["kind"]);
        table.shortens(0, 8);
        let mut page = Page::default();
        page.table(0, table);
        assert_eq!(
            set(page, Room::Cells(24)),
            "device             speed\n\
             lapto…ter 1  11.5 Mbit/s\n\
             printer        98 bit/s\n"
        );
        let mut whole = Page::default();
        let mut table = speeds();
        table.gives_way_by(&["kind"]);
        whole.table(0, table);
        assert_eq!(
            set(whole, Room::Unbounded),
            "device             kind           speed\n\
             laptop computer 1  laptop   11.5 Mbit/s\n\
             printer            printer    98 bit/s\n"
        );
    }

    #[test]
    fn a_name_two_cells_wide_a_letter_lines_up_by_its_cells() {
        let mut table = Table::new(&[Align::Left, Align::Right]);
        table.header(&["device", "speed"]);
        table.row(vec![vec![plain("\u{1f4f1} phone")], vec![plain("2 Mb/s")]]);
        table.row(vec![vec![plain("printer")], vec![plain("98 b/s")]]);
        let mut page = Page::default();
        page.table(0, table);
        assert_eq!(
            set(page, Room::Unbounded),
            "device     speed\n\
             \u{1f4f1} phone  2 Mb/s\n\
             printer   98 b/s\n",
            "an emoji takes two cells, so its row pads one fewer space"
        );
    }

    #[test]
    fn a_last_column_that_wraps_keeps_under_itself() {
        let mut table = Table::new(&[Align::Left, Align::Left]);
        table.header(&["at", "what happened"]);
        table.row(vec![
            vec![plain("17:42")],
            vec![plain("tablet 1 connected to Den on 5.2 GHz channel 52.")],
        ]);
        table.wraps(10);
        let mut page = Page::default();
        page.table(0, table);
        assert_eq!(
            set(page, Room::Cells(30)),
            "at     what happened\n\
             17:42  tablet 1 connected to\n\
             \x20      Den on 5.2 GHz\n\
             \x20      channel 52.\n"
        );
    }

    #[test]
    fn a_role_is_coloured_only_on_a_terminal_and_only_where_there_is_ink() {
        let palette = Palette {
            text: None,
            strong: Some("1".into()),
            muted: Some("37".into()),
            track: Some("2".into()),
            alert: Some("31".into()),
            warning: Some("93".into()),
        };
        let line = || vec![strong("Hall"), muted("  "), plain("online"), muted(" dBm")];
        let mut page = Page::default();
        page.line(line());
        assert_eq!(
            Ink {
                palette: Some(palette)
            }
            .page(page, Room::Unbounded),
            "\x1b[1mHall\x1b[0m  online\x1b[37m dBm\x1b[0m\n"
        );
        let mut page = Page::default();
        page.line(line());
        assert_eq!(set(page, Room::Unbounded), "Hall  online dBm\n");
    }

    fn address_facts() -> Page {
        let mut page = Page::default();
        page.facts(
            2,
            vec![
                ("ISP", vec![plain("Example ISP")]),
                (
                    "address",
                    vec![
                        plain("203.0.113.1"),
                        muted(" · "),
                        plain("fe80::7/64"),
                        muted(" · "),
                        plain("fe80::8/64"),
                    ],
                ),
            ],
        );
        page
    }

    #[test]
    fn facts_start_their_values_at_one_edge_and_wrap_under_it() {
        assert_eq!(
            set(address_facts(), Room::Unbounded),
            "  ISP      Example ISP\n  address  203.0.113.1 · fe80::7/64 · fe80::8/64\n"
        );
        assert_eq!(
            set(address_facts(), Room::Cells(40)),
            "  ISP      Example ISP\n\
             \x20 address  203.0.113.1 · fe80::7/64\n\
             \x20          fe80::8/64\n"
        );
    }

    #[test]
    fn a_line_wraps_under_its_mark() {
        let mut page = Page::default();
        page.line(vec![
            run("●", Role::Warning),
            plain(" nothing new for 2 h 5 min; eero-daemon may have stopped"),
        ]);
        assert_eq!(
            set(page, Room::Cells(40)),
            "● nothing new for 2 h 5 min; eero-daemon\n  may have stopped\n"
        );
    }
}
