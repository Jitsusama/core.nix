//! How the sites and downloads are said: into a pipe, the tab-separated
//! lines the launcher reads (modules/chrome/ChromeProgram.qml), byte for
//! byte; into a terminal, a table set at a reading measure; and with
//! `--json`, an object naming its schema.

use std::io::{self, Write as _};
use std::time::{SystemTime, UNIX_EPOCH};

use chrome_session::host;
use serde_json::{Value, json};
use terminal_kit::layout::{self, Align};
use terminal_kit::time::{rfc3339, span};
use terminal_kit::{Context, Room};

use crate::Site;
use crate::downloads::Download;

/// A site's table: when it was visited, its site and its page's title.
const SITES: [(&str, Align); 3] = [
    ("visited", Align::Left),
    ("site", Align::Left),
    ("title", Align::Left),
];

/// What goes where the sites are too wide: the site, which the title
/// mostly names.
const SITES_GIVE_WAY: [usize; 1] = [1];

/// A download's table: when it finished, its size, the site it came from
/// and its file's name, whose folder the JSON and a pipe say.
const DOWNLOADS: [(&str, Align); 4] = [
    ("finished", Align::Left),
    ("size", Align::Right),
    ("from", Align::Left),
    ("file", Align::Left),
];

/// What goes where the downloads are too wide: the site they came from,
/// then their size.
const DOWNLOADS_GIVE_WAY: [usize; 2] = [2, 1];

/// The sites, said as the context asks.
pub fn sites(context: &Context, found: &[Site]) -> io::Result<()> {
    if context.format().is_json() {
        let sites: Vec<Value> = found
            .iter()
            .map(|site| {
                json!({
                    "address": site.address,
                    "title": site.title,
                    "visited": site.visited.map(rfc3339),
                })
            })
            .collect();
        return context.answer(&json!({
            "schema": "chrome-recent-sites.sites/1",
            "sites": sites,
        }));
    }
    let mut out = io::stdout().lock();
    if context.room == Room::Unbounded {
        return found.iter().try_for_each(|site| {
            let visited = site.visited.map(|at| at.to_string()).unwrap_or_default();
            writeln!(out, "{}\t{}\t{visited}", site.address, site.title)
        });
    }
    let now = now();
    let rows: Vec<Vec<String>> = found
        .iter()
        .map(|site| {
            vec![
                ago(site.visited, now),
                host(&site.address).unwrap_or_default().to_owned(),
                site.title.clone(),
            ]
        })
        .collect();
    table(context, &SITES, &rows, &SITES_GIVE_WAY, &mut out)
}

/// The downloads, said as the context asks.
pub fn downloads(context: &Context, found: &[Download]) -> io::Result<()> {
    if context.format().is_json() {
        let downloads: Vec<Value> = found
            .iter()
            .map(|download| {
                json!({
                    "path": download.path.display().to_string(),
                    "size_bytes": download.size,
                    "finished": download.when.map(rfc3339),
                    "from": download.from,
                })
            })
            .collect();
        return context.answer(&json!({
            "schema": "chrome-recent-sites.downloads/1",
            "downloads": downloads,
        }));
    }
    let mut out = io::stdout().lock();
    if context.room == Room::Unbounded {
        return found
            .iter()
            .try_for_each(|download| writeln!(out, "{}", download.line()));
    }
    let now = now();
    let rows: Vec<Vec<String>> = found
        .iter()
        .map(|download| {
            vec![
                ago(download.when, now),
                size(download.size),
                host(&download.from).unwrap_or_default().to_owned(),
                download.path.file_name().map_or_else(
                    || download.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                ),
            ]
        })
        .collect();
    table(context, &DOWNLOADS, &rows, &DOWNLOADS_GIVE_WAY, &mut out)
}

/// A table at a reading measure, however wide the terminal, since its
/// last column is a title or a path to read.
fn table(
    context: &Context,
    columns: &[(&str, Align)],
    rows: &[Vec<String>],
    gives_way: &[usize],
    out: &mut dyn io::Write,
) -> io::Result<()> {
    let room = context.room.for_prose();
    let lines = layout::table_in(columns, rows, &context.out, room, gives_way);
    out.write_all(layout::joined(vec![lines]).as_bytes())
}

/// Seconds since 1970 now.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// How long ago a moment was, or that Chrome kept no time.
fn ago(at: Option<i64>, now: i64) -> String {
    at.map_or_else(
        || "never".to_owned(),
        |at| format!("{} ago", span(u64::try_from(now - at).unwrap_or(0))),
    )
}

/// A size in bytes as the desktop sets one: three significant digits,
/// taking the next decimal unit past 999.
fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["kB", "MB", "GB", "TB", "PB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "three significant digits are all that's said"
    )]
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 999.5 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let places = match value {
        ..9.995 => 2,
        ..99.95 => 1,
        _ => 0,
    };
    format!("{value:.places$} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_keeps_three_figures_and_takes_the_next_unit_past_999() {
        let said: Vec<String> = [0, 999, 1000, 12_345, 999_499, 999_500, 1_234_567_890]
            .into_iter()
            .map(size)
            .collect();
        assert_eq!(
            said,
            [
                "0 B", "999 B", "1.00 kB", "12.3 kB", "999 kB", "1.00 MB", "1.23 GB"
            ],
            "a size reads as the desktop sets one"
        );
    }

    #[test]
    fn a_moment_never_kept_says_so() {
        assert_eq!(ago(None, 100), "never", "no time reads as never");
        assert_eq!(
            ago(Some(40), 100),
            "1 m ago",
            "a time reads as how long ago"
        );
    }
}
