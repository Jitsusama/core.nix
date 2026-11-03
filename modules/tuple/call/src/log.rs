//! What Tuple's log says of a ring: someone calling, the ring answered or
//! turned away, and a ring that ends unanswered, which is a missed call.
//! Tuple's stream says nothing of a ring that has been seen, so this is
//! read from the daemon's log, as the shell's Tuple.js read it before.
//!
//! A line reads "02:17:41.512|<process>|<thread>|<level>|<source>|<message>",
//! its time of day in UTC, though some have fewer fields: the message is
//! whatever follows the last bar. Who's calling fills a blank in a message
//! Tuple's program holds, and what fills it isn't known, so a caller is
//! known by whatever fills it, and named only when it holds the number
//! Tuple knows them by.

/// How long a ring lasts when Tuple writes nothing of its end, as a
/// phone's would, in milliseconds.
pub const RINGS_FOR: i64 = 60_000;

/// How far past now a line's time of day can be, in milliseconds, and
/// still be today's: a minute, for a clock a little ahead of this one.
const AHEAD: i64 = 60_000;

const DAY: i64 = 86_400_000;

/// What Tuple writes as its daemon starts, after which its stream can be
/// followed.
const STARTED: &str = "daemon loop started, waiting for events...";

/// What a call's end sounds like: Tuple ending it, the other side ending
/// it, or the daemon starting or stopping, which ends any call it had.
const ENDING: [&str; 4] = [
    "invalidating call",
    "call ended",
    STARTED,
    "received 'off' message, quitting",
];

/// What a ring answered sounds like, each said whole or begun.
const TAKEN_WHOLE: [&str; 2] = ["accepting call", "call connected"];
const TAKEN_BEGUN: [&str; 2] = ["answering call with ", "auto-answering "];
/// What a ring turned away sounds like, which is missed all the same.
const TURNED_AWAY: &str = "ignoring incoming call: ";
/// Someone calling, and the blank that says who.
const RINGING: &str = "received incoming call: ";

/// A ring: who's calling, by what fills Tuple's blank, and since when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ring {
    pub who: String,
    pub since: i64,
}

/// A ring that ended unanswered, and when it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missed {
    pub who: String,
    pub at: i64,
}

/// What a line of the log changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// Whether the ring changed, begun or ended.
    pub rang: bool,
    /// A ring that ended unanswered since this program started.
    pub missed: Option<Missed>,
    /// Whether Tuple's daemon has just started.
    pub started: bool,
}

/// The ring the log says is going, if any.
#[derive(Debug)]
pub struct Log {
    /// When this program started: a ring ended before then isn't news.
    began: i64,
    ringing: Option<Ring>,
}

impl Log {
    pub fn new(began: i64) -> Log {
        Log {
            began,
            ringing: None,
        }
    }

    /// The ring going now, if any.
    pub fn ringing(&self) -> Option<&Ring> {
        self.ringing.as_ref()
    }

    /// When the ring going now is over by itself, if one is.
    pub fn ends(&self) -> Option<i64> {
        self.ringing.as_ref().map(|ring| ring.since + RINGS_FOR)
    }

    /// What a line written to the log changes, `now` being when it's read.
    pub fn heard(&mut self, line: &str, now: i64) -> Heard {
        let said = line.rsplit('|').next().unwrap_or("").trim();
        let at = written(line, now);
        let mut heard = Heard {
            started: said == STARTED,
            ..Heard::default()
        };
        if let Some(who) = said.strip_prefix(RINGING) {
            heard.rang = true;
            // A ring the log says was over before now, as one read back
            // from before this started may be, was over unanswered.
            heard.missed = self.end(at);
            self.ringing = Some(Ring {
                who: who.trim().to_owned(),
                since: at,
            });
            heard.missed = heard.missed.or_else(|| self.tick(now));
            return heard;
        }
        let taken =
            TAKEN_WHOLE.contains(&said) || TAKEN_BEGUN.iter().any(|begun| said.starts_with(begun));
        if taken {
            heard.rang = self.ringing.take().is_some();
        } else if ENDING.contains(&said) || said.starts_with(TURNED_AWAY) {
            heard.rang = self.ringing.is_some();
            heard.missed = self.end(at);
        }
        heard
    }

    /// Ends a ring that has rung its minute by `now`, as missed.
    pub fn tick(&mut self, now: i64) -> Option<Missed> {
        let over = self.ends()?;
        if now < over {
            return None;
        }
        self.end(over)
    }

    /// Ends the ring going at `at`, unanswered, saying so if that's since
    /// this started.
    fn end(&mut self, at: i64) -> Option<Missed> {
        let ring = self.ringing.take()?;
        let at = at.min(ring.since + RINGS_FOR);
        (at >= self.began).then_some(Missed { who: ring.who, at })
    }
}

/// When a line was written, in milliseconds since the epoch, `now` being
/// when it's read: a time of day later than now was yesterday's. A line
/// with no time is taken as written now.
pub fn written(line: &str, now: i64) -> i64 {
    let Some(time) = time_of_day(line) else {
        return now;
    };
    let at = now - now.rem_euclid(DAY) + time;
    if at > now + AHEAD { at - DAY } else { at }
}

/// A line's time of day in milliseconds, from its "HH:MM:SS.mmm|".
fn time_of_day(line: &str) -> Option<i64> {
    let head = line.as_bytes().get(..13)?;
    if head[2] != b':' || head[5] != b':' || head[8] != b'.' || head[12] != b'|' {
        return None;
    }
    let number = |from: usize, to: usize| -> Option<i64> {
        let digits = &head[from..to];
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| std::str::from_utf8(digits).ok()?.parse().ok())?
    };
    Some(((number(0, 2)? * 60 + number(3, 5)?) * 60 + number(6, 8)?) * 1000 + number(9, 12)?)
}

/// The number Tuple knows someone by, from what fills a line's blank:
/// the blank itself when it's a number, or one it names as an id; or ""
/// when it holds neither.
pub fn number(who: &str) -> &str {
    let bare = who.trim();
    if !bare.is_empty() && bare.bytes().all(|byte| byte.is_ascii_digit()) {
        return bare;
    }
    let bytes = who.as_bytes();
    let mut from = 0;
    while let Some(found) = who[from..].find("id") {
        let start = from + found;
        from = start + 2;
        let bounded =
            start == 0 || (!bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_');
        let rest = &who[from..];
        let digits = rest.trim_start_matches(['=', ':', ' ']);
        if !bounded || digits.len() == rest.len() {
            continue;
        }
        let end = digits
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(digits.len());
        if end > 0 {
            return &digits[..end];
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-03 at noon UTC, in milliseconds since the epoch.
    const NOON: i64 = 1_793_707_200_000;

    fn line(time: &str, message: &str) -> String {
        format!("{time}|1|2|info|calls|{message}")
    }

    #[test]
    fn a_line_is_written_today_unless_that_is_ahead_of_now() {
        assert_eq!(written(&line("11:59:00.250", "x"), NOON), NOON - 59_750);
        assert_eq!(written(&line("12:00:30.000", "x"), NOON), NOON + 30_000);
        assert_eq!(
            written(&line("23:00:00.000", "x"), NOON),
            NOON + 11 * 3_600_000 - DAY
        );
        assert_eq!(written("no time here", NOON), NOON);
    }

    #[test]
    fn someone_calling_rings_until_answered() {
        let mut log = Log::new(NOON - 10_000);
        let heard = log.heard(&line("11:59:55.000", "received incoming call: 7"), NOON);
        assert!(heard.rang);
        assert_eq!(heard.missed, None);
        assert_eq!(
            log.ringing(),
            Some(&Ring {
                who: "7".into(),
                since: NOON - 5_000
            })
        );
        let heard = log.heard(&line("11:59:58.000", "accepting call"), NOON);
        assert_eq!(
            heard,
            Heard {
                rang: true,
                ..Heard::default()
            }
        );
        assert_eq!(log.ringing(), None);
    }

    #[test]
    fn a_ring_turned_away_or_ended_is_missed() {
        for ending in [
            "ignoring incoming call: busy",
            "call ended",
            "invalidating call",
        ] {
            let mut log = Log::new(NOON - 10_000);
            log.heard(&line("11:59:55.000", "received incoming call: 7"), NOON);
            let heard = log.heard(&line("11:59:57.000", ending), NOON);
            assert!(heard.rang, "{ending}");
            assert_eq!(
                heard.missed,
                Some(Missed {
                    who: "7".into(),
                    at: NOON - 3_000
                }),
                "{ending}"
            );
        }
    }

    #[test]
    fn a_ring_is_over_after_a_minute_and_missed() {
        let mut log = Log::new(NOON - 10_000);
        log.heard(&line("11:59:55.000", "received incoming call: 7"), NOON);
        assert_eq!(log.ends(), Some(NOON + 55_000));
        assert_eq!(log.tick(NOON + 54_999), None);
        assert_eq!(
            log.tick(NOON + 55_000),
            Some(Missed {
                who: "7".into(),
                at: NOON + 55_000
            })
        );
        assert_eq!(log.ringing(), None);
        assert_eq!(log.tick(NOON + 60_000), None);
    }

    #[test]
    fn a_ring_over_before_this_started_is_not_news() {
        let mut log = Log::new(NOON);
        log.heard(&line("11:50:00.000", "received incoming call: 7"), NOON);
        assert_eq!(log.ringing(), None);
        log.heard(&line("11:59:30.000", "received incoming call: 8"), NOON);
        assert_eq!(
            log.heard(&line("11:59:40.000", "call ended"), NOON).missed,
            None
        );
    }

    #[test]
    fn a_ring_read_back_still_going_counts() {
        let mut log = Log::new(NOON);
        log.heard(&line("11:59:30.000", "received incoming call: 7"), NOON);
        assert_eq!(log.ringing().map(|ring| ring.since), Some(NOON - 30_000));
        assert_eq!(
            log.heard(&line("12:00:05.000", "call ended"), NOON + 5_000)
                .missed,
            Some(Missed {
                who: "7".into(),
                at: NOON + 5_000
            })
        );
    }

    #[test]
    fn a_ring_after_another_ends_the_first_as_missed() {
        let mut log = Log::new(NOON - 10_000);
        log.heard(&line("11:59:51.000", "received incoming call: 7"), NOON);
        let heard = log.heard(&line("11:59:55.000", "received incoming call: 8"), NOON);
        assert_eq!(heard.missed.map(|missed| missed.who), Some("7".into()));
        assert_eq!(log.ringing().map(|ring| ring.who.as_str()), Some("8"));
    }

    #[test]
    fn tuple_starting_is_said_and_ends_a_ring() {
        let mut log = Log::new(NOON - 10_000);
        log.heard(&line("11:59:55.000", "received incoming call: 7"), NOON);
        let heard = log.heard(
            &line("11:59:56.000", "daemon loop started, waiting for events..."),
            NOON,
        );
        assert!(heard.started);
        assert!(heard.rang);
        assert_eq!(log.ringing(), None);
    }

    #[test]
    fn other_lines_change_nothing() {
        let mut log = Log::new(NOON);
        assert_eq!(
            log.heard(&line("11:59:55.000", "signaler state changed"), NOON),
            Heard::default()
        );
        assert_eq!(log.heard("02:56:08.793|info", NOON), Heard::default());
    }

    #[test]
    fn a_caller_is_numbered_by_the_blank_or_an_id_in_it() {
        assert_eq!(number(" 42 "), "42");
        assert_eq!(number("User(id=42, name=Ada)"), "42");
        assert_eq!(number("peer id: 7"), "7");
        assert_eq!(number("uuid=42"), "");
        assert_eq!(number("Ada"), "");
        assert_eq!(number(""), "");
    }
}
