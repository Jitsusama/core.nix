//! What sysfs says of a camera: which node a loopback is, whether a USB
//! camera streams, and what it's called.

use std::fs;
use std::path::{Path, PathBuf};

use crate::holders::text;

/// A node's folder in sysfs.
fn video_folder(sysfs: &Path, node: &Path) -> PathBuf {
    sysfs
        .join("class/video4linux")
        .join(node.file_name().unwrap_or_default())
}

/// The node the source is, or names, once there is one, and of the card
/// when one is asked for, so a node number another camera has since been
/// given isn't taken for this one.
pub fn node_of(sysfs: &Path, source: &Path, card: Option<&str>) -> Option<PathBuf> {
    let node = if source.exists() && !source.is_file() {
        fs::canonicalize(source).ok()?
    } else {
        PathBuf::from(fs::read_to_string(source).ok()?.trim())
    };
    if node.as_os_str().is_empty() || !node.exists() {
        return None;
    }
    if let Some(card) = card {
        let name = fs::read(video_folder(sysfs, &node).join("name")).ok()?;
        if String::from_utf8_lossy(&name).trim() != card {
            return None;
        }
    }
    Some(node)
}

fn attribute(folder: &Path, name: &str) -> Option<String> {
    Some(
        fs::read_to_string(folder.join(name))
            .ok()?
            .trim()
            .to_owned(),
    )
}

/// Whether a USB camera is streaming: one of its video streaming
/// interfaces (class 0e, subclass 02) has an alternate setting other than
/// zero, which uvcvideo picks to start a stream and puts back to stop it.
/// Its interfaces sit beside the one its node's device is. A camera that
/// streams over bulk endpoints stays at zero, so it never counts.
pub fn streams(sysfs: &Path, node: &Path) -> bool {
    let Ok(device) = fs::canonicalize(video_folder(sysfs, node).join("device")) else {
        return false;
    };
    let Some(Ok(interfaces)) = device.parent().map(fs::read_dir) else {
        return false;
    };
    interfaces.flatten().any(|interface| {
        let folder = interface.path();
        folder.is_dir()
            && attribute(&folder, "bInterfaceClass").as_deref() == Some("0e")
            && attribute(&folder, "bInterfaceSubClass").as_deref() == Some("02")
            && attribute(&folder, "bAlternateSetting")
                .and_then(|setting| setting.parse::<u32>().ok())
                .is_some_and(|setting| setting != 0)
    })
}

/// FOLDER/<the camera's name>, as its driver gives it, made a single plain
/// file name.
pub fn camera_file(sysfs: &Path, node: &Path, folder: &Path) -> std::io::Result<PathBuf> {
    let name = fs::read(video_folder(sysfs, node).join("name"))?;
    let called = text(&name).replace('/', "-");
    let called = called.trim_start_matches('.');
    Ok(if called.is_empty() {
        folder.join(node.file_name().unwrap_or_default())
    } else {
        folder.join(called)
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    /// A folder of the test's own, gone when it is.
    pub struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub fn scratch(name: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("holders-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Scratch(root)
    }

    /// A USB camera as sysfs shows one, with its control interface (which
    /// the node's device is), a streaming interface, and an audio interface
    /// that has an alternate setting of its own.
    pub struct Camera {
        pub root: Scratch,
        pub sysfs: PathBuf,
        pub node: PathBuf,
        usb: PathBuf,
    }

    impl Camera {
        /// The camera, in a scratch folder named for the test.
        pub fn new(test: &str, name: &str) -> Self {
            let root = scratch(test);
            let sysfs = root.join("sys");
            let usb = sysfs.join("devices/usb2/2-1");
            for (interface, class, sub) in [
                ("2-1:1.0", "0e", "01"),
                ("2-1:1.1", "0e", "02"),
                ("2-1:1.4", "01", "02"),
            ] {
                let folder = usb.join(interface);
                fs::create_dir_all(&folder).unwrap();
                fs::write(folder.join("bInterfaceClass"), format!("{class}\n")).unwrap();
                fs::write(folder.join("bInterfaceSubClass"), format!("{sub}\n")).unwrap();
                fs::write(folder.join("bAlternateSetting"), " 0\n").unwrap();
            }
            let video = sysfs.join("class/video4linux/video0");
            fs::create_dir_all(&video).unwrap();
            symlink(usb.join("2-1:1.0"), video.join("device")).unwrap();
            fs::write(video.join("name"), format!("{name}\n")).unwrap();
            // The node stands in as a folder: it exists and isn't a file,
            // which is all that's asked of it before it's watched.
            let node = root.join("dev/video0");
            fs::create_dir_all(&node).unwrap();
            Self {
                root,
                sysfs,
                node,
                usb,
            }
        }

        pub fn alternate(&self, interface: &str, setting: u32) {
            let path = self.usb.join(interface).join("bAlternateSetting");
            fs::write(path, format!("{setting:2}\n")).unwrap();
        }
    }

    #[test]
    fn an_idle_camera_isnt_streaming() {
        let camera = Camera::new("idle", "MX Brio 705 for Business");
        assert!(!streams(&camera.sysfs, &camera.node));
    }

    #[test]
    fn a_streaming_interface_with_an_alternate_setting_streams() {
        let camera = Camera::new("streaming", "MX Brio 705 for Business");
        camera.alternate("2-1:1.1", 9);
        assert!(streams(&camera.sysfs, &camera.node));
        camera.alternate("2-1:1.1", 0);
        assert!(!streams(&camera.sysfs, &camera.node));
    }

    #[test]
    fn only_a_video_streaming_interface_counts() {
        let camera = Camera::new("audio", "MX Brio 705 for Business");
        camera.alternate("2-1:1.4", 1);
        assert!(!streams(&camera.sysfs, &camera.node));
    }

    #[test]
    fn a_camera_gone_from_sysfs_isnt_streaming() {
        let camera = Camera::new("gone", "MX Brio 705 for Business");
        assert!(!streams(&camera.sysfs, &camera.root.join("dev/video9")));
    }

    #[test]
    fn the_file_is_named_for_the_camera() {
        let camera = Camera::new("named", "MX Brio 705 for Business");
        assert_eq!(
            camera_file(
                &camera.sysfs,
                &camera.node,
                Path::new("/run/camera-holders")
            )
            .unwrap(),
            Path::new("/run/camera-holders/MX Brio 705 for Business")
        );
    }

    #[test]
    fn a_name_is_made_a_plain_file_name() {
        let camera = Camera::new("plain", "../Cam/Era\x07");
        assert_eq!(
            camera_file(
                &camera.sysfs,
                &camera.node,
                Path::new("/run/camera-holders")
            )
            .unwrap(),
            Path::new("/run/camera-holders/-Cam-Era?")
        );
    }

    #[test]
    fn a_loopback_counts_only_under_its_cards_name() {
        let camera = Camera::new("card", "Stand-in");
        let named = camera.root.join("device");
        fs::write(&named, format!("{}\n", camera.node.display())).unwrap();
        assert_eq!(
            node_of(&camera.sysfs, &named, None),
            Some(camera.node.clone())
        );
        assert_eq!(
            node_of(&camera.sysfs, &named, Some("Stand-in")),
            Some(camera.node.clone())
        );
        assert_eq!(node_of(&camera.sysfs, &named, Some("Other")), None);
        fs::write(&named, "/dev/video-not-yet\n").unwrap();
        assert_eq!(node_of(&camera.sysfs, &named, None), None);
        assert_eq!(
            node_of(&camera.sysfs, &camera.root.join("nothing"), None),
            None
        );
    }
}
