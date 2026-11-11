//! The processor's own telemetry, as processor-telemetry tells it over
//! varlink on the socket systemd holds for it each second: the
//! temperatures of its NPU, image processor and media and display
//! engines, which no hwmon chip gives and Heat's rows say beside the
//! processor's ([`crate::parts`]), and what held the processor back and
//! kept it from resting, which the Processor's rows say
//! ([`crate::telemetry_rows`]). The reader reads the processor's counters
//! once a second for as long as it's asked and nothing otherwise, so it's
//! asked only while Heat's rows are watched or the shell watches
//! `telemetry`, as it does while a Processor answer can be seen, as the
//! shell's `ProcessorTelemetry.qml` asked it while either was shown.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

/// Where systemd holds the reader's socket.
pub const PATH: &str = "/run/processor-telemetry/io.jitsusama.ProcessorTelemetry";

/// The method that tells each second's reading for as long as the
/// connection stays open.
const WATCH: &str = "io.jitsusama.ProcessorTelemetry.Watch";

/// A varlink message ends at a NUL.
const END: u8 = 0;

/// How many seconds' readings of what kept the package from resting are
/// kept, so what's said of them is their mean and doesn't jump each
/// second near a twentieth.
const RESTED: usize = 10;

/// The reading one reply says, `{ temperatures, heldBack, clusters, npu,
/// keptFromResting }`, or none where the reply is an error, as a
/// processor with no telemetry answers `NoRegion`, or isn't a reading at
/// all.
pub fn reading(reply: &[u8]) -> Option<Value> {
    let said: Value = serde_json::from_slice(reply).ok()?;
    if said.get("error").is_some() {
        return None;
    }
    said.get("parameters")
        .filter(|said| said.is_object())
        .cloned()
}

/// How the telemetry stands for whoever says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Asked, and not yet answered since.
    Asking,
    /// Answered with a reading.
    Heard,
    /// It hasn't answered since it was last heard: refused, gone, or
    /// saying it has no telemetry. A row already shown stays where it
    /// is, saying so, rather than leaving and moving the rows under it.
    Refused,
    /// It has refused without ever being heard since the daemon began,
    /// as on a machine with no reader, so what only it says is left out.
    Missing,
}

impl Standing {
    /// Its name, as the `telemetry` record says it.
    pub fn name(self) -> &'static str {
        match self {
            Standing::Asking => "asking",
            Standing::Heard => "heard",
            Standing::Refused => "refused",
            Standing::Missing => "missing",
        }
    }
}

/// The one `telemetry` record, saying how the asking stands by `state`,
/// a [`Standing`]'s name.
pub fn record(state: &str) -> Value {
    json!({ "key": "telemetry:processor", "kind": "telemetry", "state": state })
}

/// What the telemetry last said: its reading, the last seconds' of what
/// kept the package from resting, oldest first, and whether it's
/// answered, kept while it's asked and dropped once it isn't, so nothing
/// stale is ever said.
#[derive(Debug, Default)]
pub struct Heard {
    reading: Option<Value>,
    rested: VecDeque<Value>,
    refused: bool,
    once: bool,
}

impl Heard {
    /// One reply: a reading, or none where it erred or the connection
    /// ended.
    pub fn heard(&mut self, reading: Option<Value>) {
        let Some(reading) = reading else {
            self.reading = None;
            self.rested.clear();
            self.refused = true;
            return;
        };
        if let Some(resting) = reading
            .get("keptFromResting")
            .filter(|kept| kept.is_object())
        {
            self.rested.push_back(resting.clone());
            while self.rested.len() > RESTED {
                self.rested.pop_front();
            }
        }
        self.reading = Some(reading);
        self.refused = false;
        self.once = true;
    }

    /// Lets what it said go once nothing asks it, keeping only whether
    /// it was ever heard.
    pub fn dropped(&mut self) {
        self.reading = None;
        self.rested.clear();
        self.refused = false;
    }

    /// The last reading, while one stands.
    pub fn reading(&self) -> Option<&Value> {
        self.reading.as_ref()
    }

    /// The last seconds' readings of what kept the package from resting,
    /// oldest first.
    pub fn rested(&self) -> &VecDeque<Value> {
        &self.rested
    }

    /// The temperatures the last reading says, each `{ part, celsius }`.
    pub fn temperatures(&self) -> Vec<Value> {
        self.reading
            .as_ref()
            .and_then(|reading| reading["temperatures"].as_array().cloned())
            .unwrap_or_default()
    }

    /// How it stands.
    pub fn standing(&self) -> Standing {
        match (self.refused, self.once, self.reading.is_some()) {
            (true, false, _) => Standing::Missing,
            (true, true, _) => Standing::Refused,
            (false, _, true) => Standing::Heard,
            (false, _, false) => Standing::Asking,
        }
    }
}

/// Asks the reader at `path` to watch, and tells `heard` each reading,
/// or none when it errs, until `stop` is set or the connection ends;
/// then tells `heard` none, so nothing stale is kept. Returns whether it
/// was heard at all, so a caller can tell a machine with no reader from
/// one that went.
pub fn watch(path: &Path, stop: &AtomicBool, mut heard: impl FnMut(Option<Value>)) -> bool {
    let Ok(mut stream) = UnixStream::connect(path) else {
        heard(None);
        return false;
    };
    // zlink closes the connection on empty parameters given to a method
    // that takes none, so none are sent.
    let mut asked = json!({ "method": WATCH, "more": true })
        .to_string()
        .into_bytes();
    asked.push(END);
    if stream.write_all(&asked).is_err() {
        heard(None);
        return false;
    }
    let mut reader = BufReader::new(stream);
    let mut reply = Vec::new();
    let mut once = false;
    while !stop.load(Ordering::Relaxed) {
        reply.clear();
        match reader.read_until(END, &mut reply) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if reply.last() == Some(&END) {
                    reply.pop();
                }
                let said = reading(&reply);
                once |= said.is_some();
                heard(said);
            }
        }
    }
    heard(None);
    once
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    #[test]
    fn a_reply_says_its_reading() {
        let reply = br#"{"parameters":{"temperatures":[{"part":"npu","celsius":62}]}}"#;
        assert_eq!(
            reading(reply),
            Some(json!({ "temperatures": [{ "part": "npu", "celsius": 62 }] }))
        );
    }

    #[test]
    fn an_error_or_nonsense_says_none() {
        let error = br#"{"error":"io.jitsusama.ProcessorTelemetry.NoRegion"}"#;
        assert_eq!(reading(error), None);
        assert_eq!(reading(b"not json"), None);
    }

    #[test]
    fn what_was_heard_stands_until_it_refuses_and_is_missing_if_never_heard() {
        let mut heard = Heard::default();
        assert_eq!(
            heard.standing(),
            Standing::Asking,
            "asked, not yet answered"
        );
        heard.heard(None);
        assert_eq!(heard.standing(), Standing::Missing, "never heard");
        heard.dropped();
        assert_eq!(heard.standing(), Standing::Asking, "asked afresh");
        let resting = |kept: f64| json!({ "keptFromResting": { "kept": kept, "by": [] } });
        for second in 0..12 {
            heard.heard(Some(resting(f64::from(second))));
        }
        assert_eq!(heard.standing(), Standing::Heard);
        assert_eq!(heard.rested().len(), 10, "the last ten seconds' at most");
        assert_eq!(heard.rested()[0]["kept"], 2.0, "oldest first");
        heard.heard(None);
        assert_eq!(heard.standing(), Standing::Refused, "heard once, then not");
        assert!(
            heard.reading().is_none() && heard.rested().is_empty(),
            "nothing stale"
        );
    }

    #[test]
    fn a_watch_tells_each_reading_then_none_when_it_ends() {
        let folder = std::env::temp_dir().join(format!("telemetry-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder for the socket");
        let path = folder.join("socket");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("the reader's socket");
        let reader = thread::spawn(move || {
            let (mut client, _) = listener.accept().expect("a client");
            let mut asked = [0u8; 128];
            let read = client.read(&mut asked).expect("what it asked");
            let said = String::from_utf8_lossy(&asked[..read]).into_owned();
            client
                .write_all(
                    b"{\"parameters\":{\"temperatures\":[{\"part\":\"npu\",\"celsius\":62}]}}\0",
                )
                .expect("a reading");
            client
                .write_all(b"{\"error\":\"NoRegion\"}\0")
                .expect("an error");
            said
        });
        let mut heard = Vec::new();
        let once = watch(&path, &AtomicBool::new(false), |said| heard.push(said));
        let asked = reader.join().expect("the reader");
        assert!(
            asked.contains(WATCH) && asked.ends_with('\0'),
            "it asked to watch: {asked}"
        );
        assert!(once, "it was heard");
        assert_eq!(
            heard,
            [
                Some(json!({ "temperatures": [{ "part": "npu", "celsius": 62 }] })),
                None,
                None
            ]
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn no_reader_is_none_and_unheard() {
        let mut heard = Vec::new();
        let path = Path::new("/nonexistent/processor-telemetry");
        assert!(
            !watch(path, &AtomicBool::new(false), |said| heard.push(said)),
            "unheard"
        );
        assert_eq!(heard, [None]);
    }
}
