//! Holds the firmware's settings to the ones the machine declares, through
//! the kernel's firmware-attributes class, which Dell's, Lenovo's and HP's
//! drivers provide.
//!
//! ```text
//! firmware-settings [check]         names every setting that differs
//! firmware-settings apply           writes them, asking for the admin password
//! firmware-settings set NAME VALUE  writes one for now, which check then names
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, its answer as text or JSON, and a failure that says what to try.
//! It exits 1 when a setting differs, 66 when the firmware has no such
//! setting, 77 when it can't read one, as without root, and 130 when a
//! signal stops it, the password wiped first. A setting
//! that waits for a person at the firmware's setup screen is named with
//! what it waits for, and isn't counted as differing until it's taken off
//! that list.
//! `FIRMWARE_ATTRIBUTES` names the class's directory, `FIRMWARE_SETTINGS`
//! the declared settings, one NAME=VALUE a line, and
//! `FIRMWARE_SETTINGS_WAITING` those waiting, one NAME=WHAT a line, for
//! tests.
//! `FIRMWARE_SETTINGS_WAIT` says how many seconds to wait for a setting to
//! appear: a driver announces its interface before it has made the settings
//! in it.

use std::env;
use std::ffi::c_int;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// The commands it takes, a bare run being check.
const COMMANDS: [&str; 3] = ["check", "apply", "set"];
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

/// A setting that differs, as the answer names it, with what it waits for
/// where a person must take it at the firmware's setup screen.
#[derive(Debug, PartialEq, Eq)]
struct Differs {
    name: String,
    now: String,
    declared: String,
    waits_for: Option<String>,
}

/// What the firmware holds against what's declared.
#[derive(Debug, PartialEq, Eq)]
struct Look {
    differs: Vec<Differs>,
    /// Whether an interface takes some of what was written at the next boot.
    at_next_boot: bool,
}

/// A count of settings, as a sentence starts with it.
fn settings(count: usize) -> String {
    if count == 1 {
        "1 setting".to_owned()
    } else {
        format!("{count} settings")
    }
}

/// The line a setting that takes effect at the next boot adds.
const AT_NEXT_BOOT: &str = "The firmware takes some of its settings at the next boot.\n";

impl Look {
    /// How many differ and wait for no one, which fail a check.
    fn differing(&self) -> usize {
        self.differs
            .iter()
            .filter(|differs| differs.waits_for.is_none())
            .count()
    }

    /// Each setting that differs, a line each, for a person.
    fn text(&self) -> String {
        let mut text = String::new();
        for differs in &self.differs {
            // Writing to a String can't fail.
            let _ = write!(
                text,
                "{} is {}, declared {}",
                differs.name, differs.now, differs.declared
            );
            if let Some(what) = &differs.waits_for {
                let _ = write!(text, "; it waits for {what}");
            }
            text.push('\n');
        }
        if self.differs.is_empty() {
            text.push_str("The firmware's settings are as declared.\n");
        }
        if self.at_next_boot {
            text.push_str(AT_NEXT_BOOT);
        }
        text
    }

    /// The same, for a machine.
    fn json(&self) -> Value {
        let differs: Vec<Value> = self
            .differs
            .iter()
            .map(|differs| {
                json!({
                    "name": differs.name,
                    "now": differs.now,
                    "declared": differs.declared,
                    "waits_for": differs.waits_for,
                })
            })
            .collect();
        json!({
            "schema": "firmware-settings.check/1",
            "differs": differs,
            "at_next_boot": self.at_next_boot,
        })
    }

    /// Says the look as it was asked for.
    fn tell(&self, context: &Context) {
        // A closed stream, as when a pipe's reader stops early, leaves
        // nothing to say it to.
        let _ = if context.format().is_json() {
            context.answer(&self.json())
        } else {
            context.say(&self.text())
        };
    }
}

/// Why a write failed, and what to try: root where the kernel refused it.
fn unwritten(error: &io::Error, what: String, command: &str) -> Failure {
    if error.kind() == io::ErrorKind::PermissionDenied {
        Failure::new(Exit::Refused, what)
            .because(error.to_string())
            .try_this(format!("run0 firmware-settings {command}"))
    } else {
        Failure::new(Exit::Failed, what)
            .because(error.to_string())
            .try_this("firmware-settings check")
    }
}

/// The firmware's interfaces, the settings declared for them, and the
/// admin password files given a password, which are wiped as it ends.
struct Firmware {
    root: PathBuf,
    declared: PathBuf,
    /// The declared settings that wait for a person, and what for.
    waiting: PathBuf,
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
            Exit::Interrupted.end_now();
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
        let waiting = env::var_os("FIRMWARE_SETTINGS_WAITING")
            .unwrap_or_else(|| "/etc/firmware-settings-waiting".into());
        let wait = env::var("FIRMWARE_SETTINGS_WAIT")
            .ok()
            .and_then(|wait| wait.parse().ok())
            .unwrap_or(0);
        Self {
            root: root.into(),
            declared: declared.into(),
            waiting: waiting.into(),
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
    fn place(&self, name: &str) -> Result<PathBuf, Failure> {
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
        Err(Failure::new(
            Exit::NotFound,
            format!("there's no firmware setting {name}"),
        )
        .try_this("ls /sys/class/firmware-attributes/*/attributes"))
    }

    /// A setting's value, or a refusal naming the command to run as root:
    /// the kernel lets only root read most of them, and an empty value
    /// would read as a setting that differs.
    fn current(&self, dir: &Path, name: &str) -> Result<String, Failure> {
        read(&dir.join("current_value")).ok_or_else(|| {
            Failure::new(Exit::Refused, format!("it can't read {name}"))
                .because("the kernel lets only root read most settings")
                .try_this(format!("run0 firmware-settings {}", self.command))
        })
    }

    /// Each declared setting the firmware holds otherwise.
    fn differences(&self) -> Result<Vec<Difference>, Failure> {
        let text = fs::read_to_string(&self.declared).map_err(|error| {
            Failure::new(Exit::Config, "it couldn't read the declared settings")
                .because(format!("{}: {error}", self.declared.display()))
                .try_this("check that the machine imports firmware-settings' module")
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

    /// Each setting that waits for a person, and what for. No file is none.
    fn waits(&self) -> Vec<(String, String)> {
        fs::read_to_string(&self.waiting)
            .map(|text| parsed(&text))
            .unwrap_or_default()
    }

    /// Every setting that differs, one waiting for a person named with
    /// what for, so it fails nothing.
    fn look(&self) -> Result<Look, Failure> {
        let waits = self.waits();
        let differs = self
            .differences()?
            .into_iter()
            .map(|difference| Differs {
                waits_for: waits
                    .iter()
                    .find(|(name, _)| *name == difference.name)
                    .map(|(_, what)| what.clone()),
                name: difference.name,
                now: difference.now,
                declared: difference.declared,
            })
            .collect();
        Ok(Look {
            differs,
            at_next_boot: self.pending(),
        })
    }

    /// Whether an interface takes some of what was written at the next boot.
    fn pending(&self) -> bool {
        self.interfaces().iter().any(|interface| {
            read(&interface.join("attributes/pending_reboot")).as_deref() == Some("1")
        })
    }

    /// Give the admin password to each interface with one set, asking for
    /// it once: such an interface changes nothing without it.
    fn unlock(&self) -> Result<(), Failure> {
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
                return Err(unwritten(
                    &error,
                    format!("it couldn't give {} the password", file.display()),
                    &self.command,
                ));
            }
        }
        Ok(())
    }

    /// Write every declared setting, and look at what the firmware holds then.
    fn apply(&self) -> Result<Look, Failure> {
        self.unlock()?;
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
        self.look()
    }

    /// Write one setting for now, which check then names, giving what the
    /// firmware holds after.
    fn set(&self, name: &str, value: &str) -> Result<String, Failure> {
        let dir = self.place(name)?;
        self.unlock()?;
        fs::write(dir.join("current_value"), value).map_err(|error| {
            unwritten(
                &error,
                format!("the firmware wouldn't take {name} as {value}"),
                &self.command,
            )
        })?;
        Ok(read(&dir.join("current_value")).unwrap_or_default())
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

/// Runs the command asked for, saying its answer.
fn run(context: &Context, firmware: &Firmware) -> Result<(), Failure> {
    let args: Vec<&str> = context.args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["check"] => {
            let look = firmware.look()?;
            look.tell(context);
            match look.differing() {
                0 => Ok(()),
                count => Err(Failure::new(
                    Exit::Failed,
                    format!(
                        "{} the firmware holds differ from those declared",
                        settings(count)
                    ),
                )
                .try_this("run0 firmware-settings apply")),
            }
        }
        ["apply"] => {
            let look = firmware.apply()?;
            look.tell(context);
            match look.differing() {
                0 => Ok(()),
                count => Err(Failure::new(
                    Exit::Failed,
                    format!("{} wouldn't take what's declared", settings(count)),
                )
                .because("the firmware still holds them otherwise")
                .try_this("check the firmware's setup screen for what locks them")),
            }
        }
        ["set", name, value] => {
            let now = firmware.set(name, value)?;
            let at_next_boot = firmware.pending();
            // As in Look::tell, a closed stream leaves nothing to say it to.
            let _ = if context.format().is_json() {
                context.answer(&json!({
                    "schema": "firmware-settings.set/1",
                    "name": name,
                    "value": now,
                    "at_next_boot": at_next_boot,
                }))
            } else {
                let then = if at_next_boot { AT_NEXT_BOOT } else { "" };
                context.say(&format!("{name} is {now}\n{then}"))
            };
            Ok(())
        }
        ["set", ..] => Err(
            Failure::new(Exit::Usage, "set takes a setting's name and its value")
                .try_this("firmware-settings set NAME VALUE"),
        ),
        [command, ..] if COMMANDS.contains(command) => Err(Failure::new(
            Exit::Usage,
            format!("{command} takes nothing more"),
        )
        .try_this(format!("firmware-settings {command}"))),
        [word, ..] => Err(flags::unknown("command", word, &COMMANDS)),
    }
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "firmware-settings",
        "holds the firmware's settings to the ones the machine declares",
    )
    .example("firmware-settings", "names every setting that differs")
    .example(
        "run0 firmware-settings apply",
        "writes them, asking for the admin password",
    )
    .example(
        "run0 firmware-settings set PrimaryBattChargeCfg Adaptive",
        "writes one for now, which check then names",
    )
    .subcommand(
        Help::new("check", "names what differs, as a bare run does")
            .example("firmware-settings check", "every setting that differs")
            .exit(Exit::Failed, "a setting differs")
            .exit(Exit::Refused, "only root may read the setting")
            .exit(Exit::Config, "the declared settings couldn't be read"),
    )
    .subcommand(
        Help::new("apply", "writes every declared setting, then checks")
            .example(
                "run0 firmware-settings apply",
                "writes them, asking for the admin password",
            )
            .exit(Exit::Failed, "a setting wouldn't take")
            .exit(Exit::Refused, "only root may write the setting")
            .exit(Exit::Config, "the declared settings couldn't be read")
            .exit(Exit::Interrupted, "stopped, the password wiped first"),
    )
    .subcommand(
        Help::new("set", "writes one setting, for now")
            .example(
                "run0 firmware-settings set PrimaryBattChargeCfg Adaptive",
                "until apply puts it back",
            )
            .argument("<name>", "the setting, as check names it")
            .argument("<value>", "what it's set to")
            .exit(Exit::Failed, "it wouldn't take")
            .exit(Exit::NotFound, "the firmware has no such setting")
            .exit(Exit::Refused, "only root may write the setting")
            .exit(Exit::Interrupted, "stopped, the password wiped first"),
    )
    .exit(Exit::Failed, "a setting differs, or wouldn't take")
    .exit(Exit::NotFound, "the firmware has no such setting")
    .exit(Exit::Refused, "only root may read or write the setting")
    .exit(Exit::Config, "the declared settings couldn't be read")
    .exit(Exit::Interrupted, "stopped, the password wiped first")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let command = context.args.first().map_or("check", String::as_str);
        let firmware = Firmware::from_environment(command);
        wipe_when_stopped(Arc::clone(&firmware.unlocked));
        let done = run(context, &firmware);
        wipe(&firmware.unlocked);
        done
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the class, in a directory of its own, removed after.
    struct Class(PathBuf);

    impl Class {
        fn new(name: &str) -> Self {
            let root =
                env::temp_dir().join(format!("firmware-settings-{}-{name}", std::process::id()));
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
                waiting: self.0.join("waiting"),
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
        assert_eq!(
            firmware.differences().map_err(|failure| failure.exit),
            Ok(Vec::new())
        );
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
    fn a_setting_waiting_for_a_person_fails_nothing_while_another_still_does() {
        let class = Class::new("waiting");
        class.setting("class/vendor", "Downgrade", "Enabled");
        class.setting("class/vendor", "Mode", "Adaptive");
        let firmware = class.firmware("Downgrade=Disabled\n");
        fs::write(&firmware.waiting, "Downgrade=someone at the setup screen\n").unwrap();
        assert_eq!(firmware.look().unwrap().differing(), 0);
        fs::write(&firmware.declared, "Downgrade=Disabled\nMode=Custom\n").unwrap();
        assert_eq!(firmware.look().unwrap().differing(), 1);
        fs::write(&firmware.waiting, "").unwrap();
        fs::write(&firmware.declared, "Downgrade=Disabled\n").unwrap();
        assert_eq!(firmware.look().unwrap().differing(), 1);
    }

    #[test]
    fn a_look_names_what_a_setting_waits_for_in_text_and_as_null_in_json_where_nothing() {
        let look = Look {
            differs: vec![
                Differs {
                    name: "Downgrade".to_owned(),
                    now: "Enabled".to_owned(),
                    declared: "Disabled".to_owned(),
                    waits_for: Some("the setup screen".to_owned()),
                },
                Differs {
                    name: "Start".to_owned(),
                    now: "50".to_owned(),
                    declared: "75".to_owned(),
                    waits_for: None,
                },
            ],
            at_next_boot: true,
        };
        assert_eq!(
            look.text(),
            "Downgrade is Enabled, declared Disabled; it waits for the setup screen\n\
             Start is 50, declared 75\n\
             The firmware takes some of its settings at the next boot.\n"
        );
        let json = look.json();
        assert_eq!(json["schema"], "firmware-settings.check/1");
        assert_eq!(json["differs"][0]["waits_for"], "the setup screen");
        assert_eq!(json["differs"][1]["waits_for"], Value::Null);
        assert_eq!(json["at_next_boot"], true);
    }

    #[test]
    fn settings_as_declared_say_so() {
        let look = Look {
            differs: Vec::new(),
            at_next_boot: false,
        };
        assert_eq!(look.text(), "The firmware's settings are as declared.\n");
        assert_eq!(look.differing(), 0);
    }

    #[test]
    fn a_setting_the_firmware_lacks_is_missing_rather_than_differing() {
        let class = Class::new("missing");
        class.setting("class/vendor", "Mode", "Custom");
        let firmware = class.firmware("Absent=1\n");
        assert_eq!(
            firmware.differences().map_err(|failure| failure.exit),
            Err(Exit::NotFound)
        );
    }
}
