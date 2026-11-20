//! Who has each camera open, as the machine keeps it in
//! `/run/camera-holders`: a file for each camera, named for it, listing
//! the programs reading it one to a line and empty when nobody does
//! (modules/v4l2loopback's holders for each loopback camera, and
//! modules/usb-camera-holders' for each USB camera while it's plugged
//! in). Each file is written beside it, under its name and the writer's
//! pid, and renamed over it, so a name ending in a dot and a number is a
//! file being written, no camera of its own. Read here, each camera is a
//! record told only when its programs change (`camera:<name>`), so the
//! shell runs nothing and reads no file to say who's on camera.
//!
//! A holder is named by the program itself, so the record says who has a
//! camera, not who may. A program reading a camera through `PipeWire`
//! holds no camera itself: `PipeWire` does, for it. So where `PipeWire`
//! holds one, the programs reading it there are named instead, found in
//! `PipeWire`'s graph (`readers`) by the node `.devices/<name>` says the
//! camera is, or failing that by the camera's name, and `PipeWire` with
//! none reading through it is only looking at the camera, as it does as
//! one comes, and names nobody. Where its graph can't be read, `PipeWire`
//! is named, so a camera on is never said to be off.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::Root;
use crate::using::program;

/// The folder each camera's file is kept in.
pub const FOLDER: &str = "/run/camera-holders";

/// pw-dump, by its path, as the daemon's home configuration hands it; a
/// machine with none names `PipeWire` as each camera's holder it holds.
pub const PW_DUMP: &str = "MACHINE_PW_DUMP";

/// The programs that are `PipeWire` itself, holding a camera for whoever
/// reads it through them, by the name a camera's file gives them.
const PIPEWIRE: [&str; 2] = ["pipewire", "wireplumber"];

/// How long pw-dump is waited for, so a `PipeWire` that's stuck holds no
/// read of the cameras back; it answers in tens of milliseconds.
const DUMP_WAIT: Duration = Duration::from_secs(1);

/// How often the cameras are read again while `PipeWire` holds one, since
/// a program can start or stop reading it there while the camera's file
/// stays as it was.
const THROUGH_EVERY: Duration = Duration::from_secs(2);

/// Whether a file of this name in the folder is a camera's, rather than
/// one being written or hidden.
pub fn names(name: &str) -> bool {
    if name.is_empty() || name.starts_with('.') {
        return false;
    }
    !name
        .rsplit_once('.')
        .is_some_and(|(_, end)| !end.is_empty() && end.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Where `PipeWire`'s graph comes from, and whether the last read found
/// it holding a camera, which has the cameras read again on a clock.
#[derive(Default)]
pub struct PipeWire {
    /// pw-dump, or none to read no graph.
    dump: Option<PathBuf>,
    /// When the last read found `PipeWire` holding a camera.
    held_at: Option<Instant>,
}

impl PipeWire {
    /// Its graph from the pw-dump the environment names, if it names one.
    pub fn from_environment() -> Self {
        Self {
            dump: std::env::var_os(PW_DUMP)
                .filter(|dump| !dump.is_empty())
                .map(PathBuf::from),
            held_at: None,
        }
    }

    /// When the cameras are next due a read though nothing was written:
    /// while `PipeWire` holds a camera, a little after the last read.
    pub fn due(&self) -> Option<Instant> {
        self.held_at.map(|at| at + THROUGH_EVERY)
    }

    /// `PipeWire`'s graph as pw-dump says it, or none when it can't be had
    /// within `DUMP_WAIT`.
    fn graph(&self) -> Option<Vec<Value>> {
        let mut child = Command::new(self.dump.as_ref()?)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;
        let (said, heard) = mpsc::channel();
        // Its stdout closes as it ends, so what it said is the sign it's done.
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stdout.read_to_string(&mut text);
            let _ = said.send(text);
        });
        let Ok(text) = heard.recv_timeout(DUMP_WAIT) else {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        };
        let ended = child.wait().ok()?;
        if !ended.success() {
            return None;
        }
        serde_json::from_str(&text).ok()
    }
}

/// Each camera with a file, by its name, with the programs holding it in
/// the order its file lists them, each once, and those reading through
/// `PipeWire` where it holds one; none when the folder isn't there, as on
/// a machine with no camera named.
pub fn cameras(root: &Root, pipewire: &mut PipeWire) -> Vec<Value> {
    let mut graph = None;
    let mut held = false;
    let named = held_cameras(root)
        .into_iter()
        .map(|(name, programs)| {
            if !programs
                .iter()
                .any(|holder| PIPEWIRE.contains(&holder.as_str()))
            {
                return (name, programs);
            }
            held = true;
            let graph = graph.get_or_insert_with(|| pipewire.graph());
            let device = root.read(&format!("{FOLDER}/.devices/{name}"));
            let through = graph
                .as_deref()
                .and_then(|graph| readers(graph, device.as_deref().map(str::trim), &name));
            (name, named_through(programs, through))
        })
        .collect::<Vec<_>>();
    pipewire.held_at = held.then(Instant::now);
    named
        .into_iter()
        .map(|(name, programs)| {
            json!({
                "key": format!("camera:{name}"),
                "kind": "camera",
                "name": name,
                "programs": programs,
            })
        })
        .collect()
}

/// Each camera's file, by name, sorted, with the programs it lists, each
/// once.
fn held_cameras(root: &Root) -> Vec<(String, Vec<String>)> {
    let Ok(entries) = std::fs::read_dir(root.path(FOLDER)) else {
        return Vec::new();
    };
    let mut cameras: Vec<(String, Vec<String>)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| names(name))
        .filter_map(|name| {
            // A file gone between the listing and its read is a camera
            // unplugged as it was read, and is said gone with the next.
            let text = root.read(&format!("{FOLDER}/{name}"))?;
            let mut programs: Vec<String> = Vec::new();
            for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
                once(&mut programs, line);
            }
            Some((name, programs))
        })
        .collect();
    cameras.sort();
    cameras
}

/// The camera's holders with `PipeWire` given way to who reads through
/// it, in its place, or kept where who does isn't known.
fn named_through(programs: Vec<String>, through: Option<Vec<String>>) -> Vec<String> {
    let Some(through) = through else {
        return programs;
    };
    let mut named = Vec::new();
    for holder in programs {
        if PIPEWIRE.contains(&holder.as_str()) {
            for reader in &through {
                once(&mut named, reader);
            }
        } else {
            once(&mut named, &holder);
        }
    }
    named
}

/// Adds the name to the list unless it's there.
fn once(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|named| named == name) {
        names.push(name.to_owned());
    }
}

/// The programs reading a camera through `PipeWire`, as its graph shows
/// them: the camera's device, by its node or failing that by its name,
/// then its nodes, then whoever's linked to them, passing through any
/// node `PipeWire`'s own programs run, as a converter of theirs is, to
/// the program it serves. None when the graph shows no such camera, as
/// one not offered there is, and so who reads it isn't known.
pub fn readers(graph: &[Value], device: Option<&str>, card: &str) -> Option<Vec<String>> {
    let props = |object: &Value| object["info"]["props"].clone();
    let typed = |kind: &'static str| {
        graph
            .iter()
            .filter(move |object| object["type"] == format!("PipeWire:Interface:{kind}"))
    };
    let path = |object: &Value| props(object)["api.v4l2.path"].as_str().map(str::to_owned);
    let devices: Vec<u64> = {
        let by_path: Vec<u64> = typed("Device")
            .filter(|object| device.is_some() && path(object).as_deref() == device)
            .filter_map(|object| object["id"].as_u64())
            .collect();
        if by_path.is_empty() {
            typed("Device")
                .filter(|object| props(object)["api.v4l2.cap.card"] == card)
                .filter_map(|object| object["id"].as_u64())
                .collect()
        } else {
            by_path
        }
    };
    if devices.is_empty() {
        return None;
    }
    let node_client = |id: u64| {
        typed("Node")
            .find(|object| object["id"].as_u64() == Some(id))
            .and_then(|object| props(object)["client.id"].as_u64())
    };
    let binary = |client: u64| {
        typed("Client")
            .find(|object| object["id"].as_u64() == Some(client))
            .and_then(|object| {
                let props = props(object);
                props["application.process.binary"]
                    .as_str()
                    .or_else(|| props["application.name"].as_str())
                    .map(program)
            })
    };
    let mut frontier: Vec<u64> = typed("Node")
        .filter(|object| {
            props(object)["device.id"]
                .as_u64()
                .is_some_and(|id| devices.contains(&id))
        })
        .filter_map(|object| object["id"].as_u64())
        .collect();
    let mut seen = frontier.clone();
    let mut named = Vec::new();
    while let Some(node) = frontier.pop() {
        for link in typed("Link") {
            let info = &link["info"];
            if info["output-node-id"].as_u64() != Some(node) {
                continue;
            }
            let Some(input) = info["input-node-id"].as_u64() else {
                continue;
            };
            if seen.contains(&input) {
                continue;
            }
            seen.push(input);
            match node_client(input).and_then(binary) {
                Some(name) if PIPEWIRE.contains(&name.as_str()) => frontier.push(input),
                Some(name) => once(&mut named, &name),
                None => {}
            }
        }
    }
    Some(named)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn machine(name: &str) -> Root {
        let root =
            std::env::temp_dir().join(format!("machine-camera-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Root::new(root)
    }

    fn write(root: &Root, name: &str, text: &str) {
        let folder = root.path(FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(name), text).unwrap();
    }

    /// A graph as pw-dump gives one: the camera's device, its node, and
    /// for each program reading it, its client, its stream's node and
    /// the link between, with `PipeWire`'s own converter between the
    /// camera and Tuple, and a microphone Meet reads too.
    fn graph(card: &str, path: &str) -> Vec<Value> {
        let props = |props: Value| json!({"info": {"props": props}});
        let object = |id: u64, kind: &str, props_of: Value| {
            let mut object = props(props_of);
            object["id"] = json!(id);
            object["type"] = json!(format!("PipeWire:Interface:{kind}"));
            object
        };
        let link = |id: u64, from: u64, to: u64| {
            json!({"id": id, "type": "PipeWire:Interface:Link",
                   "info": {"output-node-id": from, "input-node-id": to, "state": "active"}})
        };
        vec![
            object(
                1,
                "Client",
                json!({"application.process.binary": "pipewire"}),
            ),
            object(
                2,
                "Client",
                json!({"application.process.binary": "wireplumber"}),
            ),
            object(
                3,
                "Client",
                json!({"application.process.binary": ".tuple-wrapped"}),
            ),
            object(4, "Client", json!({"application.name": "chrome"})),
            object(
                10,
                "Device",
                json!({"api.v4l2.path": path, "api.v4l2.cap.card": card,
                                        "client.id": 2}),
            ),
            object(
                11,
                "Node",
                json!({"device.id": 10, "client.id": 1,
                                      "media.class": "Video/Source"}),
            ),
            object(12, "Node", json!({"client.id": 2})),
            object(13, "Node", json!({"client.id": 3})),
            object(14, "Node", json!({"client.id": 4})),
            object(20, "Device", json!({"api.alsa.path": "hw:0"})),
            object(21, "Node", json!({"device.id": 20, "client.id": 1})),
            object(22, "Node", json!({"client.id": 4})),
            link(30, 11, 12),
            link(31, 12, 13),
            link(32, 11, 14),
            link(33, 21, 22),
        ]
    }

    #[test]
    fn a_file_being_written_or_hidden_is_no_camera() {
        for name in ["obs", "Logitech BRIO", "cam.v2", "cam.", "2"] {
            assert!(names(name), "{name} is a camera's");
        }
        for name in ["", "obs.4120", "Logitech BRIO.7", ".obs", ".devices"] {
            assert!(!names(name), "{name:?} is no camera's");
        }
    }

    #[test]
    fn each_camera_says_who_holds_it_each_once() {
        let root = machine("held");
        let mut pipewire = PipeWire::default();
        assert_eq!(
            cameras(&root, &mut pipewire),
            Vec::<Value>::new(),
            "no folder, no camera"
        );
        write(&root, "obs", "");
        write(&root, "Logitech BRIO", "zoom\n  firefox \n\nzoom\n");
        write(&root, "Logitech BRIO.4120", "zoom\n");
        fs::create_dir_all(root.path(FOLDER).join("folder")).unwrap();
        assert_eq!(
            cameras(&root, &mut pipewire),
            [
                json!({"key": "camera:Logitech BRIO", "kind": "camera", "name": "Logitech BRIO",
                       "programs": ["zoom", "firefox"]}),
                json!({"key": "camera:obs", "kind": "camera", "name": "obs", "programs": []}),
            ]
        );
        assert_eq!(pipewire.due(), None, "nothing held through PipeWire");
    }

    #[test]
    fn pipewire_unread_is_named_as_the_holder_and_read_again() {
        let root = machine("unread");
        write(&root, "camera", "pipewire\nobs\n");
        let mut pipewire = PipeWire::default();
        assert_eq!(
            cameras(&root, &mut pipewire)[0]["programs"],
            json!(["pipewire", "obs"]),
            "who reads through it unknown, it's named, so the camera reads on"
        );
        assert!(pipewire.due().is_some(), "read again while it holds one");
    }

    #[test]
    fn the_programs_reading_through_pipewire_are_found_by_the_node() {
        let graph = graph("Built-in Camera", "/dev/video32");
        assert_eq!(
            readers(&graph, Some("/dev/video32"), "camera"),
            Some(vec!["chrome".to_owned(), "tuple".to_owned()]),
        );
    }

    #[test]
    fn a_camera_with_no_node_said_is_found_by_its_name() {
        let graph = graph("MX Brio 705 for Business", "/dev/video40");
        let readers_of = |device| readers(&graph, device, "MX Brio 705 for Business");
        let both = Some(vec!["chrome".to_owned(), "tuple".to_owned()]);
        assert_eq!(readers_of(None), both);
        assert_eq!(
            readers_of(Some("/dev/video9")),
            both,
            "a node since renumbered"
        );
    }

    #[test]
    fn a_camera_pipewire_doesnt_offer_has_its_readers_unknown() {
        let graph = graph("Built-in Camera", "/dev/video32");
        assert_eq!(readers(&graph, Some("/dev/video40"), "ir-camera"), None);
        assert_eq!(readers(&[], Some("/dev/video32"), "camera"), None);
    }

    #[test]
    fn pipewire_with_no_reader_is_only_looking() {
        let mut graph = graph("Built-in Camera", "/dev/video32");
        graph.retain(|object| object["type"] != "PipeWire:Interface:Link");
        assert_eq!(
            readers(&graph, Some("/dev/video32"), "camera"),
            Some(Vec::new())
        );
    }

    #[test]
    fn who_reads_through_pipewire_takes_its_place_among_the_holders() {
        let through = Some(vec!["tuple".to_owned(), "obs".to_owned()]);
        let programs = |names: &[&str]| names.iter().map(|&name| name.to_owned()).collect();
        assert_eq!(
            named_through(
                programs(&["obs", "pipewire", "wireplumber"]),
                through.clone()
            ),
            programs(&["obs", "tuple"])
        );
        assert_eq!(
            named_through(programs(&["pipewire"]), Some(Vec::new())),
            Vec::<String>::new()
        );
        assert_eq!(
            named_through(programs(&["pipewire", "obs"]), None),
            programs(&["pipewire", "obs"])
        );
    }
}
