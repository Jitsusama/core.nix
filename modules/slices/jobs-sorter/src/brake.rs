//! Brakes the system's builds and agents' jobs while a call is on, so a
//! call never shares the processors with them.
//!
//! At 13:09 on 16 November a call ran beside a load of 63 to 105, with
//! the processors' some-pressure at 63% over a minute: Nix building work
//! here in build.slice and agents' work in jobs.slice. Their low weights
//! give way to the call's app only once it asks, and a call's audio and
//! video ask in bursts too short to win back a processor taken by a
//! compile. So through a call each of the two is held to a quarter of
//! the processors by a quota, and given them all back once it ends, as
//! [`calls::Call`] says, a few seconds after the microphone closes.
//!
//! Both are set as runtime properties, gone at the next boot: jobs.slice
//! on the account's own manager, build.slice on the system's, which
//! work-slices.rules lets an administrator set without a password.
//!
//! Each slice is followed apart, since one can be set while the other
//! fails, and one whose set failed is unknown until it's set again. A
//! sorter starts from what each slice's cpu.max reads: one holding the
//! brake's own quota is a brake a sorter before it left, and is let go
//! with no call on; any other is left as it is. The unit lets both go as
//! it stops, too.
//!
//! Through a call each braked slice's cpu.max is read on every look, and
//! one found with no quota or a larger one is set again, waiting as a
//! refusal does when it was found so within the last wait: on 16 November
//! both slices read max by 16:19 of a call braked at 15:58, nothing having
//! said it let them go, and a brake set only as a call began never came
//! back.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The slices braked through a call: which manager each is on, `--user`
/// or `--system`, and the slice.
pub const BRAKED: [(&str, &str); 2] = [("--user", "jobs.slice"), ("--system", "build.slice")];

/// A quota systemd reads as none.
const UNLIMITED: u64 = u64::MAX;

/// How long a slice that couldn't be set waits to be tried again, so a
/// refusal costs two busctl runs and two journal lines a minute rather
/// than a second.
const AGAIN: Duration = Duration::from_secs(30);

/// A quota of a quarter of `processors`, rounded up, in microseconds of
/// processor time each second, as systemd's `CPUQuotaPerSecUSec` takes.
pub fn quota(processors: usize) -> u64 {
    let quarter = u64::try_from(processors.div_ceil(4))
        .unwrap_or(u64::MAX)
        .max(1);
    quarter.saturating_mul(1_000_000)
}

/// The cgroup folder of the slice at `place` in [`BRAKED`]: the system's
/// under `cgroups`, the account's under its manager's folder, `manager`.
pub fn cgroup(place: usize, cgroups: &Path, manager: &Path) -> PathBuf {
    let (on, slice) = BRAKED[place];
    if on == "--user" {
        manager.join(slice)
    } else {
        cgroups.join(slice)
    }
}

/// A slice's `cpu.max`, as read, in microseconds each second, as
/// [`quota`] gives it, or none when it holds no quota.
fn per_second(cpu_max: &str) -> Option<u64> {
    let mut words = cpu_max.split_whitespace().map(str::parse::<u64>);
    let (Some(Ok(most)), Some(Ok(period))) = (words.next(), words.next()) else {
        return None;
    };
    (period > 0).then(|| most.saturating_mul(1_000_000) / period)
}

/// Whether a slice's `cpu.max`, as read, holds it to `quota` or less.
pub fn holds(cpu_max: &str, quota: u64) -> bool {
    per_second(cpu_max).is_some_and(|held| held <= quota)
}

/// The quota a slice is given while `calling` or not.
pub fn quota_for(calling: bool, processors: usize) -> u64 {
    if calling {
        quota(processors)
    } else {
        UNLIMITED
    }
}

/// Where the brake stands on each slice of [`BRAKED`], by its place there:
/// braked or let go as last set, or unknown, when setting it last failed,
/// and when it was last found let go mid-call.
#[derive(Default)]
pub struct Brake {
    stands: [Option<bool>; 2],
    /// When a set last failed, and whether it was to brake.
    failed: [Option<(Instant, bool)>; 2],
    lost: [Option<Instant>; 2],
}

impl Brake {
    /// A brake starting from what `read` finds of each slice's cpu.max:
    /// braked where it holds exactly `quota`, the brake's own, and let go
    /// otherwise, so a quota something else set, or none, is left alone.
    pub fn found(read: impl Fn(usize) -> Option<String>, quota: u64) -> Brake {
        let mut brake = Brake::default();
        for place in 0..BRAKED.len() {
            let ours = read(place).and_then(|cpu_max| per_second(&cpu_max)) == Some(quota);
            brake.stands[place] = Some(ours);
        }
        brake
    }

    /// The places in [`BRAKED`] of the slices to set `now`, with a call
    /// `calling` or not: those not already standing so, but for one whose
    /// set the same way failed less than [`AGAIN`] ago. A let-go that
    /// failed as a call ended doesn't hold back the brake for the next.
    pub fn due(&self, calling: bool, now: Instant) -> Vec<usize> {
        (0..BRAKED.len())
            .filter(|&slice| self.stands[slice] != Some(calling))
            .filter(|&slice| {
                self.failed[slice].is_none_or(|(at, braking)| {
                    braking != calling || now.saturating_duration_since(at) >= AGAIN
                })
            })
            .collect()
    }

    /// The slice at `slice` now stands as `calling` asks.
    pub fn set(&mut self, slice: usize, calling: bool) {
        self.stands[slice] = Some(calling);
        self.failed[slice] = None;
    }

    /// Setting the slice at `slice` failed `now`, `calling` or not. It may
    /// have landed late, as a set that timed out can, so it's unknown
    /// until set again.
    pub fn failed(&mut self, slice: usize, calling: bool, now: Instant) {
        self.stands[slice] = None;
        self.failed[slice] = Some((now, calling));
    }

    /// The places of the slices the brake set that `read` finds with no
    /// quota or more than `quota` at `now`, each of which is set again:
    /// on this look, or once it has waited, when it was found so within
    /// the last wait too, so something clearing it over and over isn't
    /// fought every second. One whose cpu.max can't be read is taken to
    /// hold, and one the brake let go isn't read.
    pub fn check(
        &mut self,
        read: impl Fn(usize) -> Option<String>,
        quota: u64,
        now: Instant,
    ) -> Vec<usize> {
        let lost: Vec<usize> = (0..BRAKED.len())
            .filter(|&place| self.stands[place] == Some(true))
            .filter(|&place| read(place).is_some_and(|cpu_max| !holds(&cpu_max, quota)))
            .collect();
        for &place in &lost {
            let again =
                self.lost[place].is_some_and(|at| now.saturating_duration_since(at) < AGAIN);
            self.stands[place] = None;
            self.failed[place] = again.then_some((now, true));
            self.lost[place] = Some(now);
        }
        lost
    }

    /// Whether every slice stands as `calling` asks.
    pub fn stands(&self, calling: bool) -> bool {
        self.stands.iter().all(|stands| *stands == Some(calling))
    }
}

/// What busctl is given to set `slice`'s quota on `manager` to `quota`, as
/// a runtime property, refused rather than asking for a password and
/// given up after 5 s, since the sorter's loop would wait on either.
pub fn quota_call(manager: &str, slice: &str, quota: u64) -> Vec<String> {
    let quota = quota.to_string();
    [
        manager,
        "--quiet",
        "--allow-interactive-authorization=no",
        "--timeout=5",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        slice,
        "true",
        "1",
        "CPUQuotaPerSecUSec",
        "t",
        &quota,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_starting_brakes_and_its_end_lets_go() {
        let now = Instant::now();
        let mut brake = Brake::default();
        brake.set(0, false);
        brake.set(1, false);
        assert!(
            brake.due(false, now).is_empty(),
            "slices with no call were set"
        );
        assert_eq!(
            brake.due(true, now),
            [0, 1],
            "a call starting braked not both"
        );
        brake.set(0, true);
        brake.set(1, true);
        assert!(brake.stands(true), "both set didn't stand braked");
        assert!(
            brake.due(true, now).is_empty(),
            "a held brake was set again"
        );
        assert_eq!(
            brake.due(false, now),
            [0, 1],
            "a call ending let go of not both"
        );
        assert_eq!(quota_for(true, 16), 4_000_000, "a call didn't hold 16 to 4");
        assert_eq!(
            quota_for(false, 16),
            u64::MAX,
            "no call didn't give all back"
        );
    }

    // A sorter that stopped mid-call may have left a brake on; a quota
    // something else set, or none, is no business of a new one.
    #[test]
    fn a_new_sorter_lets_go_of_only_its_own_brake() {
        let said = |cpu_max: [Option<&'static str>; 2]| {
            move |place: usize| cpu_max[place].map(str::to_owned)
        };
        let now = Instant::now();
        let left = Brake::found(
            said([Some("400000 100000\n"), Some("max 100000\n")]),
            4_000_000,
        );
        assert_eq!(
            left.due(false, now),
            [0],
            "a brake left braked wasn't let go alone"
        );
        let others = Brake::found(said([Some("200000 100000\n"), None]), 4_000_000);
        assert!(
            others.due(false, now).is_empty(),
            "another's quota, or an unread one, was let go"
        );
    }

    #[test]
    fn a_half_set_brake_is_let_go_and_a_refusal_waits() {
        let now = Instant::now();
        let mut brake = Brake::default();
        brake.set(0, true);
        brake.failed(1, false, now);
        assert_eq!(
            brake.due(false, now + Duration::from_secs(1)),
            [0],
            "the braked half wasn't let go, or the refused one was tried at once"
        );
        assert_eq!(
            brake.due(false, now + AGAIN),
            [0, 1],
            "a refused slice wasn't tried again once it had waited"
        );
        assert!(!brake.stands(true), "a half-set brake stood as braked");
        // A let-go that timed out may have landed, so the slice is unknown
        // both ways, and it doesn't hold back the brake for a call that
        // starts again.
        brake.set(1, true);
        brake.failed(1, false, now);
        assert_eq!(
            brake.due(true, now),
            [1],
            "a failed let-go held back the next call's brake"
        );
        assert_eq!(
            brake.due(false, now + AGAIN),
            [0, 1],
            "a slice whose let-go failed was taken as let go"
        );
    }

    // On 16 November both slices read max mid-call, and nothing set them
    // again.
    #[test]
    fn a_brake_cleared_mid_call_is_set_again_from_what_cpu_max_reads() {
        const QUOTA: u64 = 4_000_000;
        let cpu_max = std::cell::RefCell::new([Some("400000 100000\n"); 2]);
        let asked = std::cell::RefCell::new(Vec::new());
        let read = |place: usize| {
            asked.borrow_mut().push(place);
            cpu_max.borrow()[place].map(str::to_owned)
        };
        let now = Instant::now();
        let mut brake = Brake::default();
        brake.set(0, true);
        brake.set(1, true);
        assert!(
            brake.check(read, QUOTA, now).is_empty(),
            "a held brake was taken as lost"
        );
        cpu_max.borrow_mut()[1] = Some("max 100000\n");
        assert_eq!(
            brake.check(read, QUOTA, now),
            [1],
            "build.slice read max, unseen"
        );
        assert_eq!(
            brake.due(true, now),
            [1],
            "a cleared brake wasn't set again at once"
        );
        brake.set(1, true);
        let soon = now + Duration::from_secs(1);
        assert_eq!(
            brake.check(read, QUOTA, soon),
            [1],
            "a second clearing went unseen"
        );
        assert!(
            brake.due(true, soon).is_empty(),
            "a slice cleared over and over was fought every look"
        );
        assert_eq!(
            brake.due(true, soon + AGAIN),
            [1],
            "it wasn't set again once it waited"
        );
        brake.set(1, true);
        let later = soon + AGAIN + AGAIN;
        assert_eq!(
            brake.check(read, QUOTA, later),
            [1],
            "a later clearing went unseen"
        );
        assert_eq!(
            brake.due(true, later),
            [1],
            "a clearing long after the last wasn't set again at once"
        );
        brake.set(1, true);
        cpu_max.borrow_mut()[0] = Some("800000 100000\n");
        cpu_max.borrow_mut()[1] = None;
        assert_eq!(
            brake.check(read, QUOTA, soon + AGAIN),
            [0],
            "a raised quota passed as held, or an unread one as lost"
        );
        brake.set(0, false);
        asked.borrow_mut().clear();
        let _ = brake.check(read, QUOTA, soon + AGAIN);
        assert_eq!(*asked.borrow(), [1], "a slice the brake let go was read");
        assert_eq!(
            cgroup(0, Path::new("/c"), Path::new("/c/u")),
            Path::new("/c/u/jobs.slice"),
            "jobs.slice wasn't looked for under the account's manager"
        );
        assert_eq!(
            cgroup(1, Path::new("/c"), Path::new("/c/u")),
            Path::new("/c/build.slice"),
            "build.slice wasn't looked for under the system's"
        );
    }

    #[test]
    fn a_small_machine_keeps_one_processor_for_its_builds() {
        assert_eq!(quota(2), 1_000_000, "two processors left builds none");
        assert_eq!(
            quota(7),
            2_000_000,
            "seven processors gave builds other than 2"
        );
        assert_eq!(
            quota(12),
            3_000_000,
            "twelve processors gave builds other than 3"
        );
    }

    #[test]
    fn a_quota_is_set_on_the_slices_own_manager_for_this_boot_alone() {
        assert_eq!(
            quota_call("--system", "build.slice", 4_000_000),
            [
                "--system",
                "--quiet",
                "--allow-interactive-authorization=no",
                "--timeout=5",
                "call",
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "SetUnitProperties",
                "sba(sv)",
                "build.slice",
                "true",
                "1",
                "CPUQuotaPerSecUSec",
                "t",
                "4000000"
            ],
            "not build.slice's quota on the system's manager, for this boot, unasked"
        );
    }
}
