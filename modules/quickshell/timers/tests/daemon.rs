//! The daemon over its socket, on a bus of its own with a stand-in
//! notifications server: a timer started is told to every watch and rings
//! as a critical notice once its end passes; the notice's Again sets it
//! going afresh and putting it away ends it; ending one rung takes its
//! notice back; a ring with no server to take it is said once one comes;
//! a ring lost with its server is said to the one after it; what's kept
//! outlasts the daemon, its notices with it, one that ended meanwhile
//! ringing late as it starts again; a timer coming due is heard once,
//! quietly during a call; an ask it can't do is refused; and a run its
//! unit wouldn't make ends at once in the terminal kit's words.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::OwnedValue;

/// How long a test waits for what it expects before failing.
const PATIENCE: Duration = Duration::from_secs(10);

/// What each notice the stand-in was asked to show said.
#[derive(Clone, Debug)]
struct Shown {
    app: String,
    summary: String,
    body: String,
    actions: Vec<String>,
    urgency: Option<u8>,
    timeout: i32,
}

/// A notifications server standing in for the shell's, keeping what it's
/// asked.
#[derive(Clone, Default)]
struct Server {
    shown: Arc<Mutex<Vec<Shown>>>,
    closed: Arc<Mutex<Vec<u32>>>,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[expect(clippy::too_many_arguments, reason = "the specification's own call")]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "zbus hands an interface's arguments over owned"
    )]
    fn notify(
        &self,
        app: String,
        replaces: u32,
        icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, OwnedValue>,
        timeout: i32,
    ) -> u32 {
        // A ring is always a fresh notice, and wears no icon of its own.
        assert_eq!((replaces, icon.as_str()), (0, ""));
        let urgency = hints
            .get("urgency")
            .and_then(|value| u8::try_from(value).ok());
        let mut shown = self.shown.lock().unwrap();
        shown.push(Shown {
            app,
            summary,
            body,
            actions,
            urgency,
            timeout,
        });
        u32::try_from(shown.len()).unwrap()
    }

    fn close_notification(&self, id: u32) {
        self.closed.lock().unwrap().push(id);
    }
}

/// A bus of the test's own, as a session's.
struct Bus {
    daemon: Child,
    address: String,
}

impl Bus {
    fn new(folder: &Path) -> Bus {
        let socket = folder.join("bus");
        let config = folder.join("bus.conf");
        std::fs::write(
            &config,
            format!(
                r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path={}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>"#,
                socket.display()
            ),
        )
        .unwrap();
        let daemon = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.display()))
            .arg("--nofork")
            .stdout(Stdio::null())
            .spawn()
            .expect("dbus-daemon runs");
        waits_for(|| socket.exists());
        Bus {
            daemon,
            address: format!("unix:path={}", socket.display()),
        }
    }

    /// The stand-in, serving on this bus under the notifications' name.
    fn serve(&self) -> (Connection, Server) {
        let server = Server::default();
        let connection = Builder::address(self.address.as_str())
            .unwrap()
            .name("org.freedesktop.Notifications")
            .unwrap()
            .serve_at("/org/freedesktop/Notifications", server.clone())
            .unwrap()
            .build()
            .unwrap();
        (connection, server)
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

/// The daemon, running with its socket and its kept file in the test's
/// folder, on the test's bus.
struct Running {
    daemon: Child,
    socket: PathBuf,
}

impl Running {
    fn start(folder: &Path, bus: &Bus) -> Running {
        Running::heard(folder, bus, &[])
    }

    /// The daemon, given `sound`'s player, sound and call to ring aloud
    /// with, or none.
    fn heard(folder: &Path, bus: &Bus, sound: &[&Path]) -> Running {
        let socket = folder.join("daemons").join("timers.sock");
        let words = folder.join("words.json");
        std::fs::write(
            &words,
            json!({
                "app": "Timers", "unnamed": "Timer", "rang": "{span} timer",
                "late": "{span} timer, rang late", "again": "Again",
                "hours": "h", "minutes": "min", "seconds": "s",
            })
            .to_string(),
        )
        .unwrap();
        let _ = std::fs::remove_file(&socket);
        let daemon = Command::new(env!("CARGO_BIN_EXE_timers"))
            .arg(&socket)
            .arg(folder.join("state").join("held.json"))
            .arg(&words)
            .args(sound)
            .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        waits_for(|| socket.exists());
        Running { daemon, socket }
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream.set_read_timeout(Some(PATIENCE)).unwrap();
        Client {
            reader: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }

    fn stop(mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

struct Client {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    fn say(&mut self, said: &Value) {
        writeln!(self.stream, "{said}").unwrap();
    }

    fn hear(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("not JSON: {line:?}"))
    }

    /// The answer to an ask, by its id, passing changes heard meanwhile.
    fn ask(&mut self, said: &Value) -> Value {
        self.say(said);
        loop {
            let heard = self.hear();
            if heard["id"] == said["id"] && heard.get("changed").is_none() {
                return heard;
            }
        }
    }

    /// The next change a watch hears.
    fn change(&mut self) -> Value {
        loop {
            let heard = self.hear();
            if heard.get("changed").is_some() {
                return heard;
            }
        }
    }
}

fn folder(test: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("timers-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A stand-in program at `path`, running `body` in the shell found on
/// PATH, since a build's sandbox has no /bin/sh.
fn stand_in(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let shell = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|folder| folder.join("sh"))
        .find(|shell| shell.exists())
        .expect("a shell on PATH");
    std::fs::write(path, format!("#!{}\n{body}\n", shell.display())).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn waits_for(done: impl Fn() -> bool) {
    let began = Instant::now();
    while !done() {
        assert!(began.elapsed() < PATIENCE, "it never came");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Says a signal about notice `body.0` as the notifications server would,
/// once the daemon in `folder` has kept that this server gave it. Nobody
/// acts on a card the moment it shows, and a signal sooner is about a
/// notice the daemon doesn't know yet, so it would pass unheard.
fn signal(
    folder: &Path,
    connection: &Connection,
    member: &str,
    body: &(u32, impl serde::Serialize + zbus::zvariant::Type),
) {
    let server = connection.unique_name().unwrap().to_string();
    let kept = folder.join("state").join("held.json");
    waits_for(|| {
        std::fs::read_to_string(&kept)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .is_some_and(|held| {
                held["timers"].as_array().is_some_and(|timers| {
                    timers.iter().any(|timer| {
                        timer["notice"]["id"] == body.0
                            && timer["notice"]["server"] == server.as_str()
                    })
                })
            })
    });
    connection
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            member,
            body,
        )
        .unwrap();
}

#[test]
fn a_timer_started_is_watched_and_rings_as_a_notice_that_stays() {
    let folder = folder("rings");
    let bus = Bus::new(&folder);
    let (_server, server) = bus.serve();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    let hello = client.ask(&json!({"id": "hello", "ask": "hello", "version": 1}));
    assert_eq!(hello["state"], "answering");
    let all = client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    assert_eq!(all["records"], json!([]));
    let started = client.ask(&json!({"id": 1, "ask": "start", "span": 300, "name": "tea"}));
    assert_eq!(started["records"][0]["key"], "timer:1");
    assert_eq!(started["records"][0]["state"], "running");
    let rung = loop {
        let change = client.change();
        if change["changed"][0]["state"] == "rung" {
            break change;
        }
    };
    assert_eq!(rung["id"], "all");
    assert_eq!(rung["changed"][0]["late"], false);
    waits_for(|| !server.shown.lock().unwrap().is_empty());
    let shown = server.shown.lock().unwrap()[0].clone();
    assert_eq!(shown.app, "Timers");
    assert_eq!(shown.summary, "Tea");
    assert_eq!(shown.body, "1 s timer");
    assert_eq!(shown.actions, vec!["again", "Again"]);
    assert_eq!(shown.urgency, Some(2));
    assert_eq!(shown.timeout, 0);
    running.stop();
}

#[test]
fn the_notices_again_sets_it_going_and_putting_it_away_ends_it() {
    let folder = folder("acts");
    let bus = Bus::new(&folder);
    let (connection, server) = bus.serve();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    client.ask(&json!({"id": 1, "ask": "start", "span": 200, "name": "tea"}));
    waits_for(|| server.shown.lock().unwrap().len() == 1);
    signal(&folder, &connection, "ActionInvoked", &(1u32, "again"));
    let again = loop {
        let change = client.change();
        if change["changed"][0]["state"] == "running" {
            break change;
        }
    };
    assert_eq!(again["changed"][0]["key"], "timer:1");
    waits_for(|| server.shown.lock().unwrap().len() == 2);
    signal(&folder, &connection, "NotificationClosed", &(2u32, 2u32));
    // Past its ringing again, which it told first.
    let ended = loop {
        let change = client.change();
        if change["gone"] != json!([]) {
            break change;
        }
    };
    assert_eq!(ended["gone"], json!(["timer:1"]));
    let all = client.ask(&json!({"id": "get", "ask": "get", "keys": ["timer:1"]}));
    assert_eq!(all["records"], json!([]));
    running.stop();
}

#[test]
fn ending_a_rung_timer_takes_its_notice_back() {
    let folder = folder("withdraws");
    let bus = Bus::new(&folder);
    let (_connection, server) = bus.serve();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "start", "span": 100}));
    waits_for(|| server.shown.lock().unwrap().len() == 1);
    assert_eq!(server.shown.lock().unwrap()[0].summary, "Timer");
    let ended = client.ask(&json!({"id": 2, "ask": "end", "key": "timer:1"}));
    assert_eq!(ended["gone"], json!(["timer:1"]));
    waits_for(|| *server.closed.lock().unwrap() == vec![1]);
    running.stop();
}

#[test]
fn a_ring_with_no_server_to_take_it_is_said_once_one_comes() {
    let folder = folder("waits");
    let bus = Bus::new(&folder);
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    client.ask(&json!({"id": 1, "ask": "start", "span": 100, "name": "tea"}));
    assert_eq!(client.change()["changed"][0]["state"], "rung");
    let (_connection, server) = bus.serve();
    waits_for(|| server.shown.lock().unwrap().len() == 1);
    running.stop();
}

#[test]
fn what_is_kept_outlasts_the_daemon_and_one_ended_meanwhile_rings_late() {
    let folder = folder("kept");
    let bus = Bus::new(&folder);
    let (connection, server) = bus.serve();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "start", "span": 3_600_000, "name": "bread"}));
    client.ask(&json!({"id": 2, "ask": "start", "span": 600_000, "name": "tea"}));
    client.ask(&json!({"id": 3, "ask": "pause", "key": "timer:2"}));
    let soon = client.ask(&json!({"id": 4, "ask": "start", "span": 5_500, "name": "eggs"}));
    assert_eq!(soon["records"][0]["key"], "timer:3");
    running.stop();
    // A minute passes while nothing runs, as through a reboot: its end
    // is set a minute back in what was kept.
    let kept = folder.join("state").join("held.json");
    let mut held: Value = serde_json::from_str(&std::fs::read_to_string(&kept).unwrap()).unwrap();
    let ends = held["timers"][2]["ends"].as_u64().unwrap();
    held["timers"][2]["ends"] = json!(ends - 65_500);
    std::fs::write(&kept, held.to_string()).unwrap();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    let all = client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    let records = all["records"].as_array().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["name"], "bread");
    assert_eq!(records[0]["state"], "running");
    assert_eq!(records[1]["state"], "paused");
    assert_eq!(records[2]["state"], "rung");
    assert_eq!(records[2]["late"], true);
    waits_for(|| server.shown.lock().unwrap().len() == 1);
    assert_eq!(server.shown.lock().unwrap()[0].body, "6 s timer, rang late");
    // Said once, so a daemon started again after it rang says it no more
    // to the same server, once it has kept that it said it: one stopped
    // between the two says it again, as a notice once too often beats one
    // never said.
    waits_for(|| std::fs::read_to_string(&kept).is_ok_and(|text| text.contains(r#""notice":"#)));
    running.stop();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    let started = client.ask(&json!({"id": 5, "ask": "start", "span": 100}));
    assert_eq!(started["records"][0]["key"], "timer:4");
    waits_for(|| server.shown.lock().unwrap().len() == 2);
    assert_eq!(server.shown.lock().unwrap()[1].summary, "Timer");
    // The card said before it started again is still its own: put away,
    // it ends its timer.
    signal(&folder, &connection, "NotificationClosed", &(1u32, 2u32));
    let ended = loop {
        let change = client.change();
        if change["gone"] != json!([]) {
            break change;
        }
    };
    assert_eq!(ended["gone"], json!(["timer:3"]));
    running.stop();
}

#[test]
fn a_ring_lost_with_its_server_is_said_to_the_one_after_it() {
    let folder = folder("handed");
    let bus = Bus::new(&folder);
    let (connection, first) = bus.serve();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "start", "span": 100, "name": "tea"}));
    waits_for(|| first.shown.lock().unwrap().len() == 1);
    // The shell starts again, as on a switch: its cards go with it, and
    // the one after it hears the ring anew.
    drop(connection);
    let (handed, second) = bus.serve();
    waits_for(|| second.shown.lock().unwrap().len() == 1);
    assert_eq!(second.shown.lock().unwrap()[0].summary, "Tea");
    // The same while the daemon itself is down, as when a switch starts
    // both again.
    running.stop();
    drop(handed);
    let (connection, third) = bus.serve();
    let running = Running::start(&folder, &bus);
    waits_for(|| third.shown.lock().unwrap().len() == 1);
    // Putting it away on the server it's on now still ends it.
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "timer"}));
    signal(&folder, &connection, "NotificationClosed", &(1u32, 2u32));
    let ended = loop {
        let change = client.change();
        if change["gone"] != json!([]) {
            break change;
        }
    };
    assert_eq!(ended["gone"], json!(["timer:1"]));
    assert_eq!(first.shown.lock().unwrap().len(), 1);
    running.stop();
}

#[test]
fn the_stopwatch_runs_laps_and_stops_and_outlasts_the_daemon() {
    let folder = folder("stopwatch");
    let bus = Bus::new(&folder);
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    let all = client.ask(&json!({"id": "all", "ask": "watch", "kind": ["timer", "stopwatch"]}));
    assert_eq!(all["records"][0]["state"], "unstarted");
    let mut timers = running.client();
    timers.ask(&json!({"id": "timers", "ask": "watch", "kind": "timer"}));
    let mut watching = running.client();
    watching.ask(&json!({"id": "it", "ask": "watch", "kind": "stopwatch"}));
    assert_eq!(
        client.ask(&json!({"id": 1, "ask": "run"}))["records"][0]["state"],
        "running"
    );
    assert_eq!(watching.change()["changed"][0]["state"], "running");
    let lapped = client.ask(&json!({"id": 2, "ask": "lap"}));
    assert_eq!(lapped["records"][0]["laps"].as_array().unwrap().len(), 1);
    let stopped = client.ask(&json!({"id": 3, "ask": "stop"}));
    assert_eq!(stopped["records"][0]["state"], "stopped");
    let counted = stopped["records"][0]["counted"].as_u64().unwrap();
    // A watch of timers alone heard none of it: the first it hears is a
    // timer's.
    client.ask(&json!({"id": 4, "ask": "start", "span": 600_000}));
    assert_eq!(timers.change()["changed"][0]["key"], "timer:1");
    running.stop();
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    let kept = client.ask(&json!({"id": "get", "ask": "get", "keys": ["stopwatch"]}));
    assert_eq!(kept["records"][0]["state"], "stopped");
    assert_eq!(kept["records"][0]["counted"], counted);
    assert_eq!(kept["records"][0]["laps"].as_array().unwrap().len(), 1);
    let reset = client.ask(&json!({"id": 5, "ask": "reset"}));
    assert_eq!(reset["records"][0]["state"], "unstarted");
    let code = |answer: Value| answer["error"]["code"].as_str().unwrap().to_owned();
    assert_eq!(code(client.ask(&json!({"id": 6, "ask": "lap"}))), "refused");
    running.stop();
}

#[test]
fn a_timer_coming_due_is_heard_once_and_quietly_during_a_call() {
    let folder = folder("aloud");
    let bus = Bus::new(&folder);
    let (connection, server) = bus.serve();
    let record = folder.join("record");
    let calling = folder.join("calling");
    let player = folder.join("player");
    let call = folder.join("call");
    stand_in(&player, &format!("echo \"$*\" >> '{}'", record.display()));
    stand_in(&call, &format!("test -e '{}'", calling.display()));
    let sound = folder.join("ding.wav");
    let running = Running::heard(&folder, &bus, &[&player, &sound, &call]);
    let mut client = running.client();
    let heard = || {
        std::fs::read_to_string(&record)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    client.ask(&json!({"id": 1, "ask": "start", "span": 100, "name": "tea"}));
    waits_for(|| heard().len() == 1);
    assert_eq!(heard(), vec![sound.display().to_string()]);
    waits_for(|| server.shown.lock().unwrap().len() == 1);
    // Its notice said afresh to a server taking the last one's place
    // isn't heard again: it would be the next line, at full volume.
    drop(connection);
    let (_connection, after) = bus.serve();
    waits_for(|| after.shown.lock().unwrap().len() == 1);
    assert_eq!(server.shown.lock().unwrap().len(), 1);
    std::fs::write(&calling, "").unwrap();
    client.ask(&json!({"id": 2, "ask": "start", "span": 100, "name": "eggs"}));
    waits_for(|| after.shown.lock().unwrap().len() == 2);
    waits_for(|| heard().len() == 2);
    assert_eq!(heard()[1], format!("--volume=0.3 {}", sound.display()));
    // One found long past its end as the daemon starts, as after a night's
    // suspend, is said late but not heard: were it heard, its ring would
    // come next, at full volume with no call on.
    client.ask(&json!({"id": 3, "ask": "start", "span": 3_600_000, "name": "bread"}));
    let kept = folder.join("state").join("held.json");
    waits_for(|| {
        std::fs::read_to_string(&kept).is_ok_and(|text| text.matches(r#""notice":"#).count() == 2)
    });
    running.stop();
    let mut held: Value = serde_json::from_str(&std::fs::read_to_string(&kept).unwrap()).unwrap();
    let ends = held["timers"][2]["ends"].as_u64().unwrap();
    held["timers"][2]["ends"] = json!(ends - 3_700_000);
    std::fs::write(&kept, held.to_string()).unwrap();
    std::fs::remove_file(&calling).unwrap();
    let running = Running::heard(&folder, &bus, &[&player, &sound, &call]);
    waits_for(|| after.shown.lock().unwrap().len() == 3);
    assert!(after.shown.lock().unwrap()[2].body.ends_with("rang late"));
    std::fs::write(&calling, "").unwrap();
    let mut client = running.client();
    client.ask(&json!({"id": 4, "ask": "start", "span": 100}));
    waits_for(|| heard().len() == 3);
    assert_eq!(heard()[2], format!("--volume=0.3 {}", sound.display()));
    running.stop();
}

#[test]
fn an_ask_it_cant_do_is_refused() {
    let folder = folder("refused");
    let bus = Bus::new(&folder);
    let running = Running::start(&folder, &bus);
    let mut client = running.client();
    let code = |answer: Value| answer["error"]["code"].as_str().unwrap().to_owned();
    assert_eq!(
        code(client.ask(&json!({"id": 1, "ask": "pause", "key": "timer:9"}))),
        "not-found"
    );
    assert_eq!(
        code(client.ask(&json!({"id": 2, "ask": "start", "span": 0}))),
        "refused"
    );
    assert_eq!(
        code(client.ask(&json!({"id": 3, "ask": "end"}))),
        "malformed"
    );
    assert_eq!(
        code(client.ask(&json!({"id": 4, "ask": "watch", "kind": "person"}))),
        "unknown-kind"
    );
    assert_eq!(
        code(client.ask(&json!({"id": 5, "ask": "dance"}))),
        "unknown-ask"
    );
    client.ask(&json!({"id": 6, "ask": "start", "span": 60_000}));
    // A refusal says its reason, for the launcher to say in its own words.
    let refused = client.ask(&json!({"id": 7, "ask": "resume", "key": "timer:1"}));
    assert_eq!(refused["error"]["code"], "refused");
    assert_eq!(refused["error"]["reason"], "resumesPaused");
    running.stop();
}

/// Handed less than its unit hands it, or words it can't read, it ends at
/// once with the kit's exit, nothing on stdout, which no one reads, and no
/// socket left for the shell to find.
#[test]
fn a_run_its_unit_wouldnt_make_ends_at_once_in_the_kits_words() {
    let folder = folder("asked");
    let socket = folder.join("timers.sock");
    let kept = folder.join("held.json");
    let unread = folder.join("no-words.json");
    let cases: [(Vec<&Path>, i32, &str); 3] = [
        (vec![&socket, &kept], 64, "timers --help"),
        (vec![&socket, &kept, &unread, &socket], 64, "timers --help"),
        (vec![&socket, &kept, &unread], 78, "no-words.json"),
    ];
    for (args, exit, names) in cases {
        let ran = Command::new(env!("CARGO_BIN_EXE_timers"))
            .args(&args)
            .env_remove("JOURNAL_STREAM")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&ran.stderr);
        assert_eq!(
            ran.status.code(),
            Some(exit),
            "{args:?} ended {:?}, saying {said}",
            ran.status.code()
        );
        assert!(
            ran.stdout.is_empty(),
            "{args:?} said {:?} on stdout",
            String::from_utf8_lossy(&ran.stdout)
        );
        assert!(
            said.contains(names),
            "{args:?} said {said:?}, naming no {names}"
        );
        assert!(
            !socket.exists(),
            "{args:?} left a socket at {}",
            socket.display()
        );
    }
}
