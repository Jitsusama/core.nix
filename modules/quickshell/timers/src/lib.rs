//! Joel's timers as the daemon keeps them: each counts a span down to its
//! end, pauses and resumes, rings once its end has passed, and stays,
//! rung, until he ends it or starts it again. Everything here is worked
//! out from the moment it's given, never read from a clock, so the tests
//! hold it to any moment; main.rs brings the clock, answering.rs the
//! socket and ringing.rs the notices. Beside them it keeps the one
//! stopwatch, counting up from when Joel starts it, with its laps.
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

/// A rung timer's notice: its id, and the notifications server that gave
/// it, as `on_bus` says one, since an id means nothing to another server,
/// as the shell's after it starts again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub id: u32,
    pub server: String,
}

/// A server as a notice holds it: the bus's own id and the server's name
/// there, since a bus begun again hands out the same names from the
/// start, and a name alone would match a server gone with the last bus.
/// Kept while the bus is dialled again, it still matches the server that
/// stayed, so its card isn't said twice.
pub fn on_bus(bus: &str, name: &str) -> String {
    format!("{bus}/{name}")
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
            let Standing::Running { ends } = timer.standing else {
                return false;
            };
            ends <= now && now - ends <= HEARD
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
#[path = "lib_tests.rs"]
mod tests;
