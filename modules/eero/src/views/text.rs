//! Text as the views set it: runs of words each in a role, lines of runs,
//! and tables whose columns line up, so a view says what each run means
//! and the ink decides how a terminal shows it.

use std::fmt::Write as _;

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

/// A view's text: its lines, top to bottom.
#[derive(Debug, Default)]
pub struct Page {
    pub lines: Vec<Line>,
}

impl Page {
    pub fn line(&mut self, line: Line) {
        self.lines.push(line);
    }

    pub fn blank(&mut self) {
        self.lines.push(Vec::new());
    }

    pub fn extend(&mut self, lines: Vec<Line>) {
        self.lines.extend(lines);
    }

    /// Labels and their values, the labels in one muted column and every
    /// value starting at one edge.
    pub fn facts(&mut self, indent: usize, facts: Vec<(&str, Line)>) {
        let width = facts
            .iter()
            .map(|(label, _)| width(label))
            .max()
            .unwrap_or(0);
        for (label, value) in facts {
            let mut line = vec![plain(" ".repeat(indent)), muted(label)];
            if !value.is_empty() {
                line.push(plain(" ".repeat(width - self::width(label) + 2)));
                line.extend(value);
            }
            self.line(line);
        }
    }
}

/// How many columns a run of text takes. Every character the views set,
/// braille and the marks among them, takes one.
pub fn width(text: &str) -> usize {
    text.chars().count()
}

fn line_width(runs: &[Run]) -> usize {
    runs.iter().map(|run| width(&run.text)).sum()
}

/// How a column lines up its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    /// Figures by their decimal point, or where it would be, so whole
    /// numbers and tenths share a column and each unit follows its figure.
    Point,
}

/// Columns of cells, each column as wide as its widest cell and two
/// spaces from the next, the header muted.
#[derive(Debug)]
pub struct Table {
    aligns: Vec<Align>,
    rows: Vec<Row>,
}

#[derive(Debug)]
enum Row {
    Header(Vec<Line>),
    Cells(Vec<Line>),
    /// A line of its own across the table, such as a group's heading, which
    /// takes no part in the columns' widths.
    Across(Line),
}

impl Table {
    pub fn new(aligns: &[Align]) -> Self {
        Self {
            aligns: aligns.to_vec(),
            rows: Vec::new(),
        }
    }

    /// The header, each heading muted, set over its column the way the
    /// column's figures are.
    pub fn header(&mut self, headings: &[&str]) {
        self.rows.push(Row::Header(
            headings
                .iter()
                .map(|heading| vec![muted(*heading)])
                .collect(),
        ));
    }

    pub fn row(&mut self, cells: Vec<Line>) {
        self.rows.push(Row::Cells(cells));
    }

    pub fn across(&mut self, line: Line) {
        self.rows.push(Row::Across(line));
    }

    /// The part of a cell before its point, and the part from it on.
    fn split(cell: &[Run]) -> (usize, usize) {
        let all = line_width(cell);
        let Some(first) = cell.first() else {
            return (0, 0);
        };
        let before = first.text.find('.').map_or_else(
            || width(first.text.trim_end()),
            |at| width(&first.text[..at]),
        );
        (before, all - before)
    }

    pub fn lines(self, indent: usize) -> Vec<Line> {
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
        let mut lines = Vec::new();
        for row in self.rows {
            let (cells, header) = match row {
                Row::Across(line) => {
                    let mut out = vec![plain(" ".repeat(indent))];
                    out.extend(line);
                    lines.push(out);
                    continue;
                }
                Row::Header(cells) => (cells, true),
                Row::Cells(cells) => (cells, false),
            };
            let mut out = vec![plain(" ".repeat(indent))];
            let last = cells.len().saturating_sub(1);
            for (at, cell) in cells.into_iter().enumerate() {
                let size = line_width(&cell);
                let (left, right) = match self.aligns[at] {
                    Align::Left => (0, wide[at] - size),
                    Align::Right => (wide[at] - size, 0),
                    // A heading over figures sits flush right, over their units.
                    Align::Point if header => (wide[at] - size, 0),
                    Align::Point => {
                        let (b, _) = Self::split(&cell);
                        let lead = wide[at] - before[at] - after[at] + before[at] - b;
                        (lead, wide[at] - size - lead)
                    }
                };
                if at > 0 {
                    out.push(plain("  "));
                }
                out.push(plain(" ".repeat(left)));
                out.extend(cell);
                if at < last {
                    out.push(plain(" ".repeat(right)));
                }
            }
            lines.push(out);
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

    pub fn page(&self, page: &Page) -> String {
        let mut out = String::new();
        for line in &page.lines {
            let mut set = String::new();
            for run in line {
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

    #[test]
    fn figures_line_up_by_their_point_and_units_follow_them() {
        let mut table = Table::new(&[Align::Left, Align::Point]);
        table.header(&["device", "speed"]);
        table.row(vec![vec![plain("laptop")], figure("11.5", "Mbit/s")]);
        table.row(vec![vec![plain("printer")], figure("98", "bit/s")]);
        table.row(vec![vec![plain("tablet")], figure("145.4", "kbit/s")]);
        let mut page = Page::default();
        page.extend(table.lines(0));
        assert_eq!(
            Ink::plain().page(&page),
            "device          speed\n\
             laptop    11.5 Mbit/s\n\
             printer   98 bit/s\n\
             tablet   145.4 kbit/s\n"
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
        let mut page = Page::default();
        page.line(vec![
            strong("Hall"),
            muted("  "),
            plain("online"),
            muted(" dBm"),
        ]);
        assert_eq!(
            Ink {
                palette: Some(palette)
            }
            .page(&page),
            "\x1b[1mHall\x1b[0m  online\x1b[37m dBm\x1b[0m\n"
        );
        assert_eq!(Ink::plain().page(&page), "Hall  online dBm\n");
    }

    #[test]
    fn facts_start_their_values_at_one_edge() {
        let mut page = Page::default();
        page.facts(
            2,
            vec![
                ("ISP", vec![plain("Example ISP")]),
                ("address", vec![plain("203.0.113.1")]),
            ],
        );
        assert_eq!(
            Ink::plain().page(&page),
            "  ISP      Example ISP\n  address  203.0.113.1\n"
        );
    }
}
