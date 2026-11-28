//! What the loopback tests run the program through: a watch on a loopback
//! with no camera behind it, or a camera of the test's own, its files in a
//! folder of its own, started, stopped and read as its unit would, and the
//! counts of what it said. Apart from the cases, since it changes only as
//! the way a watch is run does.

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
pub struct Watch {
    pub root: PathBuf,
    pub folder: PathBuf,
    pub program: Child,
    _turn: MutexGuard<'static, ()>,
}

impl Watch {
    /// Starts the watch on a source with no node in it, as a relay that has
    /// none says, with its folder locked, so its first write fails.
    pub fn locked(name: &str) -> Self {
        Self::started(name, "", lock)
    }

    /// Starts the watch on a source with no node in it, beside the files a
    /// run killed while its camera was held left: a holders file naming a
    /// holder, and a node file saying the camera was /dev/random. The
    /// holders file's folder is locked, so its nobody can't be written;
    /// the node files' folder isn't.
    pub fn stale(name: &str) -> Self {
        Self::started(name, "", |folder| {
            fs::write(folder.join("Stand-in"), "firefox\n").unwrap();
            fs::create_dir(folder.join(".devices")).unwrap();
            fs::write(folder.join(".devices/Stand-in"), "/dev/random\n").unwrap();
            lock(folder);
        })
    }

    /// Starts the watch on a camera nothing on the machine holds, with a
    /// file where `.devices/` would be made, so its node can't be said;
    /// a folder there would only be widened.
    pub fn unnamed(name: &str) -> Self {
        Self::started(name, "/dev/random", |folder| {
            fs::write(folder.join(".devices"), "").unwrap();
        })
    }

    /// Starts the watch on a camera of the test's own, a link to a node
    /// nothing on the machine holds, which it can remove, as a relay's node
    /// goes when its camera does. A device, since only a device's holders
    /// are read.
    pub fn owned(name: &str) -> Self {
        let camera = Self::root(name).join("camera");
        Self::started(name, camera.to_str().unwrap(), |folder| {
            symlink("/dev/random", folder.parent().unwrap().join("camera")).unwrap();
        })
    }

    /// Starts the watch on a camera of the test's own, as `owned` does, but
    /// with its link in a folder of its own, which `shut` closes so the
    /// watch's reads of who holds it fail while its files can still be
    /// written; and with a file where `.devices/` would be made, so the run
    /// isn't ready until the test removes it.
    pub fn apart(name: &str) -> Self {
        let camera = Self::root(name).join("apart/camera");
        Self::started(name, camera.to_str().unwrap(), |folder| {
            let apart = folder.parent().unwrap().join("apart");
            fs::create_dir(&apart).unwrap();
            symlink("/dev/random", apart.join("camera")).unwrap();
            fs::write(folder.join(".devices"), "").unwrap();
        })
    }

    /// The camera an `apart` watch watches.
    pub fn camera(&self) -> PathBuf {
        self.root.join("apart/camera")
    }

    /// Closes an `apart` camera's folder, so nothing can look the camera up
    /// by its link, checking it took: root looks in any folder, which would
    /// pass the tests of a read that fails.
    pub fn shut(&self) {
        fs::set_permissions(self.root.join("apart"), Permissions::from_mode(0o000)).unwrap();
        assert!(
            fs::metadata(self.camera()).is_err(),
            "a shut folder still let its camera be looked up, as it does for root"
        );
    }

    /// Opens an `apart` camera's folder again.
    pub fn open(&self) {
        fs::set_permissions(self.root.join("apart"), Permissions::from_mode(0o755)).unwrap();
    }

    fn root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("holders-loopback-{name}-{}", std::process::id()))
    }

    pub fn started(name: &str, node: &str, before: impl FnOnce(&Path)) -> Self {
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

    pub fn file(&self) -> PathBuf {
        self.folder.join("Stand-in")
    }

    pub fn device(&self) -> PathBuf {
        self.folder.join(".devices/Stand-in")
    }

    /// Whether `.devices/` says the camera is /dev/random.
    pub fn named(&self) -> bool {
        self.names("/dev/random")
    }

    /// Whether `.devices/` says the camera is the node given.
    pub fn names(&self, node: impl AsRef<Path>) -> bool {
        let Ok(node) = fs::canonicalize(node) else {
            return false;
        };
        fs::read_to_string(self.device()).is_ok_and(|said| said == format!("{}\n", node.display()))
    }

    /// Holds the camera until the watch says so, then lets it go until the
    /// watch says nobody: two looks it surely made.
    pub fn looked_twice(&self) {
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
    pub fn stop(&mut self) {
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

    pub fn read(&self, name: &str) -> String {
        fs::read_to_string(self.root.join(name)).unwrap_or_default()
    }

    /// Waits up to twenty seconds for what's asked of the watch: each look
    /// reads every process's open files first, which on a machine running
    /// thousands of them under load has taken past five.
    pub fn until(&self, wanted: impl Fn(&Self) -> bool, what: &str) {
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
        let _ = fs::set_permissions(self.root.join("apart"), Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Locks a folder, so no file can be written in it, checking it took: root
/// writes in any folder, which would pass the tests of a write that fails.
fn lock(folder: &Path) {
    fs::set_permissions(folder, Permissions::from_mode(0o555)).unwrap();
    assert!(
        File::create(folder.join("probe")).is_err(),
        "a locked folder still took a file, as it does for root"
    );
}

/// How often the watch complained of one file, not the other, whose
/// complaints end the same way.
pub fn complaints_of(watch: &Watch, file: &Path) -> usize {
    let file = format!("{}: ", file.display());
    watch
        .read("complaints")
        .lines()
        .filter(|line| line.starts_with(&file) && !line.ends_with(": right again"))
        .count()
}

/// How often the watch said one file was right again.
pub fn right_again(watch: &Watch, file: &Path) -> usize {
    let said = format!("{}: right again", file.display());
    watch
        .read("complaints")
        .lines()
        .filter(|line| *line == said)
        .count()
}

/// How often the watch said it was ready, in its log.
pub fn ready(log: &str) -> usize {
    log.lines()
        .filter(|line| line.starts_with("ready in "))
        .count()
}
