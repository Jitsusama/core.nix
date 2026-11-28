//! What each access point carried, by the hour of the day and how loud it
//! was heard, so the chooser (choose.rs) can weigh one the card hasn't
//! been on in the last two minutes by more than its signal.
//!
//! The best access point changes with the hour and the room (Joel, 14 Nov
//! 2026: "the best AP depends on time of day and where i am in the
//! house"), and what's measured lately reaches back two minutes only. So
//! each minute the figures of the access point a card is on are folded
//! into what it carried before at this hour, heard about this loud, which
//! stands for the room: the same radio heard 12 dB fainter is a room or
//! two away. The latest minutes weigh most, so tonight counts for more
//! than last week. An access point the card was on within the hour is
//! known by the hour just gone as well, until this hour holds enough of
//! its own minutes to go by (`just_left`).
//!
//! A minute the card brings a download down in is learned apart, as what
//! the access point carries under load, and across the hours, since such
//! minutes are few: at a desk on 18 Nov 2026 they were 40 of 645. Its slow
//! end says how quickly the access point drains its queue for the card,
//! which is no figure of how it answers idle: that night the 5 GHz radio
//! answered at 22.6 ms idle and 68.2 under load, and the 2.4 GHz one at
//! 10.4 and 37.5.
//!
//! Only a trusted network's figures are kept on the disk, as it alone is
//! trusted to know the machine (network.rs). Any other's are learned while
//! wifid runs and go with it, since which access points a machine was
//! near, and when, would say where it has been (seen.rs).
//!
//! Long because its cases, over half of it, stay beside the private
//! rules they prove, as every module in wifid keeps them.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::choose;
use crate::echo;
use crate::measure::Measured;

/// The file in wifid's state what's learned is kept in. It was `carried`
/// until the video class was weighed (measure.rs): what that holds was
/// learned of best effort alone, which holds a point stalling a call's
/// queue as quick as its best effort, and on 23 Nov 2026 such a figure
/// drew a card back to one 81 s after it was moved off. So the hours are
/// learned again in a file of their own. A wifid from before knows nothing
/// of this one, so going back to it leaves this file on the disk, kept to
/// the networks saved again only once a wifid from after starts.
pub const FILE: &str = "carried-weighing-video";

/// The file the hours were kept in before, let go of as wifid starts:
/// nothing keeps it to the networks still saved, and which access points
/// a machine was near, and when, would say where it has been. A wifid
/// from before begins its hours again.
const BEFORE: &str = "carried";

/// Lets go of the hours kept before in wifid's state folder, and of any
/// write of them cut short, which holds the same; each is tried whatever
/// the other does, and the first error said.
pub fn let_go_of_before(state: &Path) -> io::Result<()> {
    let before = state.join(BEFORE);
    let removed = [before.with_extension("partial"), before].map(fs::remove_file);
    removed
        .into_iter()
        .find(|removed| {
            removed
                .as_ref()
                .is_err_and(|error| error.kind() != io::ErrorKind::NotFound)
        })
        .unwrap_or(Ok(()))
}

/// How many minutes an access point must have been measured at an hour,
/// from about one place, before what it carried is gone by.
const ENOUGH: u32 = 5;

/// How much each minute weighs against every one before it: the last
/// twenty or so carry most of it.
const LATEST: f64 = 0.1;

/// How wide a place is, in dB of signal: as much fainter as the chooser
/// lets an untried access point be heard (choose.rs).
const STEP: i16 = 6;

/// How few echoes a minute's figures may be of and still be learned from:
/// the first minute after a join has hardly any.
const ECHOES: usize = 30;

/// How lately the card must have been on an access point, by the boot
/// clock (`booted`), for what it carried in the hour just gone to stand
/// for it (`just_left`).
pub const LEFT_WITHIN: Duration = Duration::from_secs(3600);

/// Where a card is: on which access point of which network, at what hour
/// of the day, heard how loud, in dBm.
pub struct Here<'a> {
    pub network: &'a str,
    pub point: &'a str,
    pub hour: u8,
    pub signal: i16,
    pub trusted: bool,
    /// When the minute ended, by the boot clock (`booted`).
    pub booted: Duration,
}

/// What one access point carried at one hour, from about one place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Learnt {
    network: String,
    point: String,
    hour: u8,
    step: i16,
    /// The gateway's p95, in milliseconds, over the minutes the card
    /// brought little down.
    p95: f64,
    /// The share of echoes lost, in percent, over every minute.
    lost: f64,
    minutes: u32,
    /// How many minutes `p95` is of.
    #[serde(default)]
    idle: u32,
    /// The gateway's p95 over the minutes the card brought a download
    /// down, where there were any.
    #[serde(default)]
    loaded: Option<Loaded>,
    /// Whether it goes on the disk: its network was trusted when last
    /// measured.
    #[serde(skip)]
    kept: bool,
    /// When the card was last measured on it, by the boot clock
    /// (`booted`), while wifid runs: the boot clock begins again at each
    /// boot, so none is kept on the disk.
    #[serde(skip)]
    last: Option<Duration>,
}

/// What one access point carried under load, at one hour from one place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Loaded {
    /// The gateway's p95, in milliseconds.
    p95: f64,
    minutes: u32,
}

/// What every access point carried, by hour and place.
pub struct Learned {
    file: PathBuf,
    learnt: Vec<Learnt>,
}

impl Learned {
    /// What was learned before, kept in this file. One that can't be read
    /// is begun again empty, which leaves the chooser to signals as before.
    pub fn kept_in(file: &Path) -> Learned {
        let kept = fs::read(file).ok();
        let learnt: Option<Vec<Learnt>> = kept.and_then(|kept| serde_json::from_slice(&kept).ok());
        let mut learnt = learnt.unwrap_or_default();
        for learnt in &mut learnt {
            learnt.kept = true;
        }
        Learned {
            file: file.to_path_buf(),
            learnt,
        }
    }

    /// Folds in what the access point a card is on carried over the span,
    /// keeping it on the disk where its network is trusted. An echo never
    /// answered counts as slow as it was waited for, and none slower: a
    /// held one answering later is no better, and no worse. A minute the card
    /// brings a download down in teaches what it lost alike, and its slow
    /// end as what the access point carries under load, so points are
    /// weighed against each other by minutes alike.
    pub fn carried(&mut self, here: &Here, measured: &Measured) -> io::Result<()> {
        if measured.echoes < ECHOES {
            return Ok(());
        }
        let loaded = choose::loaded(measured);
        let p95 = measured
            .p95
            .map_or(echo::LOST, |p95| p95.min(echo::LOST))
            .as_secs_f64()
            * 1000.0;
        let step = here.signal.div_euclid(STEP);
        let same = |learnt: &Learnt| {
            learnt.point == here.point && learnt.hour == here.hour && learnt.step == step
        };
        let learnt = if let Some(at) = self.learnt.iter().position(same) {
            &mut self.learnt[at]
        } else {
            self.learnt.push(Learnt {
                network: here.network.to_owned(),
                point: here.point.to_owned(),
                hour: here.hour,
                step,
                p95,
                lost: measured.lost,
                minutes: 0,
                idle: 0,
                loaded: None,
                kept: here.trusted,
                last: None,
            });
            self.learnt.last_mut().expect("just pushed")
        };
        if loaded {
            let under = learnt.loaded.get_or_insert(Loaded { p95, minutes: 0 });
            under.p95 += (p95 - under.p95) * LATEST;
            under.minutes = under.minutes.saturating_add(1);
        } else {
            if learnt.idle == 0 {
                learnt.p95 = p95;
            }
            learnt.p95 += (p95 - learnt.p95) * LATEST;
            learnt.idle = learnt.idle.saturating_add(1);
        }
        if learnt.minutes == 0 {
            learnt.lost = measured.lost;
        }
        learnt.lost += (measured.lost - learnt.lost) * LATEST;
        learnt.minutes = learnt.minutes.saturating_add(1);
        learnt.kept = here.trusted;
        learnt.last = Some(here.booted);
        if here.trusted { self.keep() } else { Ok(()) }
    }

    /// What an access point carried at this hour, heard about this loud,
    /// while the card brought little down, once it's been measured long
    /// enough there to go by. Its figures from a step louder or fainter
    /// count too, weighed by their minutes, so a signal wavering across a
    /// step's edge isn't two places.
    pub fn of(&self, point: &str, hour: u8, signal: i16) -> Option<Measured> {
        let near = self
            .near(point, signal)
            .filter(|learnt| learnt.hour == hour);
        let figures = near.map(|learnt| (learnt.p95, learnt.idle, learnt.lost, learnt.minutes));
        figured(figures, signal)
    }

    /// What an access point the card was on within `LEFT_WITHIN` carried
    /// at this hour and the hour just gone, heard about this loud, while
    /// the card brought little down, which stands in where this hour alone
    /// has too few minutes to go by (`of`). By the hour alone, a point the
    /// card has just left reads as untried once the hour turns: on 27 Nov
    /// 2026 a card on one for all of hour 9 at 34.7 ms at p95 left it at
    /// 10:03 for carrying 519 ms, and at 10:29, with three minutes of hour
    /// 10 learned there, tried it again as untried, heard at -44 dBm, and
    /// left it again at 10:44. What it carried in the hour just gone
    /// was learned in this room as lately as anything. Only the hour just
    /// gone stands in, as the one nearest this hour. "Within" is judged
    /// on the boot clock as of `booted`, which counts the machine's sleep,
    /// so a point last measured more than an hour of boot time ago, asleep
    /// or awake, isn't just left: a lid shut at 09:55 and opened at 10:05
    /// the next day, or after a weekend, doesn't go by the hour 9 it was
    /// shut in.
    ///
    /// It reads as the worse of the two hours, not the two weighed by
    /// their minutes: the hour gone holds every day's minutes there, so
    /// weighed against it the few this hour that made the card leave count
    /// for nothing, and a point left for 250 ms reads as its usual 41 ms,
    /// quick enough to be tried again as known better. Read as the worse,
    /// a point left for carrying badly isn't gone back to within the hour.
    ///
    /// This hour's minutes count only where they're as recent, the ones
    /// that made the card leave: those of another day, kept on the disk,
    /// would let a few slow minutes a week ago bar a point that carried
    /// well all of the hour just gone. The minutes are summed to reach
    /// `ENOUGH`, yet the reading may rest on one of this hour's, which can
    /// keep the point off until the hour turns. That's meant: it's the
    /// minute the card left for, and the card off the point can't measure
    /// it lifting.
    pub fn just_left(
        &self,
        point: &str,
        hour: u8,
        signal: i16,
        booted: Duration,
    ) -> Option<Measured> {
        let gone = (hour + 23) % 24;
        let lately = |learnt: &Learnt| {
            learnt
                .last
                .is_some_and(|last| booted.saturating_sub(last) < LEFT_WITHIN)
        };
        let figures = |learnt: &Learnt| (learnt.p95, learnt.idle, learnt.lost, learnt.minutes);
        let near = || self.near(point, signal);
        let this = summed(
            near()
                .filter(|learnt| learnt.hour == hour && lately(learnt))
                .map(figures),
        );
        let before = summed(
            near()
                .filter(|learnt| learnt.hour == gone && lately(learnt))
                .map(figures),
        );
        let worse = |of: fn(&Sums) -> Option<f64>| match (of(&this), of(&before)) {
            (Some(this), Some(before)) => Some(this.max(before)),
            (this, before) => this.or(before),
        };
        let both = Sums {
            timed: this.timed + before.timed,
            p95: worse(Sums::p95).unwrap_or_default(),
            counted: this.counted + before.counted,
            lost: worse(Sums::lost).unwrap_or_default(),
        };
        measured_of(&both, signal)
    }

    /// What an access point carried while the card brought a download
    /// down, heard about this loud, at any hour, once there are enough
    /// such minutes to go by.
    pub fn loaded(&self, point: &str, signal: i16) -> Option<Measured> {
        let figures = self.near(point, signal).filter_map(|learnt| {
            let under = learnt.loaded.as_ref()?;
            Some((under.p95, under.minutes, learnt.lost, learnt.minutes))
        });
        figured(figures, signal)
    }

    /// What's been learned of an access point heard about this loud.
    fn near<'a>(&'a self, point: &'a str, signal: i16) -> impl Iterator<Item = &'a Learnt> {
        let step = signal.div_euclid(STEP);
        self.learnt
            .iter()
            .filter(move |learnt| learnt.point == point && (learnt.step - step).abs() <= 1)
    }

    /// Forgets what the access points of every network but these carried.
    pub fn keep_only(&mut self, names: &[String]) -> io::Result<()> {
        let before = self.learnt.len();
        self.learnt.retain(|learnt| names.contains(&learnt.network));
        if self.learnt.len() == before {
            return Ok(());
        }
        self.keep()
    }

    /// Writes what's kept whole or not at all, readable by wifid alone.
    fn keep(&self) -> io::Result<()> {
        let kept: Vec<&Learnt> = self.learnt.iter().filter(|learnt| learnt.kept).collect();
        let partial = self.file.with_extension("partial");
        let mut written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        written.write_all(&serde_json::to_vec(&kept)?)?;
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o600))?;
        fs::rename(partial, &self.file)
    }
}

/// What figures learned at a few hours or places come to, each a p95 of
/// so many minutes and a loss of so many, once the p95 is of `ENOUGH`
/// minutes, as heard this loud.
fn figured(figures: impl Iterator<Item = (f64, u32, f64, u32)>, signal: i16) -> Option<Measured> {
    measured_of(&summed(figures), signal)
}

/// Figures weighed together by their minutes: the p95 by the minutes
/// it's of, the share lost by every minute.
#[derive(Clone, Copy, Debug, Default)]
struct Sums {
    /// The minutes the p95 is of.
    timed: u32,
    /// The p95, in milliseconds, weighed by its minutes.
    p95: f64,
    /// Every minute measured.
    counted: u32,
    /// The share of echoes lost, weighed by every minute.
    lost: f64,
}

impl Sums {
    /// The p95, where any minute's it's of.
    fn p95(&self) -> Option<f64> {
        (self.timed > 0).then_some(self.p95)
    }

    /// The share lost, where any minute was measured.
    fn lost(&self) -> Option<f64> {
        (self.counted > 0).then_some(self.lost)
    }
}

/// Weighs `(p95, its minutes, lost, every minute)` figures together.
fn summed(figures: impl Iterator<Item = (f64, u32, f64, u32)>) -> Sums {
    let (mut timed, mut p95, mut counted, mut lost) = (0, 0.0, 0, 0.0);
    for (their_p95, their_timed, their_lost, their_counted) in figures {
        timed += their_timed;
        p95 += their_p95 * f64::from(their_timed);
        counted += their_counted;
        lost += their_lost * f64::from(their_counted);
    }
    Sums {
        timed,
        p95: p95 / f64::from(timed.max(1)),
        counted,
        lost: lost / f64::from(counted.max(1)),
    }
}

/// What weighed figures say, once they're of enough minutes to go by.
fn measured_of(sums: &Sums, signal: i16) -> Option<Measured> {
    if sums.timed < ENOUGH {
        return None;
    }
    // A figure kept negative or past what a duration holds is a file
    // written by hand or broken, and says nothing.
    let p95 = Duration::try_from_secs_f64(sums.p95 / 1000.0).ok()?;
    Some(Measured {
        echoes: 0,
        p50: None,
        p95: Some(p95),
        lost: sums.lost,
        retried: 0.0,
        failed: 0.0,
        signal: Some(i32::from(signal)),
        moving: None,
        room: None,
    })
}

/// The hour of the day here, by the machine's own time zone; midnight's
/// where the time can't be read.
pub fn hour_here() -> u8 {
    terminal_kit::local::now().map_or(0, |here| u8::try_from(here.hour).unwrap_or(0))
}

/// The kernel's clock that counts the machine's sleep, named once so a
/// test can pin it: nothing short of a suspend can show a clock counts
/// sleep, and the monotonic clock in its place would pass every other case.
const BOOT_CLOCK: libc::clockid_t = libc::CLOCK_BOOTTIME;

/// How long the machine has been up, its sleep counted, by the kernel's
/// boot clock. An `Instant` is of the monotonic clock, which stops while
/// the machine sleeps, so a day asleep would read as no time at all
/// (`just_left`).
pub fn booted() -> Duration {
    read_clock(BOOT_CLOCK)
}

/// The time on one of the kernel's clocks.
fn read_clock(clock: libc::clockid_t) -> Duration {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: it writes the one timespec, held for the call.
    let read = unsafe { libc::clock_gettime(clock, &raw mut time) };
    // Linux has had the boot clock since 2.6.39, so reading it fails only
    // as reading the monotonic clock would, which `Instant::now` panics on
    // too.
    assert_eq!(
        read,
        0,
        "clock {clock} can't be read: {}",
        io::Error::last_os_error()
    );
    let seconds = u64::try_from(time.tv_sec).unwrap_or(0);
    let nanos = u32::try_from(time.tv_nsec).unwrap_or(0);
    Duration::new(seconds, nanos)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const DESK: &str = "02:00:00:00:a4:00";
    const HALL: &str = "02:00:00:00:a6:00";

    fn carried(p95: Option<u64>, lost: f64) -> Measured {
        Measured {
            echoes: 120,
            p50: None,
            p95: p95.map(Duration::from_millis),
            lost,
            retried: 0.0,
            failed: 0.0,
            signal: None,
            moving: None,
            room: None,
        }
    }

    fn at(point: &str, hour: u8, signal: i16) -> Here<'_> {
        Here {
            network: "Lantern",
            point,
            hour,
            signal,
            trusted: true,
            booted: Duration::ZERO,
        }
    }

    /// The access point measured this many minutes in a row.
    fn minutes(learned: &mut Learned, here: &Here, measured: &Measured, count: u32) {
        for _ in 0..count {
            learned.carried(here, measured).unwrap();
        }
    }

    fn p95_of(learned: &Learned, point: &str, hour: u8, signal: i16) -> Option<u64> {
        let measured = learned.of(point, hour, signal)?;
        Some(u64::try_from(measured.p95?.as_millis()).unwrap())
    }

    #[test]
    fn the_hours_kept_before_are_let_go_of_with_any_write_cut_short() {
        let dir = tempfile::tempdir().unwrap();
        for file in [BEFORE, "carried.partial", FILE] {
            fs::write(dir.path().join(file), "{}").unwrap();
        }
        let_go_of_before(dir.path()).unwrap();
        assert!(!dir.path().join(BEFORE).exists(), "the old hours are gone");
        assert!(
            !dir.path().join("carried.partial").exists(),
            "and their write cut short"
        );
        assert!(dir.path().join(FILE).exists(), "the new hours stay");
        let_go_of_before(dir.path()).expect("nothing left to let go of is no error");
    }

    #[test]
    fn a_point_held_past_an_echo_s_wait_is_learned_no_slower_than_one_unanswered() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join(FILE));
        for _ in 0..ENOUGH {
            learned
                .carried(&at(DESK, 20, -50), &carried(Some(2400), 40.0))
                .unwrap();
            learned
                .carried(&at(HALL, 20, -50), &carried(None, 100.0))
                .unwrap();
        }
        assert_eq!(p95_of(&learned, DESK, 20, -50), Some(1000), "held");
        assert_eq!(p95_of(&learned, HALL, 20, -50), Some(1000), "unanswered");
    }

    #[test]
    fn what_an_access_point_carried_is_learned_for_that_hour_alone() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.5),
            10,
        );

        let evening = learned.of(HALL, 20, -50).expect("the evening is learned");
        assert_eq!(evening.p95, Some(Duration::from_millis(15)));
        assert!((evening.lost - 0.5).abs() < 0.01, "lost {}", evening.lost);
        assert_eq!(learned.of(HALL, 9, -50), None, "nor the morning");
        assert_eq!(learned.of(DESK, 20, -50), None, "nor another access point");
    }

    #[test]
    fn a_few_minutes_are_not_enough_to_go_by() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(&mut learned, &at(HALL, 20, -50), &carried(Some(15), 0.0), 4);

        assert_eq!(learned.of(HALL, 20, -50), None);
    }

    #[test]
    fn the_latest_minutes_weigh_most() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        let here = at(HALL, 20, -50);

        minutes(&mut learned, &here, &carried(Some(15), 0.0), 60);
        minutes(&mut learned, &here, &carried(Some(115), 0.0), 20);

        let p95 = p95_of(&learned, HALL, 20, -50).unwrap();
        assert!(
            (95..=110).contains(&p95),
            "twenty slow minutes outweigh an hour's quick: {p95}"
        );
    }

    #[test]
    fn heard_much_fainter_an_access_point_is_learned_apart() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(
            &mut learned,
            &at(HALL, 20, -45),
            &carried(Some(15), 0.0),
            10,
        );

        assert_eq!(
            p95_of(&learned, HALL, 20, -48),
            Some(15),
            "a few dB is the same room"
        );
        assert_eq!(learned.of(HALL, 20, -70), None, "a room or two away isn't");
    }

    #[test]
    fn a_minute_unanswered_counts_as_slow_and_lost() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(&mut learned, &at(HALL, 20, -50), &carried(None, 100.0), 10);

        let learnt = learned
            .of(HALL, 20, -50)
            .expect("a dead minute is learned too");
        assert!(
            learnt.p95 >= Some(Duration::from_secs(1)),
            "p95 {:?}",
            learnt.p95
        );
        assert!(learnt.lost > 99.0, "lost {}", learnt.lost);
    }

    fn downloading(p95: u64, lost: f64) -> Measured {
        Measured {
            p50: Some(Duration::from_micros(5800)),
            moving: Some(52),
            ..carried(Some(p95), lost)
        }
    }

    fn loaded_p95_of(learned: &Learned, point: &str, signal: i16) -> Option<u64> {
        let measured = learned.loaded(point, signal)?;
        Some(u64::try_from(measured.p95?.as_millis()).unwrap())
    }

    #[test]
    fn a_minute_slowed_by_what_the_card_moves_teaches_nothing_of_it_idle() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        learned
            .carried(&at(HALL, 20, -50), &downloading(143, 0.0))
            .unwrap();

        assert_eq!(
            p95_of(&learned, HALL, 20, -50),
            Some(15),
            "a download's queue says how much it brought down"
        );
    }

    #[test]
    fn what_an_access_point_carried_under_load_is_learned_apart() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        let slow_download = Measured {
            moving: Some(25),
            ..carried(Some(120), 0.0)
        };

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        minutes(&mut learned, &at(HALL, 20, -50), &slow_download, 10);
        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(30), 0.0),
            10,
        );

        assert_eq!(
            loaded_p95_of(&learned, HALL, -50),
            Some(120),
            "a minute moving 25 Mbit/s is under load, slowed or not"
        );
        let idle = p95_of(&learned, HALL, 20, -50).unwrap();
        assert!(
            (24..=25).contains(&idle),
            "idle is of the idle minutes alone: {idle}"
        );
    }

    #[test]
    fn what_an_access_point_carried_under_load_is_learned_across_the_hours() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));

        minutes(&mut learned, &at(DESK, 6, -43), &downloading(60, 0.0), 3);
        assert_eq!(
            learned.loaded(DESK, -43),
            None,
            "three minutes aren't enough"
        );
        minutes(&mut learned, &at(DESK, 7, -43), &downloading(140, 0.0), 2);

        let loaded = loaded_p95_of(&learned, DESK, -43).unwrap();
        assert_eq!(loaded, 92, "weighed by their minutes");
        assert_eq!(learned.loaded(DESK, -70), None, "a room or two away isn't");
        assert_eq!(learned.loaded(HALL, -43), None, "nor another access point");
        assert_eq!(
            learned.of(DESK, 6, -43),
            None,
            "nor is it what it carries idle"
        );
    }

    #[test]
    fn what_an_access_point_carried_under_load_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        let mut learned = Learned::kept_in(&file);

        minutes(&mut learned, &at(DESK, 7, -43), &downloading(136, 0.0), 5);

        assert_eq!(
            loaded_p95_of(&Learned::kept_in(&file), DESK, -43),
            Some(136)
        );
    }

    #[test]
    fn a_minute_slowed_by_what_the_card_moves_still_teaches_what_it_lost() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        let downloading = downloading(143, 2.0);

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        learned.carried(&at(HALL, 20, -50), &downloading).unwrap();

        let lost = learned.of(HALL, 20, -50).unwrap().lost;
        assert!(lost > 0.1, "its loss is the air's: {lost}");
    }

    #[test]
    fn a_first_minute_slowed_by_what_the_card_moves_starts_nothing_idle() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        let mut learned = Learned::kept_in(&file);

        minutes(&mut learned, &at(HALL, 20, -50), &downloading(143, 0.0), 10);
        assert_eq!(learned.of(HALL, 20, -50), None, "nothing idle is known yet");
        assert_eq!(
            Learned::kept_in(&file).of(HALL, 20, -50),
            None,
            "nor once read back after a restart"
        );
        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            ENOUGH,
        );

        assert_eq!(p95_of(&learned, HALL, 20, -50), Some(15));
        assert_eq!(loaded_p95_of(&learned, HALL, -50), Some(143));
    }

    #[test]
    fn a_minute_of_few_echoes_teaches_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        let mut joined_just_now = carried(Some(900), 0.0);
        joined_just_now.echoes = 12;

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        learned
            .carried(&at(HALL, 20, -50), &joined_just_now)
            .unwrap();

        assert_eq!(p95_of(&learned, HALL, 20, -50), Some(15));
    }

    #[test]
    fn a_trusted_network_s_figures_are_kept_for_when_wifid_next_starts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        let mut learned = Learned::kept_in(&file);

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );

        assert_eq!(p95_of(&Learned::kept_in(&file), HALL, 20, -50), Some(15));
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn another_network_s_figures_are_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        let mut learned = Learned::kept_in(&file);
        let inn = Here {
            network: "Roadside Inn",
            trusted: false,
            ..at(DESK, 20, -50)
        };

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        minutes(&mut learned, &inn, &carried(Some(30), 0.0), 10);

        assert_eq!(
            p95_of(&learned, DESK, 20, -50),
            Some(30),
            "learned while wifid runs"
        );
        let written = fs::read_to_string(&file).unwrap();
        assert!(!written.contains("Roadside Inn"), "written: {written}");
        assert!(!written.contains(DESK), "written: {written}");
    }

    #[test]
    fn a_forgotten_network_s_figures_are_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        let mut learned = Learned::kept_in(&file);
        let home = Here {
            network: "Home",
            ..at(DESK, 20, -50)
        };
        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );
        minutes(&mut learned, &home, &carried(Some(30), 0.0), 10);

        learned.keep_only(&["Home".to_owned()]).unwrap();

        assert_eq!(learned.of(HALL, 20, -50), None);
        assert_eq!(Learned::kept_in(&file).of(HALL, 20, -50), None);
        assert_eq!(p95_of(&Learned::kept_in(&file), DESK, 20, -50), Some(30));
    }

    #[test]
    fn a_file_that_cant_be_read_is_begun_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("carried");
        fs::write(&file, "not what wifid writes").unwrap();
        let mut learned = Learned::kept_in(&file);

        minutes(
            &mut learned,
            &at(HALL, 20, -50),
            &carried(Some(15), 0.0),
            10,
        );

        assert_eq!(p95_of(&Learned::kept_in(&file), HALL, 20, -50), Some(15));
    }

    /// How long after boot the card's minute on the point it left at
    /// 10:03 on 27 Nov 2026 began, standing for 09:00.
    const NINE: Duration = Duration::from_hours(5);

    /// What the card learned of the point it left at 10:03 on 27 Nov 2026:
    /// all of hour 9 at 35 ms at p95, a minute ending each minute from
    /// `NINE` on, by the boot clock.
    fn all_of_nine() -> Learned {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join(FILE));
        for minute in 1..=60 {
            let here = Here {
                booted: NINE + Duration::from_mins(minute),
                ..at(DESK, 9, -44)
            };
            learned.carried(&here, &carried(Some(35), 0.0)).unwrap();
        }
        learned
    }

    /// The p95 a point is known by as just left, as of `booted`.
    fn just_left_p95(learned: &Learned, hour: u8, booted: Duration) -> Option<u128> {
        let measured = learned.just_left(DESK, hour, -44, booted)?;
        Some(measured.p95?.as_millis())
    }

    #[test]
    fn a_point_left_within_the_hour_is_known_by_the_hour_just_gone() {
        let learned = all_of_nine();
        let last = NINE + Duration::from_hours(1);

        assert_eq!(
            just_left_p95(&learned, 10, last + Duration::from_mins(26)),
            Some(35),
            "a point left 26 minutes before read as untried, though on it all of hour 9"
        );
        assert_eq!(
            learned.of(DESK, 10, -44),
            None,
            "hour 10 alone knew the point, so the figure above wasn't shown to be hour 9's"
        );
    }

    /// That many minutes on the desk at this hour, heard at -44 dBm, each
    /// ending `ended` by the boot clock.
    fn on_the_desk(
        learned: &mut Learned,
        (hour, ended): (u8, Duration),
        measured: &Measured,
        count: u32,
    ) {
        let here = Here {
            booted: ended,
            ..at(DESK, hour, -44)
        };
        minutes(learned, &here, measured, count);
    }

    #[test]
    fn a_point_just_left_isn_t_read_by_this_hour_s_minutes_of_another_day() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE);
        let mut another_day = Learned::kept_in(&file);
        on_the_desk(&mut another_day, (10, NINE), &carried(Some(300), 0.0), 3);
        let mut learned = Learned::kept_in(&file);
        let last = NINE + Duration::from_hours(1);
        on_the_desk(&mut learned, (9, last), &carried(Some(35), 0.0), 60);

        assert_eq!(
            just_left_p95(&learned, 10, last + Duration::from_mins(20)),
            Some(35),
            "three slow minutes of hour 10 from another day, read back from the disk, \
             outweighed sixty of hour 9 at 35 ms measured just now"
        );
    }

    #[test]
    fn a_point_just_left_reads_by_the_hour_gone_where_that_was_worse() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join(FILE));
        let last = NINE + Duration::from_hours(1);
        on_the_desk(&mut learned, (9, last), &carried(Some(250), 0.0), 60);
        on_the_desk(&mut learned, (10, last), &carried(Some(35), 0.0), 2);

        assert_eq!(
            just_left_p95(&learned, 10, last + Duration::from_mins(5)),
            Some(250),
            "two quick minutes this hour stood for a point that carried 250 ms all of \
             the hour just gone"
        );
    }

    #[test]
    fn a_point_just_left_reads_the_worse_loss_of_the_two_hours() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join(FILE));
        let last = NINE + Duration::from_hours(1);
        on_the_desk(&mut learned, (9, last), &carried(Some(35), 6.0), 60);
        on_the_desk(&mut learned, (10, last), &carried(Some(35), 0.5), 2);

        let left = learned
            .just_left(DESK, 10, -44, last + Duration::from_mins(5))
            .expect("left five minutes before");
        assert!(
            (left.lost - 6.0).abs() < 0.01,
            "the point read by this hour's loss, or the two weighed, not the worse of \
             the hour gone's 6 %: {}",
            left.lost
        );
    }

    #[test]
    fn a_point_last_on_a_day_asleep_ago_is_not_just_left() {
        let learned = all_of_nine();
        let last = NINE + Duration::from_hours(1);
        let next_morning = last + Duration::from_hours(24) + Duration::from_mins(10);

        assert_eq!(
            just_left_p95(&learned, 10, next_morning),
            None,
            "a lid shut at 09:55 and opened at 10:05 the next day read as ten minutes on, \
             so yesterday's hour 9 stood in as just left"
        );
    }

    #[test]
    fn a_point_last_measured_59_minutes_of_boot_time_ago_is_just_left() {
        let learned = all_of_nine();
        let last = NINE + Duration::from_hours(1);

        assert_eq!(
            just_left_p95(&learned, 10, last + Duration::from_mins(59)),
            Some(35),
            "a point last measured 59 minutes ago read as left longer than an hour ago"
        );
    }

    // That the boot clock counts sleep can't be shown without a suspend, so
    // this pins which clock is read, and that it reads at least what the
    // monotonic clock does, read first: a constant below the machine's
    // uptime fails it.
    #[test]
    fn booted_reads_the_boot_clock_and_no_less_than_the_monotonic() {
        assert_eq!(
            BOOT_CLOCK,
            libc::CLOCK_BOOTTIME,
            "booted reads a clock other than the boot clock, which may not count sleep"
        );
        let monotonic = read_clock(libc::CLOCK_MONOTONIC);
        let booted = booted();
        assert!(
            booted >= monotonic,
            "the boot clock read less than the monotonic clock read before it: \
             {booted:?} against {monotonic:?}"
        );
    }

    #[test]
    fn the_hour_here_is_an_hour_of_the_day() {
        let hour = hour_here();
        assert!(hour < 24, "the hour here should be of the day: {hour}");
    }
}
