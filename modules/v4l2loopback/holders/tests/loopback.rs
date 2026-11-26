//! The program itself watching a loopback that has no camera behind it, run
//! as its unit runs it: the nobody it says is written once it can be, and
//! only then is the run ready; a trouble is the run's, not the step's, so
//! it's said once whether the nobody or the watch of a camera met it; and
//! the node a camera's watch couldn't say in `.devices/` is said once it
//! can be, its trouble said once, and only then is the run ready; and as
//! its camera goes or its watch stops, the node file goes only once its
//! nobody is written, and one it couldn't remove is tried again.

use std::fs::{self, File, Permissions};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::sleep;
use std::time::Duration;

/// Held by each watch while it runs, so the tests run one at a time: a
/// test holding /dev/random would make another's watch look, and do for
/// it what its timer should. So a test holds one watch at a time.
static TURN: Mutex<()> = Mutex::new(());

const SIGTERM: i32 = 15;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// A watch on a loopback, its file in a folder of its own, its log and
/// complaints in files beside it.
struct Watch {
    root: PathBuf,
    folder: PathBuf,
    program: Child,
    _turn: MutexGuard<'static, ()>,
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

    /// Starts the watch on a camera of the test's own, a link to a node
    /// nothing on the machine holds, which it can remove, as a relay's node
    /// goes when its camera does. A device, since only a device's holders
    /// are read.
    fn owned(name: &str) -> Self {
        let camera = Self::root(name).join("camera");
        Self::started(name, camera.to_str().unwrap(), |folder| {
            symlink("/dev/random", folder.parent().unwrap().join("camera")).unwrap();
        })
    }

    fn root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("holders-loopback-{name}-{}", std::process::id()))
    }

    fn started(name: &str, node: &str, before: impl FnOnce(&Path)) -> Self {
        let turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
        let root = Self::root(name);
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
            _turn: turn,
        }
    }

    fn file(&self) -> PathBuf {
        self.folder.join("Stand-in")
    }

    fn device(&self) -> PathBuf {
        self.folder.join(".devices/Stand-in")
    }

    /// Whether `.devices/` says the camera is /dev/random.
    fn named(&self) -> bool {
        self.names("/dev/random")
    }

    /// Whether `.devices/` says the camera is the node given.
    fn names(&self, node: impl AsRef<Path>) -> bool {
        let Ok(node) = fs::canonicalize(node) else {
            return false;
        };
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

    /// Stops the watch as its unit does, and waits up to twenty seconds
    /// for it to end as a stop should, not killed by the signal or failing.
    fn stop(&mut self) {
        let pid = i32::try_from(self.program.id()).unwrap();
        // SAFETY: a plain call, to our own child.
        assert_eq!(
            unsafe { kill(pid, SIGTERM) },
            0,
            "the watch couldn't be stopped"
        );
        for _ in 0..400 {
            if let Some(ended) = self.program.try_wait().unwrap() {
                assert!(
                    ended.success(),
                    "a stopped watch ended {ended}, and it complained {:?}",
                    self.read("complaints")
                );
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!("a stopped watch didn't end");
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.root.join(name)).unwrap_or_default()
    }

    /// Waits up to twenty seconds for what's asked of the watch: each look
    /// reads every process's open files first, which on a machine running
    /// thousands of them under load has taken past five.
    fn until(&self, wanted: impl Fn(&Self) -> bool, what: &str) {
        for _ in 0..400 {
            if wanted(self) {
                return;
            }
            sleep(Duration::from_millis(50));
        }
        panic!(
            "{what}: its file said {:?}, and it complained {:?}",
            fs::read_to_string(self.file()),
            self.read("complaints")
        );
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.program.kill();
        let _ = self.program.wait();
        let _ = fs::set_permissions(&self.folder, Permissions::from_mode(0o755));
        let _ = fs::set_permissions(self.folder.join(".devices"), Permissions::from_mode(0o755));
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

#[test]
fn a_node_file_it_couldnt_remove_as_its_camera_went_is_tried_again() {
    let watch = Watch::owned("removed");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| watch.names(&camera),
        "the camera's node went unsaid",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the nobody said after is the watch's doing.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // Its folder locked, so removing the node file fails; nothing is owed,
    // so no look writes it again.
    let devices = watch.folder.join(".devices");
    fs::set_permissions(&devices, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(devices.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // Gone, then let go, so the look its closing wakes finds it gone.
    fs::remove_file(&camera).unwrap();
    drop(held);
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node file it couldn't remove went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a camera gone wasn't said held by nobody",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        1,
        "a node file it couldn't remove each try was complained of each try: {:?}",
        watch.read("complaints")
    );
    fs::set_permissions(&devices, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| !watch.device().exists(),
        "a node file it couldn't remove wasn't tried again",
    );
    watch.until(
        |watch| right_again(watch, &watch.device()) == 1,
        "a node file removed at last wasn't said right again",
    );
}

#[test]
fn a_loopback_with_no_camera_is_ready_only_once_its_node_file_is_gone() {
    // A node file a killed run left, as a folder with a file in it, so
    // removing it fails.
    let watch = Watch::started("stale", "", |folder| {
        let stale = folder.join(".devices/Stand-in");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("kept"), "").unwrap();
    });
    watch.until(
        |watch| complaints_of(watch, &watch.device()) == 1,
        "a node file it couldn't remove went unsaid",
    );
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a loopback with no camera wasn't said held by nobody",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert_eq!(
        ready(&watch.read("log")),
        0,
        "a loopback whose node file stayed was said ready"
    );
    fs::remove_dir_all(watch.device()).unwrap();
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback whose node file went wasn't said ready",
    );
}

#[test]
fn a_node_file_stays_until_its_loopbacks_nobody_is_written() {
    // A node file a killed run left, beside a holders file that can't be
    // written: the folder's locked, the node files' folder isn't.
    let watch = Watch::started("ordered", "", |folder| {
        fs::create_dir(folder.join(".devices")).unwrap();
        fs::write(folder.join(".devices/Stand-in"), "/dev/random\n").unwrap();
        fs::set_permissions(folder, Permissions::from_mode(0o555)).unwrap();
        assert!(
            File::create(folder.join("probe")).is_err(),
            "a locked folder still took a file, as it does for root"
        );
    });
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write went unsaid",
    );
    // Past two more of the one-second tries with no camera.
    sleep(Duration::from_millis(2500));
    assert!(
        watch.device().exists(),
        "a node file was removed before its loopback's nobody was written"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| !watch.device().exists(),
        "a node file stayed once its loopback's nobody was written",
    );
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback whose files were right wasn't said ready",
    );
}

#[test]
fn a_stopped_watch_keeps_its_node_named_while_its_nobody_cant_be_written() {
    let mut watch = Watch::owned("stopped");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // The holders file's folder locked, the node files' folder not.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(watch.folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    watch.stop();
    assert!(
        watch.names(&camera),
        "a stopped watch unnamed its node with its nobody unwritten"
    );
}

#[test]
fn a_stopped_watch_unnames_its_node_once_its_nobody_is_written() {
    let mut watch = Watch::owned("stopping");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it stops, so the nobody said after is the stop's doing.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    watch.stop();
    drop(held);
    assert!(
        fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a stopped watch wasn't said held by nobody"
    );
    assert!(
        !watch.device().exists(),
        "a stopped watch left its node named with its nobody written"
    );
}

#[test]
fn a_gone_cameras_node_file_stays_until_its_nobody_is_written() {
    let watch = Watch::owned("went");
    let camera = watch.root.join("camera");
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a camera whose files were written wasn't said ready",
    );
    // Held as it goes, so the nobody said after is a write of its own.
    let held = File::open(&camera).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| !said.is_empty()),
        "a camera held wasn't said held",
    );
    // The holders file's folder locked, the node files' folder not.
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(watch.folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
    // Gone, then let go, so the look its closing wakes finds it gone.
    fs::remove_file(&camera).unwrap();
    drop(held);
    watch.until(
        |watch| complaints_of(watch, &watch.file()) == 1,
        "a nobody it couldn't write as its camera went went unsaid",
    );
    // Past a look of the watch's, every two seconds, and a try after it.
    sleep(Duration::from_millis(3500));
    assert!(
        watch.device().exists(),
        "a gone camera's node file was removed before its nobody was written"
    );
    fs::set_permissions(&watch.folder, Permissions::from_mode(0o755)).unwrap();
    watch.until(
        |watch| fs::read_to_string(watch.file()).is_ok_and(|said| said.is_empty()),
        "a gone camera's nobody it couldn't write wasn't tried again",
    );
    watch.until(
        |watch| !watch.device().exists(),
        "a gone camera's node file stayed once its nobody was written",
    );
}

#[test]
fn a_loopback_with_no_camera_and_no_node_files_folder_is_ready() {
    // A file where `.devices/` would be, so no node file can be there.
    let watch = Watch::started("foldless", "", |folder| {
        fs::write(folder.join(".devices"), "").unwrap();
    });
    watch.until(
        |watch| ready(&watch.read("log")) == 1,
        "a loopback with no node files' folder wasn't said ready",
    );
    assert_eq!(
        complaints_of(&watch, &watch.device()),
        0,
        "a node file that can't be there was complained of: {:?}",
        watch.read("complaints")
    );
}
