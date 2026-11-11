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
//! camera, not who may.

use serde_json::{Value, json};

use crate::Root;

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
/// the order its file lists them, each once; none when the folder isn't
/// there, as on a machine with no camera named.
pub fn cameras(root: &Root) -> Vec<Value> {
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
                if !programs.iter().any(|program| program == line) {
                    programs.push(line.to_owned());
                }
            }
            Some((name, programs))
        })
        .collect();
    cameras.sort();
    cameras
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

    #[test]
    fn a_file_being_written_or_hidden_is_no_camera() {
        for name in ["obs", "Logitech BRIO", "cam.v2", "cam.", "2"] {
            assert!(names(name), "{name} is a camera's");
        }
        for name in ["", "obs.4120", "Logitech BRIO.7", ".obs"] {
            assert!(!names(name), "{name:?} is no camera's");
        }
    }

    #[test]
    fn each_camera_says_who_holds_it_each_once() {
        let root = machine("held");
        assert_eq!(cameras(&root), Vec::<Value>::new(), "no folder, no camera");
        write(&root, "obs", "");
        write(&root, "Logitech BRIO", "zoom\n  firefox \n\nzoom\n");
        write(&root, "Logitech BRIO.4120", "zoom\n");
        fs::create_dir_all(root.path(FOLDER).join("folder")).unwrap();
        assert_eq!(
            cameras(&root),
            [
                json!({"key": "camera:Logitech BRIO", "kind": "camera", "name": "Logitech BRIO",
                       "programs": ["zoom", "firefox"]}),
                json!({"key": "camera:obs", "kind": "camera", "name": "obs", "programs": []}),
            ]
        );
    }
}
