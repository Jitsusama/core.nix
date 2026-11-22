//! Gives each slice nested right inside agents.slice the floors of the
//! units ranked in it, so those floors count.
//!
//! Background's pi host and its conversations run in
//! agents-background.slice, inside agents.slice (modules/pi/host.nix),
//! which sets no floor of its own. Under the kernel's
//! `memory_recursiveprot`, a unit's floor counts only as far as each slice
//! above it passes protection down, and a slice with none passes down
//! only what its parent's other children leave unclaimed. On 21 November
//! agents.slice's children claimed 8.3 GB of its 6.7 GB floor, leaving
//! none: the 1 GiB floor each of eight background orchestrators kept, or
//! the 2 GiB of one Joel is at, would have protected nothing there. So
//! each nested slice is set to the sum of its units' floors, which
//! protects them as if they sat in agents.slice itself.
//!
//! Only soft floors pass up: a hard one goes only to a unit right in
//! agents.slice ([`holds::in_agents`]), which agents.slice's own hard
//! floor is sized for.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::holds;

/// The floor each of `slices` nested right inside agents.slice should
/// keep, by its unit's name: the sum of the floors of the `ranked` units,
/// each as its cgroup's path and floor, sitting right in it. A slice with
/// none ranked in it keeps none, so a floor set before is given back.
pub fn floors(slices: &[PathBuf], ranked: &[(PathBuf, u64)]) -> Vec<(String, u64)> {
    slices
        .iter()
        .filter(|slice| holds::in_agents(slice))
        .filter_map(|slice| {
            let name = slice.file_name()?.to_str()?.to_owned();
            let floor = ranked
                .iter()
                .filter(|(unit, _)| unit.parent() == Some(slice.as_path()))
                .fold(0, |sum: u64, (_, floor)| sum.saturating_add(*floor));
            Some((name, floor))
        })
        .collect()
}

/// What busctl is given to set a nested slice's floor, as a runtime
/// property gone at the next boot.
pub fn floor_call(slice: &str, floor: u64) -> Vec<String> {
    let floor = floor.to_string();
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        slice,
        "true",
        "1",
        "MemoryLow",
        "t",
        &floor,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// Each nested slice's floor as last set, so only a change is asked for.
#[derive(Default)]
pub struct Nested {
    set: HashMap<String, u64>,
    /// Whether a failure to set a nested slice's floor has been said.
    pub failed: bool,
}

impl Nested {
    /// Whether `slice` should be set to `floor`: it changed, or this
    /// sorter never set it, as one before it may have left another.
    pub fn changed(&self, slice: &str, floor: u64) -> bool {
        self.set.get(slice) != Some(&floor)
    }

    /// Notes a slice as set to `floor`.
    pub fn done(&mut self, slice: &str, floor: u64) {
        self.set.insert(slice.to_owned(), floor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn path(cgroup: &str) -> PathBuf {
        PathBuf::from(cgroup)
    }

    #[test]
    fn a_nested_slice_keeps_the_floors_of_the_units_right_in_it() {
        let slices = [
            path("/cg/agents.slice"),
            path("/cg/agents.slice/agents-background.slice"),
            path("/cg/app.slice"),
        ];
        let ranked = [
            (
                path("/cg/agents.slice/agents-background.slice/pi-conversation-w@a.service"),
                GIB,
            ),
            (
                path("/cg/agents.slice/agents-background.slice/pi-host-w-background.service"),
                2 * GIB,
            ),
            (path("/cg/agents.slice/pi-host-w-attended.service"), GIB / 2),
            (path("/cg/app.slice/pi-host-w-attended.service"), GIB),
        ];
        assert_eq!(
            floors(&slices, &ranked),
            [("agents-background.slice".to_owned(), 3 * GIB)],
            "background keeps its two units' floors; agents.slice and app.slice, not nested in \
             agents.slice, are left as their own settings say"
        );
    }

    #[test]
    fn a_nested_slice_with_no_unit_ranked_in_it_gives_its_floor_back() {
        let slices = [path("/cg/agents.slice/agents-background.slice")];
        assert_eq!(
            floors(&slices, &[]),
            [("agents-background.slice".to_owned(), 0)],
            "with its units gone, background's floor goes back to none"
        );
    }

    #[test]
    fn a_nested_slice_is_set_once_until_its_floor_changes() {
        let mut nested = Nested::default();
        assert!(
            nested.changed("agents-background.slice", 0),
            "a slice this sorter never set is set, as one before it may have left a floor"
        );
        nested.done("agents-background.slice", GIB);
        assert!(
            !nested.changed("agents-background.slice", GIB),
            "the same floor isn't asked for again"
        );
        assert!(
            nested.changed("agents-background.slice", 2 * GIB),
            "a new floor is"
        );
    }

    #[test]
    fn a_nested_slices_floor_is_set_at_runtime_alone() {
        assert_eq!(
            floor_call("agents-background.slice", GIB),
            [
                "--user",
                "--quiet",
                "call",
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "SetUnitProperties",
                "sba(sv)",
                "agents-background.slice",
                "true",
                "1",
                "MemoryLow",
                "t",
                "1073741824",
            ],
            "only MemoryLow is set, and only until the next boot"
        );
    }
}
