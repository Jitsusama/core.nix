//! The program itself watching a USB camera, run as its unit runs it, over
//! a sysfs of its own: its file is there while the camera is, says nobody
//! while nobody streams, and goes when the camera's node does or the
//! program is stopped.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

struct Watch {
    root: PathBuf,
    node: PathBuf,
    file: PathBuf,
    program: Child,
}

impl Watch {
    /// A camera with its control and streaming interfaces, and the watch
    /// started on it.
    fn start(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("holders-usb-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let sysfs = root.join("sys");
        let usb = sysfs.join("devices/usb2/2-1");
        for (interface, class, sub) in [("2-1:1.0", "0e", "01"), ("2-1:1.1", "0e", "02")] {
            let folder = usb.join(interface);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("bInterfaceClass"), format!("{class}\n")).unwrap();
            fs::write(folder.join("bInterfaceSubClass"), format!("{sub}\n")).unwrap();
            fs::write(folder.join("bAlternateSetting"), " 0\n").unwrap();
        }
        let video = sysfs.join("class/video4linux/video0");
        fs::create_dir_all(&video).unwrap();
        symlink(usb.join("2-1:1.0"), video.join("device")).unwrap();
        fs::write(video.join("name"), "MX Brio 705 for Business\n").unwrap();
        // The node stands in as a folder, which inotify watches the same.
        let node = root.join("dev/video0");
        fs::create_dir_all(&node).unwrap();
        let folder = root.join("camera-holders");
        fs::create_dir_all(&folder).unwrap();
        // What a run killed mid-write would have left.
        fs::write(folder.join("MX Brio 705 for Business.99"), "").unwrap();
        let program = Command::new(env!("CARGO_BIN_EXE_loopback-holders"))
            .arg("--usb")
            .arg("--sysfs")
            .arg(&sysfs)
            .arg(&node)
            .arg(&folder)
            .spawn()
            .unwrap();
        Self {
            file: folder.join("MX Brio 705 for Business"),
            root,
            node,
            program,
        }
    }

    /// Waits up to five seconds for the camera's file to be there or not.
    fn until_there(&self, wanted: bool, what: &str) {
        for _ in 0..100 {
            if self.file.exists() == wanted {
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!("{what}");
    }

    fn ended(&mut self) -> Option<i32> {
        for _ in 0..100 {
            if let Some(status) = self.program.try_wait().unwrap() {
                return status.code();
            }
            sleep(Duration::from_millis(50));
        }
        None
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.program.kill();
        let _ = self.program.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn the_file_is_there_while_the_camera_is_and_names_nobody_idle() {
    let watch = Watch::start("there");
    watch.until_there(true, "no file while the camera's there");
    assert_eq!(fs::read_to_string(&watch.file).unwrap(), "");
    let left = watch.file.with_file_name("MX Brio 705 for Business.99");
    assert!(!left.exists(), "a killed run's file was left");
}

#[test]
fn the_file_goes_with_the_node() {
    let mut watch = Watch::start("node");
    watch.until_there(true, "no file while the camera's there");
    fs::remove_dir(&watch.node).unwrap();
    watch.until_there(false, "the file outlived the camera");
    assert_eq!(watch.ended(), Some(0));
}

#[test]
fn the_file_goes_when_it_is_stopped() {
    let mut watch = Watch::start("stopped");
    watch.until_there(true, "no file while the camera's there");
    let pid = i32::try_from(watch.program.id()).unwrap();
    // SAFETY: a plain call, to our own child.
    assert_eq!(unsafe { kill(pid, SIGTERM) }, 0);
    assert_eq!(watch.ended(), Some(0));
    assert!(!watch.file.exists());
}
