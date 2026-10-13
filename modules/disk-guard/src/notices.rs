//! What the user daemon tells its person, from how its status moved over a
//! step: one notice, replaced in place as things change, and withdrawn
//! once every disk is healthy again. Routine upkeep and a disk easing
//! from one band to a milder one say nothing, so a notice always means
//! something worth a look.

use crate::band::Band;
use crate::daemon::{FilesystemStatus, Status};
use crate::units::shown;

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

/// What to tell, given the status before a step and after it: a pass
/// below healthy that finished, else the worst band a disk entered, else
/// the notice withdrawn once no disk is below healthy.
pub fn change(before: &Status, after: &Status) -> Option<Change> {
    // A writer frozen stops somebody's work, so it's told before anything.
    if let Some(frozen) = after
        .frozen
        .iter()
        .find(|frozen| !before.frozen.iter().any(|was| was.unit == frozen.unit))
    {
        return Some(Change::Show(Notice {
            summary: format!(
                "Froze {}, which was filling {}",
                frozen.name,
                frozen.path.display()
            ),
            body: "Nothing else was safe to take. It goes on by itself once there's room, or now \
                   with disk-guard thaw."
                .to_owned(),
            urgency: Urgency::Critical,
        }));
    }
    if let Some(last) = &after.last
        && after.last != before.last
        && last.band > Band::Healthy
    {
        let free = after
            .filesystems
            .iter()
            .find(|filesystem| filesystem.path == last.path)
            .map_or_else(
                || "Some space".to_owned(),
                |filesystem| shown(filesystem.free),
            );
        let place = last.path.display();
        let freed = if last.freed == 0 {
            format!("Nothing freed on {place}")
        } else {
            format!("Freed {} on {place}", shown(last.freed))
        };
        let short = format!("{freed}, short of room");
        // Plain text: a notification server may show markup as it's written.
        let (summary, body) = match last.end.as_str() {
            "reached" => (freed, format!("{free} free now.")),
            "exhausted" => (
                short,
                format!(
                    "{free} free. Nothing more is safe to take; disk-guard plan shows what's left."
                ),
            ),
            "stalled" => (
                short,
                format!(
                    "{free} free. Deleting stopped freeing space, so the guard waits before \
                     trying again."
                ),
            ),
            failed => (short, format!("{free} free. It {failed}.")),
        };
        // Left short, whoever is filling it is worth knowing.
        let body = match &after.writer {
            Some(writer) if last.end != "reached" => format!(
                "{body} Most of the writing is {}'s (pid {}), at {} a second.",
                writer.name,
                writer.pid,
                shown(writer.rate as u64)
            ),
            _ => body,
        };
        return Some(Change::Show(Notice {
            summary,
            body,
            urgency: urgency(last.band),
        }));
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
        .filter(|filesystem| filesystem.band > was(filesystem))
        .max_by_key(|filesystem| filesystem.band);
    if let Some(filesystem) = entered {
        let state = match filesystem.band {
            Band::Healthy | Band::Pressure => "under pressure",
            Band::Urgent => "nearly full",
            Band::Critical => "critically full",
        };
        return Some(Change::Show(Notice {
            summary: format!("{} is {state}", filesystem.path.display()),
            body: format!(
                "{} free of {}. Making room.",
                shown(filesystem.free),
                shown(filesystem.total)
            ),
            urgency: urgency(filesystem.band),
        }));
    }
    let pressed = |status: &Status| {
        status
            .filesystems
            .iter()
            .any(|filesystem| filesystem.band > Band::Healthy)
    };
    (pressed(before) && !pressed(after)).then_some(Change::Withdraw)
}

fn urgency(band: Band) -> Urgency {
    if band == Band::Critical {
        Urgency::Critical
    } else {
        Urgency::Normal
    }
}
