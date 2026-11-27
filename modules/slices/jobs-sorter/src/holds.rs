//! What a window's unit is set to as Joel is at it or was, and which
//! scopes are set, so each is set once and given back once it ages out:
//! the user manager's runtime properties, through busctl. Which windows
//! those are is `src/focus.rs`'s to say.

use std::collections::{HashMap, HashSet};
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
    "the held windows' hard floors outgrow agents.slice's: raise agentsHeld and the kitty check too"
);

/// The hard floor a window Joel is at keeps on `unit`: [`HARD`] on its
/// pi's scope, where its conversation's memory is, and none on its own
/// scope, which holds a shell or a viewer and may sit in app.slice, whose
/// floor is Joel's programs'. A viewer's conversation is held hard on its
/// own unit instead (`src/hosts.rs`).
pub fn hard_for(scope: &Path, pis: &[String]) -> u64 {
    let unit = scope
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if crate::focus::runs_a_pi(unit, pis) && in_agents(scope) {
        HARD
    } else {
        0
    }
}

/// How many of the units windows Joel is at show, `viewers`, one for each
/// viewer, keep a hard floor: of the at most [`crate::focus::MOST`] held
/// windows' hard floors, what the `pis` held hard leave, one to each
/// viewer as listed: the window Joel is at first, then those he left
/// latest. A window can hold several viewers, so counting each unit's
/// alone would let them claim past agents.slice's.
pub fn shares(pis: usize, viewers: &[String]) -> HashMap<String, usize> {
    let mut given = HashMap::new();
    for unit in viewers.iter().take(crate::focus::MOST.saturating_sub(pis)) {
        *given.entry(unit.clone()).or_insert(0) += 1;
    }
    given
}

/// Whether the cgroup at `path` sits right in agents.slice, whose own
/// hard floor backs what this sorter holds hard; one in app.slice would
/// take it from Joel's programs'.
pub fn in_agents(path: &Path) -> bool {
    path.parent().and_then(Path::file_name) == Some("agents.slice".as_ref())
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

/// Where zswap says whether it's on, the file hibernation's unlock turns
/// it off and on by.
const ZSWAP_ON: &str = "/sys/module/zswap/parameters/enabled";

/// The cgroups' root, which has memory.zswap.writeback too.
const ROOT: &str = "/sys/fs/cgroup";

/// The nearest cgroup above `scope` that keeps zswap writeback off for
/// it, read through `read` as `fs::read_to_string` reads a file. While
/// zswap is on the kernel reads memory.zswap.writeback up the tree, the
/// root's among them, so under one a scope given back still keeps its
/// paged-out memory off the disk. While zswap is off the kernel lets that
/// memory go to the disk whatever the tree says, so none keeps it then.
/// The walk ends where the files do, above the root.
pub fn kept_above(scope: &Path, read: impl Fn(&Path) -> Option<String>) -> Option<PathBuf> {
    if read(Path::new(ZSWAP_ON)).is_none_or(|on| on.trim() != "Y") {
        return None;
    }
    scope
        .ancestors()
        .skip(1)
        .map_while(|cgroup| Some((cgroup, read(&cgroup.join("memory.zswap.writeback"))?)))
        .find(|(_, value)| value.trim() == "0")
        .map(|(cgroup, _)| cgroup.to_path_buf())
}

/// What the journal says of `unit` once it's kept in memory or given back.
/// `above` is asked only on a give-back, the one case a cgroup above can
/// still keep the window's memory off the disk ([`kept_above`]).
pub fn kept_said(unit: &str, keep: bool, above: impl FnOnce() -> Option<PathBuf>) -> String {
    if keep {
        return format!("Keeping in memory {unit}, a window Joel was at within two hours.");
    }
    match above() {
        None => format!("Let go to disk {unit}, a window Joel was at within two hours."),
        Some(cgroup) => {
            let keeper = if cgroup == Path::new(ROOT) {
                "the root cgroup".to_owned()
            } else {
                cgroup
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            };
            format!(
                "Let {unit} go, a window Joel was at within two hours, though {keeper} \
                 still keeps it off the disk."
            )
        }
    }
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
    /// The scopes held with a hard floor, or taken up from a sorter
    /// before this one with one: what each was held with, as a pi held
    /// past [`crate::focus::MOST`] gets none.
    hard: HashSet<PathBuf>,
    /// The scopes in `hard` only as their memory.min couldn't be read:
    /// each counts against the others, but keeps no floor of its own
    /// until it's held again.
    unread: HashSet<PathBuf>,
    /// The hard floors hosts keep only as their give-back failed
    /// ([`crate::hosts::Hosts::hard_left`]), which count against the pis'.
    pub elsewhere: usize,
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
    /// off, which the sorter before had never set. Of those, the ones
    /// `hard` says keep a hard floor count as keeping one: counting each
    /// window's own scope too took the hard floor off the pi of the window
    /// Joel is at for a look after each restart, as three others seemed to
    /// keep theirs. So only a scope this sorter would hold hard counts, a
    /// window's own never, whatever memory.min it reads; and one whose
    /// memory.min can't be read counts, as a failed give-back does, so
    /// no floor past the three is given while it might still be set. It
    /// counts for the others only: it's given a floor of its own only
    /// while fewer than three others keep theirs, or a fourth would be.
    pub fn left(
        scopes: impl IntoIterator<Item = PathBuf>,
        floor: impl Fn(&Path) -> Option<u64>,
        min: impl Fn(&Path) -> Option<u64>,
        pis: &[String],
    ) -> Self {
        let mut holding = Self::taken_up(scopes, |scope| floor(scope) == Some(FLOOR));
        // Each memory.min is read once, so a scope can't be both kept hard and read keeping none.
        for scope in holding.held.iter().filter(|scope| hard_for(scope, pis) > 0) {
            match min(scope) {
                Some(0) => {}
                Some(_) => {
                    holding.hard.insert(scope.clone());
                }
                None => {
                    holding.hard.insert(scope.clone());
                    holding.unread.insert(scope.clone());
                }
            }
        }
        holding
    }

    /// How many scopes are held, taken up or not.
    pub fn held(&self) -> usize {
        self.held.len()
    }

    /// The scopes held, taken up or not.
    pub fn scopes(&self) -> &HashSet<PathBuf> {
        &self.held
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
            hard: HashSet::new(),
            unread: HashSet::new(),
            elsewhere: 0,
            held,
            failed: false,
        }
    }

    /// The scopes to change, each with whether to hold it, from those to
    /// hold now: those to give back first, so a hard floor given back is
    /// free for a hold after it. A pi held without the hard floor it
    /// would have is held again once one is free.
    pub fn changes(&self, wanted: &[PathBuf], pis: &[String]) -> Vec<(PathBuf, bool)> {
        let free = self.hard.len() + self.elsewhere < crate::focus::MOST;
        let hold = wanted
            .iter()
            .filter(|scope| {
                !self.held.contains(*scope)
                    || self.again.contains(*scope)
                    || (free && !self.hard.contains(*scope) && hard_for(scope, pis) > 0)
            })
            .map(|scope| (scope.clone(), true));
        let give_back = self
            .held
            .iter()
            .filter(|scope| !wanted.contains(scope))
            .map(|scope| (scope.clone(), false));
        give_back.chain(hold).collect()
    }

    /// Notes a scope as held, with the hard floor it was held with, or
    /// given back.
    pub fn done(&mut self, scope: &Path, held: Option<u64>) {
        self.again.remove(scope);
        self.unread.remove(scope);
        if held.is_some() {
            self.held.insert(scope.to_path_buf());
        } else {
            self.held.remove(scope);
        }
        if held.is_some_and(|hard| hard > 0) {
            self.hard.insert(scope.to_path_buf());
        } else {
            self.hard.remove(scope);
        }
    }

    /// Whether a scope's unit is held.
    pub fn holds(&self, unit: &str) -> bool {
        self.held
            .iter()
            .any(|scope| scope.file_name().is_some_and(|name| name == unit))
    }

    /// How many of the scopes held keep a hard floor, whether wanted now
    /// or still held as a give-back fails.
    pub fn hard(&self) -> usize {
        self.hard.len()
    }

    /// The hard floor to hold `scope` with: [`hard_for`]'s while fewer
    /// than [`crate::focus::MOST`] other scopes held keep one, as a
    /// give-back that fails still does, and none past them. One that keeps
    /// a hard floor already keeps it: the kernel holds it whatever this
    /// says, and taking it off while a stale one's give-back fails took the
    /// floor from the window Joel is at and left it on those he'd left. So
    /// while one fails, more than `MOST` may hold one, past what the
    /// assertion over agents.slice's floor allows, and the kernel shares
    /// that floor among them until the give-back lands: meant, and brief.
    /// One whose memory.min couldn't be read keeps none it isn't known to
    /// have, so it's given one only as any other pi is.
    pub fn hard_floor(&self, scope: &Path, pis: &[String]) -> u64 {
        let others = self.hard.iter().filter(|held| *held != scope).count() + self.elsewhere;
        let keeps = self.hard.contains(scope) && !self.unread.contains(scope);
        if keeps || others < crate::focus::MOST {
            hard_for(scope, pis)
        } else {
            0
        }
    }

    /// Forgets the scopes that have gone.
    pub fn keep_only(&mut self, there: impl Fn(&Path) -> bool) {
        self.held.retain(|scope| there(scope));
        self.again.retain(|scope| there(scope));
        self.hard.retain(|scope| there(scope));
        self.unread.retain(|scope| there(scope));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads zswap's state as `zswap` gives it, and memory.zswap.writeback
    /// as `tree` gives it, by each cgroup's path; any other has no file.
    fn writeback<'a>(
        zswap: &'a str,
        tree: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&Path) -> Option<String> + 'a {
        move |file| {
            if file == Path::new(ZSWAP_ON) {
                return Some(format!("{zswap}\n"));
            }
            tree.iter()
                .find(|(cgroup, _)| Path::new(cgroup).join("memory.zswap.writeback") == file)
                .map(|(_, value)| format!("{value}\n"))
        }
    }

    #[test]
    fn a_slice_above_that_keeps_writeback_off_is_named() {
        let tree = [
            ("/sys/fs/cgroup", "1"),
            ("/sys/fs/cgroup/user.slice", "1"),
            ("/sys/fs/cgroup/user.slice/app.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope", "1"),
        ];
        assert_eq!(
            kept_above(
                Path::new("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope"),
                writeback("Y", &tree)
            ),
            Some(PathBuf::from("/sys/fs/cgroup/user.slice/app.slice"))
        );
    }

    #[test]
    fn a_scope_whose_own_writeback_alone_is_off_has_nothing_above() {
        let tree = [
            ("/sys/fs/cgroup", "1"),
            ("/sys/fs/cgroup/user.slice", "1"),
            ("/sys/fs/cgroup/user.slice/session.slice", "1"),
            (
                "/sys/fs/cgroup/user.slice/session.slice/kitty-1-9.scope",
                "0",
            ),
        ];
        assert_eq!(
            kept_above(
                Path::new("/sys/fs/cgroup/user.slice/session.slice/kitty-1-9.scope"),
                writeback("Y", &tree)
            ),
            None
        );
    }

    #[test]
    fn a_slice_further_up_keeps_writeback_off_past_one_that_lets_it() {
        let tree = [
            ("/sys/fs/cgroup", "1"),
            ("/sys/fs/cgroup/user.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice", "1"),
            ("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope", "1"),
        ];
        assert_eq!(
            kept_above(
                Path::new("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope"),
                writeback("Y", &tree)
            ),
            Some(PathBuf::from("/sys/fs/cgroup/user.slice"))
        );
    }

    #[test]
    fn the_nearest_of_two_slices_keeping_writeback_off_is_named() {
        let tree = [
            ("/sys/fs/cgroup", "1"),
            ("/sys/fs/cgroup/user.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope", "1"),
        ];
        assert_eq!(
            kept_above(
                Path::new("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope"),
                writeback("Y", &tree)
            ),
            Some(PathBuf::from("/sys/fs/cgroup/user.slice/app.slice"))
        );
    }

    #[test]
    fn the_roots_own_setting_is_read_too() {
        let tree = [
            ("/sys/fs/cgroup", "0"),
            ("/sys/fs/cgroup/user.slice", "1"),
            ("/sys/fs/cgroup/user.slice/app.slice", "1"),
            ("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope", "1"),
        ];
        assert_eq!(
            kept_above(
                Path::new("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope"),
                writeback("Y", &tree)
            ),
            Some(PathBuf::from(ROOT))
        );
    }

    #[test]
    fn with_zswap_off_no_slice_keeps_a_window_off_the_disk() {
        let tree = [
            ("/sys/fs/cgroup", "0"),
            ("/sys/fs/cgroup/user.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice", "0"),
            ("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope", "1"),
        ];
        let scope = Path::new("/sys/fs/cgroup/user.slice/app.slice/kitty-1-9.scope");
        assert_eq!(kept_above(scope, writeback("N", &tree)), None);
        let unread = |file: &Path| (file != Path::new(ZSWAP_ON)).then(|| "0\n".to_owned());
        assert_eq!(kept_above(scope, unread), None, "zswap's state unread");
    }

    #[test]
    fn a_keep_is_said_without_reading_the_tree() {
        assert_eq!(
            kept_said("kitty-1-9.scope", true, || panic!("the tree was read")),
            "Keeping in memory kitty-1-9.scope, a window Joel was at within two hours."
        );
    }

    #[test]
    fn a_give_back_says_whether_a_cgroup_above_still_keeps_it() {
        assert_eq!(
            kept_said("kitty-1-9.scope", false, || None),
            "Let go to disk kitty-1-9.scope, a window Joel was at within two hours."
        );
        assert_eq!(
            kept_said("kitty-1-9.scope", false, || {
                Some(PathBuf::from("/sys/fs/cgroup/user.slice/app.slice"))
            }),
            "Let kitty-1-9.scope go, a window Joel was at within two hours, though \
             app.slice still keeps it off the disk."
        );
        assert_eq!(
            kept_said("kitty-1-9.scope", false, || Some(PathBuf::from(ROOT))),
            "Let kitty-1-9.scope go, a window Joel was at within two hours, though \
             the root cgroup still keeps it off the disk."
        );
    }

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
        let hard = |scope: &str| hard_for(Path::new(scope), &pis);
        assert_eq!(
            hard("/cg/agents.slice/mycelium-pi-0193.scope"),
            HARD,
            "its pi's scope, holding the conversation, keeps it"
        );
        assert_eq!(
            hard("/cg/app.slice/mycelium-pi-0193.scope"),
            0,
            "a pi in app.slice keeps none, as that floor is Joel's programs'"
        );
        assert_eq!(
            hard("/cg/app.slice/app-kitty-2684.scope"),
            0,
            "its own scope, a shell or a viewer, keeps none"
        );
        assert_eq!(
            hard("/cg/agents.slice/kitty-1-9.scope"),
            0,
            "nor does a session window's own scope, in agents.slice beside its pi"
        );
    }

    #[test]
    fn the_held_windows_keep_at_most_their_hard_floors_between_them() {
        let viewers: Vec<String> = ["b", "a", "a", "c"].map(str::to_owned).to_vec();
        let given = shares(1, &viewers);
        assert_eq!(
            given.values().sum::<usize>(),
            2,
            "a pi held hard leaves two for the units the windows show, however many viewers"
        );
        assert_eq!(
            (given.get("b"), given.get("a")),
            (Some(&1), Some(&1)),
            "the first viewers, of the window Joel is at and then those left latest, get theirs"
        );
        assert!(
            shares(3, &viewers).is_empty() && shares(4, &viewers).is_empty(),
            "three pis held hard leave none, and a fourth, as a give-back fails, takes none away"
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
        assert_eq!(holding.changes(only(&a), &[]), [(a.clone(), true)]);
        holding.done(&a, Some(0));
        assert!(holding.holds("kitty-1-1.scope"), "held once done");
        assert_eq!(holding.changes(only(&a), &[]), []);
        assert_eq!(
            holding.changes(only(&b), &[]),
            [(a.clone(), false), (b.clone(), true)],
            "a give-back goes first, so its hard floor is free for the next hold"
        );
        holding.done(&a, None);
        assert!(!holding.holds("kitty-1-1.scope"), "given back once done");
        holding.done(&b, Some(0));
        holding.keep_only(|_| false);
        assert_eq!(
            holding.changes(&[], &[]),
            [],
            "a scope gone is nobody's to give back"
        );
    }

    #[test]
    fn a_pi_held_hard_counts_until_it_is_given_back() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = PathBuf::from("/cg/agents.slice/mycelium-pi-1.scope");
        let mut holding = Holding::default();
        holding.done(&pi, Some(HARD));
        holding.done(Path::new("/cg/app.slice/app-kitty-1.scope"), Some(0));
        assert_eq!(
            holding.changes(&[], &pis).len(),
            2,
            "both are offered for giving back"
        );
        assert_eq!(
            holding.hard(),
            1,
            "a pi held hard counts, its window's own scope not, while its give-back fails"
        );
        holding.done(&pi, None);
        assert_eq!(holding.hard(), 0, "given back, it counts no more");
    }

    #[test]
    fn a_new_pi_is_held_hard_only_while_fewer_than_most_others_are() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        let mut holding = Holding::default();
        for n in 1..=3 {
            holding.done(&pi(n), Some(HARD));
        }
        assert_eq!(
            holding.hard_floor(&pi(4), &pis),
            0,
            "three pis still held hard, as a give-back fails, leave a fourth none"
        );
        assert_eq!(
            holding.hard_floor(&pi(1), &pis),
            HARD,
            "a pi held again keeps its own"
        );
        holding.done(&pi(4), Some(0));
        assert_eq!(
            holding.hard(),
            3,
            "the fourth, held with none, isn't counted as keeping one"
        );
        assert_eq!(
            holding.changes(&[pi(1), pi(2), pi(3), pi(4)], &pis),
            [],
            "the fourth isn't offered again while the three keep theirs, or it'd be held each look"
        );
        holding.done(&pi(1), None);
        assert_eq!(
            holding.changes(&[pi(2), pi(3), pi(4)], &pis),
            [(pi(4), true)],
            "the fourth is held again once a floor is free"
        );
        assert_eq!(
            holding.hard_floor(&pi(4), &pis),
            HARD,
            "one given back leaves its floor to the next"
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
        let holding = Holding::left([held.clone(), kept, bare], floors, |_| Some(0), &[]);
        assert!(holding.holds("kitty-1-1.scope"), "a held window's floor");
        assert!(
            !holding.holds("kitty-1-2.scope"),
            "an orchestrator's floor is keep.rs's"
        );
        assert_eq!(holding.changes(&[], &[]), [(held, false)]);
    }

    #[test]
    fn a_hold_taken_up_and_still_wanted_is_set_again_once() {
        let held = PathBuf::from("kitty-1-1.scope");
        let mut holding = Holding::left([held.clone()], |_| Some(FLOOR), |_| Some(0), &[]);
        let wanted = [held.clone()];
        assert_eq!(
            holding.changes(&wanted, &[]),
            [(held.clone(), true)],
            "set again with every property this sorter sets"
        );
        holding.done(&held, Some(0));
        assert_eq!(holding.changes(&wanted, &[]), [], "once is enough");
    }

    #[test]
    fn after_a_restart_the_pi_of_the_window_joel_is_at_keeps_its_hard_floor() {
        let pis = ["mycelium-pi-".to_owned()];
        let own = |n: u8| PathBuf::from(format!("/cg/agents.slice/kitty-{n}-9.scope"));
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        // As focus::held_and_kept lists them: each window's own scope, then its pi, the window
        // Joel is at first.
        let wanted = [own(1), pi(1), own(2), pi(2), own(3), pi(3)];
        // Every scope reads a memory.min, as a window's own would if anything had set one there.
        let mut holding = Holding::left(wanted.clone(), |_| Some(FLOOR), |_| Some(HARD), &pis);
        assert_eq!(
            holding.hard(),
            3,
            "only the pis count as keeping a hard floor, not a window's own scope with a \
             memory.min left on it"
        );
        let mut given = Vec::new();
        for (scope, hold) in holding.changes(&wanted, &pis) {
            assert!(hold, "every window is still wanted");
            let hard = holding.hard_floor(&scope, &pis);
            holding.done(&scope, Some(hard));
            given.push((scope, hard));
        }
        assert_eq!(
            given,
            [
                (own(1), 0),
                (pi(1), HARD),
                (own(2), 0),
                (pi(2), HARD),
                (own(3), 0),
                (pi(3), HARD)
            ],
            "each pi is held again with its hard floor"
        );
        assert_eq!(
            holding.changes(&wanted, &pis),
            [],
            "and nothing is offered again on the next look"
        );
    }

    #[test]
    fn a_taken_up_pi_whose_memory_min_cant_be_read_counts_as_keeping_its_floor() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        let holding = Holding::left([pi(1), pi(2), pi(3)], |_| Some(FLOOR), |_| None, &pis);
        assert_eq!(
            holding.hard(),
            3,
            "a floor that can't be read is counted, as a failed give-back's is, so none past \
             the three is given"
        );
        assert_eq!(holding.hard_floor(&pi(4), &pis), 0, "a fourth pi gets none");
    }

    #[test]
    fn a_taken_up_pi_whose_memory_min_cant_be_read_gets_no_floor_past_three() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        let unread = pi(4);
        let min = |scope: &Path| (scope != unread).then_some(HARD);
        let mut holding = Holding::left([pi(1), pi(2), pi(3), pi(4)], |_| Some(FLOOR), min, &pis);
        assert_eq!(
            holding.hard_floor(&pi(4), &pis),
            0,
            "the pi whose floor can't be read gets none of its own while three others keep \
             theirs"
        );
        assert_eq!(
            holding.hard_floor(&pi(1), &pis),
            HARD,
            "a pi read keeping its floor keeps it beside one that can't be read"
        );
        let fewer = Holding::left([pi(1), pi(4)], |_| Some(FLOOR), min, &pis);
        assert_eq!(
            fewer.hard_floor(&pi(4), &pis),
            HARD,
            "with fewer than three others it gets its floor"
        );
        holding.done(&pi(4), Some(HARD));
        assert_eq!(
            holding.hard_floor(&pi(4), &pis),
            HARD,
            "and once held with one, it keeps it beside three others"
        );
    }

    #[test]
    fn a_pis_memory_min_is_read_once_so_it_cant_count_both_ways() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        // pi(4)'s memory.min can't be read the first time, and reads none after.
        let reads = std::cell::Cell::new(0);
        let min = |scope: &Path| {
            if scope != pi(4) {
                return Some(HARD);
            }
            reads.set(reads.get() + 1);
            (reads.get() > 1).then_some(0)
        };
        let holding = Holding::left([pi(1), pi(2), pi(3), pi(4)], |_| Some(FLOOR), min, &pis);
        assert_eq!(
            (reads.get(), holding.hard_floor(&pi(4), &pis)),
            (1, 0),
            "read once, as unread, it counts against the others and keeps none of its own"
        );
    }

    #[test]
    fn a_hard_floor_a_hosts_failed_give_back_keeps_counts_against_the_pis() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        let mut holding = Holding::default();
        holding.done(&pi(1), Some(HARD));
        holding.done(&pi(2), Some(HARD));
        holding.done(&pi(3), Some(0));
        holding.elsewhere = 1;
        assert_eq!(
            holding.hard_floor(&pi(3), &pis),
            0,
            "a third pi gets none while a host still keeps a share it couldn't give back"
        );
        assert_eq!(
            holding.changes(&[pi(1), pi(2), pi(3)], &pis),
            [],
            "and the pi held without one isn't held again for one"
        );
        holding.elsewhere = 0;
        assert_eq!(
            holding.hard_floor(&pi(3), &pis),
            HARD,
            "once the host's share is given back, the third pi gets one"
        );
        assert_eq!(
            holding.changes(&[pi(1), pi(2), pi(3)], &pis),
            [(pi(3), true)],
            "and is held again for it"
        );
    }

    #[test]
    fn a_stale_fourth_failing_its_give_back_leaves_the_window_joel_is_at_its_floor() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = |n: u8| PathBuf::from(format!("/cg/agents.slice/mycelium-pi-{n}.scope"));
        // The sorter before left four pis hard, one Joel has since left for good.
        let mut holding = Holding::left(
            [pi(0), pi(1), pi(2), pi(3)],
            |_| Some(FLOOR),
            |_| Some(HARD),
            &pis,
        );
        let wanted = [pi(1), pi(2), pi(3)];
        let mut given = Vec::new();
        for (scope, hold) in holding.changes(&wanted, &pis) {
            // The give-back fails, as busctl can just after a switch, so it's never noted done.
            if !hold {
                continue;
            }
            let hard = holding.hard_floor(&scope, &pis);
            holding.done(&scope, Some(hard));
            given.push((scope, hard));
        }
        assert_eq!(
            given,
            [(pi(1), HARD), (pi(2), HARD), (pi(3), HARD)],
            "a pi already keeping its floor keeps it while the stale one waits to be given \
             back, the window Joel is at's first among them"
        );
    }
}
