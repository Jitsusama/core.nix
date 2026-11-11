//! The processor's own temperatures, as processor-telemetry tells them
//! over varlink on the socket systemd holds for it: its NPU's, image
//! processor's and media and display engines', which no hwmon chip gives
//! and Heat's rows say beside the processor's ([`crate::parts`]). The
//! reader reads the processor's counters once a second for as long as
//! it's asked and nothing otherwise, so it's asked only while Heat's rows
//! are watched, as the shell's `ProcessorTelemetry.qml` asked it while Heat
//! was shown.

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

/// The temperatures one reply says, each `{ part, celsius }`, or none
/// where the reply is an error, as a processor with no telemetry answers
/// `NoRegion`, or isn't a reading at all.
pub fn temperatures(reply: &[u8]) -> Option<Vec<Value>> {
    let said: Value = serde_json::from_slice(reply).ok()?;
    if said.get("error").is_some() {
        return None;
    }
    said["parameters"]["temperatures"].as_array().cloned()
}

/// Asks the reader at `path` to watch, and tells `heard` each reading's
/// temperatures, or none when it errs, until `stop` is set or the
/// connection ends; then tells `heard` none, so nothing stale is kept.
/// Returns whether it was heard at all, so a caller can tell a machine
/// with no reader from one that went.
pub fn watch(path: &Path, stop: &AtomicBool, mut heard: impl FnMut(Option<Vec<Value>>)) -> bool {
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
                let said = temperatures(&reply);
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
    fn a_reply_says_its_temperatures() {
        let reply = br#"{"parameters":{"temperatures":[{"part":"npu","celsius":62}]}}"#;
        assert_eq!(
            temperatures(reply),
            Some(vec![json!({ "part": "npu", "celsius": 62 })])
        );
    }

    #[test]
    fn an_error_or_nonsense_says_none() {
        let error = br#"{"error":"io.jitsusama.ProcessorTelemetry.NoRegion"}"#;
        assert_eq!(temperatures(error), None);
        assert_eq!(temperatures(b"not json"), None);
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
                Some(vec![json!({ "part": "npu", "celsius": 62 })]),
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
