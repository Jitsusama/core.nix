//! Holds the firmware's settings to the ones the machine declares, through
//! the kernel's firmware-attributes class, which Dell's, Lenovo's and HP's
//! drivers provide.
//!
//! ```text
//! firmware-settings check           names every setting that differs
//! firmware-settings apply           writes them, asking for the admin password
//! firmware-settings set NAME VALUE  writes one for now, which check then names
//! ```
//!
//! It exits 1 when a setting differs, 2 when the firmware has no such
//! setting, and 77 when it can't read one, as without root.
//! `FIRMWARE_ATTRIBUTES` names the class's directory and `FIRMWARE_SETTINGS`
//! the declared settings, one NAME=VALUE a line, for tests.
//! `FIRMWARE_SETTINGS_WAIT` says how many seconds to wait for a setting to
//! appear: a driver announces its interface before it has made the settings
//! in it.

use std::env;
use std::ffi::c_int;
use std::fs::{self, File};
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

/// What it exits with when a setting differs from the declared one.
const DIFFERS: u8 = 1;
/// What it exits with when it couldn't write what it was asked to.
const FAILED: u8 = 1;
/// What it exits with when the firmware has no such setting.
const MISSING: u8 = 2;
/// What it exits with when it's asked for something it doesn't do.
const USAGE: u8 = 64;
/// What it exits with when it can't read a setting, as without root.
const UNREADABLE: u8 = 77;
/// What it exits with when a signal stops it, as a shell's 128 and SIGINT.
const INTERRUPTED: i32 = 130;
/// How many passes apply writes in before it gives up on settings that
/// won't take.
const PASSES: usize = 10;
/// stty, which turns the terminal's echo off while the password is typed.
const STTY: &str = match option_env!("STTY") {
    Some(path) => path,
    None => "stty",
};

const SIG_BLOCK: c_int = 0;
const SIGHUP: c_int = 1;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;
/// How long a signalfd's record of one signal is.
const SIGINFO: usize = 128;

/// glibc's `sigset_t`, 1024 bits.
#[repr(C)]
struct SigSet([u64; 16]);

unsafe extern "C" {
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
}

/// A declared setting the firmware holds otherwise.
#[derive(Debug, PartialEq, Eq)]
struct Difference {
    name: String,
    /// The setting's directory, under one interface's attributes.
    dir: PathBuf,
    now: String,
    declared: String,
}

/// The firmware's interfaces, the settings declared for them, and the
/// admin password files given a password, which are wiped as it ends.
struct Firmware {
    root: PathBuf,
    declared: PathBuf,
    wait: u64,
    /// The command it was run with, for the advice to run it as root.
    command: String,
    unlocked: Arc<Mutex<Vec<PathBuf>>>,
}

/// Each declared setting's name and value, from lines of NAME=VALUE,
/// skipping those with no name.
fn parsed(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(|line| line.split_once('=').unwrap_or((line, "")))
        .filter(|(name, _)| !name.is_empty())
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// What a file of the class holds, less the newlines the kernel ends it
/// with, or nothing when it can't be read.
fn read(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    Some(text.trim_end_matches('\n').to_owned())
}

/// Hold everything another thread may be wiping, whether or not a thread
/// that held it panicked.
fn held(files: &Mutex<Vec<PathBuf>>) -> MutexGuard<'_, Vec<PathBuf>> {
    files.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Turn the terminal's echo on or off, as stty's argument says.
fn stty(echo: &str) {
    // Without stty the password shows as it's typed, which is the worst
    // that goes wrong, so a failure stops nothing.
    let _ = Command::new(STTY).arg(echo).status();
}

/// Give the firmware back its admin password files empty, and the
/// terminal its echo.
///
/// The kernel keeps the password until something overwrites it, and any
/// root process could change the firmware's settings with it until then.
fn wipe(files: &Mutex<Vec<PathBuf>>) {
    if io::stdin().is_terminal() {
        stty("echo");
    }
    for file in held(files).iter() {
        // Each is wiped whether or not another could be.
        let _ = fs::write(file, "\n");
    }
}

/// Wipe the password and end once SIGINT, SIGTERM or SIGHUP arrives,
/// rather than leaving the password with the kernel.
fn wipe_when_stopped(files: Arc<Mutex<Vec<PathBuf>>>) {
    let mut set = SigSet([0; 16]);
    // SAFETY: set is a sigset_t's size, and each call only fills it in.
    let built = unsafe {
        sigemptyset(&raw mut set) == 0
            && sigaddset(&raw mut set, SIGINT) == 0
            && sigaddset(&raw mut set, SIGTERM) == 0
            && sigaddset(&raw mut set, SIGHUP) == 0
    };
    if !built {
        return;
    }
    // The descriptor is made before the signals are blocked, so a failure
    // leaves them as they were rather than blocked with nothing to hear them.
    // SAFETY: set is filled in, and -1 asks for a new descriptor.
    let fd = unsafe { signalfd(-1, &raw const set, SFD_CLOEXEC) };
    if fd < 0 {
        return;
    }
    // SAFETY: the descriptor was just made and nothing else owns it.
    let mut stops = unsafe { File::from_raw_fd(fd) };
    // Blocked here, before the thread starts, so every thread has them
    // blocked and only the descriptor hears them.
    // SAFETY: set is filled in, and the old mask isn't wanted.
    if unsafe { sigprocmask(SIG_BLOCK, &raw const set, std::ptr::null_mut()) } != 0 {
        return;
    }
    thread::spawn(move || {
        let mut record = [0; SIGINFO];
        if stops.read_exact(&mut record).is_ok() {
            wipe(&files);
            process::exit(INTERRUPTED);
        }
    });
}

impl Firmware {
    /// The firmware as the environment names it, run as `command`.
    fn from_environment(command: &str) -> Self {
        let root = env::var_os("FIRMWARE_ATTRIBUTES")
            .unwrap_or_else(|| "/sys/class/firmware-attributes".into());
        let declared =
            env::var_os("FIRMWARE_SETTINGS").unwrap_or_else(|| "/etc/firmware-settings".into());
        let wait = env::var("FIRMWARE_SETTINGS_WAIT")
            .ok()
            .and_then(|wait| wait.parse().ok())
            .unwrap_or(0);
        Self {
            root: root.into(),
            declared: declared.into(),
            wait,
            command: command.to_owned(),
            unlocked: Arc::default(),
        }
    }

    /// Each interface the class has, in the order of their names.
    fn interfaces(&self) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut interfaces: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .map(|entry| entry.path())
            .collect();
        interfaces.sort();
        interfaces
    }

    /// Where a setting lives: the first interface that has it.
    fn place(&self, name: &str) -> Result<PathBuf, u8> {
        let mut waited = 0;
        loop {
            let found = self
                .interfaces()
                .into_iter()
                .map(|interface| interface.join("attributes").join(name))
                .find(|dir| dir.is_dir());
            if let Some(dir) = found {
                return Ok(dir);
            }
            if waited >= self.wait {
                break;
            }
            thread::sleep(Duration::from_secs(1));
            waited += 1;
        }
        eprintln!("firmware-settings: no firmware setting {name}");
        Err(MISSING)
    }

    /// A setting's value, or a refusal naming the command to run as root:
    /// the kernel lets only root read most of them, and an empty value
    /// would read as a setting that differs.
    fn current(&self, dir: &Path, name: &str) -> Result<String, u8> {
        read(&dir.join("current_value")).ok_or_else(|| {
            eprintln!(
                "firmware-settings: can't read {name}; run it as root: run0 firmware-settings {}",
                self.command
            );
            UNREADABLE
        })
    }

    /// Each declared setting the firmware holds otherwise.
    fn differences(&self) -> Result<Vec<Difference>, u8> {
        let text = fs::read_to_string(&self.declared).map_err(|error| {
            eprintln!(
                "firmware-settings: can't read {}: {error}",
                self.declared.display()
            );
            FAILED
        })?;
        let mut differences = Vec::new();
        for (name, declared) in parsed(&text) {
            let dir = self.place(&name)?;
            let now = self.current(&dir, &name)?;
            if now != declared {
                differences.push(Difference {
                    name,
                    dir,
                    now,
                    declared,
                });
            }
        }
        Ok(differences)
    }

    /// Name every setting that differs, and say what to exit with.
    fn check(&self) -> u8 {
        match self.differences() {
            Err(code) => code,
            Ok(differences) if differences.is_empty() => {
                println!("The firmware's settings are as declared.");
                0
            }
            Ok(differences) => {
                for difference in differences {
                    println!(
                        "{} is {}, declared {}",
                        difference.name, difference.now, difference.declared
                    );
                }
                DIFFERS
            }
        }
    }

    /// Say when an interface takes some of what was written at the next boot.
    fn pending(&self) {
        for interface in self.interfaces() {
            if read(&interface.join("attributes/pending_reboot")).as_deref() == Some("1") {
                println!("The firmware takes some of its settings at the next boot.");
            }
        }
    }

    /// Give the admin password to each interface with one set, asking for
    /// it once: such an interface changes nothing without it.
    fn unlock(&self) -> Result<(), u8> {
        let mut password: Option<String> = None;
        for interface in self.interfaces() {
            let admin = interface.join("authentication/Admin");
            if read(&admin.join("is_enabled")).as_deref() != Some("1") {
                continue;
            }
            let password = password.get_or_insert_with(asked);
            let file = admin.join("current_password");
            // Named for wiping before it's written, so a signal between the
            // two can't leave it behind.
            let mut unlocked = held(&self.unlocked);
            unlocked.push(file.clone());
            if let Err(error) = fs::write(&file, password.as_bytes()) {
                eprintln!("firmware-settings: can't give {}: {error}", file.display());
                return Err(FAILED);
            }
        }
        Ok(())
    }

    /// Write every declared setting, and check what the firmware holds then.
    fn apply(&self) -> u8 {
        if let Err(code) = self.unlock() {
            return code;
        }
        // Some settings unlock others, as a battery's custom limits wait for
        // its custom mode, so it writes in passes until a pass changes
        // nothing.
        let mut before: Option<Vec<(String, String)>> = None;
        for _ in 0..PASSES {
            let Ok(differences) = self.differences() else {
                break;
            };
            let wanted: Vec<(String, String)> = differences
                .iter()
                .map(|difference| (difference.name.clone(), difference.declared.clone()))
                .collect();
            if wanted.is_empty() || before.as_ref() == Some(&wanted) {
                break;
            }
            for difference in &differences {
                // A setting that won't take is named by the check after.
                let _ = fs::write(difference.dir.join("current_value"), &difference.declared);
            }
            before = Some(wanted);
        }
        self.pending();
        self.check()
    }

    /// Write one setting for now, which check then names.
    fn set(&self, name: &str, value: &str) -> u8 {
        let dir = match self.place(name) {
            Ok(dir) => dir,
            Err(code) => return code,
        };
        if let Err(code) = self.unlock() {
            return code;
        }
        if let Err(error) = fs::write(dir.join("current_value"), value) {
            eprintln!("firmware-settings: can't write {name}: {error}");
            return FAILED;
        }
        println!(
            "{name} is {}",
            read(&dir.join("current_value")).unwrap_or_default()
        );
        self.pending();
        0
    }
}

/// The admin password, read from a line of input, its echo off on a
/// terminal.
fn asked() -> String {
    eprint!("Firmware admin password: ");
    let _ = io::stderr().flush();
    let terminal = io::stdin().is_terminal();
    if terminal {
        stty("-echo");
    }
    let mut line = String::new();
    // No input is an empty password, which the firmware then refuses.
    let _ = io::stdin().lock().read_line(&mut line);
    if terminal {
        stty("echo");
        eprintln!();
    }
    line.strip_suffix('\n').unwrap_or(&line).to_owned()
}

fn usage() -> u8 {
    eprintln!("usage: firmware-settings check | apply | set NAME VALUE");
    USAGE
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let command = arguments.first().map_or("", String::as_str);
    let firmware = Firmware::from_environment(command);
    wipe_when_stopped(Arc::clone(&firmware.unlocked));
    let code = match (command, arguments.len()) {
        ("check", _) => firmware.check(),
        ("apply", _) => firmware.apply(),
        ("set", 3) => firmware.set(&arguments[1], &arguments[2]),
        _ => usage(),
    };
    wipe(&firmware.unlocked);
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the class, in a directory of its own, removed after.
    struct Class(PathBuf);

    impl Class {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("firmware-settings-{}-{name}", process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn setting(&self, interface: &str, name: &str, value: &str) {
            let dir = self.0.join(interface).join("attributes").join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("current_value"), format!("{value}\n")).unwrap();
        }

        fn firmware(&self, declared: &str) -> Firmware {
            let file = self.0.join("declared");
            fs::write(&file, declared).unwrap();
            Firmware {
                root: self.0.join("class"),
                declared: file,
                wait: 0,
                command: "check".to_owned(),
                unlocked: Arc::default(),
            }
        }
    }

    impl Drop for Class {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_declared_line_splits_at_its_first_equals_and_one_without_a_name_is_skipped() {
        let settings = parsed("Mode=Custom\n\n=x\nPath=a=b\nBare\n");
        let expected = [("Mode", "Custom"), ("Path", "a=b"), ("Bare", "")];
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        assert_eq!(settings, expected);
    }

    #[test]
    fn a_setting_two_interfaces_have_is_read_from_the_first_by_name() {
        let class = Class::new("first");
        class.setting("class/a-vendor", "Mode", "Custom");
        class.setting("class/b-vendor", "Mode", "Adaptive");
        let firmware = class.firmware("Mode=Custom\n");
        assert_eq!(firmware.differences(), Ok(Vec::new()));
    }

    #[test]
    fn a_value_is_compared_without_the_newline_the_kernel_ends_it_with() {
        let class = Class::new("newline");
        class.setting("class/vendor", "Start", "50");
        let firmware = class.firmware("Start=75\n");
        let differences = firmware.differences().unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(
            (
                differences[0].now.as_str(),
                differences[0].declared.as_str()
            ),
            ("50", "75")
        );
    }

    #[test]
    fn a_setting_the_firmware_lacks_is_missing_rather_than_differing() {
        let class = Class::new("missing");
        class.setting("class/vendor", "Mode", "Custom");
        let firmware = class.firmware("Absent=1\n");
        assert_eq!(firmware.differences(), Err(MISSING));
    }
}
