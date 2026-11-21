//! Whether a log is a copy pi carried a session into, and whether its
//! parent is here to count what was copied. A copy's turns before its
//! header are its parent's, so they're counted once, by the parent, when
//! the parent's log is found where this machine would read it or the
//! history already holds the parent's session, read from any machine's
//! logs. Which is decided for the session, not for one log of it: two
//! copies of a session, or one gone on elsewhere without its parent, would
//! otherwise each count the parent's turns again.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Whether a header is that of a log pi carried a session into, from a
/// parent whose log is still there to count what was copied. A log
/// brought from another machine names its parent by that machine's path,
/// so its parent is looked for where that machine's sessions are brought,
/// in the same project folder under the same name. `held` says whether
/// the history holds the parent's session already.
pub fn carried(header: &Value, brought: Option<&Path>, held: bool) -> bool {
    parent(header).is_some_and(|parent| held || here(Path::new(parent), brought).is_file())
}

/// The session a header names as its parent, by its log's name: pi names
/// a log for when it began and the session it holds, joined by an
/// underscore.
pub fn parent_id(header: &Value) -> Option<String> {
    let stem = Path::new(parent(header)?).file_stem()?.to_str()?;
    let id = stem.rsplit_once('_').map_or(stem, |(_, id)| id);
    (!id.is_empty()).then(|| id.to_owned())
}

/// Whether a brought log names a parent that hasn't been brought yet.
/// Logs arrive in whatever order whatever brings them keeps, so a copy can
/// land before its parent; it's read whole meanwhile and checked again on
/// each pass, to be read again for its own turns once the parent lands.
pub fn awaits_parent(header: &Value, brought: Option<&Path>, held: bool) -> bool {
    brought.is_some() && parent(header).is_some() && !carried(header, brought, held)
}

/// The parent a session's header names, if it names one.
fn parent(header: &Value) -> Option<&str> {
    if header.get("type").and_then(Value::as_str) != Some("session") {
        return None;
    }
    header.get("parentSession").and_then(Value::as_str)
}

/// Where a parent a log names is on this machine: where it says, or, for
/// a brought log, its project folder and name under the brought sessions.
fn here(parent: &Path, brought: Option<&Path>) -> PathBuf {
    let Some(brought) = brought else {
        return parent.to_path_buf();
    };
    let name = parent.file_name().unwrap_or_default();
    match parent.parent().and_then(Path::file_name) {
        Some(project) => brought.join(project).join(name),
        None => brought.join(name),
    }
}
