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
//! Only a trusted network's figures are kept on the disk, as it alone is
//! trusted to know the machine (network.rs). Any other's are learned while
//! wifid runs and go with it, since which access points a machine was
//! near, and when, would say where it has been (seen.rs).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::echo;
use crate::measure::Measured;

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
    /// The gateway's p95, in milliseconds.
    p95: f64,
    /// The share of echoes lost, in percent.
    lost: f64,
    minutes: u32,
    /// Whether it goes on the disk: its network was trusted when last
    /// measured.
    #[serde(skip)]
    kept: bool,
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
    /// answered counts as slow as it was waited for.
    pub fn carried(&mut self, here: &Here, measured: &Measured) -> io::Result<()> {
        if measured.echoes < ECHOES {
            return Ok(());
        }
        let p95 = measured.p95.unwrap_or(echo::LOST).as_secs_f64() * 1000.0;
        let step = here.signal.div_euclid(STEP);
        let same = |learnt: &&mut Learnt| {
            learnt.point == here.point && learnt.hour == here.hour && learnt.step == step
        };
        match self.learnt.iter_mut().find(same) {
            Some(learnt) => {
                learnt.p95 += (p95 - learnt.p95) * LATEST;
                learnt.lost += (measured.lost - learnt.lost) * LATEST;
                learnt.minutes = learnt.minutes.saturating_add(1);
                learnt.kept = here.trusted;
            }
            None => self.learnt.push(Learnt {
                network: here.network.to_owned(),
                point: here.point.to_owned(),
                hour: here.hour,
                step,
                p95,
                lost: measured.lost,
                minutes: 1,
                kept: here.trusted,
            }),
        }
        if here.trusted { self.keep() } else { Ok(()) }
    }

    /// What an access point carried at this hour, heard about this loud,
    /// once it's been measured long enough there to go by. Its figures
    /// from a step louder or fainter count too, weighed by their minutes,
    /// so a signal wavering across a step's edge isn't two places.
    pub fn of(&self, point: &str, hour: u8, signal: i16) -> Option<Measured> {
        let step = signal.div_euclid(STEP);
        let near = self.learnt.iter().filter(|learnt| {
            learnt.point == point && learnt.hour == hour && (learnt.step - step).abs() <= 1
        });
        let (mut minutes, mut p95, mut lost) = (0, 0.0, 0.0);
        for learnt in near {
            minutes += learnt.minutes;
            p95 += learnt.p95 * f64::from(learnt.minutes);
            lost += learnt.lost * f64::from(learnt.minutes);
        }
        if minutes < ENOUGH {
            return None;
        }
        let whole = f64::from(minutes);
        Some(Measured {
            echoes: 0,
            p50: None,
            p95: Some(Duration::from_secs_f64(p95 / whole / 1000.0)),
            lost: lost / whole,
            retried: 0.0,
            failed: 0.0,
            signal: Some(i32::from(signal)),
        })
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

/// The hour of the day here, by the machine's own time zone; midnight's
/// where the time can't be read.
pub fn hour_here() -> u8 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let Ok(now) = libc::time_t::try_from(since.as_secs()) else {
        return 0;
    };
    // SAFETY: a tm is plain numbers and a pointer, for which zero is null.
    let mut here: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: it reads the one time and writes the one tm it's given.
    let read = unsafe { libc::localtime_r(&raw const now, &raw mut here) };
    if read.is_null() {
        return 0;
    }
    u8::try_from(here.tm_hour).unwrap_or(0)
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
