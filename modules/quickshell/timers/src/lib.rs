//! Joel's timers as the daemon keeps them: each counts a span down to its
//! end, pauses and resumes, rings once its end has passed, and stays,
//! rung, until he ends it or starts it again. Everything here is worked
//! out from the moment it's given, never read from a clock, so the tests
//! hold it to any moment; main.rs brings the clock, the socket and the
//! notices. Beside them it keeps the one stopwatch, counting up from
//! when Joel starts it, with its laps.
//!
//! Every time is milliseconds since the Unix epoch, a time of day rather
//! than time since boot, so a timer kept on disk means the same moment
//! after a restart, a reboot or a suspend.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The daemon interface's version, as the shell's Daemon.qml says hello.
pub const VERSION: u64 = 1;

/// The kind of record each timer is.
pub const KIND: &str = "timer";

/// The kind of record the one stopwatch is, and its key.
pub const STOPWATCH: &str = "stopwatch";

/// The most laps the stopwatch keeps, so what's kept stays small however
/// long it runs.
pub const LAPS: usize = 999;

/// How long past its end a timer may ring and still be on time. Later than
/// that, it ended while the laptop slept or the daemon was down, and says
/// so.
pub const LATE: u64 = 5_000;

/// How long past its end a timer coming due is still heard aloud. Later,
/// as after a reboot or a night's suspend, its notice saying it rang late
/// is enough, and a ding for a tea that went cold hours ago is noise.
pub const HEARD: u64 = 60_000;

/// The longest span a timer takes: a week, past which it's a reminder,
/// not a timer.
pub const LONGEST: u64 = 7 * 24 * 60 * 60 * 1000;

/// The longest name a timer keeps, in characters.
pub const NAMED: usize = 64;

/// How a timer stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Standing {
    /// Counting down to `ends`.
    Running { ends: u64 },
    /// Held with `left` to go.
    Paused { left: u64 },
    /// Its end passed and it rang at `rang`, `late` when that was well
    /// after its end.
    Rung { rang: u64, late: bool },
}

/// One timer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timer {
    pub key: String,
    /// What Joel called it, or nothing, which the shell words as its own.
    pub name: String,
    /// The whole span it counts, as set and with what was added since.
    pub span: u64,
    /// When it was set going, last.
    pub set: u64,
    #[serde(flatten)]
    pub standing: Standing,
    /// The notice its ring was said as, while it's rung and said, kept
    /// with it so a daemon started again still knows its card, and one
    /// said to a notifications server that's since gone is said again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<Notice>,
}

/// A rung timer's notice: its id, and the bus name of the notifications
/// server that gave it, since an id means nothing to another server, as
/// the shell's after it starts again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub id: u32,
    pub server: String,
}

impl Timer {
    /// The record a client is told, as the daemon interface has it.
    pub fn json(&self) -> Value {
        let mut record = json!({
            "key": self.key, "kind": KIND, "name": self.name, "span": self.span,
            "set": self.set,
        });
        let fields = match self.standing {
            Standing::Running { ends } => json!({"state": "running", "ends": ends}),
            Standing::Paused { left } => json!({"state": "paused", "left": left}),
            Standing::Rung { rang, late } => json!({"state": "rung", "rang": rang, "late": late}),
        };
        if let (Some(record), Value::Object(fields)) = (record.as_object_mut(), fields) {
            record.extend(fields);
        }
        record
    }

    /// What it has left to count at `now`, none once it has rung.
    pub fn left(&self, now: u64) -> u64 {
        match self.standing {
            Standing::Running { ends } => ends.saturating_sub(now),
            Standing::Paused { left } => left,
            Standing::Rung { .. } => 0,
        }
    }
}

/// Why an ask about a timer changed nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No timer is kept by this key.
    Unknown(String),
    /// What was asked doesn't fit it: why, by a reason the launcher says in
    /// its own words (words.json's timer and stopwatch), and in a sentence
    /// for anything else that asks.
    Unfit {
        reason: &'static str,
        why: &'static str,
    },
}

impl Refusal {
    /// The daemon interface's error for it.
    pub fn json(&self, id: &Value) -> Value {
        let (code, reason, message) = match self {
            Refusal::Unknown(key) => ("not-found", "gone", format!("no timer {key}")),
            Refusal::Unfit { reason, why } => ("refused", *reason, (*why).to_owned()),
        };
        json!({"id": id, "error": {"code": code, "reason": reason, "message": message}})
    }
}

fn unfit(reason: &'static str, why: &'static str) -> Refusal {
    Refusal::Unfit { reason, why }
}

/// The one stopwatch: what it counted in the stretches it ran before,
/// since when it has run now, while it runs, and its count at each lap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stopwatch {
    pub counted: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    #[serde(default)]
    pub laps: Vec<u64>,
}

impl Stopwatch {
    /// The record a client is told: running, stopped, or unstarted while
    /// it has counted nothing.
    pub fn json(&self) -> Value {
        let state = match self.since {
            Some(_) => "running",
            None if self.counted == 0 && self.laps.is_empty() => "unstarted",
            None => "stopped",
        };
        let mut record = json!({
            "key": STOPWATCH, "kind": STOPWATCH, "state": state, "counted": self.counted,
            "laps": self.laps,
        });
        if let Some(since) = self.since {
            record["since"] = json!(since);
        }
        record
    }

    /// What it has counted by `now`.
    pub fn count(&self, now: u64) -> u64 {
        self.counted + self.since.map_or(0, |since| now.saturating_sub(since))
    }

    /// Sets it counting on from what it has.
    pub fn run(&mut self, now: u64) -> Result<&Stopwatch, Refusal> {
        if self.since.is_some() {
            return Err(unfit("runningAlready", "the stopwatch is running already"));
        }
        self.since = Some(now);
        Ok(self)
    }

    /// Holds what it has counted.
    pub fn stop(&mut self, now: u64) -> Result<&Stopwatch, Refusal> {
        let Some(since) = self.since else {
            return Err(unfit("pausesRunning", "only a running stopwatch pauses"));
        };
        self.counted += now.saturating_sub(since);
        self.since = None;
        Ok(self)
    }

    /// Marks its count now as a lap.
    pub fn lap(&mut self, now: u64) -> Result<&Stopwatch, Refusal> {
        if self.since.is_none() {
            return Err(unfit("lapsRunning", "only a running stopwatch laps"));
        }
        if self.laps.len() >= LAPS {
            return Err(unfit("lapsFull", "the stopwatch keeps 999 laps at most"));
        }
        self.laps.push(self.count(now));
        Ok(self)
    }

    /// Takes it back to nothing counted, its laps with it.
    pub fn reset(&mut self) -> &Stopwatch {
        *self = Stopwatch::default();
        self
    }
}

/// Every timer kept, in the order they were set, the number the next one
/// takes in its key, and the stopwatch, which what was kept before it came
/// lacks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub timers: Vec<Timer>,
    pub next: u64,
    #[serde(default)]
    pub stopwatch: Stopwatch,
}

impl Held {
    /// What was kept at `path`, or nothing kept where there's no file or
    /// it can't be read, which is said.
    pub fn read(path: &Path) -> Held {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|why| {
                eprintln!("<4>couldn't read {}: {why}", path.display());
                Held::default()
            }),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Held::default(),
            Err(why) => {
                eprintln!("<4>couldn't read {}: {why}", path.display());
                Held::default()
            }
        }
    }

    /// Keeps them at `path`, whole or not at all: written beside it and
    /// moved into its place, so a crash midway leaves what was kept before.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let beside = path.with_extension("json.new");
        std::fs::write(&beside, serde_json::to_vec(self)?)?;
        std::fs::rename(&beside, path)
    }

    pub fn get(&self, key: &str) -> Option<&Timer> {
        self.timers.iter().find(|timer| timer.key == key)
    }

    fn find(&mut self, key: &str) -> Result<&mut Timer, Refusal> {
        self.timers
            .iter_mut()
            .find(|timer| timer.key == key)
            .ok_or_else(|| Refusal::Unknown(key.to_owned()))
    }

    /// Sets a new one going for `span`, called `name`.
    pub fn start(&mut self, name: &str, span: u64, now: u64) -> Result<&Timer, Refusal> {
        let span = fitting(span)?;
        self.next += 1;
        self.timers.push(Timer {
            key: format!("{KIND}:{}", self.next),
            name: name.trim().chars().take(NAMED).collect(),
            span,
            set: now,
            standing: Standing::Running { ends: now + span },
            notice: None,
        });
        Ok(&self.timers[self.timers.len() - 1])
    }

    /// Holds a running one where it is.
    pub fn pause(&mut self, key: &str, now: u64) -> Result<&Timer, Refusal> {
        let timer = self.find(key)?;
        let Standing::Running { ends } = timer.standing else {
            return Err(unfit("pausesRunning", "only a running timer pauses"));
        };
        timer.standing = Standing::Paused {
            left: ends.saturating_sub(now),
        };
        Ok(timer)
    }

    /// Sets a paused one counting again from where it was held.
    pub fn resume(&mut self, key: &str, now: u64) -> Result<&Timer, Refusal> {
        let timer = self.find(key)?;
        let Standing::Paused { left } = timer.standing else {
            return Err(unfit("resumesPaused", "only a paused timer resumes"));
        };
        timer.standing = Standing::Running { ends: now + left };
        Ok(timer)
    }

    /// Sets it going afresh for its whole span, however it stands.
    pub fn again(&mut self, key: &str, now: u64) -> Result<&Timer, Refusal> {
        let timer = self.find(key)?;
        timer.set = now;
        timer.standing = Standing::Running {
            ends: now + timer.span,
        };
        timer.notice = None;
        Ok(timer)
    }

    /// Gives it `more` to count: a running or paused one longer, and a rung
    /// one going again for that long.
    pub fn add(&mut self, key: &str, more: u64, now: u64) -> Result<&Timer, Refusal> {
        let more = fitting(more)?;
        let timer = self.find(key)?;
        if timer.left(now) + more > LONGEST {
            return Err(unfit("pastAWeek", "a timer counts a week at most"));
        }
        timer.standing = match timer.standing {
            Standing::Running { ends } => Standing::Running { ends: ends + more },
            Standing::Paused { left } => Standing::Paused { left: left + more },
            Standing::Rung { .. } => {
                timer.set = now;
                timer.notice = None;
                timer.span = 0;
                Standing::Running { ends: now + more }
            }
        };
        timer.span = (timer.span + more).min(LONGEST);
        Ok(timer)
    }

    /// Takes it away, whole.
    pub fn end(&mut self, key: &str) -> Result<Timer, Refusal> {
        let at = self
            .timers
            .iter()
            .position(|timer| timer.key == key)
            .ok_or_else(|| Refusal::Unknown(key.to_owned()))?;
        Ok(self.timers.remove(at))
    }

    /// Whether any running one whose end has passed by `now` passed it
    /// recently enough to be heard, asked before `due` rings them.
    pub fn heard(&self, now: u64) -> bool {
        self.timers.iter().any(|timer| {
            matches!(timer.standing, Standing::Running { ends } if ends <= now && now - ends <= HEARD)
        })
    }

    /// Rings every running one whose end has passed by `now`, saying which.
    pub fn due(&mut self, now: u64) -> Vec<String> {
        let mut rung = Vec::new();
        for timer in &mut self.timers {
            if let Standing::Running { ends } = timer.standing
                && ends <= now
            {
                timer.standing = Standing::Rung {
                    rang: now,
                    late: now - ends > LATE,
                };
                timer.notice = None;
                rung.push(timer.key.clone());
            }
        }
        rung
    }

    /// The soonest end of those running, when the daemon next wakes.
    pub fn next_end(&self) -> Option<u64> {
        self.timers
            .iter()
            .filter_map(|timer| match timer.standing {
                Standing::Running { ends } => Some(ends),
                _ => None,
            })
            .min()
    }

    /// Those rung and not said as a notice to `server`, the notifications
    /// server there is now: never said, or said to one that's gone.
    pub fn unsaid(&self, server: &str) -> Vec<String> {
        self.timers
            .iter()
            .filter(|timer| unsaid(timer, server))
            .map(|timer| timer.key.clone())
            .collect()
    }

    /// The timer whose notice `server` gave as `id`.
    pub fn noticed(&self, id: u32, server: &str) -> Option<&Timer> {
        self.timers.iter().find(|timer| {
            timer
                .notice
                .as_ref()
                .is_some_and(|notice| notice.id == id && notice.server == server)
        })
    }
}

/// Whether a timer is rung and not said as a notice to `server`.
pub fn unsaid(timer: &Timer, server: &str) -> bool {
    matches!(timer.standing, Standing::Rung { .. })
        && timer
            .notice
            .as_ref()
            .is_none_or(|notice| notice.server != server)
}

/// A span a timer can count: more than nothing and a week at most.
fn fitting(span: u64) -> Result<u64, Refusal> {
    if span == 0 {
        Err(unfit("nothing", "a timer counts more than nothing"))
    } else if span > LONGEST {
        Err(unfit("pastAWeek", "a timer counts a week at most"))
    } else {
        Ok(span)
    }
}

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Every record of the kinds asked, all of them when none is, then
    /// each change to one.
    Watch {
        id: Value,
        kinds: Vec<String>,
    },
    Unwatch {
        id: Value,
    },
    Get {
        id: Value,
        keys: Vec<String>,
    },
    Start {
        id: Value,
        name: String,
        span: u64,
    },
    Pause {
        id: Value,
        key: String,
    },
    Resume {
        id: Value,
        key: String,
    },
    Again {
        id: Value,
        key: String,
    },
    Add {
        id: Value,
        key: String,
        span: u64,
    },
    End {
        id: Value,
        key: String,
    },
    /// The stopwatch's asks.
    Run {
        id: Value,
    },
    Stop {
        id: Value,
    },
    Lap {
        id: Value,
    },
    Reset {
        id: Value,
    },
    /// A watch of a kind it doesn't keep.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// An ask missing what it needs, in a sentence.
    Malformed {
        id: Value,
        why: &'static str,
    },
    Unknown {
        id: Value,
        ask: String,
    },
}

/// Reads one line a client said.
pub fn ask(line: &str) -> Ask {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Ask::Unknown {
            id: Value::Null,
            ask: String::new(),
        };
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said.get("ask").and_then(Value::as_str).unwrap_or("");
    let key = said.get("key").and_then(Value::as_str).map(str::to_owned);
    let span = said.get("span").and_then(Value::as_u64);
    let keyed = |make: fn(Value, String) -> Ask| match key.clone() {
        Some(key) => make(id.clone(), key),
        None => Ask::Malformed {
            id: id.clone(),
            why: "it names no key",
        },
    };
    match asked {
        "hello" => Ask::Hello { id },
        "watch" => {
            let kinds = kinds(said.get("kind"));
            match kinds.iter().find(|kind| ![KIND, STOPWATCH].contains(kind)) {
                Some(kind) => Ask::UnknownKind {
                    id,
                    kind: (*kind).to_owned(),
                },
                None => Ask::Watch {
                    id,
                    kinds: kinds.into_iter().map(str::to_owned).collect(),
                },
            }
        }
        "unwatch" => Ask::Unwatch { id },
        "get" => Ask::Get {
            keys: said
                .get("keys")
                .and_then(Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            id,
        },
        "start" => match span {
            Some(span) => Ask::Start {
                name: said
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                span,
                id,
            },
            None => Ask::Malformed {
                id,
                why: "it gives no span",
            },
        },
        "pause" => keyed(|id, key| Ask::Pause { id, key }),
        "resume" => keyed(|id, key| Ask::Resume { id, key }),
        "again" => keyed(|id, key| Ask::Again { id, key }),
        "end" => keyed(|id, key| Ask::End { id, key }),
        "run" => Ask::Run { id },
        "stop" => Ask::Stop { id },
        "lap" => Ask::Lap { id },
        "reset" => Ask::Reset { id },
        "add" => match (key, span) {
            (Some(key), Some(span)) => Ask::Add { id, key, span },
            _ => Ask::Malformed {
                id,
                why: "it needs a key and a span",
            },
        },
        other => Ask::Unknown {
            id,
            ask: other.to_owned(),
        },
    }
}

/// The kinds a watch asks for, given alone or as a list, as the shell's
/// Daemon.qml sends them.
fn kinds(kind: Option<&Value>) -> Vec<&str> {
    match kind {
        Some(Value::String(kind)) => vec![kind.as_str()],
        Some(Value::Array(kinds)) => kinds.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Whether a watch of `kinds` is told of a record of `kind`: every kind,
/// when it named none.
pub fn watched(kinds: &[String], kind: &str) -> bool {
    kinds.is_empty() || kinds.iter().any(|watched| watched == kind)
}

/// The daemon interface's error, by its code.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// The words a notice says, from the launcher's vocabulary, as home.nix
/// gives them: who signs it, a timer with no name of its own, what it says
/// on time and late, Again's act, and the units a span is said in.
#[derive(Clone, Debug, Deserialize)]
pub struct Words {
    pub app: String,
    pub unnamed: String,
    pub rang: String,
    pub late: String,
    pub again: String,
    pub hours: String,
    pub minutes: String,
    pub seconds: String,
}

impl Words {
    /// What a rung timer's notice says: its name, starting with a capital
    /// as the launcher's row names it, then what it was.
    pub fn notice(&self, timer: &Timer) -> (String, String) {
        let mut letters = timer.name.chars();
        let name = match letters.next() {
            Some(first) => first.to_uppercase().chain(letters).collect(),
            None => self.unnamed.clone(),
        };
        let late = matches!(timer.standing, Standing::Rung { late: true, .. });
        let said = if late { &self.late } else { &self.rang };
        (name, said.replace("{span}", &self.span(timer.span)))
    }

    /// A span in its largest units, rounded to the second: "10 min",
    /// "1 h 30 min", "1 min 30 s".
    pub fn span(&self, span: u64) -> String {
        let seconds = span.div_ceil(1000);
        let parts = [
            (seconds / 3600, &self.hours),
            (seconds / 60 % 60, &self.minutes),
            (seconds % 60, &self.seconds),
        ];
        let said: Vec<String> = parts
            .iter()
            .filter(|(count, _)| *count > 0)
            .map(|(count, unit)| format!("{count} {unit}"))
            .collect();
        if said.is_empty() {
            format!("0 {}", self.seconds)
        } else {
            said.join(" ")
        }
    }
}

/// Now, as a time of day.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: u64 = 60_000;

    fn words() -> Words {
        Words {
            app: "Timers".to_owned(),
            unnamed: "Timer".to_owned(),
            rang: "{span} timer".to_owned(),
            late: "{span} timer, rang late".to_owned(),
            again: "Again".to_owned(),
            hours: "h".to_owned(),
            minutes: "min".to_owned(),
            seconds: "s".to_owned(),
        }
    }

    #[test]
    fn a_timer_started_counts_its_span_from_now_and_ends_then() {
        let mut held = Held::default();
        let timer = held.start(" tea ", 10 * MINUTE, 1_000).unwrap().clone();
        assert_eq!(timer.key, "timer:1");
        assert_eq!(timer.name, "tea");
        assert_eq!(timer.standing, Standing::Running { ends: 601_000 });
        assert_eq!(held.next_end(), Some(601_000));
        assert_eq!(held.start("eggs", MINUTE, 2_000).unwrap().key, "timer:2");
        assert_eq!(held.next_end(), Some(62_000));
    }

    #[test]
    fn a_span_of_nothing_or_past_a_week_is_refused() {
        let mut held = Held::default();
        assert!(matches!(
            held.start("", 0, 0),
            Err(Refusal::Unfit {
                reason: "nothing",
                ..
            })
        ));
        assert!(matches!(
            held.start("", LONGEST + 1, 0),
            Err(Refusal::Unfit {
                reason: "pastAWeek",
                ..
            })
        ));
        assert!(held.timers.is_empty());
    }

    #[test]
    fn a_long_name_is_cut_to_what_it_keeps() {
        let mut held = Held::default();
        let name = "é".repeat(NAMED + 10);
        assert_eq!(
            held.start(&name, MINUTE, 0).unwrap().name.chars().count(),
            NAMED
        );
    }

    #[test]
    fn a_paused_timer_holds_what_it_had_left_and_resumes_from_there() {
        let mut held = Held::default();
        held.start("tea", 10 * MINUTE, 0).unwrap();
        let paused = held.pause("timer:1", 4 * MINUTE).unwrap().clone();
        assert_eq!(paused.standing, Standing::Paused { left: 6 * MINUTE });
        assert_eq!(paused.left(100 * MINUTE), 6 * MINUTE);
        assert_eq!(held.next_end(), None);
        assert_eq!(held.due(100 * MINUTE), Vec::<String>::new());
        let resumed = held.resume("timer:1", 20 * MINUTE).unwrap();
        assert_eq!(resumed.standing, Standing::Running { ends: 26 * MINUTE });
    }

    #[test]
    fn pausing_or_resuming_what_doesnt_fit_is_refused() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        assert!(matches!(
            held.resume("timer:1", 0),
            Err(Refusal::Unfit {
                reason: "resumesPaused",
                ..
            })
        ));
        assert_eq!(
            held.pause("timer:9", 0),
            Err(Refusal::Unknown("timer:9".to_owned()))
        );
    }

    #[test]
    fn a_timer_coming_due_is_heard_only_within_a_minute_of_its_end() {
        let mut held = Held::default();
        assert!(!held.heard(MINUTE));
        held.start("tea", MINUTE, 0).unwrap();
        assert!(!held.heard(MINUTE - 1));
        assert!(held.heard(MINUTE));
        assert!(held.heard(MINUTE + HEARD));
        assert!(!held.heard(MINUTE + HEARD + 1));
        // One heard among those due is enough to sound the ring once.
        held.start("eggs", MINUTE + HEARD, 0).unwrap();
        assert!(held.heard(MINUTE + HEARD + 1));
        held.due(MINUTE + HEARD + 1);
        assert!(!held.heard(MINUTE + HEARD + 1));
    }

    #[test]
    fn a_timer_rings_once_its_end_passes_saying_whether_it_was_late() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        held.start("eggs", 2 * MINUTE, 0).unwrap();
        assert_eq!(held.due(MINUTE - 1), Vec::<String>::new());
        assert_eq!(held.due(MINUTE), vec!["timer:1".to_owned()]);
        assert_eq!(
            held.get("timer:1").unwrap().standing,
            Standing::Rung {
                rang: MINUTE,
                late: false
            }
        );
        assert_eq!(held.due(MINUTE + 1), Vec::<String>::new());
        assert_eq!(held.due(10 * MINUTE), vec!["timer:2".to_owned()]);
        assert_eq!(
            held.get("timer:2").unwrap().standing,
            Standing::Rung {
                rang: 10 * MINUTE,
                late: true
            }
        );
        assert_eq!(held.unsaid(":1.4"), vec!["timer:1", "timer:2"]);
        held.timers[0].notice = Some(Notice {
            id: 7,
            server: ":1.4".to_owned(),
        });
        assert_eq!(held.unsaid(":1.4"), vec!["timer:2"]);
        // Said to a server that's gone, it's unsaid to the one after it.
        assert_eq!(held.unsaid(":1.9"), vec!["timer:1", "timer:2"]);
        assert_eq!(held.noticed(7, ":1.4").unwrap().key, "timer:1");
        assert_eq!(held.noticed(7, ":1.9"), None);
        assert_eq!(held.next_end(), None);
    }

    #[test]
    fn again_sets_it_going_for_its_whole_span_however_it_stands() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        held.due(MINUTE);
        held.timers[0].notice = Some(Notice {
            id: 7,
            server: ":1.4".to_owned(),
        });
        let again = held.again("timer:1", 5 * MINUTE).unwrap();
        assert_eq!(again.standing, Standing::Running { ends: 6 * MINUTE });
        assert_eq!(again.set, 5 * MINUTE);
        assert_eq!(again.notice, None);
    }

    #[test]
    fn adding_lengthens_one_counting_and_sets_a_rung_one_going() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        let added = held.add("timer:1", MINUTE, 30_000).unwrap().clone();
        assert_eq!(added.standing, Standing::Running { ends: 2 * MINUTE });
        assert_eq!(added.span, 2 * MINUTE);
        held.pause("timer:1", MINUTE).unwrap();
        let paused = held.add("timer:1", MINUTE, MINUTE).unwrap().clone();
        assert_eq!(paused.standing, Standing::Paused { left: 2 * MINUTE });
        held.resume("timer:1", MINUTE).unwrap();
        held.due(3 * MINUTE);
        let again = held.add("timer:1", MINUTE, 4 * MINUTE).unwrap();
        assert_eq!(again.standing, Standing::Running { ends: 5 * MINUTE });
        assert_eq!(again.span, MINUTE);
    }

    #[test]
    fn ending_takes_it_away_and_an_unknown_one_is_refused() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        assert_eq!(held.end("timer:1").unwrap().name, "tea");
        assert!(held.timers.is_empty());
        assert_eq!(
            held.end("timer:1"),
            Err(Refusal::Unknown("timer:1".to_owned()))
        );
        // A key isn't given twice, even once the one holding it has gone.
        assert_eq!(held.start("", MINUTE, 0).unwrap().key, "timer:2");
    }

    #[test]
    fn what_is_kept_reads_back_as_it_was() {
        let folder = std::env::temp_dir().join(format!("timers-kept-{}", std::process::id()));
        let path = folder.join("held.json");
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        held.start("eggs", MINUTE, 0).unwrap();
        held.pause("timer:2", 1_000).unwrap();
        held.stopwatch.run(0).unwrap();
        held.stopwatch.lap(500).unwrap();
        held.write(&path).unwrap();
        assert_eq!(Held::read(&path), held);
        assert_eq!(Held::read(&folder.join("none.json")), Held::default());
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_record_says_how_it_stands_in_the_interfaces_fields() {
        let mut held = Held::default();
        held.start("tea", MINUTE, 0).unwrap();
        assert_eq!(
            held.timers[0].json(),
            json!({"key": "timer:1", "kind": "timer", "name": "tea", "span": MINUTE,
                "set": 0, "state": "running", "ends": MINUTE})
        );
        held.due(MINUTE);
        assert_eq!(held.timers[0].json()["state"], "rung");
        assert_eq!(held.timers[0].json()["late"], false);
    }

    #[test]
    fn a_notice_names_the_timer_and_says_its_span_in_its_largest_units() {
        let words = words();
        let mut held = Held::default();
        held.start("", 90 * MINUTE, 0).unwrap();
        held.start("tea", 10 * MINUTE, 0).unwrap();
        held.due(10 * MINUTE);
        assert_eq!(
            words.notice(&held.timers[1]),
            ("Tea".to_owned(), "10 min timer".to_owned())
        );
        held.due(100 * MINUTE);
        assert_eq!(
            words.notice(&held.timers[0]),
            ("Timer".to_owned(), "1 h 30 min timer, rang late".to_owned())
        );
        assert_eq!(words.span(90_000), "1 min 30 s");
        assert_eq!(words.span(1), "1 s");
    }

    #[test]
    fn an_ask_is_read_with_what_it_needs_or_refused() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"start","span":600000,"name":"tea"}"#),
            Ask::Start {
                id: json!(1),
                name: "tea".to_owned(),
                span: 600_000
            }
        );
        assert_eq!(
            ask(r#"{"id":2,"ask":"pause","key":"timer:1"}"#),
            Ask::Pause {
                id: json!(2),
                key: "timer:1".to_owned()
            }
        );
        assert!(matches!(
            ask(r#"{"id":3,"ask":"pause"}"#),
            Ask::Malformed { .. }
        ));
        assert!(matches!(
            ask(r#"{"id":4,"ask":"start"}"#),
            Ask::Malformed { .. }
        ));
        assert_eq!(
            ask(r#"{"id":"all","ask":"watch","kind":["timer","stopwatch"]}"#),
            Ask::Watch {
                id: json!("all"),
                kinds: vec!["timer".to_owned(), "stopwatch".to_owned()]
            }
        );
        assert_eq!(ask(r#"{"id":5,"ask":"lap"}"#), Ask::Lap { id: json!(5) });
        assert_eq!(
            ask(r#"{"id":"all","ask":"watch","kind":"person"}"#),
            Ask::UnknownKind {
                id: json!("all"),
                kind: "person".to_owned()
            }
        );
        assert!(matches!(ask("not json"), Ask::Unknown { .. }));
    }

    #[test]
    fn a_watch_hears_only_the_kinds_it_asked_for_or_every_kind() {
        let timers = vec!["timer".to_owned()];
        assert!(watched(&timers, KIND));
        assert!(!watched(&timers, STOPWATCH));
        assert!(watched(&[], STOPWATCH));
    }

    #[test]
    fn the_stopwatch_counts_while_it_runs_and_holds_its_count_stopped() {
        let mut held = Held::default();
        assert_eq!(held.stopwatch.json()["state"], "unstarted");
        held.stopwatch.run(1_000).unwrap();
        assert_eq!(held.stopwatch.count(4_000), 3_000);
        assert!(matches!(
            held.stopwatch.run(4_000),
            Err(Refusal::Unfit {
                reason: "runningAlready",
                ..
            })
        ));
        held.stopwatch.stop(5_000).unwrap();
        assert_eq!(held.stopwatch.count(60_000), 4_000);
        assert_eq!(held.stopwatch.json()["state"], "stopped");
        assert!(matches!(
            held.stopwatch.stop(6_000),
            Err(Refusal::Unfit {
                reason: "pausesRunning",
                ..
            })
        ));
        held.stopwatch.run(10_000).unwrap();
        assert_eq!(
            held.stopwatch.json(),
            json!({"key": "stopwatch", "kind": "stopwatch", "state": "running",
                "counted": 4_000, "since": 10_000, "laps": []})
        );
        assert_eq!(held.stopwatch.count(11_000), 5_000);
    }

    #[test]
    fn a_lap_marks_its_count_only_while_it_runs_and_reset_clears_them() {
        let mut held = Held::default();
        assert!(matches!(
            held.stopwatch.lap(0),
            Err(Refusal::Unfit {
                reason: "lapsRunning",
                ..
            })
        ));
        held.stopwatch.run(0).unwrap();
        held.stopwatch.lap(2_000).unwrap();
        held.stopwatch.stop(3_000).unwrap();
        held.stopwatch.run(10_000).unwrap();
        held.stopwatch.lap(12_000).unwrap();
        assert_eq!(held.stopwatch.laps, vec![2_000, 5_000]);
        held.stopwatch.laps = vec![0; LAPS];
        assert!(matches!(
            held.stopwatch.lap(13_000),
            Err(Refusal::Unfit {
                reason: "lapsFull",
                ..
            })
        ));
        held.stopwatch.reset();
        assert_eq!(held.stopwatch, Stopwatch::default());
    }

    #[test]
    fn what_was_kept_before_the_stopwatch_came_reads_with_one_unstarted() {
        let folder = std::env::temp_dir().join(format!("timers-older-{}", std::process::id()));
        let path = folder.join("held.json");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&path, r#"{"timers":[],"next":3}"#).unwrap();
        let held = Held::read(&path);
        assert_eq!(held.next, 3);
        assert_eq!(held.stopwatch, Stopwatch::default());
        std::fs::remove_dir_all(folder).unwrap();
    }
}
