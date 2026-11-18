//! A night job's hours, `01:30-05:15` by the local clock, and whether a
//! start falls within them.
//!
//! A timer tries the job only at night, but a switch can start the timer,
//! and the timer its job at once, whatever the hour: on 17 November a
//! switch at 05:59 started a heavy repack, after its last try at 04:01.
//! So `run` checks the clock itself, and a start outside its hours runs
//! nothing, as a start on a busy machine does.

use std::fmt;

const DAY: u16 = 24 * 60;

/// A stretch of the day, from its first minute up to the one it ends
/// before, running past midnight when it ends before it begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hours {
    from: u16,
    until: u16,
}

impl Hours {
    /// Hours as `--hours` takes them, `HH:MM-HH:MM`.
    pub fn read(said: &str) -> Result<Self, String> {
        let wrong = || format!("--hours takes FROM-UNTIL, as 01:30-05:15, not {said}");
        let (from, until) = said.split_once('-').ok_or_else(wrong)?;
        let hours = Hours {
            from: minute(from).ok_or_else(wrong)?,
            until: minute(until).ok_or_else(wrong)?,
        };
        if hours.from == hours.until {
            return Err(format!("--hours {said} holds no minute"));
        }
        Ok(hours)
    }

    /// Whether `minute` of the day falls within the hours.
    pub fn hold(self, minute: u16) -> bool {
        if self.from < self.until {
            (self.from..self.until).contains(&minute)
        } else {
            minute >= self.from || minute < self.until
        }
    }
}

impl fmt::Display for Hours {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}-{}", clock(self.from), clock(self.until))
    }
}

/// `HH:MM` as minutes past midnight, for two digits each and a time the
/// day has.
fn minute(said: &str) -> Option<u16> {
    let (hour, minute) = said.split_once(':')?;
    let digits = |part: &str| part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(hour) || !digits(minute) {
        return None;
    }
    let (hour, minute): (u16, u16) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

/// Minutes past midnight as `HH:MM`.
pub fn clock(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// The minute of the day now, by the local clock; None when the clock
/// can't be read. A zone glibc can't read is UTC's, for the timer that
/// starts the job as much as here, so the two agree. A test's machine can stand a file holding
/// `HH:MM` in for the clock, named by `NIGHT_SHIFT_CLOCK` and read at each
/// look, as `NIGHT_SHIFT_PROC` stands in for its pressures.
pub fn now() -> Option<u16> {
    if let Some(file) = std::env::var_os("NIGHT_SHIFT_CLOCK") {
        return minute(std::fs::read_to_string(file).ok()?.trim());
    }
    let minute = u16::try_from(terminal_kit::local::now()?.minute_of_day()).ok()?;
    (minute < DAY).then_some(minute)
}

#[cfg(test)]
mod tests {
    use super::Hours;

    fn hours(said: &str) -> Hours {
        Hours::read(said).expect("well-formed hours")
    }

    #[test]
    fn a_night_holds_from_its_first_minute_up_to_its_last() {
        let night = hours("01:30-05:15");
        for (minute, held) in [
            (89, false),
            (90, true),
            (240, true),
            (314, true),
            (315, false),
        ] {
            assert_eq!(night.hold(minute), held, "minute {minute} of {night}");
        }
    }

    #[test]
    fn hours_ending_before_they_begin_run_past_midnight() {
        let late = hours("22:00-02:00");
        for (minute, held) in [
            (1319, false),
            (1320, true),
            (1439, true),
            (0, true),
            (119, true),
            (120, false),
            (720, false),
        ] {
            assert_eq!(late.hold(minute), held, "minute {minute} of {late}");
        }
    }

    #[test]
    fn hours_say_themselves_as_they_were_given() {
        assert_eq!(
            hours("01:30-05:15").to_string(),
            "01:30-05:15",
            "the hours read back otherwise"
        );
    }

    #[test]
    fn hours_the_day_hasnt_or_that_hold_nothing_are_refused() {
        for said in [
            "25:00-05:00",
            "01:60-05:00",
            "1:30-05:00",
            "01:30",
            "01:30-",
            "01:30-01:30",
            "ab:cd-05:00",
            "01:30-05:15-06:00",
        ] {
            assert!(Hours::read(said).is_err(), "{said} was taken as hours");
        }
    }
}
