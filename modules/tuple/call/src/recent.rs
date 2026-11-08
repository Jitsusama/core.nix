//! The calls Joel had lately, newest first, for Tuple's Recent in the
//! launcher: each call that ended, with everyone who was in it at any
//! point, when it connected and when it ended, and each call missed, with
//! its caller, when it rang and when the ring ended. Tuple keeps no such
//! history itself: its stream says only the call under way, its log only
//! rings, a time of day with no date, and its contacts only whether
//! someone was called lately. So tuple-call keeps it, a JSON line a call
//! in `tuple-call/recent.jsonl` in the state folder, so it outlives a
//! restart, and a line that can't be read is passed over rather than
//! losing the rest.

use serde_json::{Value, json};

/// How many calls are kept, ended and missed together: as many as the
/// launcher's Recent lists, nine, so 1 to 9 pick one (Recents.qml), and
/// nothing else reads them, so a longer history would be kept for nobody.
pub const KEPT: usize = 9;

/// Someone a call was with, as Tuple knew them then.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Someone {
    /// The id Tuple knows them by, or "" when the log didn't say it.
    pub id: String,
    pub name: String,
    pub email: String,
}

impl Someone {
    /// Whether it says nothing of who they were, as a missed call from a
    /// caller the log doesn't number does.
    pub fn is_nobody(&self) -> bool {
        self.id.is_empty() && self.name.is_empty() && self.email.is_empty()
    }

    fn json(&self) -> Value {
        json!({"id": self.id, "name": self.name, "email": self.email})
    }

    fn read(value: &Value) -> Option<Someone> {
        let words = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        Some(Someone {
            id: words("id")?,
            name: words("name")?,
            email: words("email")?,
        })
    }
}

/// One call that's over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Past {
    /// Everyone else who was in it, in the order they came, or the caller
    /// of a missed call; none for a caller the log doesn't number.
    pub people: Vec<Someone>,
    /// When it connected, or when a missed call began to ring, in
    /// milliseconds since the epoch.
    pub since: i64,
    /// When it ended, or when the ring did.
    pub ended: i64,
    /// Whether it rang and wasn't answered.
    pub missed: bool,
}

impl Past {
    /// As the record says it and its file keeps it.
    pub fn json(&self) -> Value {
        json!({
            "people": self.people.iter().map(Someone::json).collect::<Vec<_>>(),
            "since": self.since,
            "ended": self.ended,
            "missed": self.missed,
        })
    }

    /// One the file kept, or none from a line that isn't one.
    pub fn read(line: &str) -> Option<Past> {
        let value: Value = serde_json::from_str(line).ok()?;
        let people = value
            .get("people")?
            .as_array()?
            .iter()
            .map(Someone::read)
            .collect::<Option<Vec<_>>>()?;
        Some(Past {
            people,
            since: value.get("since")?.as_i64()?,
            ended: value.get("ended")?.as_i64()?,
            missed: value.get("missed")?.as_bool()?,
        })
    }
}

/// The calls kept, newest first by when each began, as the launcher
/// lists them by the time each says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    pub calls: Vec<Past>,
}

impl Recent {
    /// The calls a file kept, its unreadable lines passed over.
    pub fn read(text: &str) -> Recent {
        let mut recent = Recent::default();
        for past in text.lines().filter_map(Past::read) {
            recent.keep(past);
        }
        recent
    }

    /// The file's text: a line a call, newest first.
    pub fn lines(&self) -> String {
        let mut text = String::new();
        for past in &self.calls {
            text.push_str(&past.json().to_string());
            text.push('\n');
        }
        text
    }

    /// Keeps a call in its place by when it began, letting the oldest go
    /// past [`KEPT`].
    pub fn keep(&mut self, past: Past) {
        let at = self
            .calls
            .iter()
            .position(|kept| kept.since < past.since)
            .unwrap_or(self.calls.len());
        self.calls.insert(at, past);
        self.calls.truncate(KEPT);
    }

    /// Takes back the call that began at `since` and was kept as ended,
    /// for a call Tuple's stream says again after it stopped saying it, as
    /// it does when its follow ends while the call goes on: Tuple's start
    /// is the call's own, so the same start is the same call.
    pub fn reopen(&mut self, since: i64) -> Option<Past> {
        let at = self
            .calls
            .iter()
            .position(|kept| !kept.missed && kept.since == since)?;
        Some(self.calls.remove(at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ada() -> Someone {
        Someone {
            id: "7".into(),
            name: "Ada Brightwater".into(),
            email: "ada@example.com".into(),
        }
    }

    fn call(since: i64, missed: bool) -> Past {
        Past {
            people: vec![ada()],
            since,
            ended: since + 60_000,
            missed,
        }
    }

    #[test]
    fn calls_are_kept_newest_first_by_when_each_began() {
        let mut recent = Recent::default();
        for since in [2_000, 5_000, 1_000, 4_000] {
            recent.keep(call(since, since == 4_000));
        }
        let order: Vec<_> = recent.calls.iter().map(|past| past.since).collect();
        assert_eq!(order, [5_000, 4_000, 2_000, 1_000]);
    }

    #[test]
    fn only_the_newest_are_kept() {
        let mut recent = Recent::default();
        for since in 0..20 {
            recent.keep(call(since, since % 3 == 0));
        }
        assert_eq!(recent.calls.len(), KEPT);
        assert_eq!(recent.calls[0].since, 19);
        assert_eq!(recent.calls[KEPT - 1].since, 11);
        // One older than all those kept goes as it comes.
        recent.keep(call(3, false));
        assert_eq!(recent.calls[KEPT - 1].since, 11);
    }

    #[test]
    fn what_was_written_reads_back() {
        let mut recent = Recent::default();
        recent.keep(call(1_000, true));
        recent.keep(Past {
            people: vec![
                ada(),
                Someone {
                    id: "8".into(),
                    ..Someone::default()
                },
            ],
            since: 2_000,
            ended: 2_520_000,
            missed: false,
        });
        assert_eq!(Recent::read(&recent.lines()), recent);
    }

    #[test]
    fn a_line_that_cant_be_read_is_passed_over() {
        let good = call(1_000, false).json().to_string();
        let text = format!(
            "{good}\n{{\"people\":[],\"since\":\n\nnot json\n{}\n\
             {{\"people\":[{{\"id\":7}}],\"since\":1,\"ended\":2,\"missed\":false}}\n",
            call(3_000, true).json()
        );
        assert_eq!(
            Recent::read(&text).calls,
            [call(3_000, true), call(1_000, false)]
        );
    }

    #[test]
    fn a_call_said_again_is_taken_back_and_a_missed_one_never() {
        let mut recent = Recent::default();
        recent.keep(call(1_000, true));
        recent.keep(call(2_000, false));
        assert_eq!(recent.reopen(1_000), None);
        assert_eq!(recent.reopen(2_000), Some(call(2_000, false)));
        assert_eq!(recent.calls, [call(1_000, true)]);
    }
}
