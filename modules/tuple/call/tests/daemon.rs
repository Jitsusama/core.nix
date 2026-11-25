//! The daemon over its socket, with a stand-in for Tuple: its stream is
//! followed only while Tuple's daemon answers, and a call, a sign-in gone
//! and a stream that ends are each said to a watch as they happen, and
//! a call that ended kept in Recent's file, as the caller's when a ring
//! answered began it, and read back after a restart; a ring and a missed
//! call are read from the log as it's written, a new log from its start;
//! each act runs Tuple's command, a camera stopped is off before Tuple's
//! stream says so, a stop Tuple is slow to answer is answered as such,
//! not run again while it runs within its hold, run again past it or once
//! it's done, and still counts once done, though a stop or mute done after
//! the stream said Joel's camera or mute turned, or once another call
//! began, connected or not, leaves the record as the stream says; a
//! stream followed afresh journals the camera it says on; opening the
//! room keeps its link and makes the call in it Joel's; and an ask or
//! kind it doesn't know is refused.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};

use daemon_client::{Client, until};
use serde_json::{Value, json};

/// Tuple's command, in the shell the tests run in: `state follow` says
/// each line of `stream` in its folder as it's written there, until the
/// file `ends` appears or the folder goes; any other command is written to `asked`, a line
/// each, `new` printing the room's link, `call accept` failing as Tuple
/// does with nothing ringing, and any command stalling while the file
/// `stalls-<its words>` is there, as `stalls-webcam-stop` stalls a stop.
/// The daemon's journal is kept in `journal`.
const TUPLE: &str = r#"
here=$(dirname "$0")
if [ "$3 $4" = "state follow" ]; then
  echo started >> "$here/followed"
  n=0
  # Its folder gone is its test over, should the daemon have been killed
  # with it still following.
  while [ -d "$here" ] && [ ! -e "$here/ends" ]; do
    lines=$(wc -l < "$here/stream" 2>/dev/null || echo 0)
    if [ "$lines" -gt "$n" ]; then
      tail -n +$((n + 1)) "$here/stream" | head -n $((lines - n))
      n=$lines
    fi
    sleep 0.05
  done
  exit 3
fi
echo "$*" >> "$here/asked"
# A command Tuple is slow on waits while the test holds stalls-<its words>.
while [ -e "$here/stalls-$1-$2" ]; do sleep 0.05; done
case "$*" in
  new) echo "Joining https://tuple.app/c/joels-room"; sleep 5 ;;
  "call accept") echo "no call is ringing" >&2; exit 1 ;;
esac
"#;

/// The daemon, with a folder standing in for the runtime, data and state
/// folders, and Tuple's daemon's socket, which answers while it's held.
struct Running {
    daemon: Child,
    folder: PathBuf,
    socket: PathBuf,
    tuples: Option<UnixListener>,
}

fn now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// A line of Tuple's log at `at`, in milliseconds since the epoch.
fn logged(at: i64, message: &str) -> String {
    let ms = at.rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03}|1|2|info|calls|{message}\n",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

const STATE: &str = "{\"type\":\"state\",\"data\":{\"contacts\":[{\"id\":7,\
    \"full_name\":\"Ada Brightwater\",\"email\":\"ada@example.com\"}],\
    \"sign_in\":\"signed_in\",\"rooms\":{}}}";

const CALL: &str = "{\"type\":\"current_call\",\"data\":{\"id\":\"c-1\",\"started_at\":1000,\
    \"local\":{\"audio_enabled\":true,\"connection_state\":\"connected\",\
    \"share_state\":\"audio\",\"webcam_enabled\":false},\"participants\":[{\"id\":7,\
    \"audio_enabled\":false,\"connection_state\":\"connected\",\"share_state\":\"screen\",\
    \"webcam_enabled\":true}]}}";

impl Running {
    fn start(test: &str, answering: bool) -> Running {
        Running::through(test, answering, &[], &[])
    }

    /// Started with Tuple answering and its acts' waits given by `waits`,
    /// as `--answers-within` and `--held-for` give them, so a test waits
    /// on its deadline no longer than it must, and one with steps to take
    /// before Tuple answers never races it.
    fn waiting(test: &str, waits: &[&str]) -> Running {
        Running::through(test, true, &[], waits)
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, answering: bool, launcher: &[&str], waits: &[&str]) -> Running {
        let folder = std::env::temp_dir().join(format!("tuple-call-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for inner in ["run/daemons", "data/tuple/0", "state", "bin"] {
            std::fs::create_dir_all(folder.join(inner)).unwrap();
        }
        // The running shell, as the Nix sandbox has no /usr/bin/env.
        let shell = std::env::var("BASH").unwrap_or_else(|_| "/bin/sh".to_owned());
        let tuple = folder.join("bin/tuple");
        std::fs::write(&tuple, format!("#!{shell}\n{TUPLE}")).unwrap();
        std::fs::set_permissions(&tuple, std::fs::Permissions::from_mode(0o755)).unwrap();
        let socket = folder.join("run/daemons/tuple-call.sock");
        let tuples = answering.then(|| answering_at(&folder));
        let program = env!("CARGO_BIN_EXE_tuple-call");
        let mut command = match launcher.split_first() {
            Some((first, arguments)) => {
                let mut command = Command::new(first);
                for argument in arguments {
                    command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
                }
                command.arg(program);
                command
            }
            None => Command::new(program),
        };
        let daemon = spawned(command.args(waits), &folder);
        until("the socket never came", || socket.exists());
        Running {
            daemon,
            folder,
            socket,
            tuples,
        }
    }

    /// The daemon stopped and started again over the same folders, its
    /// socket gone with it, as a restart leaves them.
    fn restart(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_file(&self.socket);
        self.daemon = spawned(
            &mut Command::new(env!("CARGO_BIN_EXE_tuple-call")),
            &self.folder,
        );
        until("the socket never came back", || self.socket.exists());
    }

    /// Tuple's daemon answering on its socket.
    fn answer(&mut self) {
        self.tuples = Some(answering_at(&self.folder));
    }

    fn path(&self, inner: &str) -> PathBuf {
        self.folder.join(inner)
    }

    /// Tuple's stream saying a line.
    fn streams(&self, line: &str) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path("bin/stream"))
            .unwrap();
        writeln!(file, "{line}").unwrap();
    }

    /// Tuple's log having a line written to it.
    fn logs(&self, line: &str) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path("data/tuple/0/log.txt"))
            .unwrap();
        file.write_all(line.as_bytes()).unwrap();
    }

    fn read(&self, inner: &str) -> Vec<String> {
        std::fs::read_to_string(self.path(inner))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn client(&self) -> Client {
        Client::connect(&self.socket)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = std::fs::write(self.path("bin/ends"), "");
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// What these tests hear of a watch of the call, beyond an ask's answer.
trait Following {
    /// The record as a watch is told of it, once `wanted` holds of it.
    fn until(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value;
    /// How Tuple stands, once it's `word`: as health says it already, or
    /// as a watch is told once it changes. A watch asked for without a
    /// hello, as these ask, is told only what moves after it begins.
    fn stands(&mut self, word: &str);
    /// The call as a watch of it begins.
    fn watch(&mut self) -> Value;
}

impl Following for Client {
    fn until(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        loop {
            let said = self.heard();
            if let Some(record) = said["changed"].get(0)
                && wanted(record)
            {
                return record.clone();
            }
            assert!(said.get("error").is_none(), "{what}: {said}");
        }
    }

    fn stands(&mut self, word: &str) {
        let health = self.ask(&json!({"id": "stands", "ask": "health"}));
        if health["sources"]["tuple"]["state"] == word {
            return;
        }
        loop {
            let said = self.heard();
            if said["sources"]["tuple"] == word {
                return;
            }
        }
    }

    fn watch(&mut self) -> Value {
        let said = self.ask(&json!({"id": "all", "ask": "watch", "kind": "call"}));
        said["records"][0].clone()
    }
}

/// The daemon, run by `command`, over the test's folders.
fn spawned(command: &mut Command, folder: &std::path::Path) -> Child {
    command
        .arg(folder.join("run/daemons/tuple-call.sock"))
        .arg(folder.join("bin/tuple"))
        .env("XDG_RUNTIME_DIR", folder.join("run"))
        .env("XDG_DATA_HOME", folder.join("data"))
        .env("XDG_STATE_HOME", folder.join("state"))
        .stderr(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(folder.join("journal"))
                .unwrap(),
        )
        .spawn()
        .unwrap()
}

/// Tuple's daemon's socket in the test's runtime folder, answering for as
/// long as it's held.
fn answering_at(folder: &std::path::Path) -> UnixListener {
    UnixListener::bind(folder.join("run/tuple-0")).unwrap()
}

#[test]
fn a_stopped_tuple_is_never_started_and_its_call_is_none() {
    let running = Running::start("stopped", false);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello", "version": 1, "client": "test"}));
    assert_eq!(
        (&said["version"], &said["domain"], &said["state"]),
        (&json!(1), &json!("tuple"), &json!("answering"))
    );
    assert_eq!(said["sources"], json!({"tuple": "starting"}));
    let record = client.watch();
    assert_eq!(
        (&record["key"], &record["kind"], &record["state"]),
        (&json!("call:current"), &json!("call"), &json!("none"))
    );
    client.stands("starting");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(running.read("bin/followed"), [] as [std::string::String; 0]);
    assert_eq!(running.read("bin/asked"), [] as [std::string::String; 0]);
}

#[test]
fn a_call_is_said_to_a_watch_as_it_changes() {
    let running = Running::start("call", true);
    running.streams(STATE);
    let mut client = running.client();
    client.watch();
    running.streams(CALL);
    let record = client.until("no call was said", |record| record["state"] == "connected");
    assert_eq!(record["since"], 1000);
    assert_eq!(record["muted"], false);
    assert_eq!(
        record["people"],
        json!([{"id": "7", "name": "Ada Brightwater", "email": "ada@example.com",
                "connection": "connected", "muted": true, "sharing": true, "camera": true}])
    );
    assert!(record["heard"].as_i64().is_some());
    let said = client.ask(&json!({"id": 2, "ask": "get", "keys": ["call:current"]}));
    assert_eq!(said["records"][0]["state"], "connected");
    running.streams("{\"type\":\"current_call\",\"data\":null}");
    client.until("the call's end wasn't said", |record| {
        record["state"] == "none"
    });
}

#[test]
fn a_call_that_ends_is_kept_in_recent_and_read_back_after_a_restart() {
    let mut running = Running::start("recent", true);
    running.streams(STATE);
    let mut client = running.client();
    client.watch();
    // Ada rings and Joel answers, so the call that follows was hers.
    running.logs(&logged(now(), "received incoming call: 7"));
    client.until("the ring wasn't said", |record| {
        record["state"] == "ringing"
    });
    running.logs(&logged(now(), "accepting call"));
    client.until("the answer wasn't said", |record| record["state"] == "none");
    running.streams(CALL);
    client.until("the call wasn't said", |record| {
        record["state"] == "connected"
    });
    running.streams("{\"type\":\"current_call\",\"data\":null}");
    let record = client.until("the call wasn't kept", |record| {
        record["recent"]
            .as_array()
            .is_some_and(|recent| !recent.is_empty())
    });
    let kept = &record["recent"][0];
    assert_eq!(
        (
            &kept["people"],
            &kept["since"],
            &kept["missed"],
            &kept["caller"]
        ),
        (
            &json!([{"id": "7", "name": "Ada Brightwater", "email": "ada@example.com"}]),
            &json!(1000),
            &json!(false),
            &json!("7")
        )
    );
    assert_eq!(running.read("state/tuple-call/recent.jsonl").len(), 1);
    // Tuple's stream, as the daemon follows it again, says nothing of the
    // call, so only the file can.
    std::fs::write(running.path("bin/stream"), format!("{STATE}\n")).unwrap();
    drop(client);
    running.restart();
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "get", "keys": ["call:current"]}));
    assert_eq!(said["records"][0]["recent"], json!([kept]));
}

#[test]
fn a_sign_in_gone_and_a_stream_that_ends_are_said() {
    let running = Running::start("signed-out", true);
    running.streams("{\"type\":\"state\",\"data\":{\"contacts\":[],\"sign_in\":\"signed_out\"}}");
    let mut client = running.client();
    client.watch();
    client.stands("signed-out");
    std::fs::write(running.path("bin/ends"), "").unwrap();
    client.stands("refused");
    let said = client.ask(&json!({"id": 2, "ask": "health"}));
    assert_eq!(said["sources"]["tuple"]["state"], "refused");
    assert!(
        said["sources"]["tuple"]["reason"]
            .as_str()
            .is_some_and(|why| why.contains("ended")),
        "{said}"
    );
}

#[test]
fn a_sign_in_tuple_finishes_once_it_reaches_its_servers_is_heard() {
    let running = Running::start("late-sign-in", true);
    running.streams("{\"type\":\"state\",\"data\":{\"contacts\":[],\"sign_in\":\"signed_out\"}}");
    let mut client = running.client();
    client.watch();
    client.stands("signed-out");
    // Tuple signed in as it reached its servers, saying nothing more on the
    // stream followed now; only a stream followed afresh says so.
    std::fs::write(running.path("bin/stream"), format!("{STATE}\n")).unwrap();
    running.logs(&logged(
        now(),
        "signaler state changed: synchronizing -> connected",
    ));
    client.stands("answering");
    assert_eq!(
        running.read("bin/followed").len(),
        2,
        "Tuple's stream wasn't followed afresh, once"
    );
}

#[test]
fn tuple_starting_is_followed_at_once() {
    let mut running = Running::start("starts", false);
    let mut client = running.client();
    client.watch();
    // Past the first look, which found nothing answering.
    std::thread::sleep(Duration::from_millis(300));
    running.answer();
    running.streams(
        "{\"type\":\"state\",\"data\":{\"contacts\":[],\"sign_in\":\"signed_in\",\
         \"rooms\":{\"personal\":[{\"http_value\":\"https://tuple.app/c/listed\",\
         \"created_at\":\"2026-07-31T12:00:00Z\"}]}}}",
    );
    let began = Instant::now();
    running.logs(&logged(now(), "daemon loop started, waiting for events..."));
    // What Tuple's state said is heard before that Tuple answers, so a
    // client acting on its answering holds the room already.
    let mut room = Value::Null;
    loop {
        let said = client.heard();
        if let Some(record) = said["changed"].get(0) {
            room = record["room"].clone();
        }
        if said["sources"]["tuple"] == "answering" {
            break;
        }
    }
    assert_eq!(room, "https://tuple.app/c/listed");
    assert!(
        began.elapsed() < Duration::from_secs(3),
        "followed only at the next look, {:?} on",
        began.elapsed()
    );
    assert_eq!(running.read("bin/followed").len(), 1);
}

#[test]
fn a_ring_and_a_missed_call_come_from_the_log() {
    let running = Running::start("ring", true);
    running.streams(STATE);
    running.logs(&logged(now() - 3_600_000, "received incoming call: 9"));
    let mut client = running.client();
    // Once Tuple's contacts are read, to name the caller by.
    until("Tuple's stream was never followed", || {
        running.client().ask(&json!({"id": 1, "ask": "hello"}))["sources"]["tuple"] == "answering"
    });
    let record = client.watch();
    assert_eq!(record["state"], "none", "an hour-old ring rang on");
    running.logs(&logged(now(), "received incoming call: 7"));
    let record = client.until("the ring wasn't said", |record| {
        record["state"] == "ringing"
    });
    assert_eq!(record["ringing"]["name"], "Ada Brightwater");
    running.logs(&logged(now(), "ignoring incoming call: busy"));
    let record = client.until("the missed call wasn't said", |record| {
        record["missed"]
            .as_array()
            .is_some_and(|missed| !missed.is_empty())
    });
    assert_eq!(record["state"], "none");
    assert_eq!(record["missed"][0]["email"], "ada@example.com");
}

#[test]
fn a_log_written_afresh_is_read_from_its_start() {
    let running = Running::start("afresh", true);
    running.logs(&"filler line that is long enough to be longer than the new log\n".repeat(4));
    let mut client = running.client();
    client.watch();
    let fresh = running.path("data/tuple/0/log.new");
    std::fs::write(&fresh, logged(now(), "received incoming call: 7")).unwrap();
    std::fs::rename(&fresh, running.path("data/tuple/0/log.txt")).unwrap();
    client.until("the new log's ring wasn't said", |record| {
        record["state"] == "ringing"
    });
}

#[test]
fn each_act_runs_tuples_command_and_answers_once_done() {
    let running = Running::start("acts", true);
    let mut client = running.client();
    for (asked, args) in [
        (
            json!({"ask": "join", "link": "go to https://tuple.app/c/abc"}),
            "call join https://tuple.app/c/abc",
        ),
        (json!({"ask": "mute", "muted": true}), "call mute"),
        (json!({"ask": "mute", "muted": false}), "call unmute"),
        (json!({"ask": "share"}), "share"),
        (json!({"ask": "camera", "on": true}), "webcam start"),
        (json!({"ask": "camera", "on": false}), "webcam stop"),
        (json!({"ask": "leave"}), "call leave"),
    ] {
        let mut asked = asked;
        asked["id"] = json!(args);
        let said = client.ask(&asked);
        assert_eq!(said["done"], true, "{args}: {said}");
        assert_eq!(
            running.read("bin/asked").last().map(String::as_str),
            Some(args)
        );
    }
    let said = client.ask(&json!({"id": "a", "ask": "accept"}));
    assert_eq!(said["error"]["code"], "refused");
    assert_eq!(said["error"]["message"], "no call is ringing");
    let said = client.ask(&json!({"id": "j", "ask": "join", "link": "https://example.com"}));
    assert_eq!(said["error"]["code"], "refused");
}

#[test]
fn a_camera_stopped_is_off_ahead_of_tuples_stream() {
    let running = Running::start("camera", true);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    let filming = CALL.replace(
        "\"share_state\":\"audio\",\"webcam_enabled\":false",
        "\"share_state\":\"audio\",\"webcam_enabled\":true",
    );
    running.streams(&filming);
    watching.until("Joel's camera wasn't said on", |record| {
        record["camera"] == true
    });
    // Asked apart from the watch, so a change said ahead of the answer
    // isn't read as it.
    let said = running
        .client()
        .ask(&json!({"id": "off", "ask": "camera", "on": false}));
    assert_eq!(said["done"], true, "the camera's stop: {said}");
    watching.until("Joel's camera stayed on once its stop was done", |record| {
        record["camera"] == false
    });
}

/// Joel's camera on in the call, as Tuple's stream says it.
fn filming() -> String {
    CALL.replace(
        "\"share_state\":\"audio\",\"webcam_enabled\":false",
        "\"share_state\":\"audio\",\"webcam_enabled\":true",
    )
}

/// Joel muted in the call, as Tuple's stream says it.
fn muted() -> String {
    CALL.replace(
        "\"local\":{\"audio_enabled\":true",
        "\"local\":{\"audio_enabled\":false",
    )
}

#[test]
fn a_camera_stop_tuple_is_slow_to_answer_is_told_and_still_counts() {
    let running = Running::waiting("stalled", &["--answers-within", "1"]);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    running.streams(&filming());
    watching.until("Joel's camera wasn't said on", |record| {
        record["camera"] == true
    });
    let stall = running.path("bin/stalls-webcam-stop");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let said = asking.ask(&json!({"id": "off", "ask": "camera", "on": false}));
    assert_eq!(
        said["error"]["code"], "unanswered",
        "a stop Tuple hadn't answered wasn't told so: {said}"
    );
    let again = asking.ask(&json!({"id": "again", "ask": "camera", "on": false}));
    assert_eq!(
        again["error"]["message"], "Tuple hasn't answered the last time it was asked",
        "a stop asked again while Tuple ran the last wasn't answered at once: {again}"
    );
    // Ada unmuting turns nothing of Joel's, so the stop still counts once
    // Tuple has done it.
    running.streams(&filming().replace(
        "\"id\":7,\"audio_enabled\":false",
        "\"id\":7,\"audio_enabled\":true",
    ));
    watching.until("Ada wasn't said unmuted", |record| {
        record["people"][0]["muted"] == false
    });
    std::fs::remove_file(&stall).unwrap();
    watching.until("a stop Tuple did late didn't count", |record| {
        record["camera"] == false
    });
    until("the late stop wasn't journaled", || {
        let journal = running.read("journal");
        journal
            .iter()
            .any(|line| line.contains("webcam stop hasn't answered in 1 second"))
            && journal
                .iter()
                .any(|line| line.contains("webcam stop answered late"))
    });
    let after = asking.ask(&json!({"id": "after", "ask": "camera", "on": false}));
    assert_eq!(
        after["done"], true,
        "a stop asked once Tuple answered the last wasn't run: {after}"
    );
    let stops = running.read("bin/asked");
    assert_eq!(
        stops.iter().filter(|line| *line == "webcam stop").count(),
        2,
        "a stop asked again while Tuple ran the last ran Tuple again: {stops:?}"
    );
}

#[test]
fn a_stop_done_after_the_camera_turned_again_leaves_it_on() {
    let running = Running::waiting("turned", &["--answers-within", "60"]);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    running.streams(&filming());
    watching.until("Joel's camera wasn't said on", |record| {
        record["camera"] == true
    });
    let stall = running.path("bin/stalls-webcam-stop");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let stopping =
        std::thread::spawn(move || asking.ask(&json!({"id": "off", "ask": "camera", "on": false})));
    until("the stop wasn't asked of Tuple", || {
        running
            .read("bin/asked")
            .iter()
            .any(|line| line == "webcam stop")
    });
    // Joel stops his camera in Tuple's window and starts it again, as the
    // notice of a stop unanswered sends him to.
    running.streams(CALL);
    watching.until("Joel's camera wasn't said off", |record| {
        record["camera"] == false
    });
    running.streams(&filming());
    watching.until("Joel's camera wasn't said on again", |record| {
        record["camera"] == true
    });
    std::fs::remove_file(&stall).unwrap();
    let said = stopping.join().unwrap();
    assert_eq!(
        said["done"], true,
        "the stop Tuple did wasn't answered done: {said}"
    );
    let record = running.client().watch();
    assert_eq!(
        record["camera"], true,
        "a stop done after the camera turned again marked it off: {record}"
    );
}

#[test]
fn a_mute_done_after_joel_unmuted_by_hand_leaves_him_unmuted() {
    let running = Running::waiting("unmuted", &["--answers-within", "60"]);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    running.streams(CALL);
    watching.until("the call wasn't said", |record| {
        record["state"] == "connected"
    });
    let stall = running.path("bin/stalls-call-mute");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let muting = std::thread::spawn(move || {
        asking.ask(&json!({"id": "mute", "ask": "mute", "muted": true}))
    });
    until("the mute wasn't asked of Tuple", || {
        running
            .read("bin/asked")
            .iter()
            .any(|line| line == "call mute")
    });
    // Tuple mutes him before its command ends, and he unmutes in Tuple's
    // window.
    running.streams(&muted());
    watching.until("Joel wasn't said muted", |record| record["muted"] == true);
    running.streams(CALL);
    watching.until("Joel wasn't said unmuted", |record| {
        record["muted"] == false
    });
    std::fs::remove_file(&stall).unwrap();
    let said = muting.join().unwrap();
    assert_eq!(
        said["done"], true,
        "the mute Tuple did wasn't answered done: {said}"
    );
    let record = running.client().watch();
    assert_eq!(
        record["muted"], false,
        "a mute done after Joel unmuted by hand marked him muted: {record}"
    );
}

#[test]
fn a_mute_done_once_another_call_began_leaves_it_alone() {
    let running = Running::waiting("another", &["--answers-within", "60"]);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    running.streams(CALL);
    watching.until("the call wasn't said", |record| {
        record["state"] == "connected"
    });
    let stall = running.path("bin/stalls-call-mute");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let muting = std::thread::spawn(move || {
        asking.ask(&json!({"id": "mute", "ask": "mute", "muted": true}))
    });
    until("the mute wasn't asked of Tuple", || {
        running
            .read("bin/asked")
            .iter()
            .any(|line| line == "call mute")
    });
    running.streams("{\"type\":\"current_call\",\"data\":null}");
    watching.until("the call's end wasn't said", |record| {
        record["state"] == "none"
    });
    running.streams(&CALL.replace("\"started_at\":1000", "\"started_at\":2000"));
    watching.until("the next call wasn't said", |record| {
        record["since"] == 2000
    });
    std::fs::remove_file(&stall).unwrap();
    let said = muting.join().unwrap();
    assert_eq!(
        said["done"], true,
        "the mute Tuple did wasn't answered done: {said}"
    );
    let record = running.client().watch();
    assert_eq!(
        record["muted"], false,
        "a mute asked in the last call marked Joel muted in the next: {record}"
    );
}

#[test]
fn a_mute_done_once_another_connecting_call_began_leaves_it_alone() {
    let running = Running::waiting("connecting", &["--answers-within", "60"]);
    running.streams(STATE);
    let mut watching = running.client();
    watching.watch();
    // Tuple says when a call began only once it connects, so two calls
    // still connecting are told apart by nothing they say.
    let connecting = CALL.replace("\"started_at\":1000,", "");
    running.streams(&connecting);
    watching.until("the call wasn't said", |record| {
        record["state"] == "connected"
    });
    let stall = running.path("bin/stalls-call-mute");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let muting = std::thread::spawn(move || {
        asking.ask(&json!({"id": "mute", "ask": "mute", "muted": true}))
    });
    until("the mute wasn't asked of Tuple", || {
        running
            .read("bin/asked")
            .iter()
            .any(|line| line == "call mute")
    });
    running.streams("{\"type\":\"current_call\",\"data\":null}");
    watching.until("the call's end wasn't said", |record| {
        record["state"] == "none"
    });
    running.streams(&connecting);
    watching.until("the next call wasn't said", |record| {
        record["state"] == "connected"
    });
    std::fs::remove_file(&stall).unwrap();
    let said = muting.join().unwrap();
    assert_eq!(
        said["done"], true,
        "the mute Tuple did wasn't answered done: {said}"
    );
    let record = running.client().watch();
    assert_eq!(
        record["muted"], false,
        "a mute asked in one connecting call marked Joel muted in the next: {record}"
    );
}

#[test]
fn a_command_held_past_its_hold_lets_its_act_be_asked_afresh() {
    let running = Running::waiting("held", &["--answers-within", "1", "--held-for", "2"]);
    running.streams(STATE);
    running.streams(&filming());
    let mut watching = running.client();
    watching.watch();
    watching.until("Joel's camera wasn't said on", |record| {
        record["camera"] == true
    });
    let stall = running.path("bin/stalls-webcam-stop");
    std::fs::write(&stall, "").unwrap();
    let mut asking = running.client();
    let first = Instant::now();
    let said = asking.ask(&json!({"id": "first", "ask": "camera", "on": false}));
    assert_eq!(
        said["error"]["code"], "unanswered",
        "a stop Tuple hadn't answered wasn't told so: {said}"
    );
    let again = asking.ask(&json!({"id": "again", "ask": "camera", "on": false}));
    assert_eq!(
        again["error"]["message"], "Tuple hasn't answered the last time it was asked",
        "a stop asked again within its hold wasn't answered at once: {again}"
    );
    // Past the two seconds the first stop holds its act for, counted from
    // a moment after this test's clock began, so the next is Tuple's to
    // run afresh.
    std::thread::sleep(Duration::from_millis(2300).saturating_sub(first.elapsed()));
    let afresh = asking.ask(&json!({"id": "afresh", "ask": "camera", "on": false}));
    assert_eq!(
        afresh["error"]["message"], "Tuple hasn't answered in 1 second",
        "a stop asked past the last one's hold wasn't run afresh: {afresh}"
    );
    let stops = running.read("bin/asked");
    assert_eq!(
        stops.iter().filter(|line| *line == "webcam stop").count(),
        2,
        "a stop asked past the last one's hold didn't run Tuple again: {stops:?}"
    );
    std::fs::remove_file(&stall).unwrap();
    until("both stops weren't journaled once done", || {
        running
            .read("journal")
            .iter()
            .filter(|line| line.contains("webcam stop answered late"))
            .count()
            == 2
    });
}

#[test]
fn a_stream_followed_afresh_journals_the_camera_it_says_on() {
    let running = Running::start("afresh", true);
    running.streams(STATE);
    running.streams(&filming());
    let mut watching = running.client();
    watching.watch();
    let journaled = || {
        running
            .read("journal")
            .iter()
            .filter(|line| line.contains("Tuple says Joel's camera is on in the call"))
            .count()
    };
    until("the camera on wasn't journaled", || journaled() == 1);
    std::fs::write(running.path("bin/ends"), "").unwrap();
    watching.stands("refused");
    // Tuple, started again, says the call and the camera on afresh, which
    // its stream ending had left unsaid.
    std::fs::remove_file(running.path("bin/ends")).unwrap();
    std::fs::write(
        running.path("bin/stream"),
        format!("{STATE}\n{}\n", filming()),
    )
    .unwrap();
    running.logs(&logged(now(), "daemon loop started, waiting for events..."));
    until("the camera on, said afresh, wasn't journaled", || {
        journaled() == 2
    });
}

#[test]
fn opening_the_room_answers_and_keeps_its_link() {
    let running = Running::start("open", true);
    let mut client = running.client();
    client.watch();
    let said = client.ask(&json!({"id": "o", "ask": "open"}));
    assert_eq!(said["link"], "https://tuple.app/c/joels-room");
    assert_eq!(
        running.read("state/tuple-call/room"),
        ["https://tuple.app/c/joels-room"]
    );
    let said = client.ask(&json!({"id": "l", "ask": "list"}));
    assert_eq!(said["records"][0]["room"], "https://tuple.app/c/joels-room");
    // The room opened starts a call in it, so one coming to it came to
    // Joel's call.
    running.streams(CALL);
    client.until("the call wasn't said", |record| {
        record["state"] == "connected"
    });
    running.streams("{\"type\":\"current_call\",\"data\":null}");
    let record = client.until("the call wasn't kept", |record| {
        record["recent"]
            .as_array()
            .is_some_and(|recent| !recent.is_empty())
    });
    assert_eq!(record["recent"][0]["caller"], "");
}

#[test]
fn an_ask_or_kind_it_does_not_know_is_refused() {
    let running = Running::start("unknown", false);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "person"}));
    assert_eq!(said["error"]["code"], "unknown-kind");
    let said = client.ask(&json!({"id": 2, "ask": "dance"}));
    assert_eq!(said["error"]["code"], "unknown-ask");
    let said = client.ask(&json!({"id": 3, "ask": "unwatch"}));
    assert_eq!(said["unwatched"], true);
}

#[test]
fn it_answers_on_the_socket_systemd_hands_it() {
    let running = Running::through(
        "handed",
        false,
        &["systemd-socket-activate", "--listen={socket}"],
        &[],
    );
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(said["domain"], "tuple");
}
