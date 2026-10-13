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
        let freed = shown(last.freed);
        let (summary, body) = match last.end.as_str() {
            "reached" => (
                format!("Freed {freed} on {place}"),
                format!("{free} free now."),
            ),
            "exhausted" => (
                format!("Freed {freed} on {place}, short of room"),
                format!(
                    "{free} free. Nothing more is safe to take: `disk-guard plan` shows \
                     what's left."
                ),
            ),
            "stalled" => (
                format!("Freed {freed} on {place}, short of room"),
                format!(
                    "{free} free. Deleting stopped freeing space, so the guard waits before \
                     trying again."
                ),
            ),
            failed => (
                format!("Freed {freed} on {place}, short of room"),
                format!("{free} free. It {failed}."),
            ),
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
