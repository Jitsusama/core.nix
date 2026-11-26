//! The program itself watching a loopback that has no camera behind it, run
//! as its unit runs it: the nobody it says is written once it can be, and
//! only then is the run ready; a trouble is the run's, not the step's, so
//! it's said once whether the nobody or the watch of a camera met it; and
//! the node a camera's watch couldn't say in `.devices/` is said once it
//! can be, its trouble said once, and only then is the run ready.

use std::fs::{self, File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

/// A watch on a loopback, its file in a folder of its own, its log and
/// complaints in files beside it.
struct Watch {
    root: PathBuf,
    folder: PathBuf,
    program: Child,
}

impl Watch {
    /// Starts the watch on a source with no node in it, as a relay that has
    /// none says, with its folder locked, so its first write fails.
    fn locked(name: &str) -> Self {
        Self::started(name, "", |folder| {
            fs::set_permissions(folder, Permissions::from_mode(0o555)).unwrap();
            assert!(
                File::create(folder.join("probe")).is_err(),
                "a locked folder still took a file, as it does for root"
            );
        })
    }

    /// Starts the watch on a camera nothing on the machine holds, with a
    /// file where `.devices/` would be made, so its node can't be said;
    /// a folder there would only be widened.
    fn unnamed(name: &str) -> Self {
        Self::started(name, "/dev/random", |folder| {
            fs::write(folder.join(".devices"), "").unwrap();
        })
    }

    fn started(name: &str, node: &str, before: impl FnOnce(&Path)) -> Self {
        let root =
            std::env::temp_dir().join(format!("holders-loopback-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let folder = root.join("camera-holders");
        fs::create_dir_all(&folder).unwrap();
        let source = root.join("source");
        fs::write(&source, node).unwrap();
        before(&folder);
        let program = Command::new(env!("CARGO_BIN_EXE_loopback-holders"))
            .arg("--sysfs")
            .arg(root.join("sys"))
            .arg(&source)
            .arg(folder.join("Stand-in"))
            .stdout(File::create(root.join("log")).unwrap())
            .stderr(File::create(root.join("complaints")).unwrap())
            .spawn()
            .unwrap();
        Self {
            root,
            folder,
            program,
        }
    }

    fn file(&self) -> PathBuf {
        self.folder.join("Stand-in")
    }

    fn device(&self) -> PathBuf {
        self.folder.join(".devices/Stand-in")
    }

    /// Whether `.devices/` says the camera is the node it watches.
    fn named(&self) -> bool {
        let node = fs::canonicalize("/dev/random").unwrap();
        fs::read_to_string(self.device()).is_ok_and(|said| said == format!("{}\n", node.display()))
    }

    /// Holds the camera until the watch says so, then lets it go until the
    /// watch says nobody: two looks it surely made.
    fn looked_twice(&self) {
        let held = File::open("/dev/random").unwrap();
        self.until(
            |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
            "a camera held wasn't said held",
        );
        drop(held);
        self.until(
            |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
            "a camera let go wasn't said held by nobody",
        );
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.root.join(name)).unwrap_or_default()
    }

    /// Waits up to five seconds for what's asked of the watch.
    fn until(&self, wanted: impl Fn(&Self) -> bool, what: &str) {
        for _ in 0..100 {
            if wanted(self) {
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!("{what}: it complained {:?}", self.read("complaints"));
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.program.kill();
        let _ = self.program.wait();
        let _ = fs::set_permissions(&self.folder, Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// How often the watch complained of one file, not the other, whose
/// complaints end the same way.
fn complaints_of(watch: &Watch, file: &Path) -> usize {
    let file = format!("{}: ", file.display());
    watch
        .read("complaints")
        .lines()
        .filter(|line| line.starts_with(&file) && !line.ends_with(": right again"))
        .count()
}

/// How often the watch said one file was right again.
fn right_again(watch: &Watch, file: &Path) -> usize {
    let said = format!("{}: right again", file.display());
    watch
        .read("complaints")
        .lines()
        .filter(|line| *line == said)
        .count()
}

fn ready(log: &str) -> usize {
    log.lines()
        .filter(|line| line.starts_with("ready in "))
        .count()
}

#[test]
fn a_loopbacks_nobody_it_couldnt_write_is_written_once_it_can_be() {
    let watch = Watch::locked("unwritten");
    watch.until(
        |watch| watch.read("complaints").contains("Permission denied"),
        "a nobody it couldn't write went unsaid",
    );
    // Two more looks' worth, to see it neither ready nor complaining again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a nobody it couldn't write was said ready"
    );
    assert_eq!(
        watch
            .read("complaints")
            .matches("Permission denied")
            .count(),
        1,
        "a write failing each look was complained of each look"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a nobody it couldn't write wasn't tried again",
    );
    watch.until(
        |watch| ready(&watch.read("log")) > 0,
        "a nobody written at last wasn't said ready",
    );
    watch.until(
        |watch| right_again(watch, &watch.file()) > 0,
        "a write that worked again went unsaid",
    );
    assert_eq!(
        ready(&watch.read("log")),
        1,
        "the watch didn't say once that it's ready: {}",
        watch.read("log")
    );
}

#[test]
fn a_trouble_the_nobody_met_is_the_cameras_watch_to_clear() {
    let watch = Watch::locked("handed");
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // A camera nothing on the machine holds comes while the folder is still
    // locked, so its watch meets the trouble the nobody did.
    fs::write(watch.root.join("source"), "/dev/random").unwrap();
    watch.until(
        |watch| watch.read("complaints").contains(".devices/Stand-in"),
        "the camera that came wasn't watched",
    );
    // Past the watch's next look, two seconds on, which writes again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        complaints_of(&watch, &watch.file()),
        1,
        "the camera's watch complained again of what the nobody had: {:?}",
        watch.read("complaints")
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| right_again(watch, &watch.file()) == 1,
        "the camera's watch didn't say right again what the nobody met",
    );
    watch.until(
        Watch::named,
        "the node the camera's watch couldn't say wasn't said once it could be",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera's file written at last wasn't said ready",
    );
    // A watch with its files right and nobody holding the camera looks only
    // as the camera is opened or closed, so it's made to look, each look
    // finding its files right.
    watch.looked_twice();
    assert_eq!(
        right_again(&watch, &watch.file()),
        1,
        "a file right on each look was said right again each look: {:?}",
        watch.read("complaints")
    );
}

#[test]
fn a_camera_is_ready_only_once_its_node_is_said() {
    let watch = Watch::unnamed("unnamed");
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node it couldn't say went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a camera nothing holds wasn't said held by nobody",
    );
    // Past the watch's next look, two seconds on, which tries the node again.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a camera whose node went unsaid was said ready"
    );
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        1,
        "a node it couldn't say each look was complained of each look: {:?}",
        watch.read("complaints")
    );
    fs::remove_file(watch.folder.join(".devices")).unwrap();
    watch.until(
        Watch::named,
        "a node it couldn't say wasn't said once it could be",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose node was said at last wasn't said ready",
    );
    watch.until(
        |watch| right_again(watch, &watch.device()) == 1,
        "a node said at last wasn't said right again",
    );
    watch.looked_twice();
    assert_eq!(
        right_again(&watch, &watch.device()),
        1,
        "a node right on each look was said right again each look: {:?}",
        watch.read("complaints")
    );
}
