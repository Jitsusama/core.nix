//! What the guard sees, in sentences Joel can read at a glance: each
//! disk's room and what the guard is doing about it, the machine's worst,
//! and each thing it cleared. The status file carries them for the desktop,
//! which shows them as they are rather than working them out again.

use crate::band::Band;
use crate::units::shown;
use std::fmt::Write;
use std::path::Path;

/// How soon a disk has to be filling for a healthy one to say when: a disk
/// two days from full is worth knowing about before it's short of room.
const TELL_WITHIN: f64 = 2.0 * 86400.0;

/// One filesystem's room, as a sentence needs it.
#[derive(Clone, Copy, Debug)]
pub struct Room<'a> {
    pub path: &'a Path,
    pub band: Band,
    pub free: u64,
    pub size: u64,
    /// Seconds until it's full at the present fill, if it's filling.
    pub full_in: Option<f64>,
    /// Whether the last pass in this band or a worse one found nothing
    /// more it may clear, so saying it's clearing would be untrue.
    pub cleared_all: bool,
}

/// What one filesystem's room is, and what the guard does about it.
pub fn filesystem(room: &Room, home: Option<&Path>) -> String {
    let name = named(room.path, home);
    let free = shown(room.free);
    let mut said = match (room.band, room.cleared_all) {
        (Band::Healthy, _) => format!("{name} has {free} free."),
        (Band::Filling, true) => format!(
            "{name} is filling, with {free} free. \
             The guard has cleared all it may; what's left is in use or kept."
        ),
        (Band::NearlyFull, true) => format!(
            "{name} is nearly full, with {free} free. \
             The guard has cleared all it safely can; delete something you don't need."
        ),
        (Band::CriticallyFull, true) => format!(
            "{name} is critically full, with {free} free. Saves may fail; \
             the guard has cleared all it can and may freeze what is filling it."
        ),
        (Band::Filling, false) => format!(
            "{name} is filling, with {free} free. \
             The guard is clearing builds and caches nobody has used lately."
        ),
        (Band::NearlyFull, false) => format!(
            "{name} is nearly full, with {free} free. \
             The guard is clearing everything it safely can."
        ),
        (Band::CriticallyFull, false) => format!(
            "{name} is critically full, with {free} free. Saves may fail; \
             the guard is clearing everything it can and may freeze what is filling it."
        ),
    };
    if let Some(full_in) = room.full_in.filter(|full_in| *full_in < TELL_WITHIN) {
        // Writing to a String can't fail.
        _ = write!(said, " At this rate it fills in {}.", span(full_in));
    }
    said
}

/// The one sentence the machine's row shows: its worst disk's, or when
/// every disk has room and none is filling fast, how much the system disk
/// has, or without it the tightest watched: a small build disk with room
/// leading would read as a machine nearly full.
pub fn machine(rooms: &[Room], home: Option<&Path>) -> String {
    let Some(worst) = rooms.iter().max_by_key(|room| room.band) else {
        return "Not watching any disk yet.".to_owned();
    };
    if worst.band != Band::Healthy {
        return filesystem(worst, home);
    }
    let filling = rooms
        .iter()
        .filter_map(|room| room.full_in.map(|full_in| (room, full_in)))
        .filter(|(_, full_in)| *full_in < TELL_WITHIN)
        .min_by(|(_, one), (_, other)| one.total_cmp(other));
    if let Some((room, _)) = filling {
        return filesystem(room, home);
    }
    let tightest = rooms
        .iter()
        .find(|room| room.path == Path::new("/"))
        .or_else(|| rooms.iter().min_by_key(|room| room.free))
        .unwrap_or(worst);
    format!(
        "Plenty of room: {} free on {}.",
        shown(tightest.free),
        lower(&named(tightest.path, home))
    )
}

/// What one thing the guard cleared was, where it was and how big its
/// files were as read, which on a compressed disk can be several times
/// what clearing them freed, so it's said as theirs: a pass's total says
/// what the disk got back.
pub fn cleared(kind: &str, path: &Path, bytes: u64, home: Option<&Path>) -> String {
    let place = tilde(path, home);
    let size = shown(bytes);
    match label(kind) {
        Some(what) => format!("Cleared {what}, {place} ({size} of files)."),
        None => format!("Cleared {place} ({size} of files)."),
    }
}

/// What a kind of candidate is called in a sentence.
fn label(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "target" => "a Rust build",
        "replaced" => "an older Rust build",
        "variant" => "a build variant",
        "install" => "an install",
        "scratch" => "scratch space",
        "snapshot" => "a snapshot",
        "git-temporary" => "what an unfinished git fetch left",
        "workspace" => "a workspace",
        "temp" => "a temporary file",
        "trash" => "something from the trash",
        "cache" => "a cache",
        "coredump" => "a crash dump",
        "nix-build" => "a leftover Nix build",
        "garbage" => "unused Nix store paths",
        "generations" => "old system generations",
        "producer" => "a tool's own leftovers",
        "log" => "a log",
        "podman" => "container images",
        _ => return None,
    })
}

/// A filesystem as people know it, at the start of a sentence.
pub fn named(path: &Path, home: Option<&Path>) -> String {
    if path == Path::new("/") {
        "The disk".to_owned()
    } else if path == Path::new("/nix/store") || path == Path::new("/nix") {
        "The Nix store".to_owned()
    } else if home.is_some_and(|home| path == home) {
        "Your home folder".to_owned()
    } else {
        tilde(path, home)
    }
}

/// A name from `named` for the middle of a sentence.
pub fn lower(name: &str) -> String {
    match name.strip_prefix("The ") {
        Some(rest) => format!("the {rest}"),
        None => match name.strip_prefix("Your ") {
            Some(rest) => format!("your {rest}"),
            None => name.to_owned(),
        },
    }
}

/// A path, with the home folder written `~`.
pub fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// A length of time as people read one, in its largest two units as
/// every command says one (docs/commands.md, Figures and Words): `15 h
/// 2 m` rather than `15.0 h`, to the nearest second.
pub fn span(seconds: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a span is never negative, and the guard's are far short of a u64 of seconds"
    )]
    let whole = seconds.max(0.0).round() as u64;
    terminal_kit::time::span(whole)
}
