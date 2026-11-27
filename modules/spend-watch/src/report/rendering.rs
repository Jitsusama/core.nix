//! A report rendered as text, markdown, JSON or CSV: the formats and
//! their writers, the terminal's table and sentences among them.

use super::Report;
use crate::money::percent;
use std::fmt::Write as _;
use terminal_kit::{Pen, Room, layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Markdown,
    Json,
    Csv,
}

impl Format {
    pub fn parse(text: &str) -> Option<Format> {
        match text {
            "text" => Some(Format::Text),
            "markdown" | "md" => Some(Format::Markdown),
            "json" => Some(Format::Json),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

pub fn render(report: &Report, format: Format) -> String {
    match format {
        Format::Json => serde_json::to_string_pretty(report).unwrap_or_default() + "\n",
        Format::Csv => render_csv(report),
        Format::Markdown => render_markdown(report),
        Format::Text => render_text_in(report, &Pen::plain(), Room::Unbounded),
    }
}

// Writing to a String can't fail, so what `write!` returns below is let go.

/// A report's rows as CSV, with a header.
fn render_csv(report: &Report) -> String {
    let mut out = String::from("name,dollars,share,estimated\n");
    for row in &report.rows {
        _ = writeln!(
            out,
            "{},{:.2},{:.4},{}",
            csv_field(&row.name),
            row.dollars,
            row.share,
            row.estimated
        );
    }
    out
}

/// A report as Markdown: its title, what it covers, a table and its notes.
fn render_markdown(report: &Report) -> String {
    let mut out = format!("## {}\n\n", report.title);
    if !report.shown.is_empty() {
        _ = write!(
            out,
            "{} from {} to {}, {}.\n\n",
            report.shown, report.from, report.to, report.source
        );
    }
    // The caps' rows are what changed and when, not spend.
    if !report.rows.is_empty() && report.period == "caps" {
        out.push_str("| When | What changed |\n|---|---|\n");
        for row in &report.rows {
            _ = writeln!(
                out,
                "| {} | {} |",
                row.name.replace('|', "/"),
                row.shown.replace('|', "/")
            );
        }
        out.push('\n');
    } else if !report.rows.is_empty() {
        out.push_str("| | Spend | Share |\n|---|---:|---:|\n");
        for row in &report.rows {
            let share = if row.share > 0.0 {
                percent(row.share)
            } else {
                String::new()
            };
            _ = writeln!(
                out,
                "| {} | {} | {share} |",
                row.name.replace('|', "/"),
                row.shown.replace('|', "/")
            );
        }
        out.push('\n');
    }
    for note in &report.notes {
        _ = writeln!(out, "- {note}");
    }
    out
}

/// The inset of a report's rows under its title.
const INSET: &str = "  ";

/// A report as plain text, set to `room`: its rows in the kit's table
/// where it has columns, and as the kit's facts where each says a
/// sentence. In a terminal its prose wraps, a name too long for its
/// column is cut at a whole word, or shortened in its middle where it's
/// one word, and a sentence hangs under itself; into a pipe nothing
/// gives way.
pub fn render_text_in(report: &Report, pen: &Pen, room: Room) -> String {
    let prose = room.for_prose();
    let mut out = format!("{}\n", report.title);
    if !report.shown.is_empty() {
        let covers = format!(
            "{} from {} to {}, {}.",
            report.shown, report.from, report.to, report.source
        );
        for line in layout::paragraph(&covers, 0, prose) {
            _ = writeln!(out, "{line}");
        }
    }
    out.push('\n');
    let inside = match room.cells() {
        Some(cells) => Room::Cells(cells.saturating_sub(INSET.len())),
        None => Room::Unbounded,
    };
    let lines = if report.columns.is_empty() {
        sentences(report, pen, inside)
    } else {
        table(report, pen, inside)
    };
    for line in lines {
        _ = writeln!(out, "{INSET}{line}");
    }
    if !report.rows.is_empty() && !report.notes.is_empty() {
        out.push('\n');
    }
    for note in &report.notes {
        for line in layout::paragraph(note, 0, prose) {
            _ = writeln!(out, "{line}");
        }
    }
    out
}

/// A report's rows in its columns, each name cut to the room the other
/// columns leave, so a long quest's title never moves its figures.
fn table(report: &Report, pen: &Pen, room: Room) -> Vec<String> {
    // The fewest cells a name is cut to, so it still tells rows apart.
    const FEWEST: usize = 24;
    if report.rows.is_empty() {
        return Vec::new();
    }
    let others: usize = report
        .columns
        .iter()
        .skip(1)
        .enumerate()
        .map(|(at, (label, _))| {
            report
                .rows
                .iter()
                .filter_map(|row| row.cells.get(at))
                .map(|cell| layout::width(cell))
                .chain([layout::width(label)])
                .max()
                .unwrap_or(0)
                + layout::GAP
        })
        .sum();
    let rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|row| {
            let name = room.cells().map_or_else(
                || row.name.clone(),
                |cells| layout::cut(&row.name, cells.saturating_sub(others).max(FEWEST)),
            );
            std::iter::once(name)
                .chain(row.cells.iter().cloned())
                .collect()
        })
        .collect();
    // Where even a name cut short leaves too little room, the columns
    // past what it spent go, the last first.
    let gives_way: Vec<usize> = (2..report.columns.len()).rev().collect();
    layout::table_in(&report.columns, &rows, pen, room, &gives_way)
}

/// A report whose rows each say a sentence, as facts: its name, and the
/// sentence beside it or under it.
fn sentences(report: &Report, pen: &Pen, room: Room) -> Vec<String> {
    let facts: Vec<(&str, Option<&str>)> = report
        .rows
        .iter()
        .map(|row| (row.name.as_str(), Some(row.shown.as_str())))
        .collect();
    layout::facts(&facts, "unknown", pen, room)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Row;
    use crate::report::cuts::{spend_cells, spend_columns};

    /// A report with a name too long for a narrow terminal, and a note
    /// longer than any line: a cut where its rows have a share, sentences
    /// where they don't.
    fn a_wide_report(period: &str, share: f64, shown: &str) -> Report {
        let cut = share > 0.0;
        let row = |name: &str| Row {
            name: name.into(),
            dollars: 12.5,
            share,
            shown: shown.into(),
            estimated: false,
            cells: if cut {
                spend_cells(shown, share)
            } else {
                Vec::new()
            },
        };
        Report {
            title: "Spend by quest".into(),
            period: period.into(),
            from: "2026-11-07".into(),
            to: "2026-11-13".into(),
            total: 25.0,
            shown: "$25.00".into(),
            source: "pi's logs at list prices".into(),
            rest: None,
            columns: if cut {
                spend_columns("quest")
            } else {
                Vec::new()
            },
            rows: vec![
                row("QEST-20261112-0TUD09 Exquisite Command Lines for People and Machines"),
                row("short"),
            ],
            notes: vec![
                "Of $25.00 spent at list prices, $3.00 was cache written and $1.00 of that \
                 rebuilt context a session already had."
                    .into(),
            ],
        }
    }

    fn past(said: &str, cells: usize) -> Vec<&str> {
        said.lines()
            .filter(|line| layout::width(line) > cells)
            .collect()
    }

    #[test]
    fn a_cut_fits_a_narrow_terminal() {
        let said = render_text_in(
            &a_wide_report("last week", 0.5, "$12.50"),
            &Pen::plain(),
            Room::Cells(60),
        );
        assert!(past(&said, 60).is_empty(), "lines ran past 60:\n{said}");
        assert!(
            said.contains("  QEST-20261112-0TUD09 Exquisite Command") && said.contains('…'),
            "the long name wasn't cut at a whole word:\n{said}"
        );
    }

    #[test]
    fn a_cut_heads_its_columns_and_lines_its_figures_up_under_them() {
        let said = render_text_in(
            &a_wide_report("last week", 0.5, "$12.50"),
            &Pen::plain(),
            Room::Cells(60),
        );
        let head = said
            .lines()
            .find(|line| line.contains("SPENT"))
            .unwrap_or_else(|| panic!("the cut's columns weren't headed:\n{said}"));
        assert_eq!(
            head.split_whitespace().collect::<Vec<_>>(),
            ["QUEST", "SPENT", "SHARE"],
            "the cut was headed otherwise:\n{said}"
        );
        assert!(head.starts_with("  QUEST"), "the head isn't inset:\n{said}");
        let ends = |line: &str, word: &str| {
            line.find(word)
                .map(|at| layout::width(&line[..at + word.len()]))
        };
        for line in said.lines().filter(|line| line.contains("$12.50")) {
            assert_eq!(
                ends(line, "$12.50"),
                ends(head, "SPENT"),
                "a figure didn't end under its label:\n{said}"
            );
            assert_eq!(
                ends(line, "50%"),
                ends(head, "SHARE"),
                "a share didn't end under its label:\n{said}"
            );
        }
    }

    #[test]
    fn a_sentence_row_hangs_under_itself() {
        let sentence = "the default budget's cap rose from $100 to $200 at the start of the month";
        let said = render_text_in(
            &a_wide_report("caps", 0.0, sentence),
            &Pen::plain(),
            Room::Cells(60),
        );
        assert!(past(&said, 60).is_empty(), "lines ran past 60:\n{said}");
        let words = said.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            words.contains("$200 at the start of the month short"),
            "a sentence row lost its words:\n{said}"
        );
    }

    #[test]
    fn into_a_pipe_a_report_is_set_as_its_files_are() {
        let report = a_wide_report("last week", 0.5, "$12.50");
        let said = render_text_in(&report, &Pen::plain(), Room::Unbounded);
        assert_eq!(
            said,
            render(&report, Format::Text),
            "a report into a pipe differs from the one the daemon keeps"
        );
        // Its title, what it covers, a gap, the head and two rows, a gap
        // and its note.
        assert_eq!(
            said.lines().count(),
            8,
            "a pipe's report gave way somewhere:\n{said}"
        );
        assert!(
            said.contains("for People and Machines  $12.50"),
            "a pipe's name was cut:\n{said}"
        );
    }
}
