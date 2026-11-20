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
        Self::saying(name, device, false)
    }

    /// The same, its log and complaints going to /dev/full, which takes no
    /// write, as a journal stream that has gone takes none.
    fn unheard(name: &str, device: &Path) -> Self {
        Self::saying(name, Some(device), true)
    }

    fn saying(name: &str, device: Option<&Path>, unheard: bool) -> Self {
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
        // What a killed run of a camera since unplugged left, saying the
        // node this one is now, and another camera's, saying its own.
        let devices = folder.join(".devices");
        fs::create_dir_all(&devices).unwrap();
        let real = fs::canonicalize(&node).unwrap();
        fs::write(devices.join("Unplugged"), format!("{}\n", real.display())).unwrap();
        fs::write(devices.join("Another"), "/dev/video9\n").unwrap();
        // And one another camera's run is writing, saying this node too.
        fs::write(devices.join("Plugged.123"), format!("{}\n", real.display())).unwrap();
        let debugfs = root.join("debug");
        let stats = debugfs.join("usb/uvcvideo/2-7-1/stats");
        fs::create_dir_all(stats.parent().unwrap()).unwrap();
        let log = root.join("log");
        let mut command = Command::new(env!("CARGO_BIN_EXE_loopback-holders"));
        command
            .arg("--usb")
            .arg("--sysfs")
            .arg(&sysfs)
            .arg("--debugfs")
            .arg(&debugfs)
            .arg(&node)
            .arg(&folder);
        if unheard {
            let full = || File::options().write(true).open("/dev/full").unwrap();
            command.stdout(full()).stderr(full());
        } else {
            command.stdout(File::create(&log).unwrap());
        }
        let program = command.spawn().unwrap();
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

    /// Where the camera's node is said, beside its file.
    fn device(&self) -> PathBuf {
        let name = self.file.file_name().unwrap();
        self.file.parent().unwrap().join(".devices").join(name)
    }

    /// Waits up to five seconds for the camera's file, and the file saying
    /// its node, to be there or not.
    fn until_there(&self, wanted: bool, what: &str) {
        for _ in 0..100 {
            if self.file.exists() == wanted && self.device().exists() == wanted {
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!("{what}");
    }

    /// Waits up to five seconds for the camera's file to name someone, or
    /// nobody.
    fn until_named(&self, someone: bool, what: &str) {
        for _ in 0..100 {
            if fs::read_to_string(&self.file).is_ok_and(|named| named.is_empty() != someone) {
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!("{what}");
    }

    /// Stops the program as systemd does.
    fn stop(&self) {
        let pid = i32::try_from(self.program.id()).unwrap();
        // SAFETY: a plain call, to our own child.
        assert_eq!(unsafe { kill(pid, SIGTERM) }, 0);
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
    let device = watch.device();
    let devices = device.parent().unwrap();
    assert!(
        !devices.join("Unplugged").exists(),
        "a killed run's file saying this camera's node was left"
    );
    assert_eq!(
        fs::read_to_string(devices.join("Another")).unwrap(),
        "/dev/video9\n",
        "another camera's node file was touched"
    );
    assert!(
        devices.join("Plugged.123").exists(),
        "a file being written was taken from its writer"
    );
}

#[test]
fn while_the_camera_streams_its_log_says_what_it_gave() {
    // /dev/zero stands in for the camera's node: nothing else holds it,
    // while the other test's watch holds /dev/full.
    let mut watch = Watch::on("record", Some(Path::new("/dev/zero")));
    watch.until_there(true, "no file while the camera's there");
    watch.count(0);
    // This test holds the camera, and it streams.
    let _held = File::open(&watch.node).unwrap();
    fs::write(&watch.streaming, " 1\n").unwrap();
    watch.until_named(true, "the test isn't named");
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
            // The camera was asked: /dev/zero answers it isn't V4L2's.
            let asked = "; format unknown: Inappropriate ioctl for device (os error 25)";
            assert!(line.contains(asked), "{line}");
            // Stopped mid-stream, it says the stream's end as it goes.
            watch.stop();
            assert_eq!(watch.ended(), Some(0));
            let log = fs::read_to_string(&watch.log).unwrap();
            let last = log.lines().last().unwrap();
            assert!(last.ends_with("; then the watch stopped"), "{log}");
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("nothing said of the stream in thirty seconds");
}

#[test]
fn a_log_that_takes_no_write_doesnt_stop_the_watch() {
    let mut watch = Watch::unheard("unheard", Path::new("/dev/full"));
    watch.until_there(true, "no file while the camera's there");
    watch.count(0);
    let held = File::open(&watch.node).unwrap();
    fs::write(&watch.streaming, " 1\n").unwrap();
    watch.until_named(true, "the test isn't named");
    // Its first line, and its complaint, come ten seconds on; neither lands.
    sleep(Duration::from_secs(11));
    assert!(
        watch.program.try_wait().unwrap().is_none(),
        "the watch stopped"
    );
    drop(held);
    fs::write(&watch.streaming, " 0\n").unwrap();
    watch.until_named(false, "the watch no longer follows the camera");
    // A stop's line, always said, comes after the file is written, so only
    // a watch that lived through it names the next stream.
    let _held = File::open(&watch.node).unwrap();
    fs::write(&watch.streaming, " 1\n").unwrap();
    watch.until_named(true, "the watch didn't live through its stop line");
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
    watch.stop();
    assert_eq!(watch.ended(), Some(0));
    assert!(!watch.file.exists());
    assert!(!watch.device().exists(), "its node was still said");
}
