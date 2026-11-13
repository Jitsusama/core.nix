//! What the agents daemon says are agents' windows (`agent_panes.rs`),
//! heard for as long as this runs: one watch of its `agent-window`
//! records, which it tells only as a session's window comes or goes. A
//! list served without agents', kitty's open list, is served without
//! their panes, saying how many it left out, and Programs counts their
//! windows apart from Joel's; both are made again as it tells. Until it
//! serves the kind, or from when it stops answering, what was last heard
//! stands, nothing at first, and the watch is asked again once it hangs
//! up, as it does when it's started anew.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use programs::agent_panes::{self, Pane};
use programs::source::{self, Source, Without};
use serde_json::{Value, json};

use super::{Daemon, Shared, follow, keep, lock, rows, tabs};

/// The one watch held of the agents daemon.
const WATCH: &str = r#"{"id":"programs","ask":"watch","kind":"agent-window","place":"agents"}"#;

/// The agents' windows as last heard, by their keys, and what each list
/// served without them last printed, by its source's place.
#[derive(Default)]
pub struct Agents {
    records: BTreeMap<String, Value>,
    printed: BTreeMap<usize, String>,
}

/// Where the agents daemon answers: `PROGRAMS_AGENTS`, else beside this
/// daemon's own socket, as systemd puts both.
pub fn socket(own: &Path) -> PathBuf {
    std::env::var_os("PROGRAMS_AGENTS")
        .map_or_else(|| own.with_file_name("agents.sock"), PathBuf::from)
}

/// Hears the agents daemon for as long as this runs (follow.rs).
pub fn hear(shared: &Arc<Shared>, socket: &Path) {
    let said = |said: &Value| told(shared, said);
    follow::follow(
        socket,
        WATCH,
        "the agents daemon",
        "kitty's page and Programs leave out the agents' windows last heard",
        &said,
    );
}

/// What the agents daemon told, and each list and row made again from it.
/// An answer refusing the kind, as one from before it served it, changes
/// nothing.
fn told(shared: &Shared, said: &Value) {
    if said.get("error").is_some() {
        return;
    }
    let mut daemon = lock(&shared.daemon);
    follow::records(&mut daemon.agents.records, said);
    let records: Vec<Value> = daemon
        .agents
        .printed
        .iter()
        .map(|(&at, printed)| made(&daemon.agents, &shared.sources[at], printed))
        .collect();
    keep(&mut daemon, records, &[]);
    if rows::watched(&daemon) {
        rows::make(shared, &mut daemon);
    }
    if tabs::watched(&daemon) {
        tabs::make(shared, &mut daemon);
    }
}

/// The windows daemon's keys for the agents' windows.
pub fn windows(daemon: &Daemon) -> BTreeSet<String> {
    daemon
        .agents
        .records
        .values()
        .filter_map(agent_panes::window)
        .map(str::to_owned)
        .collect()
}

/// A source's list as its command printed it, served without the
/// agents' panes where it says so, what it printed kept to serve again
/// as they change.
pub fn listed(daemon: &mut Daemon, at: usize, source: &Source, printed: &str) -> Value {
    if source.without != Some(Without::Agents) {
        return source::recent(source, printed);
    }
    daemon.agents.printed.insert(at, printed.to_owned());
    made(&daemon.agents, source, printed)
}

fn made(agents: &Agents, source: &Source, printed: &str) -> Value {
    let panes: BTreeSet<Pane> = agents
        .records
        .values()
        .filter_map(agent_panes::pane)
        .collect();
    let (left, taken) = agent_panes::without(printed, &panes);
    let mut record = source::recent(source, &left);
    record["agents"] = json!(taken);
    record
}
