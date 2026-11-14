//! The tab in front of each of niri's windows a get names
//! (`front:<app>:<niri id>`, front.rs), made as the get comes from what
//! the program has open, just read, and each window's title asked of the
//! windows daemon then. Its watch (rows.rs) asks for no title, so a page
//! retitling a window wakes nothing here; a get asks it once, so the
//! title is the one Joel sees as he says a line over the window.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use programs::front;
use serde_json::{Value, json};

use super::{Shared, keep, lock};

/// How long the windows daemon is waited on, past which no window has
/// its tab in front said, rather than the get waiting on it.
const ASKING: Duration = Duration::from_millis(500);

/// Whether these keys name a window of `app`, so what it has open is
/// read before they're made.
pub fn names(app: &str, keys: &[String]) -> bool {
    keys.iter()
        .filter_map(|key| front::named(key))
        .any(|(named, _)| named == app)
}

/// Keeps the record of the tab in front of each window these keys name,
/// and says gone each one whose tab can't be told.
pub fn read(shared: &Shared, keys: &[String]) {
    let named: Vec<(&String, &str, u64)> = keys
        .iter()
        .filter_map(|key| front::named(key).map(|(app, window)| (key, app, window)))
        .collect();
    if named.is_empty() {
        return;
    }
    let titles = titles(&shared.windows, named.iter().map(|(_, _, window)| *window))
        .inspect_err(|why| eprintln!("<4>no window's title was heard, so no tab in front: {why}"))
        .unwrap_or_default();
    let mut daemon = lock(&shared.daemon);
    let (mut records, mut gone) = (Vec::new(), Vec::new());
    for (key, app, window) in named {
        let opened = daemon.held.get(&format!("opened:{app}"));
        match titles
            .get(&window)
            .zip(opened)
            .and_then(|(titled, opened)| front::record(app, window, titled, opened))
        {
            Some(record) => records.push(record),
            None => gone.push(key.clone()),
        }
    }
    keep(&mut daemon, records, &gone);
}

/// Each of these windows' titles, as the windows daemon at `socket` says
/// them now.
fn titles(
    socket: &Path,
    windows: impl Iterator<Item = u64>,
) -> std::io::Result<BTreeMap<u64, String>> {
    let keys: Vec<String> = windows.map(|window| format!("window:{window}")).collect();
    let stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(ASKING))?;
    stream.set_write_timeout(Some(ASKING))?;
    writeln!(
        &stream,
        "{}",
        json!({"id": "front", "ask": "get", "keys": keys})
    )?;
    for line in BufReader::new(&stream).lines() {
        let Ok(answer) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        if answer["id"] != "front" {
            continue;
        }
        return Ok(answer["records"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| {
                let window = record["key"]
                    .as_str()?
                    .strip_prefix("window:")?
                    .parse()
                    .ok()?;
                Some((window, record["title"].as_str()?.to_owned()))
            })
            .collect());
    }
    Err(std::io::Error::other("it hung up without answering"))
}
