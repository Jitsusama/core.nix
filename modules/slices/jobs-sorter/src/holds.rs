//! What a window's unit is set to as Joel is at it or was, and which
//! scopes are set, so each is set once and given back once it ages out:
//! the user manager's runtime properties, through busctl. Which windows
//! those are is `src/focus.rs`'s to say.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::focus::FLOOR;

/// What busctl is given to hold a window's unit with [`FLOOR`], a `hard`
/// floor ([`hard_for`]), no zswap writeback and systemd-oomd's avoid
/// preference, or, with no `hard`, to give all four back, as runtime
/// properties gone at the next boot. One given back while it's `kept`
/// keeps writeback off ([`crate::focus::KEEP`]).
pub fn hold_call(unit: &str, hard: Option<u64>, kept: bool) -> Vec<String> {
    let (floor, writeback, preference, hard) = match hard {
        Some(hard) => (FLOOR.to_string(), "false", "avoid", hard.to_string()),
        None if kept => ("0".to_owned(), "false", "none", "0".to_owned()),
        None => ("0".to_owned(), "true", "none", "0".to_owned()),
    };
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "4",
        "MemoryLow",
        "t",
        &floor,
        "MemoryZSwapWriteback",
        "b",
        writeback,
        "ManagedOOMPreference",
        "s",
        preference,
        "MemoryMin",
        "t",
        &hard,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// What a held window keeps hard, once, where its conversation's memory
/// is: a floor the kernel never reclaims below, where [`FLOOR`] gives way
/// under the pressure that matters. On 21 November a pi's window with a
/// soft floor and no hard one had the kernel reclaim below its floor
/// 3,610,883 times. Set whole, not to what the unit holds as it's held,
/// so a window just opened or come back to grows into it; the kernel
/// counts a floor past what a unit holds only as far as it holds.
pub const HARD: u64 = 512 << 20;

// The at most `MOST` held windows' hard floors fit inside agents.slice's own beside the sessions'
// kitty's 512M (agentsHeld in modules/slices/nixos.nix), or the kernel shares it among them all.
const _: () = assert!(
    crate::focus::MOST as u64 * HARD + (512 << 20) <= 2048 << 20,
    "the held windows' hard floors outgrow agents.slice's"
);

/// The hard floor a window Joel is at keeps on `unit`: [`HARD`] on its
/// pi's scope, where its conversation's memory is, and none on its own
/// scope, which holds a shell or a viewer and may sit in app.slice, whose
/// floor is Joel's programs'. A viewer's conversation is held hard on its
/// own unit instead (`src/hosts.rs`).
pub fn hard_for(unit: &str, pis: &[String]) -> u64 {
    if crate::focus::runs_a_pi(unit, pis) {
        HARD
    } else {
        0
    }
}

/// What the cgroup at `path` holds now, from its memory.current.
pub fn current(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path.join("memory.current"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// What busctl is given to keep a window's unit's paged-out memory
/// compressed in memory, with no zswap writeback, or to give that back,
/// as a runtime property gone at the next boot ([`crate::focus::KEEP`]).
pub fn keep_call(unit: &str, keep: bool) -> Vec<String> {
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "1",
        "MemoryZSwapWriteback",
        "b",
        if keep { "false" } else { "true" },
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// The scopes held, each by its path, so each is set once and given back
/// once it ages out.
#[derive(Debug, Default)]
pub struct Holding {
    held: HashSet<PathBuf>,
    /// The holds taken up from a sorter before this one, set again the
    /// first time each is wanted, as that sorter may have set fewer
    /// properties than [`hold_call`] does now.
    again: HashSet<PathBuf>,
    /// Whether a failure to hold a window has been said.
    pub failed: bool,
}

impl Holding {
    /// The scopes a sorter before this one held, as their floor of
    /// exactly [`FLOOR`] says, so the first look gives back those not
    /// wanted. A hold is the user manager's, set at runtime, and outlives
    /// the sorter that set it: otherwise each restart would leave up to
    /// [`crate::focus::MOST`] stale floors sharing agents.slice's own until their windows
    /// ended. An orchestrator's floor is smaller, and `src/keep.rs`'s.
    /// Those still wanted are held again, once: on 11 November a switch
    /// left three windows with their floor but without writeback held
    /// off, which the sorter before had never set.
    pub fn left(
        scopes: impl IntoIterator<Item = PathBuf>,
        floor: impl Fn(&Path) -> Option<u64>,
    ) -> Self {
        Self::taken_up(scopes, |scope| floor(scope) == Some(FLOOR))
    }

    /// The scopes a sorter before this one set as `set` says, to give
    /// back those not wanted, and set again once those that are.
    pub fn taken_up(
        scopes: impl IntoIterator<Item = PathBuf>,
        set: impl Fn(&Path) -> bool,
    ) -> Self {
        let held: HashSet<PathBuf> = scopes.into_iter().filter(|scope| set(scope)).collect();
        Self {
            again: held.clone(),
            held,
            failed: false,
        }
    }

    /// The scopes to change, each with whether to hold it, from those to
    /// hold now.
    pub fn changes(&self, wanted: &[PathBuf]) -> Vec<(PathBuf, bool)> {
        let hold = wanted
            .iter()
            .filter(|scope| !self.held.contains(*scope) || self.again.contains(*scope))
            .map(|scope| (scope.clone(), true));
        let give_back = self
            .held
            .iter()
            .filter(|scope| !wanted.contains(scope))
            .map(|scope| (scope.clone(), false));
        hold.chain(give_back).collect()
    }

    /// Notes a scope as held or given back.
    pub fn done(&mut self, scope: &Path, hold: bool) {
        self.again.remove(scope);
        if hold {
            self.held.insert(scope.to_path_buf());
        } else {
            self.held.remove(scope);
        }
    }

    /// Whether a scope's unit is held.
    pub fn holds(&self, unit: &str) -> bool {
        self.held
            .iter()
            .any(|scope| scope.file_name().is_some_and(|name| name == unit))
    }

    /// Forgets the scopes that have gone.
    pub fn keep_only(&mut self, there: impl Fn(&Path) -> bool) {
        self.held.retain(|scope| there(scope));
        self.again.retain(|scope| there(scope));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_is_held_with_a_floor_in_memory_and_avoided_or_given_all_back() {
        let held = hold_call("kitty-1-9.scope", Some(300 << 20), true);
        assert_eq!(
            held[8..],
            [
                "kitty-1-9.scope",
                "true",
                "4",
                "MemoryLow",
                "t",
                "2147483648",
                "MemoryZSwapWriteback",
                "b",
                "false",
                "ManagedOOMPreference",
                "s",
                "avoid",
                "MemoryMin",
                "t",
                "314572800"
            ]
        );
        let back = hold_call("kitty-1-9.scope", None, false);
        assert_eq!(back[13], "0");
        assert_eq!(back[16], "true");
        assert_eq!(back[19], "none");
        assert_eq!(back[22], "0", "the hard floor is given back with the rest");
        let still_kept = hold_call("kitty-1-9.scope", None, true);
        assert_eq!(
            (still_kept[13].as_str(), still_kept[16].as_str()),
            ("0", "false"),
            "one given back while kept keeps its memory off the disk"
        );
    }

    #[test]
    fn a_held_window_keeps_its_hard_floor_on_its_pi_alone() {
        let pis = ["mycelium-pi-".to_owned()];
        assert_eq!(
            hard_for("mycelium-pi-0193.scope", &pis),
            HARD,
            "its pi's scope, holding the conversation, keeps it"
        );
        assert_eq!(
            hard_for("app-kitty-2684.scope", &pis),
            0,
            "its own scope, a shell or a viewer, keeps none"
        );
    }

    #[test]
    fn a_kept_window_has_writeback_alone_set_or_given_back() {
        let kept = keep_call("kitty-1-9.scope", true);
        assert_eq!(
            kept[8..],
            [
                "kitty-1-9.scope",
                "true",
                "1",
                "MemoryZSwapWriteback",
                "b",
                "false"
            ]
        );
        assert_eq!(keep_call("kitty-1-9.scope", false)[13], "true");
    }

    #[test]
    fn a_scope_is_held_once_and_given_back_once_it_ages_out() {
        let a = PathBuf::from("/a/kitty-1-1.scope");
        let b = PathBuf::from("/a/kitty-1-2.scope");
        let mut holding = Holding::default();
        let only = std::slice::from_ref;
        assert_eq!(holding.changes(only(&a)), [(a.clone(), true)]);
        holding.done(&a, true);
        assert!(holding.holds("kitty-1-1.scope"), "held once done");
        assert_eq!(holding.changes(only(&a)), []);
        assert_eq!(
            holding.changes(only(&b)),
            [(b.clone(), true), (a.clone(), false)]
        );
        holding.done(&a, false);
        assert!(!holding.holds("kitty-1-1.scope"), "given back once done");
        holding.done(&b, true);
        holding.keep_only(|_| false);
        assert_eq!(
            holding.changes(&[]),
            [],
            "a scope gone is nobody's to give back"
        );
    }

    #[test]
    fn a_sorter_before_this_ones_holds_are_taken_up_so_they_are_given_back() {
        let held = PathBuf::from("kitty-1-1.scope");
        let kept = PathBuf::from("kitty-1-2.scope");
        let bare = PathBuf::from("kitty-1-3.scope");
        let floors = |scope: &Path| match scope.to_str() {
            Some("kitty-1-1.scope") => Some(FLOOR),
            Some("kitty-1-2.scope") => Some(1 << 30),
            _ => Some(0),
        };
        let holding = Holding::left([held.clone(), kept, bare], floors);
        assert!(holding.holds("kitty-1-1.scope"), "a held window's floor");
        assert!(
            !holding.holds("kitty-1-2.scope"),
            "an orchestrator's floor is keep.rs's"
        );
        assert_eq!(holding.changes(&[]), [(held, false)]);
    }

    #[test]
    fn a_hold_taken_up_and_still_wanted_is_set_again_once() {
        let held = PathBuf::from("kitty-1-1.scope");
        let mut holding = Holding::left([held.clone()], |_| Some(FLOOR));
        let wanted = [held.clone()];
        assert_eq!(
            holding.changes(&wanted),
            [(held.clone(), true)],
            "set again with every property this sorter sets"
        );
        holding.done(&held, true);
        assert_eq!(holding.changes(&wanted), [], "once is enough");
    }
}
