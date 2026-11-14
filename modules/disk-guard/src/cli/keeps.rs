//! What `disk-guard keeps` reads back: every folder kept, or the keep
//! covering one folder. Adding and letting go are `acts::keep` and
//! `acts::unkeep`, which `keeps FOLDER add` and `remove` reach.

use super::{Env, failed, said_path, wall};
use crate::keeps::{Keep, Keeps};
use crate::words::{span, tilde};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use terminal_kit::layout::{self, Align};
use terminal_kit::{Context, Exit, Failure, time};

/// The list's columns: the folder names each keep, so it comes first.
const COLUMNS: [(&str, Align); 4] = [
    ("FOLDER", Align::Left),
    ("KEPT", Align::Right),
    ("BY", Align::Left),
    ("WHY", Align::Left),
];

/// Where the room is short, who kept it goes first, then since when; the
/// reason is cut rather than lost.
const GIVES_WAY: [usize; 2] = [2, 1];

/// Every folder kept, with who kept it, since when and why.
pub(super) fn every(env: &Env, context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    let keeps = read(env)?;
    if context.format().is_json() {
        let keeps: Vec<Value> = keeps.iter().map(|(path, keep)| said(path, keep)).collect();
        let answer = json!({"schema": "disk-guard.keeps/1", "keeps": keeps});
        return context.answer_to(out, &answer).map_err(failed);
    }
    if keeps.is_empty() {
        return writeln!(out, "Nothing is kept.").map_err(failed);
    }
    let now = wall();
    let home = home();
    // A table's first column never shortens, so a long folder is shortened
    // here, in its middle, to leave the reason room; into a pipe it's whole.
    let folder_room = context
        .room
        .cells()
        .map(|cells| (cells * 2 / 5).max(layout::SHORTENED));
    let rows: Vec<Vec<String>> = keeps
        .iter()
        .map(|(path, keep)| {
            let folder = tilde(path, home.as_deref());
            vec![
                folder_room.map_or_else(|| folder.clone(), |room| layout::shortened(&folder, room)),
                format!("{} ago", span(now - keep.at)),
                by(keep).to_owned(),
                keep.reason.clone(),
            ]
        })
        .collect();
    let lines = layout::table_in(&COLUMNS, &rows, &context.out, context.room, &GIVES_WAY);
    out.write_all(layout::joined(vec![lines]).as_bytes())
        .map_err(failed)
}

/// The keep covering `path`, kept on it or on a folder holding it, or
/// why there's none.
pub(super) fn covering(
    env: &Env,
    path: &Path,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    // A kept folder may have gone since, and its keep still stands.
    let path = fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .map_err(failed)?;
    let keeps = read(env)?;
    let Some((kept, keep)) = crate::keeps::kept_over(&keeps, &path) else {
        return Err(
            Failure::new(Exit::NotFound, format!("{} isn't kept", path.display()))
                .because("no keep is on it or on a folder holding it")
                .try_this("disk-guard keeps"),
        );
    };
    if context.format().is_json() {
        let answer = json!({
            "schema": "disk-guard.kept/1",
            "path": said_path(&path),
            "kept": said(kept, keep),
        });
        return context.answer_to(out, &answer).map_err(failed);
    }
    let folder = tilde(kept, home().as_deref());
    let since = format!("{} ago", span(wall() - keep.at));
    let facts = [
        ("folder", Some(folder.as_str())),
        ("why", Some(keep.reason.as_str())),
        ("kept", Some(since.as_str())),
        ("by", Some(by(keep))),
    ];
    let lines = layout::facts(&facts, "", &context.out, context.room);
    out.write_all(layout::joined(vec![lines]).as_bytes())
        .map_err(failed)
}

/// The home folder, written `~` where a folder kept is said to a person.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn read(env: &Env) -> Result<Keeps, Failure> {
    crate::keeps::load(&env.state).map_err(|error| {
        Failure::new(Exit::Failed, "the keeps can't be read").because(error.to_string())
    })
}

/// Who a keep is Joel's or his machine's: a declared one is the
/// configuration's, and only it lets that go.
fn by(keep: &Keep) -> &'static str {
    if keep.declared { "machine" } else { "you" }
}

/// A keep as JSON says it: its moment in RFC 3339, as every schema does.
fn said(path: &Path, keep: &Keep) -> Value {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a keep's wall seconds are whole seconds well within i64"
    )]
    let at = keep.at as i64;
    json!({
        "path": said_path(path),
        "reason": keep.reason,
        "since": time::rfc3339(at),
        "declared": keep.declared,
    })
}
