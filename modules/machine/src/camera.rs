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
//! camera, not who may. Where `PipeWire` holds one, for the programs
//! reading it through `PipeWire`, they're named in its place, as
//! `pipewire` finds them, and a camera on is never said to be off.

mod pipewire;

use std::collections::HashSet;
use std::time::Instant;

use serde_json::{Value, json};

pub use pipewire::{Global, PW_CLI, PipeWire, readers, registry};

use crate::Root;
use pipewire::{PIPEWIRE, State, through_pipewire};

/// The folder each camera's file is kept in.
pub const FOLDER: &str = "/run/camera-holders";

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

/// Each camera with a file, by its name, with the programs holding it in
/// the order its file lists them, each once, and those reading through
/// `PipeWire` where it holds one, as the registry last listed shows them;
/// none when the folder isn't there, as on a machine with no camera
/// named.
pub fn cameras(root: &Root, pipewire: &PipeWire) -> Vec<Value> {
    let mut state = pipewire.state();
    let mut held = HashSet::new();
    let named = held_cameras(root)
        .into_iter()
        .map(|(name, programs)| {
            if !through_pipewire(&programs) {
                return (name, programs);
            }
            held.insert(name.clone());
            let device = root.read(&format!("{FOLDER}/.devices/{name}"));
            let through = state
                .graph
                .as_deref()
                .and_then(|graph| readers(graph, device.as_deref().map(str::trim), &name, root));
            if unseen(&mut state, &name, through.is_some()) {
                eprintln!(
                    "<4>PipeWire holds {name}, but its registry shows no such camera, so it's \
                     said held by PipeWire"
                );
            }
            (name, named_through(programs, through))
        })
        .collect::<Vec<_>>();
    state.unfound.retain(|name| held.contains(name));
    state.held_at = (!held.is_empty()).then(Instant::now);
    drop(state);
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

/// Whether to say that `PipeWire` holds a camera its registry shows
/// nothing of, as one whose format has moved on or whose camera is offered
/// under another path does, since it reads as `PipeWire` alone: said from
/// a listing that came whole this time, not one kept through a failure,
/// which says its own why, and once until the camera's shown or let go.
fn unseen(state: &mut State, name: &str, shown: bool) -> bool {
    if shown {
        state.unfound.remove(name);
        return false;
    }
    state.graph.is_some() && state.failing.is_none() && state.unfound.insert(name.to_owned())
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::pipewire::LIST_WAIT;
    use super::pipewire::tests::{listed_whole, listing, machine, named, processes, pw_cli};
    use super::*;

    fn write(root: &Root, name: &str, text: &str) {
        let folder = root.path(FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(name), text).unwrap();
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
        let pipewire = PipeWire::new(&root, None);
        assert_eq!(
            cameras(&root, &pipewire),
            Vec::<Value>::new(),
            "no folder, no camera"
        );
        write(&root, "obs", "");
        write(&root, "Logitech BRIO", "zoom\n  firefox \n\nzoom\n");
        write(&root, "Logitech BRIO.4120", "zoom\n");
        fs::create_dir_all(root.path(FOLDER).join("folder")).unwrap();
        assert_eq!(
            cameras(&root, &pipewire),
            [
                json!({"key": "camera:Logitech BRIO", "kind": "camera", "name": "Logitech BRIO",
                       "programs": ["zoom", "firefox"]}),
                json!({"key": "camera:obs", "kind": "camera", "name": "obs", "programs": []}),
            ]
        );
        assert_eq!(pipewire.due(), None, "nothing held through PipeWire");
    }

    #[test]
    fn who_reads_through_pipewire_takes_its_place_among_the_holders() {
        let programs = |names: &[&str]| names.iter().map(|&name| name.to_owned()).collect();
        assert_eq!(
            named_through(
                programs(&["obs", "pipewire", "wireplumber"]),
                named(&["obs", "tuple"])
            ),
            programs(&["obs", "tuple"])
        );
        assert_eq!(
            named_through(programs(&["pipewire", "obs"]), None),
            programs(&["pipewire", "obs"])
        );
    }

    #[test]
    fn a_camera_held_through_pipewire_names_its_readers_and_is_read_again() {
        let root = machine("through");
        processes(&root);
        write(&root, "camera", "pipewire\nobs\n");
        fs::create_dir_all(root.path(FOLDER).join(".devices")).unwrap();
        fs::write(root.path(FOLDER).join(".devices/camera"), "/dev/video32\n").unwrap();
        let listed = root.path("/listed");
        let mut text = listing("Built-in Camera", "/dev/video32", 0).into_bytes();
        // A client named in bytes that are no UTF-8 spoils none of the rest.
        text.extend(
            b"\tid 90, type PipeWire:Interface:Client/3\n \t\tapplication.name = \"\xff\"\n",
        );
        fs::write(&listed, text).unwrap();
        let runs = root.path("/runs");
        let _alone = crate::acts::tests::alone();
        let cli = pw_cli(
            &root,
            &format!(
                "echo >> '{}'; [ \"$1\" = ls ] && cat '{}'",
                runs.display(),
                listed.display()
            ),
        );
        let pipewire = PipeWire::new(&root, Some(cli));
        let tries = listed_whole(&pipewire);
        // A try killed before its shell could say so leaves no line.
        let ran = fs::read_to_string(&runs).unwrap().lines().count();
        assert!(
            (1..=tries).contains(&ran),
            "pw-cli is run once a listing: {ran} runs in {tries} tries"
        );
        assert_eq!(
            cameras(&root, &pipewire)[0]["programs"],
            json!(["chrome", "tuple", "obs"]),
            "by the node its device file says, a line and all"
        );
        assert!(pipewire.due().is_some(), "read again while it holds one");
        let ran = fs::read_to_string(&runs).unwrap();
        write(&root, "camera", "obs\n");
        pipewire.refresh();
        assert_eq!(cameras(&root, &pipewire)[0]["programs"], json!(["obs"]));
        assert_eq!(pipewire.due(), None, "not read again once it lets go");
        assert_eq!(
            fs::read_to_string(&runs).unwrap(),
            ran,
            "pw-cli is run only while PipeWire holds a camera"
        );
    }

    #[test]
    fn a_camera_its_registry_shows_nothing_of_is_said_once_until_it_does() {
        let root = machine("unfound");
        processes(&root);
        write(&root, "camera", "pipewire\n");
        let listed = root.path("/listed");
        fs::write(&listed, listing("Built-in Camera", "/dev/video40", 0)).unwrap();
        let _alone = crate::acts::tests::alone();
        let cli = pw_cli(&root, &format!("cat '{}'", listed.display()));
        let pipewire = PipeWire::new(&root, Some(cli));
        listed_whole(&pipewire);
        let programs = || cameras(&root, &pipewire)[0]["programs"].clone();
        pipewire.state().failing = Some("a listing that failed".to_owned());
        assert_eq!(programs(), json!(["pipewire"]));
        assert!(
            pipewire.state().unfound.is_empty(),
            "not said from a listing kept through a failure"
        );
        pipewire.state().failing = None;
        assert_eq!(programs(), json!(["pipewire"]));
        assert!(pipewire.state().unfound.contains("camera"), "said");
        assert!(
            !unseen(&mut pipewire.state(), "camera", false),
            "said only once"
        );
        fs::write(&listed, listing("camera", "/dev/video40", 0)).unwrap();
        listed_whole(&pipewire);
        assert_eq!(
            programs(),
            json!(["chrome", "tuple"]),
            "found by its name once it's shown"
        );
        assert!(
            pipewire.state().unfound.is_empty(),
            "to be said again once it's shown"
        );
        fs::write(&listed, listing("Built-in Camera", "/dev/video40", 0)).unwrap();
        listed_whole(&pipewire);
        programs();
        assert!(pipewire.state().unfound.contains("camera"), "said again");
        write(&root, "other", "pipewire\n");
        write(&root, "camera", "");
        programs();
        assert!(
            !pipewire.state().unfound.contains("camera"),
            "to be said again once it's let go"
        );
    }

    #[test]
    fn a_registry_that_cant_be_listed_leaves_pipewire_named() {
        let root = machine("unlisted");
        processes(&root);
        write(&root, "camera", "pipewire\n");
        // Read whole, it would name the camera's readers.
        let listed = root.path("/listed");
        fs::write(&listed, listing("camera", "/dev/video32", 0)).unwrap();
        let fails = format!("cat '{}'; exit 1", listed.display());
        let _alone = crate::acts::tests::alone();
        for (why, body) in [
            ("none handed over", None),
            ("one that fails after its listing", Some(fails.as_str())),
            ("one that says nothing it can read", Some("echo nope")),
            ("one that hangs", Some("exec sleep 5")),
            ("one that says too much", Some("exec yes")),
        ] {
            let pipewire = PipeWire::new(&root, body.map(|body| pw_cli(&root, body)));
            let started = Instant::now();
            pipewire.refresh();
            let took = started.elapsed();
            assert!(took < LIST_WAIT * 2, "{why}: waited out");
            if why == "one that hangs" {
                assert!(took >= LIST_WAIT, "{why}: waited for, not failed at once");
            }
            // A shell forked on a starved builder may miss the wait, so the
            // two that answer are asked again until they have.
            let answered = || {
                let failing = pipewire.state().failing.clone();
                match why {
                    "one that says too much" => failing.is_some_and(|why| why.contains("more")),
                    "one that says nothing it can read" => failing.is_none(),
                    _ => true,
                }
            };
            for _ in 0..4 {
                if answered() {
                    break;
                }
                pipewire.refresh();
            }
            if why == "one that says too much" {
                // Its cap says so, where a wait run out would say that.
                assert!(
                    pipewire
                        .state()
                        .failing
                        .as_deref()
                        .is_some_and(|why| why.contains("more than")),
                    "{why}: refused, not read as far as it got"
                );
            }
            assert_eq!(
                cameras(&root, &pipewire)[0]["programs"],
                json!(["pipewire"]),
                "{why}: the camera reads on"
            );
            assert_eq!(
                pipewire.state().unfound.contains("camera"),
                why == "one that says nothing it can read",
                "{why}: said to show no such camera only from a listing read"
            );
            assert!(pipewire.due().is_some(), "{why}: read again");
        }
    }
}
