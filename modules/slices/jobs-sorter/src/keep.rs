//! Keeps the sessions that orchestrate others through a night's memory.
//!
//! An orchestrator is a pi session other sessions report to: it holds the
//! plan, hands out work and starts sessions again when they die, so losing
//! one stops every session under it. On 7 November at 22:34, 99 pi
//! processes held 21 GB of swap, every one at the account's
//! `oom_score_adj` of 200, and systemd-oomd watches the agents' slice for
//! memory pressure with nothing marking which window to spare.
//!
//! So each window holding an orchestrator gets a floor of [`FLOOR`] and
//! systemd-oomd's avoid preference, which it honours under a user's own
//! slice when it kills for pressure, and every other pi gives way first
//! to the kernel's OOM killer at [`GIVES_WAY`], while an orchestrator stays
//! at the account's 200. A window that no longer holds one is given both
//! back. Work is already at 900 or 1000, as `oom.rs` says.
//!
//! With tiers (src/tiers.rs), a pi gives way [`STEP`] more for each tier
//! its quest, as its presence names it, sits below the first, so the
//! kernel ends an unranked session before walgit's: with five tiers,
//! walgit's at 500 and unranked's at 625, under all work still. A window
//! Joel was at within [`SPARED_FOR`] keeps the account's 200, as an
//! orchestrator's does, since a pi he was typing to minutes ago is one he
//! is coming back to.
//!
//! Who orchestrates comes from the harness's mesh, as `--orchestrators`
//! names its presence folder: a session another session running this boot
//! names as its `orchestratedBy`, as cadence watches them too.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

/// The memory an orchestrator's window keeps under pressure: past what
/// one held on 7 November, 0.3 to 0.5 GB, and nine of them within the
/// agents' slice's own floor.
pub const FLOOR: u64 = 1 << 30;

/// The memory any other window running a pi keeps under pressure, softly
/// and without oomd's avoid. Only an orchestrator's window kept a floor
/// of its own here, and on 21 November at 07:15, 17 of Joel's 20 windows'
/// pis held 196 to 448 MB each in swap; a stall counts against his window
/// whether he's at it or not.
pub const PI_FLOOR: u64 = 512 << 20;

/// The floor a window keeps, by what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Floor {
    /// An orchestrator: [`FLOOR`] and oomd's avoid.
    Orchestrator,
    /// Any other pi: [`PI_FLOOR`].
    Pi,
    /// No pi: systemd's defaults.
    Nothing,
}

impl Floor {
    /// The memory.low it sets.
    pub fn bytes(self) -> u64 {
        match self {
            Floor::Orchestrator => FLOOR,
            Floor::Pi => PI_FLOOR,
            Floor::Nothing => 0,
        }
    }
}

/// The floor of a window running `pis` pis, whether any `orchestrates`.
pub fn floor_of(pis: usize, orchestrates: bool) -> Floor {
    if orchestrates {
        Floor::Orchestrator
    } else if pis > 0 {
        Floor::Pi
    } else {
        Floor::Nothing
    }
}

/// The adjustment a pi that orchestrates nothing is given, between the
/// account's and work's, so the kernel ends a builder's session before an
/// orchestrator's, and work before either.
pub const GIVES_WAY: i32 = 500;

/// The account's own adjustment, which an orchestrator keeps or is
/// lowered back to.
pub const ACCOUNT: i32 = 200;

/// How much more a pi gives way for each tier its quest sits below the
/// first: eight below stays under the first tier's work waiting
/// elsewhere.
pub const STEP: i32 = 25;

/// How long after Joel was at a pi's window it keeps the account's
/// adjustment, as P9R1KI set on 12 November.
pub const SPARED_FOR: Duration = Duration::from_mins(30);

/// What a session says of itself, as far as this reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    pub addresses: Vec<String>,
    pub orchestrated_by: Option<String>,
    pub quest: Option<String>,
    pub pid: Option<u32>,
    pub boot: Option<String>,
}

/// Reads one presence file's text, or none when it isn't one.
pub fn presence(text: &str) -> Option<Presence> {
    let value: Value = serde_json::from_str(text).ok()?;
    let addresses = value
        .get("addresses")?
        .as_array()?
        .iter()
        .filter_map(|address| address.as_str().map(str::to_owned))
        .collect();
    let owner = value.get("owner");
    Some(Presence {
        addresses,
        orchestrated_by: value
            .get("orchestratedBy")
            .and_then(Value::as_str)
            .map(str::to_owned),
        quest: value
            .get("quest")
            .and_then(Value::as_str)
            .map(str::to_owned),
        pid: owner
            .and_then(|owner| owner.get("pid"))
            .and_then(Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
        boot: owner
            .and_then(|owner| owner.get("bootId"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

/// Every presence in `dir`, passing over what isn't one.
pub fn read(dir: &Path) -> Vec<Presence> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| presence(&fs::read_to_string(path).ok()?))
        .collect()
}

/// The pids of the orchestrators running in `boot`: sessions that another
/// session running in it names as its orchestrator. A session that ended,
/// or ran in another boot, names nobody and is nobody.
pub fn orchestrators(
    presences: &[Presence],
    boot: &str,
    runs: impl Fn(u32) -> bool,
) -> HashSet<u32> {
    let live: Vec<&Presence> = presences
        .iter()
        .filter(|session| session.boot.as_deref() == Some(boot) && session.pid.is_some_and(&runs))
        .collect();
    let named: HashSet<&str> = live
        .iter()
        .filter_map(|session| session.orchestrated_by.as_deref())
        .collect();
    live.iter()
        .filter(|session| {
            session
                .addresses
                .iter()
                .any(|address| named.contains(address.as_str()))
        })
        .filter_map(|session| session.pid)
        .collect()
}

/// The quest each session running in `boot` is on, by its pid.
pub fn quests(
    presences: &[Presence],
    boot: &str,
    runs: impl Fn(u32) -> bool,
) -> HashMap<u32, String> {
    presences
        .iter()
        .filter(|session| session.boot.as_deref() == Some(boot))
        .filter_map(|session| {
            Some((
                session.pid.filter(|&pid| runs(pid))?,
                session.quest.clone()?,
            ))
        })
        .collect()
}

/// The adjustment a pi wants: the account's for one `spared`, an
/// orchestrator or one Joel was at within [`SPARED_FOR`], and otherwise
/// [`GIVES_WAY`] and [`STEP`] more for each of the `rank` tiers its
/// quest sits below the first.
pub fn adjustment(spared: bool, rank: usize) -> i32 {
    if spared {
        ACCOUNT
    } else {
        let below = STEP.saturating_mul(i32::try_from(rank).unwrap_or(i32::MAX));
        GIVES_WAY.saturating_add(below)
    }
}

/// Puts `pid` at `wanted` in `proc`, the kernel's process files, unless
/// it's there already. One that left meanwhile is no loss; anything else
/// is said once.
pub fn adjust(proc: &Path, pid: u32, wanted: i32, warned: &mut bool) {
    let file = proc.join(pid.to_string()).join("oom_score_adj");
    let Some(now) = fs::read_to_string(&file)
        .ok()
        .and_then(|text| text.trim().parse::<i32>().ok())
    else {
        return;
    };
    if now == wanted {
        return;
    }
    if let Err(error) = fs::write(&file, format!("{wanted}\n"))
        && error.raw_os_error() != Some(3)
        && !std::mem::replace(warned, true)
    {
        eprintln!("Couldn't put {pid} at {wanted} for the OOM killer: {error}");
    }
}

/// What busctl is given to keep a window's unit at its [`Floor`], with
/// systemd-oomd's avoid preference for an orchestrator's, or to give both
/// back, as runtime properties gone at the next boot.
pub fn keep_call(unit: &str, floor: Floor) -> Vec<String> {
    let (floor, preference) = match floor {
        Floor::Orchestrator => (FLOOR.to_string(), "avoid"),
        Floor::Pi => (PI_FLOOR.to_string(), "none"),
        Floor::Nothing => ("0".to_owned(), "none"),
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
        "2",
        "MemoryLow",
        "t",
        &floor,
        "ManagedOOMPreference",
        "s",
        preference,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// The windows kept, each at its floor, so each is set once and changed
/// once what it holds changes.
#[derive(Debug, Default)]
pub struct Keeping {
    kept: HashMap<String, Floor>,
    /// Whether a failure to keep a window has been said.
    pub failed: bool,
}

impl Keeping {
    /// The windows whose memory.low reads at an orchestrator's floor or a
    /// pi's, taken as a sorter before this one kept them, though only their
    /// height says so. A sorter's floors are
    /// the user manager's runtime properties, so they outlive it, and a
    /// sorter that took up none would fit its first looks as though nothing
    /// kept them. Each is changed, as any kept window is, once what it holds
    /// wants another floor: one that holds no pi is given back, whoever set
    /// its floor at that height.
    pub fn left(
        scopes: impl IntoIterator<Item = std::path::PathBuf>,
        floor: impl Fn(&Path) -> Option<u64>,
    ) -> Self {
        let kept = scopes
            .into_iter()
            .filter_map(|scope| {
                let kept = match floor(&scope)? {
                    FLOOR => Floor::Orchestrator,
                    PI_FLOOR => Floor::Pi,
                    _ => return None,
                };
                Some((scope.file_name()?.to_str()?.to_owned(), kept))
            })
            .collect();
        Self {
            kept,
            failed: false,
        }
    }

    /// The windows to change, each with its floor, from each window's unit
    /// and the floor it wants. One not kept, nor taken up as kept by
    /// [`Keeping::left`], and holding none is left as it is.
    pub fn changes(&self, windows: &[(String, Floor)]) -> Vec<(String, Floor)> {
        windows
            .iter()
            .filter(|(unit, floor)| {
                self.kept.get(unit).copied().unwrap_or(Floor::Nothing) != *floor
            })
            .cloned()
            .collect()
    }

    /// Notes a window as kept at `floor`, or forgotten at none, as one a
    /// hold gave back is, so it's kept again from nothing.
    pub fn done(&mut self, unit: &str, floor: Floor) {
        if floor == Floor::Nothing {
            self.kept.remove(unit);
        } else {
            self.kept.insert(unit.to_owned(), floor);
        }
    }

    /// The windows kept at a floor, by unit, each with the memory.low it was
    /// set.
    pub fn floors(&self) -> impl Iterator<Item = (&str, u64)> {
        self.kept
            .iter()
            .map(|(unit, floor)| (unit.as_str(), floor.bytes()))
    }

    /// Forgets the windows no longer among `units`, which have closed.
    pub fn keep_only(&mut self, units: &[String]) {
        self.kept.retain(|unit, _| units.contains(unit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOT: &str = "3c4d23d2";

    #[test]
    fn the_floors_a_sorter_before_kept_are_taken_up_by_their_memory_low() {
        let scopes = ["kitty-1-1.scope", "kitty-1-2.scope", "kitty-1-3.scope"]
            .map(|name| std::path::PathBuf::from("/agents.slice").join(name));
        let floor = |scope: &Path| match scope.file_name()?.to_str()? {
            "kitty-1-1.scope" => Some(FLOOR),
            "kitty-1-2.scope" => Some(PI_FLOOR),
            _ => Some(2 << 30),
        };
        let keeping = Keeping::left(scopes, floor);
        let mut floors: Vec<_> = keeping.floors().collect();
        floors.sort_unstable();
        assert_eq!(
            floors,
            [("kitty-1-1.scope", FLOOR), ("kitty-1-2.scope", PI_FLOOR)],
            "a floor keep.rs sets wasn't taken up, or another floor was"
        );
        assert!(
            keeping
                .changes(&[("kitty-1-2.scope".to_owned(), Floor::Pi)])
                .is_empty(),
            "a pi's window taken up at its floor was kept again"
        );
        assert_eq!(
            keeping.changes(&[("kitty-1-1.scope".to_owned(), Floor::Nothing)]),
            [("kitty-1-1.scope".to_owned(), Floor::Nothing)],
            "a window taken up as kept that holds no pi now wasn't given back"
        );
    }

    fn session(address: &str, by: Option<&str>, pid: u32, boot: &str) -> Presence {
        Presence {
            addresses: vec![address.to_owned()],
            orchestrated_by: by.map(str::to_owned),
            quest: Some(format!("QEST-{address}")),
            pid: Some(pid),
            boot: Some(boot.to_owned()),
        }
    }

    #[test]
    fn a_presence_is_read_as_the_harness_writes_it() {
        let text = r#"{"session":"01a1","addresses":["WCJQSG","D1BE"],
            "worksOn":"a title saying \"orchestratedBy\" in passing",
            "owner":{"pid":7973,"startedAt":1794029159510,"bootId":"3c4d23d2"},
            "orchestratedBy":"P9R1KI","quest":"QEST-20261007-WCJQSG","state":"busy"}"#;
        assert_eq!(
            presence(text),
            Some(Presence {
                addresses: vec!["WCJQSG".into(), "D1BE".into()],
                orchestrated_by: Some("P9R1KI".into()),
                quest: Some("QEST-20261007-WCJQSG".into()),
                pid: Some(7973),
                boot: Some(BOOT.into()),
            })
        );
        assert_eq!(
            presence("{\"state\":\"busy\"}"),
            None,
            "no addresses, no session"
        );
        assert_eq!(presence("not json"), None);
    }

    #[test]
    fn an_orchestrator_is_a_session_a_running_one_names() {
        let running = |pid: u32| pid != 4;
        let found = orchestrators(
            &[
                session("ORCH", None, 1, BOOT),
                session("BLDR", Some("ORCH"), 2, BOOT),
                session("IDLE", None, 3, BOOT),
                session("GONE", None, 5, BOOT),
                session("DEAD", Some("GONE"), 4, BOOT),
                session("PAST", None, 6, BOOT),
                session("OLD", Some("PAST"), 7, "another boot"),
            ],
            BOOT,
            running,
        );
        assert_eq!(
            found,
            HashSet::from([1]),
            "only ORCH: GONE is named by one that ended, PAST by one from another boot"
        );
    }

    #[test]
    fn a_spared_pi_keeps_the_accounts_adjustment_and_any_other_gives_way() {
        assert_eq!(adjustment(true, 0), ACCOUNT);
        assert_eq!(adjustment(true, 5), ACCOUNT, "spared in any tier");
        assert_eq!(
            adjustment(false, 0),
            GIVES_WAY,
            "the first tier, or no tiers"
        );
        const {
            assert!(
                ACCOUNT < GIVES_WAY && GIVES_WAY < crate::oom::AFTER,
                "a spared pi must rank below the rest, and every pi below work"
            );
        };
    }

    #[test]
    fn a_pi_in_a_lower_tier_gives_way_before_a_higher_tiers() {
        assert_eq!(adjustment(false, 1), GIVES_WAY + STEP);
        assert_eq!(
            adjustment(false, 5),
            625,
            "unranked, after five tiers, goes first among pis"
        );
        assert!(
            adjustment(false, 2) > adjustment(false, 1),
            "a lower tier's pi must score above a higher tier's"
        );
    }

    #[test]
    fn a_running_sessions_quest_is_read_by_its_pid() {
        let running = |pid: u32| pid != 4;
        let mut none = session("NONE", None, 3, BOOT);
        none.quest = None;
        let found = quests(
            &[
                session("BLDR", None, 2, BOOT),
                none,
                session("DEAD", None, 4, BOOT),
                session("OLD", None, 7, "another boot"),
            ],
            BOOT,
            running,
        );
        assert_eq!(
            found,
            HashMap::from([(2, "QEST-BLDR".to_owned())]),
            "one on no quest, one ended and one from another boot say nothing"
        );
    }

    #[test]
    fn an_adjustment_is_written_only_where_it_differs() {
        let proc = std::env::temp_dir().join(format!("keep-{}", std::process::id()));
        for (pid, adj) in [(30, "200"), (31, "500"), (32, "500")] {
            let dir = proc.join(pid.to_string());
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("oom_score_adj"), format!("{adj}\n")).unwrap();
        }
        let mut warned = false;
        adjust(&proc, 30, GIVES_WAY, &mut warned);
        adjust(&proc, 31, GIVES_WAY, &mut warned);
        adjust(&proc, 32, ACCOUNT, &mut warned);
        adjust(&proc, 33, ACCOUNT, &mut warned);
        let adj = |pid: u32| {
            fs::read_to_string(proc.join(pid.to_string()).join("oom_score_adj")).unwrap()
        };
        assert_eq!(adj(30), "500\n", "a builder's pi gives way");
        assert_eq!(adj(31), "500\n");
        assert_eq!(
            adj(32),
            "200\n",
            "an orchestrator raised by hand comes back down"
        );
        assert!(!warned, "a process that left is no loss");
        fs::remove_dir_all(&proc).unwrap();
    }

    #[test]
    fn a_window_is_kept_with_a_floor_and_avoided_or_given_both_back() {
        let kept = keep_call("kitty-1-9.scope", Floor::Orchestrator);
        assert_eq!(
            kept[8..],
            [
                "kitty-1-9.scope",
                "true",
                "2",
                "MemoryLow",
                "t",
                "1073741824",
                "ManagedOOMPreference",
                "s",
                "avoid"
            ]
        );
        let back = keep_call("kitty-1-9.scope", Floor::Nothing);
        assert_eq!(back[13], "0");
        assert_eq!(back[16], "none");
    }

    #[test]
    fn a_window_running_a_pi_keeps_a_soft_floor_and_isnt_avoided() {
        let kept = keep_call("mycelium-pi-55deb2a5.scope", Floor::Pi);
        assert_eq!(
            kept[13],
            (512_u64 << 20).to_string(),
            "a pi's window keeps 512 MiB"
        );
        assert_eq!(
            kept[16], "none",
            "oomd may still end it before an orchestrator's"
        );
        const {
            assert!(
                PI_FLOOR < FLOOR,
                "an orchestrator's window keeps more than any pi's"
            );
        };
    }

    #[test]
    fn a_windows_floor_is_by_what_it_holds() {
        assert_eq!(floor_of(1, true), Floor::Orchestrator);
        assert_eq!(floor_of(2, false), Floor::Pi);
        assert_eq!(
            floor_of(0, false),
            Floor::Nothing,
            "a window running no pi keeps nothing"
        );
    }

    #[test]
    fn a_window_is_kept_once_and_given_back_once_it_holds_no_orchestrator() {
        let mut keeping = Keeping::default();
        let windows = [
            ("a".to_owned(), Floor::Orchestrator),
            ("b".to_owned(), Floor::Nothing),
            ("c".to_owned(), Floor::Pi),
        ];
        assert_eq!(
            keeping.changes(&windows),
            [
                ("a".to_owned(), Floor::Orchestrator),
                ("c".to_owned(), Floor::Pi)
            ],
            "one never kept and holding none is left"
        );
        keeping.done("a", Floor::Orchestrator);
        keeping.done("c", Floor::Pi);
        assert_eq!(keeping.changes(&windows), Vec::<(String, Floor)>::new());
        assert_eq!(
            keeping.changes(&[("a".to_owned(), Floor::Pi)]),
            [("a".to_owned(), Floor::Pi)],
            "an orchestrator's window that holds none goes down to a pi's"
        );
        assert_eq!(
            keeping.changes(&[("c".to_owned(), Floor::Nothing)]),
            [("c".to_owned(), Floor::Nothing)],
            "a pi's window whose pi has gone gives its floor back"
        );
        keeping.done("c", Floor::Nothing);
        assert_eq!(
            keeping.changes(&[("c".to_owned(), Floor::Pi)]),
            [("c".to_owned(), Floor::Pi)],
            "one forgotten, as a hold given back forgets it, is kept again"
        );
        keeping.keep_only(&["b".to_owned()]);
        assert!(
            keeping
                .changes(&[("a".to_owned(), Floor::Nothing)])
                .is_empty(),
            "a closed"
        );
    }
}
