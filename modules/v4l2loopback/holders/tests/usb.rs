//! The program itself watching a USB camera, run as its unit runs it, over
//! a sysfs of its own: its file is there while the camera is, says nobody
//! while nobody streams, goes when the camera's node does or the program
//! is stopped, and while the camera streams, its log says how.

use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
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
    /// The streaming interface, uvcvideo's count of it, and the log.
    streaming: PathBuf,
    stats: PathBuf,
    log: PathBuf,
    program: Child,
}

impl Watch {
    /// A camera with its control and streaming interfaces, and the watch
    /// started on it, its node standing in as a folder.
    fn start(name: &str) -> Self {
        Self::on(name, None)
    }

    /// The same, with a real device's node for the camera's, which a
    /// program holding it is seen holding, as only a node under /dev is.
    fn on(name: &str, device: Option<&Path>) -> Self {
        let root = std::env::temp_dir().join(format!("holders-usb-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let sysfs = root.join("sys");
        let usb = sysfs.join("devices/usb2/2-1");
        fs::create_dir_all(&usb).unwrap();
        fs::write(usb.join("busnum"), "2\n").unwrap();
        fs::write(usb.join("devnum"), "7\n").unwrap();
        for (interface, class, sub, number) in
            [("2-1:1.0", "0e", "01", "00"), ("2-1:1.1", "0e", "02", "01")]
        {
            let folder = usb.join(interface);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("bInterfaceNumber"), format!("{number}\n")).unwrap();
            fs::write(folder.join("bInterfaceClass"), format!("{class}\n")).unwrap();
            fs::write(folder.join("bInterfaceSubClass"), format!("{sub}\n")).unwrap();
            fs::write(folder.join("bAlternateSetting"), " 0\n").unwrap();
        }
        // The node stands in as a folder, which inotify watches the same.
        let node = device.map_or_else(|| root.join("dev/video0"), Path::to_owned);
        if device.is_none() {
            fs::create_dir_all(&node).unwrap();
        }
        let video = sysfs
            .join("class/video4linux")
            .join(node.file_name().unwrap());
        fs::create_dir_all(&video).unwrap();
        symlink(usb.join("2-1:1.0"), video.join("device")).unwrap();
        fs::write(video.join("name"), "MX Brio 705 for Business\n").unwrap();
        let folder = root.join("camera-holders");
        fs::create_dir_all(&folder).unwrap();
        // What a run killed mid-write would have left.
        fs::write(folder.join("MX Brio 705 for Business.99"), "").unwrap();
        let debugfs = root.join("debug");
        let stats = debugfs.join("usb/uvcvideo/2-7-1/stats");
        fs::create_dir_all(stats.parent().unwrap()).unwrap();
        let log = root.join("log");
        let program = Command::new(env!("CARGO_BIN_EXE_loopback-holders"))
            .arg("--usb")
            .arg("--sysfs")
            .arg(&sysfs)
            .arg("--debugfs")
            .arg(&debugfs)
            .arg(&node)
            .arg(&folder)
            .stdout(File::create(&log).unwrap())
            .spawn()
            .unwrap();
        Self {
            file: folder.join("MX Brio 705 for Business"),
            streaming: usb.join("2-1:1.1/bAlternateSetting"),
            stats,
            log,
            root,
            node,
            program,
        }
    }

    /// Has uvcvideo say it has counted these frames of the stream.
    fn count(&self, frames: u64) {
        let text = format!("frames:  {frames}\npackets: 0\nempty:   0\nerrors:  0\ninvalid: 0\n");
        fs::write(&self.stats, text).unwrap();
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
fn while_the_camera_streams_its_log_says_what_it_gave() {
    // /dev/full stands in for the camera's node: nothing else holds it.
    let watch = Watch::on("record", Some(Path::new("/dev/full")));
    watch.until_there(true, "no file while the camera's there");
    watch.count(0);
    // This test holds the camera, and it streams.
    let _held = File::open(&watch.node).unwrap();
    fs::write(&watch.streaming, " 1\n").unwrap();
    for _ in 0..100 {
        if !fs::read_to_string(&watch.file).unwrap().is_empty() {
            break;
        }
        sleep(Duration::from_millis(50));
    }
    assert_ne!(
        fs::read_to_string(&watch.file).unwrap(),
        "",
        "the test isn't named"
    );
    sleep(Duration::from_secs(2));
    watch.count(300);
    // The first line comes ten seconds after the stream was first seen, so
    // it has counted 300 frames over ten to eleven seconds.
    for _ in 0..600 {
        let log = fs::read_to_string(&watch.log).unwrap();
        if let Some(line) = log.lines().next() {
            assert!(line.starts_with("streams to \""), "{line}");
            let rate: f64 = line
                .split_once("gave ")
                .and_then(|(_, rest)| rest.split_once(" fps, 0 packets in error"))
                .and_then(|(rate, _)| rate.parse().ok())
                .unwrap_or_else(|| panic!("{line}"));
            assert!((25.0..=30.0).contains(&rate), "{line}");
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("nothing said of the stream in thirty seconds");
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
