//! The monitors beside the panel, whose brightness follows the panel's.
//!
//! A monitor takes its brightness over DDC/CI, a small protocol on its
//! cable's I2C bus, which the kernel offers as /dev/i2c-N. Only the buses of
//! DisplayPort connectors, and those a dock's MST hub makes for the monitors
//! behind it, are tried; the panel's own and the machine's other buses are
//! left alone. udev lets the person at the seat open those buses.
//!
//! A monitor answers slowly, a second or so for a read, so each is read once,
//! to learn its brightest, and after that only written. Each has a thread of
//! its own that owns its bus, so the daemon never waits on one, and it writes
//! only the latest level it was asked for: a move of many steps becomes as
//! many writes as the monitor takes, never a queue of them.
//!
//! Monitors come and go as their buses do, which a watch on /dev hears, so
//! nothing is looked for until one does.

use std::collections::HashMap;
use std::ffi::{CString, c_char, c_int, c_ulong};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

// std links libc, so these come from it.
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

/// The ioctl that says which address on the bus reads and writes go to.
const I2C_SLAVE: c_ulong = 0x0703;
/// Where a monitor answers DDC/CI.
const DDC_CI: u8 = 0x37;
/// The VCP code that holds a monitor's brightness.
const BRIGHTNESS: u8 = 0x10;
/// How long a monitor takes to have an answer ready.
const ANSWERING: Duration = Duration::from_millis(40);
/// How long a monitor wants between one command and the next.
const BETWEEN: Duration = Duration::from_millis(50);

const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
/// The bus's permissions changed, as when udev opens it to the seat.
const IN_ATTRIB: u32 = 0x4;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
/// Events were lost, so anything could have changed.
const IN_Q_OVERFLOW: u32 = 0x4000;

/// A message from the host: who it's from, how long it is, what it says, and
/// a checksum that counts the monitor's address too.
fn message(payload: &[u8]) -> Vec<u8> {
    // A DDC/CI message says at most 32 bytes, which fits.
    let mut message = vec![0x51, 0x80 | payload.len() as u8];
    message.extend_from_slice(payload);
    message.push(message.iter().fold(0x6E, |sum, byte| sum ^ byte));
    message
}

/// Asks what a VCP code holds.
fn get_vcp(vcp: u8) -> Vec<u8> {
    message(&[0x01, vcp])
}

/// Sets a VCP code.
fn set_vcp(vcp: u8, value: u16) -> Vec<u8> {
    let [high, low] = value.to_be_bytes();
    message(&[0x03, vcp, high, low])
}

/// What a monitor says a VCP code holds.
#[derive(Debug, PartialEq)]
struct Reading {
    max: u16,
    now: u16,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// A monitor's answer to get_vcp, whose checksum counts the host's address.
fn reading(reply: &[u8], vcp: u8) -> io::Result<Reading> {
    let [
        0x6E,
        0x88,
        0x02,
        result,
        code,
        _kind,
        max_high,
        max_low,
        now_high,
        now_low,
        sum,
    ] = *reply
    else {
        return Err(refused(format!("it answered {reply:02x?}")));
    };
    if reply[..10].iter().fold(0x50, |sum, byte| sum ^ byte) != sum {
        return Err(refused(format!(
            "its answer's checksum is wrong: {reply:02x?}"
        )));
    }
    if result != 0 {
        return Err(refused(format!("it has no VCP code {vcp:#04x}")));
    }
    if code != vcp {
        return Err(refused(format!(
            "it answered for VCP code {code:#04x}, not {vcp:#04x}"
        )));
    }
    let max = u16::from_be_bytes([max_high, max_low]);
    if max == 0 {
        return Err(refused("its brightest is 0".into()));
    }
    Ok(Reading {
        max,
        now: u16::from_be_bytes([now_high, now_low]),
    })
}

/// The value nearest a share of a monitor's brightest.
fn value_for(level: f64, max: u16) -> u16 {
    // In range: a share of the brightest is between 0 and max.
    (level.clamp(0.0, 1.0) * f64::from(max)).round() as u16
}

/// A monitor's bus, opened to talk to it.
struct Bus {
    file: File,
}

impl Bus {
    fn open(path: &Path) -> io::Result<Bus> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        // SAFETY: the descriptor is the file's, open for the whole call, and
        // I2C_SLAVE takes the address as a plain number.
        if unsafe { ioctl(file.as_raw_fd(), I2C_SLAVE, c_ulong::from(DDC_CI)) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Bus { file })
    }

    fn send(&mut self, message: &[u8]) -> io::Result<()> {
        let sent = self.file.write(message)?;
        if sent != message.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("only {sent} of {} bytes went", message.len()),
            ));
        }
        Ok(())
    }

    fn read_vcp(&mut self, vcp: u8) -> io::Result<Reading> {
        self.send(&get_vcp(vcp))?;
        thread::sleep(ANSWERING);
        let mut reply = [0; 11];
        let read = self.file.read(&mut reply);
        thread::sleep(BETWEEN);
        reading(&reply[..read?], vcp)
    }

    fn write_vcp(&mut self, vcp: u8, value: u16) -> io::Result<()> {
        let sent = self.send(&set_vcp(vcp, value));
        thread::sleep(BETWEEN);
        sent
    }
}

/// What a monitor's thread is asked for: the latest level, and whether its
/// bus has gone.
#[derive(Default)]
struct Wanted {
    level: Option<f64>,
    gone: bool,
}

impl Wanted {
    /// Asks for a level, in place of any not yet taken.
    fn want(&mut self, level: f64) {
        self.level = Some(level);
    }

    /// The level asked for since the last one taken, if any.
    fn take(&mut self) -> Option<f64> {
        self.level.take()
    }
}

/// What the daemon and a monitor's thread share.
struct Shared {
    wanted: Mutex<Wanted>,
    woken: Condvar,
}

impl Shared {
    fn wanted(&self) -> MutexGuard<'_, Wanted> {
        // A thread that panicked holding it left a level, which is still one.
        self.wanted.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits for a level to be asked for, or none once the bus has gone.
    fn next(&self) -> Option<f64> {
        let mut wanted = self.wanted();
        loop {
            if wanted.gone {
                return None;
            }
            if let Some(level) = wanted.take() {
                return Some(level);
            }
            wanted = self
                .woken
                .wait(wanted)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// A monitor's thread, which ends once this is dropped.
struct Monitor {
    shared: Arc<Shared>,
}

impl Monitor {
    fn start(name: &str, bus: Bus, level: f64) -> io::Result<Monitor> {
        let shared = Arc::new(Shared {
            wanted: Mutex::new(Wanted {
                level: Some(level),
                gone: false,
            }),
            woken: Condvar::new(),
        });
        let owned = name.to_owned();
        let theirs = Arc::clone(&shared);
        thread::Builder::new()
            .name(format!("monitor on {name}"))
            .spawn(move || follow(&owned, bus, &theirs))?;
        Ok(Monitor { shared })
    }

    fn want(&self, level: f64) {
        self.shared.wanted().want(level);
        self.shared.woken.notify_one();
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.shared.wanted().gone = true;
        self.shared.woken.notify_one();
    }
}

/// A monitor's thread: learns its brightest, then sets each level asked
/// for, unless it already shows it.
fn follow(name: &str, mut bus: Bus, shared: &Shared) {
    let found = bus
        .read_vcp(BRIGHTNESS)
        .or_else(|_| bus.read_vcp(BRIGHTNESS));
    let Reading { max, now } = match found {
        Ok(found) => found,
        Err(_) if shared.wanted().gone => return,
        Err(error) => {
            eprintln!(
                "<4>nothing on {name} answers for its brightness, so it's left alone: {error}"
            );
            return;
        }
    };
    eprintln!("<6>the monitor on {name} follows the panel, {now} of {max}");
    let mut shown = now;
    let mut failing = false;
    while let Some(level) = shared.next() {
        let value = value_for(level, max);
        if value == shown {
            continue;
        }
        match bus.write_vcp(BRIGHTNESS, value) {
            Ok(()) => {
                shown = value;
                failing = false;
            }
            // Once, rather than for every step of a move.
            Err(error) if !failing => {
                eprintln!("<4>can't set the monitor on {name}: {error}");
                failing = true;
            }
            Err(_) => {}
        }
    }
    eprintln!("<6>the monitor on {name} has gone");
}

/// Whether a DRM connector is a DisplayPort one, not the panel's eDP.
fn is_displayport(connector: &str) -> bool {
    connector
        .strip_prefix("card")
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(_, kind)| kind.starts_with("DP-"))
}

/// Whether a bus in /sys/class/i2c-dev reaches a monitor: one an MST hub
/// made for a monitor behind it, or a DisplayPort connector's own.
fn reaches_a_monitor(bus: &Path) -> bool {
    let name = fs::read_to_string(bus.join("name")).unwrap_or_default();
    if name.trim() == "DPMST" {
        return true;
    }
    fs::canonicalize(bus.join("device")).is_ok_and(|device| {
        device
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|connector| is_displayport(&connector.to_string_lossy()))
    })
}

/// The buses that reach a monitor, by name, and have a node in `dev`.
fn buses(class: &Path, dev: &Path) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(class) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // The node goes before the class's entry does, so a bus going is
        // known from the node.
        if name.starts_with("i2c-") && dev.join(&name).exists() && reaches_a_monitor(&entry.path())
        {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

/// Whether inotify's events name a bus, or lost some that might have.
fn names_a_bus(events: &[u8]) -> bool {
    // Each is a descriptor, a mask, a cookie, a length, then a name that
    // long, padded with NULs.
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let mask = word(4);
        let length = word(12) as usize;
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        if mask & IN_Q_OVERFLOW != 0 || name.starts_with(b"i2c-") {
            return true;
        }
        rest = after;
    }
    false
}

/// A watch on `dev` that wakes when a node is made, removed, or opened to
/// someone.
fn watch(dev: &Path) -> io::Result<AsyncFd<File>> {
    // SAFETY: it takes only flags.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just made, and nothing else holds it.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let path = CString::new(dev.as_os_str().as_bytes())?;
    // SAFETY: the descriptor is the file's, and the path a C string that
    // outlives the call.
    let mask = IN_CREATE | IN_DELETE | IN_ATTRIB;
    if unsafe { inotify_add_watch(file.as_raw_fd(), path.as_ptr(), mask) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the File is moved into the AsyncFd, which owns it until it's
    // dropped, and a File keeps one descriptor open for its life.
    unsafe { AsyncFd::register_with_interest(file, Interest::READABLE) }.map_err(io::Error::from)
}

/// The monitors there are, each following the panel's level.
pub struct Monitors {
    class: PathBuf,
    dev: PathBuf,
    /// The panel's level, as a share of its brightest.
    level: f64,
    /// Each bus found, with its monitor's thread, or none while it can't be
    /// opened, as before udev has opened it to the seat.
    buses: HashMap<String, Option<Monitor>>,
    watch: Option<AsyncFd<File>>,
}

impl Monitors {
    /// The monitors there are now, set to the panel's level.
    pub fn find(level: f64) -> Monitors {
        let dev = PathBuf::from("/dev");
        // Watched before they're looked for, so one that comes between isn't
        // missed.
        let watch = watch(&dev)
            .inspect_err(|error| eprintln!("<4>can't hear monitors come and go: {error}"))
            .ok();
        let mut monitors = Monitors {
            class: PathBuf::from("/sys/class/i2c-dev"),
            dev,
            level,
            buses: HashMap::new(),
            watch,
        };
        monitors.look();
        monitors
    }

    /// The panel's level is now this.
    pub fn follow(&mut self, level: f64) {
        if level.to_bits() == self.level.to_bits() {
            return;
        }
        self.level = level;
        for monitor in self.buses.values().flatten() {
            monitor.want(level);
        }
    }

    /// Waits for a bus to come, go or be opened to the seat, or forever when
    /// that can't be heard.
    pub async fn changed(&mut self) -> io::Result<()> {
        let Some(watch) = &self.watch else {
            return std::future::pending().await;
        };
        let mut events = [0; 4096];
        loop {
            let mut ready = watch.readable().await?;
            let mut heard = false;
            loop {
                let mut file = ready.get_inner();
                match file.read(&mut events) {
                    Ok(read) => heard |= names_a_bus(&events[..read]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        ready.clear_ready();
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            if heard {
                return Ok(());
            }
        }
    }

    /// Looks for the monitors again: a new one's thread starts, at the
    /// panel's level, and a gone one's ends.
    pub fn look(&mut self) {
        let found = match buses(&self.class, &self.dev) {
            Ok(found) => found,
            Err(error) => {
                eprintln!("<4>can't look for monitors: {error}");
                return;
            }
        };
        self.buses.retain(|name, _| found.contains(name));
        for name in found {
            let tried = match self.buses.get(&name) {
                Some(Some(_)) => continue,
                Some(None) => true,
                None => false,
            };
            let monitor = Bus::open(&self.dev.join(&name))
                .and_then(|bus| Monitor::start(&name, bus, self.level));
            match monitor {
                Ok(monitor) => {
                    self.buses.insert(name, Some(monitor));
                }
                // udev opens a bus to the seat a moment after it comes, and
                // the watch hears it then.
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    if !tried {
                        eprintln!("<5>{name} isn't open to the seat yet");
                    }
                    self.buses.insert(name, None);
                }
                Err(error) => {
                    if !tried {
                        eprintln!("<4>can't open {name}: {error}");
                    }
                    self.buses.insert(name, None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn asking_for_a_vcp_code_is_framed_and_summed() {
        assert_eq!(get_vcp(0x10), [0x51, 0x82, 0x01, 0x10, 0xAC]);
    }

    #[test]
    fn setting_a_vcp_code_is_framed_and_summed() {
        let sent = set_vcp(0x10, 0x0140);
        assert_eq!(sent[..6], [0x51, 0x84, 0x03, 0x10, 0x01, 0x40]);
        let sum = [0x6E, 0x51, 0x84, 0x03, 0x10, 0x01, 0x40]
            .iter()
            .fold(0, |sum, byte| sum ^ byte);
        assert_eq!(sent[6], sum);
        assert_eq!(sent.len(), 7);
    }

    /// A monitor's answer, summed as one sends it.
    fn answer(result: u8, vcp: u8, max: u16, now: u16) -> Vec<u8> {
        let [max_high, max_low] = max.to_be_bytes();
        let [now_high, now_low] = now.to_be_bytes();
        let mut reply = vec![
            0x6E, 0x88, 0x02, result, vcp, 0x00, max_high, max_low, now_high, now_low,
        ];
        reply.push(reply.iter().fold(0x50, |sum, byte| sum ^ byte));
        reply
    }

    #[test]
    fn a_good_answer_gives_the_brightest_and_the_brightness_now() {
        assert_eq!(
            reading(&answer(0, 0x10, 100, 40), 0x10).unwrap(),
            Reading { max: 100, now: 40 }
        );
    }

    #[test]
    fn an_answer_with_a_wrong_checksum_is_refused() {
        let mut reply = answer(0, 0x10, 100, 40);
        reply[10] ^= 1;
        assert!(reading(&reply, 0x10).is_err());
    }

    #[test]
    fn an_answer_for_another_vcp_code_is_refused() {
        assert!(reading(&answer(0, 0x12, 100, 40), 0x10).is_err());
    }

    #[test]
    fn an_answer_that_says_no_such_code_is_refused() {
        assert!(reading(&answer(1, 0x10, 100, 40), 0x10).is_err());
    }

    #[test]
    fn a_short_answer_is_refused() {
        assert!(reading(&[0x6E, 0x80, 0xBE], 0x10).is_err());
    }

    #[test]
    fn the_panel_s_share_becomes_the_monitor_s_nearest_value() {
        assert_eq!(value_for(0.0, 100), 0);
        assert_eq!(value_for(1.0, 100), 100);
        assert_eq!(value_for(1.0 / 3.0, 100), 33);
        assert_eq!(value_for(2.0 / 3.0, 100), 67);
        assert_eq!(value_for(0.5, 101), 51);
        assert_eq!(value_for(2.0, 100), 100);
    }

    #[test]
    fn only_the_latest_level_asked_for_is_taken() {
        let mut wanted = Wanted::default();
        assert_eq!(wanted.take(), None);
        for level in [0.1, 0.2, 0.3] {
            wanted.want(level);
        }
        assert_eq!(wanted.take(), Some(0.3));
        assert_eq!(wanted.take(), None);
    }

    #[test]
    fn a_gone_monitor_s_thread_is_asked_for_nothing_more() {
        let shared = Shared {
            wanted: Mutex::new(Wanted {
                level: Some(0.5),
                gone: true,
            }),
            woken: Condvar::new(),
        };
        assert_eq!(shared.next(), None);
    }

    /// A stand-in sysfs and /dev, gone when the test is.
    struct Tree(PathBuf);

    impl Tree {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("backlight-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for dir in ["class", "dev", "devices"] {
                fs::create_dir_all(root.join(dir)).unwrap();
            }
            Tree(root)
        }

        /// A bus of this name, whose device sits at `device` under devices.
        fn add(&self, bus: &str, name: &str, device: &str) {
            let device = self.0.join("devices").join(device);
            fs::create_dir_all(&device).unwrap();
            let dir = self.0.join("class").join(bus);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("name"), format!("{name}\n")).unwrap();
            symlink(&device, dir.join("device")).unwrap();
            fs::write(self.0.join("dev").join(bus), "").unwrap();
        }

        fn buses(&self) -> Vec<String> {
            buses(&self.0.join("class"), &self.0.join("dev")).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_bus_behind_an_mst_hub_reaches_a_monitor() {
        let tree = Tree::new("mst");
        tree.add("i2c-13", "DPMST", "i2c-13");
        assert_eq!(tree.buses(), ["i2c-13"]);
    }

    #[test]
    fn a_displayport_connector_s_bus_reaches_a_monitor_and_the_panel_s_does_not() {
        let tree = Tree::new("connectors");
        tree.add(
            "i2c-7",
            "AUX USBC1/DDI TC1/PHY TC1",
            "card1/card1-DP-2/i2c-7",
        );
        tree.add("i2c-3", "AUX A/DDI A/PHY A", "card1/card1-eDP-1/i2c-3");
        tree.add("i2c-5", "i915 gmbus dpb", "card1/card1-HDMI-A-1/i2c-5");
        assert_eq!(tree.buses(), ["i2c-7"]);
    }

    #[test]
    fn the_machine_s_other_buses_are_left_alone() {
        let tree = Tree::new("others");
        tree.add(
            "i2c-0",
            "SMBus I801 adapter at efa0",
            "pci0000:00/0000:00:1f.4/i2c-0",
        );
        tree.add(
            "i2c-1",
            "Synopsys DesignWare I2C adapter",
            "pci0000:00/i2c-1",
        );
        assert!(tree.buses().is_empty());
    }

    #[test]
    fn a_bus_whose_node_has_gone_is_gone() {
        let tree = Tree::new("gone");
        tree.add("i2c-13", "DPMST", "i2c-13");
        tree.add("i2c-14", "DPMST", "i2c-14");
        fs::remove_file(tree.0.join("dev/i2c-14")).unwrap();
        assert_eq!(tree.buses(), ["i2c-13"]);
    }

    #[test]
    fn no_class_of_buses_is_none_found() {
        let tree = Tree::new("no-class");
        assert!(
            buses(&tree.0.join("missing"), &tree.0.join("dev"))
                .unwrap()
                .is_empty()
        );
    }

    /// An inotify event for a name, padded as the kernel pads it.
    fn event(mask: u32, name: &str) -> Vec<u8> {
        let mut padded = name.as_bytes().to_vec();
        if !name.is_empty() {
            padded.resize((name.len() / 16 + 1) * 16, 0);
        }
        let mut event = Vec::new();
        event.extend_from_slice(&1i32.to_ne_bytes());
        event.extend_from_slice(&mask.to_ne_bytes());
        event.extend_from_slice(&0u32.to_ne_bytes());
        event.extend_from_slice(&(padded.len() as u32).to_ne_bytes());
        event.extend_from_slice(&padded);
        event
    }

    #[test]
    fn a_bus_coming_is_heard_among_other_nodes() {
        let mut events = event(IN_CREATE, "tty7");
        events.extend(event(IN_CREATE, "i2c-13"));
        assert!(names_a_bus(&events));
    }

    #[test]
    fn other_nodes_coming_and_going_are_not_a_bus() {
        let mut events = event(IN_CREATE, "hidraw3");
        events.extend(event(IN_DELETE, "input/event9"));
        assert!(!names_a_bus(&events));
    }

    #[test]
    fn lost_events_might_have_named_a_bus() {
        assert!(names_a_bus(&event(IN_Q_OVERFLOW, "")));
    }
}
