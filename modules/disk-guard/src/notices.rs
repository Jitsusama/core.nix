//! What the user daemon tells its person, from how its status moved over a
//! step: one notice, replaced in place as things change, and withdrawn
//! once no disk is nearly full. A notice says what happened, why it
//! matters, and what to do or that nothing is needed. A disk under
//! pressure is the launcher's to show and never a notice: the guard passes
//! over it every half hour or so, mostly taking nothing, and a notice each
//! time taught Joel to ignore them. Routine upkeep and a disk easing say
//! nothing either.

use crate::band::Band;
use crate::daemon::{FilesystemStatus, LastPass, Status};
use crate::units::{shown, whole};
use crate::words::{lower, named};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Urgency {
    Normal,
    Critical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub summary: String,
    pub body: String,
    pub urgency: Urgency,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Shows this, in place of whatever was shown.
    Show(Notice),
    /// Takes away whatever was shown.
    Withdraw,
}

/// What to tell, given the status before a step and after it: a writer
/// frozen, else a pass from urgent on that finished, else the worse band
/// a disk entered from urgent on, else the notice withdrawn once no disk
/// is nearly full.
pub fn change(before: &Status, after: &Status) -> Option<Change> {
    // A writer frozen stops somebody's work, so it's told before anything.
    if let Some(frozen) = after
        .frozen
        .iter()
        .find(|frozen| !before.frozen.iter().any(|was| was.unit == frozen.unit))
    {
        let disk = disk(&frozen.path);
        let free = after
            .filesystems
            .iter()
            .find(|filesystem| filesystem.path == frozen.path)
            .map_or_else(|| "little".to_owned(), |filesystem| shown(filesystem.free));
        return Some(Change::Show(Notice {
            summary: format!("Froze {}, which was filling {}", frozen.name, lower(&disk)),
            body: format!(
                "{disk} had {free} free and nothing else was safe to clear. {} carries on by \
                 itself once there's room, or now if you run disk-guard thaw.",
                frozen.name
            ),
            urgency: Urgency::Critical,
        }));
    }
    if let Some(last) = &after.last
        && after.last != before.last
        && last.band >= Band::Urgent
    {
        return Some(Change::Show(passed(last, after)));
    }
    let was = |filesystem: &FilesystemStatus| {
        before
            .filesystems
            .iter()
            .find(|earlier| earlier.path == filesystem.path)
            .map_or(Band::Healthy, |earlier| earlier.band)
    };
    let entered = after
        .filesystems
        .iter()
        .filter(|filesystem| filesystem.band >= Band::Urgent && filesystem.band > was(filesystem))
        .max_by_key(|filesystem| filesystem.band);
    if let Some(filesystem) = entered {
        let disk = disk(&filesystem.path);
        let free = shown(filesystem.free);
        let critical = filesystem.band == Band::Critical;
        return Some(Change::Show(if critical {
            Notice {
                summary: format!("{disk} is critically full"),
                body: format!(
                    "{free} free, and saves may fail. The guard is clearing what it can, and \
                     freezes whatever is filling the disk if that isn't enough."
                ),
                urgency: Urgency::Critical,
            }
        } else {
            Notice {
                summary: format!("{disk} is nearly full"),
                body: format!(
                    "{free} free. The guard is clearing everything it safely can; nothing is \
                     needed from you yet."
                ),
                urgency: Urgency::Normal,
            }
        }));
    }
    let pressed = |status: &Status| {
        status
            .filesystems
            .iter()
            .any(|filesystem| filesystem.band >= Band::Urgent)
    };
    (pressed(before) && !pressed(after)).then_some(Change::Withdraw)
}

/// The notice of `last`, a pass from urgent on that finished: what it
/// cleared, and what's left to do, if anything.
fn passed(last: &LastPass, after: &Status) -> Notice {
    let disk = disk(&last.path);
    let free = after
        .filesystems
        .iter()
        .find(|filesystem| filesystem.path == last.path)
        .map_or_else(|| "little".to_owned(), |filesystem| shown(filesystem.free));
    let urgency = if last.band == Band::Critical {
        Urgency::Critical
    } else {
        Urgency::Normal
    };
    // Plain text: a notification server may show markup as it's written.
    if last.end == "reached" {
        return Notice {
            summary: format!("Made room on {}", lower(&disk)),
            body: format!(
                "Cleared {} of builds and caches; {free} free now. Nothing is needed from you.",
                shown(last.freed)
            ),
            urgency,
        };
    }
    let cleared = if last.freed == 0 {
        format!("Nothing cleared, and only {free} is free")
    } else {
        format!("Cleared {}, but only {free} is free", shown(last.freed))
    };
    let what = match last.end.as_str() {
        "exhausted" => format!(
            "{cleared} and nothing else is safe to clear. Delete something you don't need; \
             disk-guard plan lists what's left."
        ),
        "stalled" => format!(
            "{cleared}. Deleting stopped freeing space, so the guard waits a little before \
             trying again; nothing is needed from you yet."
        ),
        failed => format!("{cleared}. The guard {failed}. disk-guard journal says more."),
    };
    // Left short, whoever is filling it is worth knowing.
    let body = match &after.writer {
        Some(writer) => format!(
            "{what} {} (pid {}) is writing most, at {} a second.",
            writer.name,
            writer.pid,
            shown(whole(writer.rate))
        ),
        None => what,
    };
    let still = if last.band == Band::Critical {
        "still critically full"
    } else {
        "still nearly full"
    };
    Notice {
        summary: format!("{disk} is {still}"),
        body,
        urgency,
    }
}

/// A disk as people know it, at the start of a sentence. A notice is the
/// account's, but a disk is named by where it's mounted, never under home.
fn disk(path: &std::path::Path) -> String {
    named(path, None)
}
