//! Lets the lock go after a resume from hibernation, since the disk prompt
//! has just asked for Joel's password, as he chose: one prompt, not two. A
//! wake from sleep keeps its lock. And a hibernation that rolled back, with
//! the machine never off, is never left on in silence.
//!
//! ```text
//! hibernate-unlock pre|post <action> [<disk>...]
//! hibernate-unlock rescue [<why>...]
//! hibernate-unlock fits
//! hibernate-unlock restoring
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: `fits` answers in words fitting the terminal,
//! or as `hibernate-unlock.fits/1` with `--json`, and exits 1 when the
//! image wouldn't fit.
//!
//! systemd-sleep runs it before and after each sleep, naming the action
//! (`suspend`, `hibernate`, `hybrid-sleep` or `suspend-then-hibernate`), and
//! the disks are those the initrd opens. Before anything that writes an
//! image it keeps the TPM's reset and restart counts in `/run`, which the
//! image keeps too, and the number of the kernel's last log record. After,
//! it lets every session's lock go only when both hold:
//!
//! - A count went up, so the firmware started the machine: the image was
//!   read back, through the initrd, rather than a hybrid sleep waking from
//!   its sleep. Linux tells the TPM to save its state going down, so the
//!   firmware's start counts as a restart, not a reset; a wake from sleep
//!   moves neither.
//! - Every disk can only be opened with something typed: each TPM token
//!   asks for a PIN, each FIDO2 token for its PIN, a recovery key is typed,
//!   and a keyslot with no token is a password. Any other token keeps the
//!   lock, and the log names it. A disk the TPM opens alone would let anyone
//!   through, and with no disk named nothing is let go.
//!
//! It asks `tpm2_readclock` for the counts, `cryptsetup` for each disk's
//! tokens and `loginctl` to let the locks go, each from `PATH`. Anything it
//! can't read keeps the lock, and it never fails the sleep.
//!
//! A hibernation back with neither count moved rolled back: the kernel
//! couldn't make the image, couldn't write it, or a device refused to power
//! off once it was written, when the kernel tells systemd it worked. The
//! kernel's log since the sleep began says which, and the rescue is told.
//! Ten seconds later, once the sleep is over, `rescue` runs on its own.
//! With the lid open, a notice tells whoever is signed in why, that nothing
//! was lost, and what to do. With the lid shut nobody is looking, and a
//! laptop left on in a bag runs flat and hot. Short of memory or swap, a
//! second try fails the same way and then powers off, losing what's open,
//! so the laptop goes to sleep instead, the notice waiting for the lid to
//! open. Otherwise it hibernates once more, and powers off if that rolls
//! back too.
//!
//! Before an image is made it sets the hard memory floors aside, and puts
//! them back after, or the kernel can't take back what they guard to make
//! room (floors.rs). It switches zswap off too, and on again after, or
//! what's swapped out for the image can stay in memory (zswap.rs). `fits`
//! says whether the image would fit, by the kernel's sums, before anything
//! is asked to hibernate (fits.rs), and exits 1 when it wouldn't.
//!
//! `restoring` runs in the initrd just before the image is read, keeping
//! when in an EFI variable, so that once the machine is back the hook says
//! how long the restore took (restored.rs).

mod fits;
mod floors;
mod restored;
mod zswap;

use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use terminal_kit::{Exit, Failure, Help};

/// Where the counts from before the image was written are kept.
const KEPT: &str = "/run/hibernate-unlock";

/// Where the number of the kernel's last log record before a hibernation is
/// kept, so what it logged after can be read alone.
const LOGGED: &str = "/run/hibernate-unlock-logged";

/// Where the hard memory floors set aside for an image are kept until
/// they're put back.
const FLOORS: &str = "/run/hibernate-unlock-floors";

/// Where the cgroups are, the floors in them.
const CGROUPS: &str = "/sys/fs/cgroup";

/// Kept while zswap is switched off for an image.
const ZSWAP_OFF: &str = "/run/hibernate-unlock-zswap";

/// Kept while a rolled-back hibernation is tried again, so a second
/// rollback powers off rather than trying forever.
const TRIED_AGAIN: &str = "/run/hibernate-unlock-tried-again";

/// Why a hibernation rolled back, as the kernel's log says.
#[derive(Debug, PartialEq, Eq, Clone)]
enum Why {
    /// Memory was too full to make the image.
    Memory,
    /// The swap file the image goes to wasn't there.
    SwapGone,
    /// The swap file had no room left for the image.
    SwapFull,
    /// A device's driver, by its name, refused to stop.
    Device(String),
    /// A program, by its name, wouldn't pause.
    Program(String),
    /// Something refused to stop, and the kernel didn't say what.
    Refused,
    /// The image was made but couldn't be written.
    Unwritten,
    /// Something woke the laptop as it went down.
    Woken,
    /// The kernel didn't say.
    Unsaid,
}

impl Why {
    /// What the kernel's log records, in order, say went wrong. When several
    /// things did, the one that stopped it comes first: a device refusing to
    /// power off is logged as a wakeup too.
    fn from_log(said: &[String]) -> Self {
        let any = |of: &[&str]| {
            said.iter()
                .any(|line| of.iter().any(|it| line.contains(*it)))
        };
        let freezing = |ended: &str| {
            said.iter()
                .any(|line| line.starts_with("Freezing ") && line.contains(ended))
        };
        if any(&[
            "PM: hibernation: Error -12 creating image",
            "PM: hibernation: Image allocation is ",
            "PM: hibernation: Cannot allocate",
        ]) {
            return Self::Memory;
        }
        if any(&["PM: Cannot find swap device", "PM: Swap header not found"]) {
            return Self::SwapGone;
        }
        // Compressed, the image runs out of swap as it's written, which the
        // writer says as ENOSPC's -28.
        if any(&["PM: Not enough free swap", "PM: Image saving failed: -28"]) {
            return Self::SwapFull;
        }
        if let Some(driver) = said.iter().find_map(|line| refusing_driver(line)) {
            return Self::Device(driver);
        }
        if let Some(program) = refusing_program(said) {
            return Self::Program(program);
        }
        if any(&["failed to power down"]) || freezing(" failed after ") {
            return Self::Refused;
        }
        if said.iter().any(|line| unwritten(line)) {
            return Self::Unwritten;
        }
        if any(&["Wakeup event detected during hibernation"]) || freezing(" aborted after ") {
            return Self::Woken;
        }
        Self::Unsaid
    }

    /// As the rescue is told it, one word and a name where there is one.
    fn args(&self) -> Vec<String> {
        let (word, name) = match self {
            Self::Memory => ("memory", None),
            Self::SwapGone => ("swap-gone", None),
            Self::SwapFull => ("swap-full", None),
            Self::Device(driver) => ("device", Some(driver)),
            Self::Program(program) => ("program", Some(program)),
            Self::Refused => ("refused", None),
            Self::Unwritten => ("unwritten", None),
            Self::Woken => ("woken", None),
            Self::Unsaid => ("unsaid", None),
        };
        std::iter::once(word.to_owned())
            .chain(name.cloned())
            .collect()
    }

    fn from_args(args: &[String]) -> Self {
        match args {
            [word] if word == "memory" => Self::Memory,
            [word] if word == "swap-gone" => Self::SwapGone,
            [word] if word == "swap-full" => Self::SwapFull,
            [word, driver] if word == "device" => Self::Device(driver.clone()),
            [word, program] if word == "program" => Self::Program(program.clone()),
            [word] if word == "refused" => Self::Refused,
            [word] if word == "unwritten" => Self::Unwritten,
            [word] if word == "woken" => Self::Woken,
            _ => Self::Unsaid,
        }
    }

    /// Whether hibernating again would fail the same way.
    fn fails_again(&self) -> bool {
        matches!(self, Self::Memory | Self::SwapGone | Self::SwapFull)
    }
}

/// The driver a device's refusal names: the kernel logs a device's line as
/// its driver, its device and the message, and the device is a bus address
/// nobody knows the laptop's parts by.
fn refusing_driver(line: &str) -> Option<String> {
    let (who, said) = line.split_once(": PM: ")?;
    if !(said.starts_with("failed to ") || said.starts_with("dpm_run_callback(): ")) {
        return None;
    }
    who.split_whitespace().next().map(str::to_owned)
}

/// The program that wouldn't pause: the kernel logs each task it couldn't
/// freeze after saying it failed, as `task:<name> state:...`. A kernel
/// thread is no program to close, so only user space's are named.
fn refusing_program(said: &[String]) -> Option<String> {
    let failed = said
        .iter()
        .position(|line| line.contains("Freezing user space processes failed after"))?;
    said[failed + 1..].iter().find_map(|line| {
        let task = line.split_once("task:")?.1;
        let name = task.split_once(" state:")?.0.trim();
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// Whether a line is the swap writer's, or its compressor's, failing.
fn unwritten(line: &str) -> bool {
    let Some(said) = line.split_once("PM: ").map(|(_, said)| said) else {
        return false;
    };
    [
        "Image saving failed",
        "Cannot get swap writer",
        "Could not allocate comp",
        "Cannot start compression threads",
        "Cannot start CRC32 thread",
        "Failed to allocate ",
    ]
    .iter()
    .any(|start| said.starts_with(start))
        || said.contains(" compression failed")
        || said.contains(" compressed length")
}

/// One record of the kernel's log, as `/dev/kmsg` gives it:
/// `<level>,<number>,<time>,<flags>;<message>`, then any of its fields on
/// lines of their own.
#[derive(Debug, PartialEq, Eq)]
struct Record {
    number: u64,
    message: String,
}

impl Record {
    fn parse(raw: &str) -> Option<Self> {
        let (head, rest) = raw.split_once(';')?;
        Some(Self {
            number: head.split(',').nth(1)?.parse().ok()?,
            message: rest.lines().next().unwrap_or_default().to_owned(),
        })
    }
}

/// Every record the kernel's log still holds, oldest first, or none if it
/// can't be read. Read without blocking, it ends where the log does.
fn kernel_log() -> Vec<Record> {
    let Ok(mut log) = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/kmsg")
    else {
        return Vec::new();
    };
    let mut records = Vec::new();
    // The longest record the kernel gives a reader.
    let mut buffer = vec![0_u8; 8192];
    loop {
        match log.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => records.extend(Record::parse(&String::from_utf8_lossy(&buffer[..read]))),
            // A record written over before it was read; the next follows.
            Err(error) if error.kind() == ErrorKind::BrokenPipe => {}
            Err(_) => break,
        }
    }
    records
}

/// What a notice says: who, why, then what to do.
#[derive(Debug, PartialEq, Eq)]
struct Notice {
    title: String,
    body: String,
}

/// The notice for a rollback, said with the laptop still on, or once it
/// has gone to sleep instead.
fn notice(why: &Why, slept: bool) -> Notice {
    let kept = if slept {
        "Nothing was lost: with the lid shut, it went to sleep instead."
    } else {
        "Nothing was lost; the laptop is still on."
    };
    let again = "Hibernate again in a moment; if it fails twice, put the laptop to sleep instead.";
    let close = "Close something you don't need, then hibernate again.";
    let (title, said, todo) = match why {
        Why::Memory => (
            "Not enough memory to hibernate".to_owned(),
            "Memory was too full to copy before writing it to the swap file.".to_owned(),
            close.to_owned(),
        ),
        Why::SwapGone => (
            "No swap file to hibernate to".to_owned(),
            "The swap file that memory is written to wasn't there.".to_owned(),
            "Put it to sleep instead: hibernating again would fail the same way.".to_owned(),
        ),
        Why::SwapFull => (
            "Not enough swap to hibernate".to_owned(),
            "The swap file had no room left for memory.".to_owned(),
            close.to_owned(),
        ),
        Why::Device(driver) => (
            format!("{driver} wouldn't let it hibernate"),
            format!("The driver {driver} refused to stop its device."),
            again.to_owned(),
        ),
        Why::Program(program) => (
            format!("{program} wouldn't let it hibernate"),
            format!("{program} wouldn't pause."),
            format!("Close {program}, then hibernate again."),
        ),
        Why::Refused => (
            "A device wouldn't let it hibernate".to_owned(),
            "Something refused to stop, and the kernel didn't say what.".to_owned(),
            again.to_owned(),
        ),
        Why::Unwritten => (
            "Hibernation couldn't write memory".to_owned(),
            "Memory couldn't be written to the swap file.".to_owned(),
            again.to_owned(),
        ),
        Why::Woken => (
            "Woken while hibernating".to_owned(),
            "Something woke the laptop as it went down: a key, the lid or a cable.".to_owned(),
            "Hibernate again, and leave it be until it's off.".to_owned(),
        ),
        Why::Unsaid => (
            "Hibernation didn't finish".to_owned(),
            "The kernel didn't say why.".to_owned(),
            again.to_owned(),
        ),
    };
    Notice {
        title,
        body: format!("{said} {kept} {todo}"),
    }
}

/// What a rescue does once a hibernation rolled back.
#[derive(Debug, PartialEq, Eq)]
enum Rescue {
    HibernateAgain,
    PowerOff,
    Sleep,
    Say,
}

/// With the lid open someone is there to be told. With it shut, sleep
/// where a second try would fail the same way; otherwise one more try,
/// then off.
fn rescue(lid_shut: bool, tried_again: bool, why: &Why) -> Rescue {
    match (lid_shut, tried_again) {
        (false, _) => Rescue::Say,
        (true, _) if why.fails_again() => Rescue::Sleep,
        (true, false) => Rescue::HibernateAgain,
        (true, true) => Rescue::PowerOff,
    }
}

/// What follows the rescue asking the laptop to sleep: nothing once it
/// sleeps, and off when it couldn't, so a laptop with its lid shut is never
/// left on in a bag.
fn after_asking_to_sleep(slept: bool) -> Option<Rescue> {
    (!slept).then_some(Rescue::PowerOff)
}

/// Whether a lid's `/proc/acpi/button/lid/*/state` says it's shut.
fn says_shut(state: &str) -> bool {
    state.split_whitespace().last() == Some("closed")
}

/// The users signed in at a seat, from `loginctl list-sessions --json=short`.
fn seated(sessions: &Value) -> Vec<String> {
    let mut users: Vec<String> = sessions
        .as_array()
        .into_iter()
        .flatten()
        .filter(|session| {
            session["seat"]
                .as_str()
                .is_some_and(|seat| !seat.is_empty())
        })
        .filter_map(|session| session["user"].as_str().map(str::to_owned))
        .collect();
    users.sort();
    users.dedup();
    users
}

/// The TPM's counts of the times it started afresh and the times it started
/// from saved state, the two a firmware start moves.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Starts {
    reset: u64,
    restart: u64,
}

impl Starts {
    /// Reads `tpm2_readclock`'s `reset_count:` and `restart_count:` lines.
    fn parse(said: &str) -> Option<Self> {
        let count = |name: &str| {
            said.lines()
                .filter_map(|line| line.trim().strip_prefix(name))
                .find_map(|rest| rest.strip_prefix(':')?.trim().parse().ok())
        };
        Some(Self {
            reset: count("reset_count")?,
            restart: count("restart_count")?,
        })
    }

    /// As kept in `/run`: the two counts on one line.
    fn kept(self) -> String {
        format!("{} {}\n", self.reset, self.restart)
    }

    fn unkept(text: &str) -> Option<Self> {
        let mut counts = text.split_whitespace().map(str::parse);
        let (Some(Ok(reset)), Some(Ok(restart)), None) =
            (counts.next(), counts.next(), counts.next())
        else {
            return None;
        };
        Some(Self { reset, restart })
    }

    /// Whether the firmware started the machine between `self` and `now`.
    fn started_since(self, now: Self) -> bool {
        now.reset > self.reset || now.restart > self.restart
    }
}

/// Whether a sleep writes an image the machine can start again from.
fn writes_an_image(action: &str) -> bool {
    matches!(
        action,
        "hibernate" | "hybrid-sleep" | "suspend-then-hibernate"
    )
}

/// The first token in a disk's LUKS2 header, as `cryptsetup luksDump
/// --dump-json-metadata` prints it, that opens the disk with nothing typed,
/// or nothing when every one asks. A keyslot with no token is a password,
/// and a recovery key is typed like one.
fn unasked(header: &Value) -> Option<String> {
    let Some(tokens) = header["tokens"].as_object() else {
        return Some("its header lists no tokens".to_owned());
    };
    tokens.iter().find_map(|(slot, token)| {
        let kind = token["type"].as_str().unwrap_or("no type");
        let asks = match kind {
            "systemd-tpm2" => token["tpm2-pin"] == true,
            "systemd-fido2" => token["fido2-clientPin-required"] == true,
            "systemd-recovery" => true,
            _ => false,
        };
        (!asks).then(|| format!("token {slot} ({kind}) opens it unasked"))
    })
}

/// What a command printed, or nothing if it couldn't run or failed.
fn said(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn starts() -> Option<Starts> {
    Starts::parse(&said("tpm2_readclock", &[])?)
}

/// Why a disk could open without the prompt asking for anything, or
/// nothing when it couldn't.
fn disk_unasked(disk: &str) -> Option<String> {
    let header = said("cryptsetup", &["luksDump", "--dump-json-metadata", disk])
        .and_then(|json| serde_json::from_str::<Value>(&json).ok());
    match header {
        Some(header) => unasked(&header),
        None => Some("its header couldn't be read".to_owned()),
    }
}

/// Before an image is written, keep the TPM's counts and the kernel log's
/// place. Before a sleep that writes none, forget any kept from a hibernate
/// that never happened.
fn before(action: &str) {
    let _ = fs::remove_file(KEPT);
    let _ = fs::remove_file(LOGGED);
    if !writes_an_image(action) {
        return;
    }
    match floors::lift(Path::new(CGROUPS), Path::new(FLOORS)) {
        Ok(0) => {}
        Ok(lifted) => eprintln!("hibernate-unlock: set {lifted} memory floors aside for the image"),
        Err(error) => eprintln!("hibernate-unlock: couldn't set the memory floors aside: {error}"),
    }
    match zswap::set_aside(Path::new(zswap::ENABLED), Path::new(ZSWAP_OFF)) {
        Ok(false) => {}
        Ok(true) => eprintln!("hibernate-unlock: switched zswap off for the image"),
        Err(error) => eprintln!("hibernate-unlock: couldn't switch zswap off: {error}"),
    }
    // What the graphics drivers hold as the image is sized: they copy it
    // out as they stop, and the reserve has to leave room for that.
    for card in graphics_memory() {
        eprintln!("hibernate-unlock: {card}");
    }
    match starts() {
        Some(now) => {
            if let Err(error) = fs::write(KEPT, now.kept()) {
                eprintln!("hibernate-unlock: couldn't keep the TPM's counts: {error}");
            }
        }
        None => eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays"),
    }
    match kernel_log().last() {
        Some(last) => {
            if let Err(error) = fs::write(LOGGED, last.number.to_string()) {
                eprintln!("hibernate-unlock: couldn't keep the kernel log's place: {error}");
            }
        }
        None => eprintln!("hibernate-unlock: couldn't read the kernel's log"),
    }
}

/// What the kernel logged since `before` kept its place.
fn logged_since_before() -> Vec<String> {
    let Some(since) = fs::read_to_string(LOGGED)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
    else {
        return Vec::new();
    };
    kernel_log()
        .into_iter()
        .filter(|record| record.number > since)
        .map(|record| record.message)
        .collect()
}

/// After the machine is back, let the locks go if it came back through the
/// disk prompt, and rescue a hibernation that never went off.
fn after(action: &str, disks: &[String]) {
    match floors::restore(Path::new(CGROUPS), Path::new(FLOORS)) {
        Ok(0) => {}
        Ok(restored) => eprintln!("hibernate-unlock: put {restored} memory floors back"),
        Err(error) => eprintln!("hibernate-unlock: couldn't put the memory floors back: {error}"),
    }
    match zswap::put_back(Path::new(zswap::ENABLED), Path::new(ZSWAP_OFF)) {
        Ok(false) => {}
        Ok(true) => eprintln!("hibernate-unlock: switched zswap on again"),
        Err(error) => eprintln!("hibernate-unlock: couldn't switch zswap on again: {error}"),
    }
    let logged = logged_since_before();
    let _ = fs::remove_file(LOGGED);
    let Some(then) = fs::read_to_string(KEPT)
        .ok()
        .and_then(|t| Starts::unkept(&t))
    else {
        return;
    };
    let _ = fs::remove_file(KEPT);
    let Some(now) = starts() else {
        eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays");
        return;
    };
    if !then.started_since(now) {
        if action == "hibernate" {
            rolled_back(&Why::from_log(&logged));
        } else {
            eprintln!(
                "hibernate-unlock: woke without a start, so through no prompt; the lock stays"
            );
        }
        return;
    }
    let _ = fs::remove_file(TRIED_AGAIN);
    say_how_long_the_restore_took();
    if disks.is_empty() {
        eprintln!("hibernate-unlock: no disk asked at the prompt; the lock stays");
        return;
    }
    if let Some((disk, why)) = disks
        .iter()
        .find_map(|disk| disk_unasked(disk).map(|why| (disk, why)))
    {
        eprintln!("hibernate-unlock: {disk}: {why}; the lock stays");
        return;
    }
    match said("loginctl", &["unlock-sessions"]) {
        Some(_) => eprintln!("hibernate-unlock: back through the disk prompt; the lock is let go"),
        None => eprintln!("hibernate-unlock: loginctl couldn't let the lock go"),
    }
}

/// Says how long the restore took, when this start's initrd kept when it
/// began.
fn say_how_long_the_restore_took() {
    let variable = Path::new(restored::VARIABLE);
    let Some(then) = restored::kept(variable) else {
        eprintln!("hibernate-unlock: the initrd kept no count, so how long it took is unsaid");
        return;
    };
    // The firmware keeps it across starts, so a later restore whose initrd
    // couldn't keep one would read this one.
    if let Err(error) = restored::forget(variable) {
        eprintln!("hibernate-unlock: couldn't remove the restore's count: {error}");
    }
    let said = restored::counted()
        .zip(restored::rate())
        .and_then(|(now, rate)| restored::said(then, now, rate));
    match said {
        Some(said) => eprintln!("hibernate-unlock: {said}"),
        None => eprintln!("hibernate-unlock: couldn't count how long the restore took"),
    }
}

/// Keeps when the restore began, in the initrd, never failing it.
fn restoring() {
    let Some(now) = restored::counted() else {
        eprintln!("hibernate-unlock: no counter to keep when the restore began");
        return;
    };
    if let Err(error) = restored::keep(Path::new(restored::VARIABLE), now) {
        eprintln!("hibernate-unlock: couldn't keep when the restore began: {error}");
    }
}

/// Asks systemd to run the rescue in ten seconds, once this sleep is over
/// and the sessions are thawed, with the same programs on its `PATH`.
fn rolled_back(why: &Why) {
    let words = why.args().join(" ");
    eprintln!("hibernate-unlock: the hibernation rolled back, so the machine is still on: {words}");
    let Ok(me) = env::current_exe() else {
        eprintln!("hibernate-unlock: couldn't find itself to rescue the machine");
        return;
    };
    let path = format!("--setenv=PATH={}", env::var("PATH").unwrap_or_default());
    let me = me.to_string_lossy();
    let why = why.args();
    let mut args = vec![
        "--on-active=10s",
        "--collect",
        "--quiet",
        "--description=Rescues a hibernation that rolled back",
        &path,
        &me,
        "rescue",
    ];
    args.extend(why.iter().map(String::as_str));
    if said("systemd-run", &args).is_none() {
        eprintln!("hibernate-unlock: systemd-run couldn't start the rescue");
    }
}

fn lid_shut() -> bool {
    fs::read_dir("/proc/acpi/button/lid")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|lid| fs::read_to_string(lid.path().join("state")).ok())
        .any(|state| says_shut(&state))
}

fn on_path(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// Puts a notice in each seated user's session, through their own systemd.
fn say_to_each_session(notice: &Notice) {
    eprintln!(
        "hibernate-unlock: it says: {}: {}",
        notice.title, notice.body
    );
    let Some(notify) = on_path("notify-send") else {
        eprintln!("hibernate-unlock: no notify-send to say the hibernation didn't finish");
        return;
    };
    let notify = notify.to_string_lossy();
    let sessions = said("loginctl", &["list-sessions", "--json=short"])
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null);
    for user in seated(&sessions) {
        let machine = format!("--machine={user}@.host");
        let args = [
            machine.as_str(),
            "--user",
            "--collect",
            "--quiet",
            &notify,
            "--urgency=critical",
            "--app-name=Hibernation",
            &notice.title,
            &notice.body,
        ];
        if said("systemd-run", &args).is_none() {
            eprintln!("hibernate-unlock: couldn't tell {user} the hibernation didn't finish");
        }
    }
}

/// What the rescue ten seconds after a rollback does.
fn rescue_now(why: &Why) {
    let tried_again = Path::new(TRIED_AGAIN).exists();
    match rescue(lid_shut(), tried_again, why) {
        Rescue::HibernateAgain => {
            eprintln!("hibernate-unlock: the lid is shut, so it hibernates again");
            let _ = fs::write(TRIED_AGAIN, "");
            let _ = said("systemctl", &["hibernate", "--check-inhibitors=no"]);
        }
        Rescue::PowerOff => {
            eprintln!(
                "hibernate-unlock: the lid is shut and it rolled back twice, so it powers off"
            );
            power_off();
        }
        Rescue::Sleep => {
            eprintln!(
                "hibernate-unlock: the lid is shut and hibernating again would fail the same \
                 way, so it sleeps"
            );
            let _ = fs::remove_file(TRIED_AGAIN);
            say_to_each_session(&notice(why, true));
            let slept = said("systemctl", &["suspend", "--check-inhibitors=no"]).is_some();
            if after_asking_to_sleep(slept) == Some(Rescue::PowerOff) {
                eprintln!(
                    "hibernate-unlock: the lid is shut and it couldn't sleep, so it powers off"
                );
                power_off();
            }
        }
        Rescue::Say => {
            eprintln!("hibernate-unlock: the lid is open, so it says so");
            let _ = fs::remove_file(TRIED_AGAIN);
            say_to_each_session(&notice(why, false));
        }
    }
}

/// Says whether an image would fit now, exiting 1 when it wouldn't. What
/// it can't read it doesn't stand in the way of.
/// Says whether the image would fit, as words or as
/// `hibernate-unlock.fits/1`, failing when it wouldn't.
fn fits_now(context: &terminal_kit::Context) -> Result<(), Failure> {
    let read = |path: &str| fs::read_to_string(path).ok();
    let memory = read("/proc/meminfo").and_then(|text| fits::Memory::parse(&text));
    let reserve = read("/sys/power/reserved_size").and_then(|text| text.trim().parse().ok());
    let swaps = read("/proc/swaps");
    let zones = read("/proc/zoneinfo");
    let (Some(memory), Some(reserve), Some(swaps), Some(zones)) = (memory, reserve, swaps, zones)
    else {
        return say(
            context,
            &json!({"schema": "hibernate-unlock.fits/1", "fits": null}),
            "It couldn't tell whether the image fits, so it doesn't stand in the way.",
        );
    };
    let fit = fits::Fit::of(
        memory,
        fits::Zones::parse(&zones),
        reserve,
        fits::swap_room(&swaps),
    );
    let facts = json!({
        "schema": "hibernate-unlock.fits/1",
        "fits": fit.fits(),
        "to_save_bytes": fit.least,
        "room_bytes": fit.most,
    });
    if fit.fits() {
        return say(context, &facts, &fit.said());
    }
    if context.format().is_json() {
        context.answer(&facts).map_err(|error| unsaid(&error))?;
    }
    Err(Failure::new(Exit::Failed, "there isn't enough memory to hibernate").because(fit.short()))
}

/// Says `facts` with `--json`, else `words` set to the terminal's measure.
fn say(context: &terminal_kit::Context, facts: &Value, words: &str) -> Result<(), Failure> {
    let written = if context.format().is_json() {
        context.answer(facts)
    } else {
        let lines = terminal_kit::layout::paragraph(words, 0, context.room.for_prose());
        context.say(&(lines.join("\n") + "\n"))
    };
    written.map_err(|error| unsaid(&error))
}

fn unsaid(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
}

/// The video memory and the system memory it maps (GTT) in use on each card
/// whose driver says, one line apiece. amdgpu says; others, xe among them,
/// log nothing.
fn graphics_memory() -> Vec<String> {
    let mut cards: Vec<String> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|card| {
            let name = card.file_name().into_string().ok()?;
            let device = card.path().join("device");
            let read = |file: &str| fs::read_to_string(device.join(file)).ok();
            let vram = read("mem_info_vram_used")?;
            let gtt = read("mem_info_gtt_used")?;
            (!name.contains('-')).then(|| {
                format!(
                    "{name} holds {} bytes of video memory and {} of GTT",
                    vram.trim(),
                    gtt.trim()
                )
            })
        })
        .collect();
    cards.sort();
    cards
}

fn power_off() {
    let _ = fs::remove_file(TRIED_AGAIN);
    let _ = said("systemctl", &["poweroff", "--check-inhibitors=no"]);
}

fn help() -> Help {
    Help::new(
        "hibernate-unlock",
        "lets the lock go after a resume from hibernation",
    )
    .example(
        "hibernate-unlock fits",
        "whether memory's image would fit now",
    )
    .subcommand(
        Help::new("fits", "says whether a hibernation's image would fit")
            .example(
                "hibernate-unlock fits",
                "whether memory's image would fit now",
            )
            .example("hibernate-unlock fits --json", "the same, as JSON")
            .exit(Exit::Failed, "the image wouldn't fit"),
    )
    .subcommand(sleeps("pre", "what systemd-sleep runs before a sleep"))
    .subcommand(sleeps("post", "what systemd-sleep runs after a sleep"))
    .subcommand(
        Help::new("rescue", "tells of a hibernation that rolled back")
            .argument("[why...]", "memory, swap-gone, device DRIVER...")
            .example("hibernate-unlock rescue memory", "memory ran short")
            .exit(Exit::Done, NEVER_FAILS),
    )
    .subcommand(
        Help::new(
            "restoring",
            "what the initrd runs before it reads the image back",
        )
        .example("hibernate-unlock restoring", "what the initrd runs")
        .exit(Exit::Done, NEVER_FAILS),
    )
    .exit(Exit::Failed, "the image wouldn't fit")
}

/// What a step that never fails what it runs within says of its exit.
const NEVER_FAILS: &str = "always; what it found goes to the journal";

/// One of the steps systemd-sleep runs around a sleep, which logs what it
/// found and never fails the sleep.
fn sleeps(when: &str, about: &str) -> Help {
    Help::new(when, about)
        .argument("<action>", "the sleep, as hibernate or suspend")
        .argument("[disk...]", "the encrypted disks the lock asks for")
        .example(
            &format!("hibernate-unlock {when} hibernate"),
            "what systemd-sleep runs",
        )
        .exit(Exit::Done, NEVER_FAILS)
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        // Each of the sleep's own steps logs what it found and never fails
        // the sleep, so only `fits` answers with a failing exit.
        match context.args.as_slice() {
            [when, action, ..] if when == "pre" => before(action),
            [when, action, disks @ ..] if when == "post" => after(action, disks),
            [what, why @ ..] if what == "rescue" => rescue_now(&Why::from_args(why)),
            [what] if what == "fits" => return fits_now(context),
            [what] if what == "restoring" => restoring(),
            _ => {
                return Err(Failure::new(Exit::Usage, "that isn't one of its commands")
                    .try_this("hibernate-unlock --help"));
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const READCLOCK: &str = "time: 3354065\nclock_info:\n  clock: 1776610977\n  \
                             reset_count: 95\n  restart_count: 0\n  safe: yes\n";

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|&line| line.to_owned()).collect()
    }

    /// Every case, with the kernel's own lines for it (7.3.1's
    /// `kernel/power` and `drivers/base/power/main.c`), those around them
    /// included.
    fn cases() -> Vec<(Why, Vec<String>)> {
        vec![
            (
                Why::Memory,
                // The Surface Laptop 4's, on 4 November 2026.
                lines(&[
                    "PM: hibernation: Allocated 745783 pages for snapshot",
                    "PM: hibernation: Normal pages needed: 1089703 + 1024, available pages: 863209",
                    "PM: hibernation: Error -12 creating image",
                ]),
            ),
            (
                Why::Memory,
                lines(&["PM: hibernation: Image allocation is 56721 pages short"]),
            ),
            (
                Why::Memory,
                lines(&["PM: hibernation: Cannot allocate copy bitmap"]),
            ),
            (
                Why::SwapGone,
                lines(&[
                    "PM: Cannot find swap device, try swapon -a",
                    "PM: Cannot get swap writer",
                ]),
            ),
            (Why::SwapGone, lines(&["PM: Swap header not found!"])),
            (Why::SwapFull, lines(&["PM: Not enough free swap"])),
            (
                Why::SwapFull,
                lines(&[
                    "PM: Compressing and saving image data (120034 pages)...",
                    "PM: Image saving failed: -28",
                ]),
            ),
            (
                Why::Device("btintel_pcie".to_owned()),
                // The XPS 14's Bluetooth refusing to power off, which the
                // kernel then logs as a wakeup.
                lines(&[
                    "btintel_pcie 0000:00:14.7: PM: failed to hibernate async: error -16",
                    "PM: hibernation: Wakeup event detected during hibernation, rolling back.",
                ]),
            ),
            (
                Why::Device("amdgpu".to_owned()),
                lines(&[
                    "amdgpu 0000:03:00.0: PM: dpm_run_callback(): pci_pm_freeze returns -16",
                    "amdgpu 0000:03:00.0: PM: failed to freeze async: error -16",
                ]),
            ),
            (
                Why::Program("fuse-thing".to_owned()),
                lines(&[
                    "Freezing user space processes failed after 20.006 seconds (1 tasks \
                     refusing to freeze, wq_busy=0):",
                    "task:fuse-thing      state:D stack:0     pid:4242  tgid:4242  ppid:1",
                ]),
            ),
            (
                Why::Refused,
                lines(&[
                    "Freezing remaining freezable tasks failed after 20.004 seconds (1 tasks \
                     refusing to freeze, wq_busy=0):",
                    "task:kworker/2:1     state:D stack:0     pid:99    tgid:99    ppid:2",
                ]),
            ),
            (
                Why::Refused,
                lines(&["PM: hibernation: Some devices failed to power down, aborting"]),
            ),
            (
                Why::Unwritten,
                lines(&["PM: lz4 compression failed", "PM: Image saving failed: -5"]),
            ),
            (
                Why::Unwritten,
                lines(&["PM: Cannot start compression threads"]),
            ),
            (
                Why::Woken,
                lines(&[
                    "PM: hibernation: Wakeup event detected during hibernation, rolling back.",
                ]),
            ),
            (
                Why::Woken,
                lines(&[
                    "Freezing user space processes aborted after 0.012 seconds (3 tasks \
                     refusing to freeze, wq_busy=0):",
                ]),
            ),
            (
                Why::Unsaid,
                lines(&[
                    "PM: hibernation: hibernation entry",
                    "PM: hibernation: hibernation exit",
                ]),
            ),
            (Why::Unsaid, Vec::new()),
        ]
    }

    #[test]
    fn the_kernels_log_says_why_a_hibernation_rolled_back() {
        for (why, said) in cases() {
            assert_eq!(Why::from_log(&said), why, "{said:?}");
        }
    }

    #[test]
    fn a_record_of_the_kernels_log_is_read_with_its_number() {
        assert_eq!(
            Record::parse(
                "3,2813,4021375310,-;amdgpu 0000:03:00.0: PM: failed to freeze: error -16\n \
                 SUBSYSTEM=pci\n DEVICE=+pci:0000:03:00.0\n"
            ),
            Some(Record {
                number: 2813,
                message: "amdgpu 0000:03:00.0: PM: failed to freeze: error -16".to_owned(),
            })
        );
        assert_eq!(Record::parse("no record"), None);
        assert_eq!(Record::parse("6,x,1,-;nothing"), None);
    }

    #[test]
    fn the_rescue_is_told_why_as_it_was_found() {
        for (why, _) in cases() {
            assert_eq!(Why::from_args(&why.args()), why);
        }
        assert_eq!(Why::from_args(&[]), Why::Unsaid);
    }

    /// Every notice, word for word, with the laptop still on.
    #[test]
    fn each_notice_says_why_that_nothing_was_lost_and_what_to_do() {
        let kept = "Nothing was lost; the laptop is still on.";
        let again = "Hibernate again in a moment; if it fails twice, put the laptop to sleep \
                     instead.";
        let close = "Close something you don't need, then hibernate again.";
        let said = [
            (
                Why::Memory,
                "Not enough memory to hibernate",
                format!(
                    "Memory was too full to copy before writing it to the swap file. {kept} \
                     {close}"
                ),
            ),
            (
                Why::SwapGone,
                "No swap file to hibernate to",
                format!(
                    "The swap file that memory is written to wasn't there. {kept} Put it to \
                     sleep instead: hibernating again would fail the same way."
                ),
            ),
            (
                Why::SwapFull,
                "Not enough swap to hibernate",
                format!("The swap file had no room left for memory. {kept} {close}"),
            ),
            (
                Why::Device("amdgpu".to_owned()),
                "amdgpu wouldn't let it hibernate",
                format!("The driver amdgpu refused to stop its device. {kept} {again}"),
            ),
            (
                Why::Program("fuse-thing".to_owned()),
                "fuse-thing wouldn't let it hibernate",
                format!(
                    "fuse-thing wouldn't pause. {kept} Close fuse-thing, then hibernate again."
                ),
            ),
            (
                Why::Refused,
                "A device wouldn't let it hibernate",
                format!(
                    "Something refused to stop, and the kernel didn't say what. {kept} {again}"
                ),
            ),
            (
                Why::Unwritten,
                "Hibernation couldn't write memory",
                format!("Memory couldn't be written to the swap file. {kept} {again}"),
            ),
            (
                Why::Woken,
                "Woken while hibernating",
                format!(
                    "Something woke the laptop as it went down: a key, the lid or a cable. \
                     {kept} Hibernate again, and leave it be until it's off."
                ),
            ),
            (
                Why::Unsaid,
                "Hibernation didn't finish",
                format!("The kernel didn't say why. {kept} {again}"),
            ),
        ];
        for (why, title, body) in said {
            assert_eq!(
                notice(&why, false),
                Notice {
                    title: title.to_owned(),
                    body
                }
            );
        }
    }

    #[test]
    fn a_notice_said_as_it_sleeps_says_it_slept() {
        assert_eq!(
            notice(&Why::Memory, true).body,
            "Memory was too full to copy before writing it to the swap file. Nothing was \
             lost: with the lid shut, it went to sleep instead. Close something you don't \
             need, then hibernate again."
        );
    }

    #[test]
    fn with_no_swap_file_it_never_says_to_hibernate_again() {
        let Notice { body, .. } = notice(&Why::SwapGone, false);
        assert!(!body.contains("then hibernate again"), "{body}");
    }

    #[test]
    fn every_notice_is_plain_text() {
        for (why, _) in cases() {
            for slept in [false, true] {
                let Notice { title, body } = notice(&why, slept);
                for text in [title, body] {
                    assert!(text.is_ascii(), "{text}");
                    assert!(!text.contains("--"), "{text}");
                }
            }
        }
    }

    #[test]
    fn a_rollback_with_the_lid_open_is_said() {
        for (why, _) in cases() {
            assert_eq!(rescue(false, false, &why), Rescue::Say);
            assert_eq!(rescue(false, true, &why), Rescue::Say);
        }
    }

    #[test]
    fn short_of_memory_or_swap_with_the_lid_shut_it_sleeps() {
        for why in [Why::Memory, Why::SwapGone, Why::SwapFull] {
            assert_eq!(rescue(true, false, &why), Rescue::Sleep, "{why:?}");
            assert_eq!(rescue(true, true, &why), Rescue::Sleep, "{why:?}");
        }
    }

    #[test]
    fn a_laptop_that_cannot_sleep_powers_off() {
        assert_eq!(after_asking_to_sleep(true), None);
        assert_eq!(after_asking_to_sleep(false), Some(Rescue::PowerOff));
    }

    #[test]
    fn otherwise_with_the_lid_shut_it_tries_once_then_powers_off() {
        for (why, _) in cases().into_iter().filter(|(why, _)| !why.fails_again()) {
            assert_eq!(rescue(true, false, &why), Rescue::HibernateAgain, "{why:?}");
            assert_eq!(rescue(true, true, &why), Rescue::PowerOff, "{why:?}");
        }
    }

    #[test]
    fn the_counts_are_read_from_tpm2_readclock() {
        assert_eq!(
            Starts::parse(READCLOCK),
            Some(Starts {
                reset: 95,
                restart: 0
            })
        );
    }

    #[test]
    fn a_reading_without_both_counts_is_no_reading() {
        assert_eq!(Starts::parse("clock_info:\n  reset_count: 95\n"), None);
    }

    #[test]
    fn the_counts_come_back_as_they_were_kept() {
        let starts = Starts {
            reset: 95,
            restart: 3,
        };
        assert_eq!(Starts::unkept(&starts.kept()), Some(starts));
        assert_eq!(Starts::unkept("95"), None);
        assert_eq!(Starts::unkept("95 3 1"), None);
    }

    #[test]
    fn a_firmware_start_moves_one_count_and_a_wake_neither() {
        let then = Starts {
            reset: 95,
            restart: 0,
        };
        assert!(then.started_since(Starts {
            reset: 95,
            restart: 1
        }));
        assert!(then.started_since(Starts {
            reset: 96,
            restart: 0
        }));
        assert!(!then.started_since(then));
    }

    #[test]
    fn only_sleeps_that_write_an_image_are_kept() {
        for action in ["hibernate", "hybrid-sleep", "suspend-then-hibernate"] {
            assert!(writes_an_image(action), "{action}");
        }
        assert!(!writes_an_image("suspend"));
    }

    #[test]
    fn the_lid_reads_shut_only_when_it_says_closed() {
        assert!(says_shut("state:      closed\n"));
        assert!(!says_shut("state:      open\n"));
        assert!(!says_shut(""));
    }

    #[test]
    fn only_users_at_a_seat_are_told_once_each() {
        let sessions = json!([
            {"session": "1", "user": "joel", "seat": "seat0"},
            {"session": "2", "user": "joel", "seat": "seat0"},
            {"session": "3", "user": "ssh", "seat": null},
            {"session": "4", "user": "cron", "seat": ""}
        ]);
        assert_eq!(seated(&sessions), vec!["joel".to_owned()]);
        assert_eq!(seated(&Value::Null), [] as [std::string::String; 0]);
    }

    #[test]
    fn a_disk_whose_tokens_all_ask_is_typed() {
        // The three the guide enrols, the recovery key's as its installed
        // header in modules/guide/fixtures/qemu-canokey has it.
        let header = json!({"tokens": {
            "0": {"type": "systemd-tpm2", "tpm2-pin": true},
            "1": {"type": "systemd-fido2", "fido2-clientPin-required": true},
            "2": {"type": "systemd-recovery", "keyslots": ["3"]}
        }});
        assert_eq!(unasked(&header), None);
        assert_eq!(
            unasked(&json!({"tokens": {}})),
            None,
            "a password alone is typed"
        );
    }

    #[test]
    fn a_disk_any_token_opens_unasked_is_not() {
        for token in [
            json!({"type": "systemd-tpm2", "tpm2-pin": false}),
            json!({"type": "systemd-tpm2"}),
            json!({"type": "systemd-fido2", "fido2-clientPin-required": false}),
            json!({"type": "systemd-pkcs11"}),
        ] {
            let kind = token["type"].as_str().unwrap_or_default().to_owned();
            let header =
                json!({"tokens": {"0": {"type": "systemd-tpm2", "tpm2-pin": true}, "1": token}});
            assert_eq!(
                unasked(&header),
                Some(format!("token 1 ({kind}) opens it unasked")),
            );
        }
        assert_eq!(
            unasked(&json!({})),
            Some("its header lists no tokens".to_owned())
        );
    }
}
