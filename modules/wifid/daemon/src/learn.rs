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
//! than last week.
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

/// Where a card is: on which access point of which network, at what hour
/// of the day, heard how loud, in dBm.
pub struct Here<'a> {
    pub network: &'a str,
    pub point: &'a str,
    pub hour: u8,
    pub signal: i16,
    pub trusted: bool,
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
    let (mut timed, mut p95, mut counted, mut lost) = (0, 0.0, 0, 0.0);
    for (their_p95, their_timed, their_lost, their_counted) in figures {
        timed += their_timed;
        p95 += their_p95 * f64::from(their_timed);
        counted += their_counted;
        lost += their_lost * f64::from(their_counted);
    }
    if timed < ENOUGH {
        return None;
    }
    // A figure kept negative or past what a duration holds is a file
    // written by hand or broken, and says nothing.
    let p95 = Duration::try_from_secs_f64(p95 / f64::from(timed) / 1000.0).ok()?;
    Some(Measured {
        echoes: 0,
        p50: None,
        p95: Some(p95),
        lost: lost / f64::from(counted.max(1)),
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

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

    #[test]
    fn the_hour_here_is_an_hour_of_the_day() {
        let hour = hour_here();
        assert!(hour < 24, "the hour here should be of the day: {hour}");
    }
}
