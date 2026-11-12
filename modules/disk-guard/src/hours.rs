//! The hours of the day a heavy read of the disk may run in, such as
//! linking the Nix store's identical files, which reads every file not yet
//! linked. Its idle weights hold it back for the processor, but on a disk
//! with no I/O scheduler and no io.cost the weight of its reads counts for
//! nothing: on 11 November one started at 20:06, the moment a disk turned
//! healthy, and read the store at full speed beside the person at it.

use serde::{Deserialize, Deserializer, de};

/// A span of the local day, from a minute to a minute, past midnight when
/// it ends before it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hours {
    from: u32,
    to: u32,
}

impl Hours {
    /// The hours `HH:MM-HH:MM` names, such as `01:30-07:00` or, past
    /// midnight, `23:00-06:00`.
    pub fn parse(text: &str) -> Result<Hours, String> {
        let (from, to) = text
            .split_once('-')
            .ok_or_else(|| format!("{text:?} isn't HH:MM-HH:MM"))?;
        let (from, to) = (minute(from)?, minute(to)?);
        if from == to {
            return Err(format!("{text:?} starts as it ends"));
        }
        Ok(Hours { from, to })
    }

    /// Whether `minute`, counted from local midnight, falls within them.
    pub fn hold(&self, minute: u32) -> bool {
        if self.from < self.to {
            (self.from..self.to).contains(&minute)
        } else {
            minute >= self.from || minute < self.to
        }
    }

    /// Whether the local time at `wall` falls within them. A time the C
    /// library can't place falls outside, so a heavy read waits.
    pub fn hold_at(&self, wall: f64) -> bool {
        crate::clock::minute_of_day(wall).is_some_and(|minute| self.hold(minute))
    }
}

/// A minute of the day from `HH:MM`.
fn minute(text: &str) -> Result<u32, String> {
    let bad = || format!("{text:?} isn't HH:MM");
    let (hour, minute) = text.split_once(':').ok_or_else(bad)?;
    let (hour, minute): (u32, u32) = (
        hour.parse().map_err(|_| bad())?,
        minute.parse().map_err(|_| bad())?,
    );
    if hour >= 24 || minute >= 60 || text.len() != 5 {
        return Err(bad());
    }
    Ok(hour * 60 + minute)
}

impl<'de> Deserialize<'de> for Hours {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Hours::parse(&text).map_err(de::Error::custom)
    }
}
