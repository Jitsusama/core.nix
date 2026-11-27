//! The subjects' hubs the realm's mycelium node names, and the needs a
//! sender puts to them, for each sender that once sent what no owner took
//! to one fallback cell: cadence's late findings (its `findings/hubs.rs`)
//! and the pains pain-points hears said to the void or in a session that
//! ended (its `hear/route.rs`). One cell taking all of them made it every
//! such sender's funnel; a hub spreads them over the cells holding their
//! subject.
//!
//! The node names a hub for each subject a cell declares (`list hub`). What
//! a sender has, a finding's key or his words, takes in a subject when it
//! says every word of it, read and judged by one rule with the harness's
//! routing (`reading.rs`), and the subject of most words is the narrowest,
//! so `crashes:chrome` takes a chrome crash before `crashes` does. Which of
//! a hub's holders gets it is each sender's to choose, as each counts what
//! its holders hold its own way.
//!
//! Each need put is kept as one line in the mesh's day file, in the shape
//! the harness keeps its own needs in (`lib/mesh/need-route.ts`'s
//! `NeedRecord`, which a line it wrote pins below), so one judge reads
//! every sender's.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

mod reading;

pub use reading::words;

/// A subject's hub, as the node names it: the cells it names as holding
/// the subject, best first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hub {
    /// A word, or one narrowed by colons, dots or slashes: `cadence:failed`.
    pub subject: String,
    /// The cell the hub is, by its address.
    pub cell: String,
    pub fits: Vec<String>,
}

impl Hub {
    /// The hub `cell` for `subject`, naming `fits` as its holders, best
    /// first: the one way to make one, so every crate's tests build it
    /// alike.
    pub fn new(subject: &str, cell: &str, fits: &[&str]) -> Self {
        Self {
            subject: subject.to_owned(),
            cell: cell.to_owned(),
            fits: fits.iter().map(|f| (*f).to_owned()).collect(),
        }
    }
}

/// A need as the mesh's day file keeps it, in the harness's shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Need {
    #[serde(rename = "need")]
    pub id: String,
    pub asker: String,
    pub words: Vec<String>,
    /// Each cell after the asker, the holder last.
    pub path: Vec<String>,
    pub fit: String,
    /// When it was put, in milliseconds.
    pub at: i64,
    /// How many cells the realm had.
    pub cells: usize,
}

impl Need {
    /// The `n`th need `asker` put at `at_ms`, by the subject's `words`, to
    /// `fit` through `hub`, in a realm of `cells`: its id is the asker's
    /// and the moment's with its place among them, so no two share one,
    /// and its path runs through the hub to the holder, or is the holder
    /// alone where the hub holds it.
    #[must_use]
    pub fn put(
        asker: &str,
        n: usize,
        at_ms: i64,
        words: Vec<String>,
        (hub, fit): (&str, &str),
        cells: usize,
    ) -> Self {
        Self {
            id: format!("{asker}/{}.{n}", base36(at_ms)),
            asker: asker.to_owned(),
            words,
            path: if hub == fit {
                vec![fit.to_owned()]
            } else {
                vec![hub.to_owned(), fit.to_owned()]
            },
            fit: fit.to_owned(),
            at: at_ms,
            cells,
        }
    }
}

/// Whether `said` is a plain name, of letters, digits and `:./_-`, and
/// short, so what a cell declared can't put words of its own into a
/// message a sender sends, or a path where a socket's name goes.
fn plain(said: &str, longest: usize) -> bool {
    !said.is_empty()
        && said.len() <= longest
        && said
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ":./_-".contains(c))
}

/// The longest subject taken from the node.
const SUBJECT_AT_MOST: usize = 80;

/// The hubs in the node's answer to `list hub`: each record names one hub
/// and its fits, or several under `hubs`. A node that holds no hubs
/// refuses the kind (`unknown-kind`), which lists none, as the harness
/// reads it; one record that can't be read is left out, so it costs only
/// its own hubs.
///
/// # Errors
///
/// When the answer isn't one, or carries another error, such as a node
/// too old to list, so a sender says why it read no hubs rather than
/// reading it as a node with none.
pub fn from_list(line: &str) -> io::Result<Vec<Hub>> {
    #[derive(Deserialize)]
    struct Named {
        hub: Option<String>,
        #[serde(default)]
        fits: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Record {
        key: String,
        #[serde(flatten)]
        named: Named,
        #[serde(default)]
        hubs: Vec<Named>,
    }
    #[derive(Deserialize)]
    struct Refusal {
        #[serde(default)]
        code: String,
        #[serde(default)]
        message: String,
    }
    #[derive(Deserialize)]
    struct Listed {
        error: Option<Refusal>,
        #[serde(default)]
        records: Vec<serde_json::Value>,
    }
    let listed = serde_json::from_str::<Listed>(line).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the node's list of hubs couldn't be read: {error}"),
        )
    })?;
    if let Some(refusal) = listed.error {
        if refusal.code == "unknown-kind" {
            return Ok(Vec::new());
        }
        return Err(io::Error::other(format!(
            "the node refused the list of hubs: {} ({})",
            refusal.message, refusal.code
        )));
    }
    let mut hubs = Vec::new();
    for record in listed.records {
        // One the node wrote in another shape names no hub of its own.
        let Ok(record) = serde_json::from_value::<Record>(record) else {
            continue;
        };
        let Some(subject) = record
            .key
            .strip_prefix("hub:")
            .filter(|s| plain(s, SUBJECT_AT_MOST))
        else {
            continue;
        };
        let named = if record.hubs.is_empty() {
            vec![record.named]
        } else {
            record.hubs
        };
        hubs.extend(named.into_iter().filter_map(|n| {
            Some(Hub {
                subject: subject.to_owned(),
                cell: n.hub?,
                fits: n.fits,
            })
        }));
    }
    Ok(hubs)
}

/// The most lines read for the list's answer, past the hello's and any
/// the node says of its own accord.
const LINES_AT_MOST: usize = 16;

/// The hubs the node at `socket` names, asked as the harness asks it: the
/// answer is the line with the list's id. A node that can't be asked, or
/// never answers the list, is an error, so a sender says so rather than
/// reading it as one with no hubs.
///
/// # Errors
///
/// When the node can't be reached or read, never answers the list, or
/// answers it with an error other than holding no hubs, or hasn't within
/// [`LISTED_AT_MOST`].
pub fn listed(socket: &Path) -> io::Result<Vec<Hub>> {
    listed_within(socket, LISTED_AT_MOST)
}

/// How long the whole ask of the node's hubs may take, connect and every
/// line read: a sender asks it holding its own run's lock, so a node that
/// never answers mustn't hold every later run behind it (#6774's round,
/// F17). A connect to a node whose backlog is full blocks with no timeout
/// of its own, and each read's would otherwise add up line by line.
pub const LISTED_AT_MOST: Duration = Duration::from_secs(5);

/// The hubs the node at `socket` names, as [`listed`] reads them, or an
/// error once `within` has passed. The ask runs on a thread of its own,
/// which a node that never answers leaves behind until the sender ends.
///
/// # Errors
///
/// As [`listed`], and `TimedOut` when the node hasn't answered `within`.
pub fn listed_within(socket: &Path, within: Duration) -> io::Result<Vec<Hub>> {
    let (answered, answer) = std::sync::mpsc::channel();
    let socket = socket.to_owned();
    std::thread::spawn(move || {
        // The asker gave up waiting once it's gone, and nothing is lost.
        let _ = answered.send(asked(&socket));
    });
    answer.recv_timeout(within).unwrap_or_else(|gone| {
        Err(match gone {
            std::sync::mpsc::RecvTimeoutError::Timeout => io::Error::new(
                io::ErrorKind::TimedOut,
                format!("the node didn't list its hubs within {within:?}"),
            ),
            std::sync::mpsc::RecvTimeoutError::Disconnected => {
                io::Error::other("the ask of the node's hubs ended without an answer")
            }
        })
    })
}

/// The hubs the node at `socket` names, asked with no bound of its own on
/// the whole: [`listed_within`] gives it one.
fn asked(socket: &Path) -> io::Result<Vec<Hub>> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"{\"id\":1,\"ask\":\"hello\"}\n")?;
    let list = r#"{"id":2,"ask":"list","kind":"hub","fields":["hub","fits","hubs"]}"#;
    stream.write_all(format!("{list}\n").as_bytes())?;
    for line in BufReader::new(stream).lines().take(LINES_AT_MOST) {
        let line = line?;
        let id = serde_json::from_str::<serde_json::Value>(&line)
            .ok()
            .and_then(|v| v.get("id").and_then(serde_json::Value::as_i64));
        if id == Some(2) {
            return from_list(&line);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the node never answered the list of hubs",
    ))
}

/// The node's socket for the realm `MYCELIUM_REALM` names, else the one
/// the node keeps under `state_home`, in `runtime`; none for a realm
/// whose name isn't a plain one.
#[must_use]
pub fn socket(runtime: &Path, state_home: &Path, realm: Option<String>) -> Option<PathBuf> {
    let realm = realm.filter(|r| !r.is_empty()).or_else(|| {
        let kept = std::fs::read_to_string(state_home.join("mycelium/realm")).ok()?;
        Some(kept.trim().to_owned()).filter(|r| !r.is_empty())
    })?;
    let plain = realm
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| runtime.join("mycelium").join(format!("{realm}.sock")))
}

/// The hubs whose subject `said` takes in, saying every word of it as the
/// harness judges a word said (`reading::says`), whose hub cell is `live`,
/// each with its subject's words: the subject of most words first, then
/// the order the node names them in.
#[must_use]
pub fn taking<'a>(
    said: &[String],
    hubs: &'a [Hub],
    live: &impl Fn(&str) -> bool,
) -> Vec<(&'a Hub, Vec<String>)> {
    let said = reading::routed(said);
    let mut matched: Vec<(&Hub, Vec<String>)> = hubs
        .iter()
        .filter(|h| live(&h.cell))
        .filter_map(|h| {
            let subject = reading::subject_words(&h.subject);
            let all = !subject.is_empty() && subject.iter().all(|w| reading::says(w, &said));
            all.then_some((h, subject))
        })
        .collect();
    // A stable sort, so among subjects of as many words the node's order
    // stands.
    matched.sort_by_key(|(_, subject)| std::cmp::Reverse(subject.len()));
    matched
}

/// `n` in base 36, as the harness writes a need's moment.
fn base36(n: i64) -> String {
    let mut n = u64::try_from(n).unwrap_or_default();
    let mut digits = Vec::new();
    loop {
        let d = u32::try_from(n % 36).unwrap_or_default();
        digits.push(char::from_digit(d, 36).unwrap_or('0'));
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

/// The UTC day holding `at_ms`, milliseconds since 1970, as YYYY-MM-DD,
/// which names the harness's day file of needs.
fn utc_day(at_ms: i64) -> String {
    let days = at_ms.div_euclid(86_400_000);
    // Days since 1970 as a civil date, by Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Adds `needs` to their day's file under the mesh's `root`, the
/// harness's folder, a line each, appended whole, so a session keeping a
/// need at once never interleaves. The first need's day names the file.
///
/// # Errors
///
/// When the folder or the file can't be made or written.
pub fn keep(root: &Path, needs: &[Need]) -> io::Result<()> {
    let Some(first) = needs.first() else {
        return Ok(());
    };
    let folder = root.join("mesh-workflow/needs");
    std::fs::create_dir_all(&folder)?;
    let mut lines = String::new();
    for need in needs {
        lines.push_str(&serde_json::to_string(need)?);
        lines.push('\n');
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join(format!("{}.jsonl", utc_day(first.at))))?
        .write_all(lines.as_bytes())
}

#[cfg(test)]
mod tests;
