//! The daemon as senders and the shell meet it: a session bus of the
//! test's own, started with dbus-daemon from the build's inputs, the
//! daemon serving the notifications name on it, and a client asking it
//! over its socket.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use daemon_client::{Client, Daemon, until};
use serde_json::{Value, json};
use zbus::blocking::{Connection, MessageIterator, Proxy, fdo::DBusProxy};
use zbus::zvariant::Value as Variant;

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

/// A session bus of the test's own, listening in the test's folder,
/// which goes with it.
struct Bus {
    child: Child,
    address: String,
    folder: PathBuf,
}

impl Bus {
    fn start(folder: &Path) -> Bus {
        let config = folder.join("bus.conf");
        std::fs::write(
            &config,
            format!(
                "<busconfig><type>session</type>\
                 <listen>unix:dir={}</listen><auth>EXTERNAL</auth>\
                 <policy context=\"default\"><allow send_destination=\"*\" eavesdrop=\"true\"/>\
                 <allow eavesdrop=\"true\"/><allow own=\"*\"/></policy></busconfig>",
                folder.display()
            ),
        )
        .expect("the bus's config is written");
        let mut child = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.display()))
            .arg("--nofork")
            .arg("--print-address")
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon starts");
        let mut address = String::new();
        BufReader::new(child.stdout.take().expect("its address is piped"))
            .read_line(&mut address)
            .expect("dbus-daemon says its address");
        Bus {
            child,
            address: address.trim().to_owned(),
            folder: folder.to_owned(),
        }
    }

    fn connect(&self) -> Connection {
        zbus::blocking::connection::Builder::address(self.address.as_str())
            .and_then(zbus::blocking::connection::Builder::build)
            .expect("the bus is reached")
    }
}

/// Who holds the notifications name on the bus this reaches, if anyone.
fn owner(connection: &Connection) -> Option<String> {
    let bus = DBusProxy::new(connection).expect("the bus answers");
    let name = NAME.try_into().expect("the name is well formed");
    bus.get_name_owner(name).ok().map(|owner| owner.to_string())
}

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// A folder of the test's own, with every sender's level, none set.
fn folder(test: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("notices-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is made");
    std::fs::write(folder.join("levels.json"), "{}").expect("the levels are written");
    folder
}

/// The daemon over the test's folder and bus.
fn notices(folder: &Path, bus: &Bus) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_notices"));
    command
        .arg("--socket")
        .arg(folder.join("notices.sock"))
        .arg("--state")
        .arg(folder.join("state"))
        .arg("--levels")
        .arg(folder.join("levels.json"))
        .arg("--senders")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/senders.json"))
        .arg("--quiet")
        .arg(folder.join("quiet"))
        .env("DBUS_SESSION_BUS_ADDRESS", &bus.address);
    command
}

/// Sends a notice as a sender does, saying the id it was given.
fn notify(
    connection: &Connection,
    summary: &str,
    actions: &[&str],
    hints: HashMap<&str, Variant>,
) -> u32 {
    Proxy::new(connection, NAME, PATH, NAME)
        .and_then(|proxy| {
            proxy.call(
                "Notify",
                &(
                    "kitty",
                    0u32,
                    "kitty",
                    summary,
                    "<b>make</b> finished",
                    actions,
                    hints,
                    5000i32,
                ),
            )
        })
        .expect("the notice is taken")
}

/// The record held under this key, as the shell would get it.
fn got(client: &mut Client, id: u64, key: &str) -> Value {
    let answer = client.ask(&json!({"id": id, "ask": "get", "keys": [key]}));
    answer["records"][0].clone()
}

/// The record held under this key once there is one: the bus answers a
/// notice as it's taken, and the socket tells it a moment after, as the
/// daemon's loop hands it over.
fn held(client: &mut Client, key: &str) -> Value {
    let (client, found) = (RefCell::new(client), RefCell::new(Value::Null));
    until(&format!("{key} was never held"), || {
        let record = got(&mut client.borrow_mut(), 1, key);
        let there = !record.is_null();
        *found.borrow_mut() = record;
        there
    });
    found.into_inner()
}

#[test]
fn a_notice_is_one_record_and_its_acts_are_said_on_the_bus() {
    let folder = folder("acts");
    let bus = Bus::start(&folder);
    let connection = bus.connect();
    // Its history goes as it stops, and the bus takes the rest.
    let daemon = Daemon::start(
        &mut notices(&folder, &bus),
        folder.join("notices.sock"),
        folder.join("state"),
    );
    until("the daemon never took the name", || {
        owner(&connection).is_some()
    });
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    let watched = client.ask(&json!({"id": 2, "ask": "watch", "kind": "notice"}));
    assert_eq!(watched["records"], json!([]), "none held yet: {watched}");

    let capabilities: Vec<String> = Proxy::new(&connection, NAME, PATH, NAME)
        .and_then(|proxy| proxy.call("GetCapabilities", &()))
        .expect("it says what it does");
    assert_eq!(
        capabilities,
        [
            "body",
            "body-markup",
            "body-hyperlinks",
            "actions",
            "icon-static",
            "inline-reply"
        ]
    );

    let hints = HashMap::from([
        ("urgency", Variant::U8(2)),
        ("desktop-entry", Variant::from("kitty")),
    ]);
    let id = notify(&connection, "Done", &["default", "Open"], hints);
    let key = format!("notice:{id}");
    let record = client.changed(&key);
    let notice = &record["notice"];
    assert_eq!(record["kind"], "notice");
    assert_eq!(notice["summary"], "Done");
    assert_eq!(
        notice["body"], "<b>make</b> finished",
        "the markup kept as sent"
    );
    assert_eq!(notice["entry"], "kitty");
    assert_eq!(notice["urgency"], "critical");
    assert_eq!(notice["live"], true);
    assert_eq!(
        notice["actions"],
        json!([{"id": "default", "text": "Open"}])
    );

    let signals = MessageIterator::for_match_rule(
        "type='signal',interface='org.freedesktop.Notifications'",
        &connection,
        None,
    )
    .expect("the signals are heard");
    let invoked = json!({"id": 3, "ask": "invoke", "notice": id, "action": "default",
        "token": "token-1"});
    assert_eq!(client.ask(&invoked)["done"], true);
    // The token first, so the sender can raise its window with it as it
    // hears the action, then the action, then the card closed.
    let said: Vec<(String, String)> = signals
        .take(3)
        .map(|message| {
            let message = message.expect("a signal is heard");
            let member = message
                .header()
                .member()
                .map(ToString::to_string)
                .unwrap_or_default();
            let body = if member == "NotificationClosed" {
                let (from, why): (u32, u32) = message.body().deserialize().expect("(uu)");
                assert_eq!(from, id);
                why.to_string()
            } else {
                let (from, what): (u32, String) = message.body().deserialize().expect("(us)");
                assert_eq!(from, id);
                what
            };
            (member, body)
        })
        .collect();
    assert_eq!(
        said,
        [
            ("ActivationToken".to_owned(), "token-1".to_owned()),
            ("ActionInvoked".to_owned(), "default".to_owned()),
            ("NotificationClosed".to_owned(), "2".to_owned()),
        ]
    );
    assert_eq!(
        got(&mut client, 4, &key)["notice"]["live"],
        false,
        "closed once acted on"
    );
    let again = client.ask(&invoked);
    assert_eq!(
        again["error"]["code"], "not-open",
        "a closed card does nothing: {again}"
    );
}

#[test]
fn the_name_is_never_taken_from_another_server() {
    let folder = folder("owned");
    let bus = Bus::start(&folder);
    let other = bus.connect();
    other
        .request_name(NAME)
        .expect("another server takes the name");
    let daemon = RefCell::new(
        notices(&folder, &bus)
            .stderr(Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    until("the daemon never stopped", || {
        daemon.borrow_mut().try_wait().ok().flatten().is_some()
    });
    let stopped = daemon
        .into_inner()
        .wait_with_output()
        .expect("the daemon is waited on");
    let said = String::from_utf8_lossy(&stopped.stderr);
    assert!(
        !stopped.status.success() && said.contains("is another's"),
        "it stops failing rather than serving: {} {said}",
        stopped.status
    );
    let owner = owner(&other);
    assert_eq!(
        owner.as_deref(),
        other.unique_name().map(|name| name.as_str())
    );
}

#[test]
fn the_history_and_its_ids_outlive_a_restart() {
    let folder = folder("restart");
    let bus = Bus::start(&folder);
    let connection = bus.connect();
    let socket = folder.join("notices.sock");
    let start = || {
        let daemon = notices(&folder, &bus).spawn().expect("the daemon starts");
        until("the daemon never took the name", || {
            owner(&connection).is_some()
        });
        until("the socket never came", || socket.exists());
        daemon
    };
    let mut daemon = start();
    // One red pixel, sent as pixels rather than a file.
    let pixel = Variant::from((1i32, 1i32, 3i32, false, 8i32, 3i32, vec![255u8, 0, 0]));
    let first = notify(
        &connection,
        "First",
        &[],
        HashMap::from([("image-data", pixel)]),
    );
    let key = format!("notice:{first}");
    let mut client = Client::connect(&socket);
    let picture = held(&mut client, &key)["notice"]["picture"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let file = picture
        .strip_prefix("file://")
        .expect("the picture is a file");
    assert!(
        Path::new(file).is_file(),
        "the pixels are written: {picture}"
    );
    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = std::fs::remove_file(&socket);

    let mut daemon = start();
    let mut client = Client::connect(&socket);
    let kept = held(&mut client, &key);
    assert_eq!(
        kept["notice"]["summary"], "First",
        "kept over the restart: {kept}"
    );
    assert_eq!(kept["notice"]["picture"], picture.as_str());
    let second = notify(&connection, "Second", &[], HashMap::new());
    assert!(
        second > first,
        "no id is given twice: {first} then {second}"
    );
    let _ = daemon.kill();
    let _ = daemon.wait();
}
