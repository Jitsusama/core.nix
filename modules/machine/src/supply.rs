//! Each power supply the kernel knows (`supply:<name>`): the battery, the
//! charger and each USB-C port, with every figure its folder in
//! `/sys/class/power_supply` has, a USB-C port's roles from its folder in
//! `/sys/class/typec`, and what `UPower` works out of the battery, its time
//! to empty or full, as main.rs last asked it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::{Root, heat, rounded};

const SUPPLIES: &str = "/sys/class/power_supply";

/// How long what a battery's charging is set to is kept before its files
/// are read again. On a Dell each of the three is a call into the
/// firmware, 10 to 23 ms and a 64 KiB buffer the kernel finds whole, and
/// one that found none failed the read (2026-11-12, 22:56), though only a
/// change Joel makes moves them. When the battery was made and its
/// health are firmware calls too, about 2.7 ms each, and never or seldom
/// move (2026-11-13).
pub const SETTINGS_KEPT: Duration = Duration::from_secs(60);

/// How soon a battery's setting is read again when its file is there but
/// answered nothing, as when the kernel found no buffer for the call.
/// Only once: a file that answers nothing twice waits `SETTINGS_KEPT`, so
/// one that never answers isn't asked every few seconds.
pub const SETTINGS_AGAIN: Duration = Duration::from_secs(5);

/// What each battery's charging is set to, its charge types and its
/// limit's ends, as last read, by its folder, with when it was made and
/// its health, so a supply read each second asks the firmware once a
/// minute. A limit set here is read again at once (`forget`). A file
/// that's there but answered nothing keeps what it said last, and is read
/// again `SETTINGS_AGAIN` on, once. Each battery's temperature that was
/// slow to read, and when, is kept here too.
pub struct Settings {
    read: HashMap<String, Read>,
    temperatures: HashMap<String, (Instant, Option<u64>)>,
    temperature_slow_from: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            read: HashMap::new(),
            temperatures: HashMap::new(),
            temperature_slow_from: heat::SLOW_FILE,
        }
    }
}

/// A battery's settings as last read, when, and how many reads running
/// one of its files answered nothing.
struct Read {
    at: Instant,
    set: Set,
    missed: u32,
}

#[derive(Clone, Default)]
struct Set {
    types: Option<String>,
    start: Option<u64>,
    stop: Option<u64>,
    made: Option<String>,
    health: Option<String>,
}

impl Settings {
    /// What the battery in `folder` is set to, read again once it's been
    /// kept `SETTINGS_KEPT` by `now`.
    fn of(&mut self, root: &Root, folder: &str, now: Instant) -> Set {
        let last = self.read.get(folder);
        if let Some(read) = last {
            let kept = if read.missed == 1 {
                SETTINGS_AGAIN
            } else {
                SETTINGS_KEPT
            };
            if now.saturating_duration_since(read.at) < kept {
                return read.set.clone();
            }
        }
        let was = last.map(|read| read.set.clone()).unwrap_or_default();
        let mut missed = false;
        let mut or_last = |file: &str, said: Option<String>, was: Option<String>| {
            if said.is_none() && root.path(&format!("{folder}/{file}")).exists() {
                missed = true;
                return was;
            }
            said
        };
        let types = or_last(
            "charge_types",
            root.said(&format!("{folder}/charge_types")),
            was.types,
        );
        let figure = |file: &str| root.said(&format!("{folder}/{file}"));
        let start = or_last(
            START,
            figure(START),
            was.start.as_ref().map(ToString::to_string),
        );
        let stop = or_last(
            STOP,
            figure(STOP),
            was.stop.as_ref().map(ToString::to_string),
        );
        let made = or_last("manufacture_year", made(root, folder), was.made);
        let health = or_last("health", figure("health"), was.health);
        let set = Set {
            types,
            start: start.and_then(|n| n.parse().ok()),
            stop: stop.and_then(|n| n.parse().ok()),
            made,
            health,
        };
        let missed = match last {
            Some(read) if missed => read.missed + 1,
            None if missed => 1,
            _ => 0,
        };
        self.read.insert(
            folder.to_owned(),
            Read {
                at: now,
                set: set.clone(),
                missed,
            },
        );
        set
    }

    /// The temperature of the battery in `folder`, in tenths of a degree:
    /// as last read while that's within `SLOW_KEPT` of `now` where a read
    /// of it was slow, as a slow zone's is, and read again otherwise.
    fn temperature(&mut self, root: &Root, folder: &str, now: Instant) -> Option<u64> {
        if let Some((at, said)) = self.temperatures.get(folder)
            && now.saturating_duration_since(*at) < heat::SLOW_KEPT
        {
            return *said;
        }
        let began = Instant::now();
        let said = root.number(&format!("{folder}/temp"));
        if began.elapsed() >= self.temperature_slow_from || self.temperatures.contains_key(folder) {
            self.temperatures.insert(folder.to_owned(), (now, said));
        }
        said
    }

    /// Read the battery `name` again at its next read, as once its limit
    /// is set.
    pub fn forget(&mut self, name: &str) {
        self.read.remove(&format!("{SUPPLIES}/{name}"));
    }
}

/// When the battery in `folder` was made, as its firmware says the day.
fn made(root: &Root, folder: &str) -> Option<String> {
    let whole = |file: &str| root.number(&format!("{folder}/manufacture_{file}"));
    let (year, month, day) = (whole("year")?, whole("month")?, whole("day")?);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// The choice a kernel list marks, as `[sink]` in `source [sink]`, and
/// every choice it lists. A file saying one thing alone, as an older
/// kernel's does, has that one in use.
pub fn chosen(text: Option<String>) -> (Option<String>, Vec<String>) {
    let Some(text) = text else {
        return (None, vec![]);
    };
    let mut chosen = None;
    let all = text
        .split_whitespace()
        .map(|word| {
            let plain = word.trim_start_matches('[').trim_end_matches(']');
            if word.starts_with('[') {
                chosen = Some(plain.to_owned());
            }
            plain.to_owned()
        })
        .collect::<Vec<String>>();
    if chosen.is_none() && all.len() == 1 {
        chosen = all.first().cloned();
    }
    (chosen, all)
}

/// A figure the kernel gives in millionths, as volts, amps, watts, amp
/// hours and watt hours, in whole units.
fn millionths(root: &Root, folder: &str, file: &str) -> Option<f64> {
    let value: i64 = root.said(&format!("{folder}/{file}"))?.parse().ok()?;
    i32::try_from(value / 1000)
        .ok()
        .map(|thousandths| f64::from(thousandths) / 1000.0)
}

/// Whether Joel's account may write a file, as the charge limit's.
fn writable(root: &Root, folder: &str, file: &str) -> Option<bool> {
    let path = root.path(&format!("{folder}/{file}"));
    if !path.exists() {
        return None;
    }
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    // SAFETY: access reads the path it's given, a string ending in nul.
    Some(unsafe { libc::access(path.as_ptr(), libc::W_OK) } == 0)
}

/// Whether Joel's account may set the charge limit: its stop, and its
/// start where the battery has one, since the shell writes both. None
/// for a battery with no limit.
fn limit_settable(root: &Root, folder: &str) -> Option<bool> {
    let stop = writable(root, folder, "charge_control_end_threshold")?;
    Some(stop && writable(root, folder, "charge_control_start_threshold").unwrap_or(true))
}

/// A figure the kernel gives in thousandths with its unit, as "5000mV"
/// or "3000mA", in whole units.
fn thousandths(root: &Root, path: &str) -> Option<Value> {
    let said = root.said(path)?;
    let digits: String = said.chars().take_while(char::is_ascii_digit).collect();
    let value: u32 = digits.parse().ok()?;
    Some(rounded(f64::from(value) / 1000.0, 3))
}

/// What a charger plugged into a port offers, as its power delivery
/// folder lists them, in the order the kernel numbers them: a fixed
/// supply's volts and amps, or a programmable one's from its least to its
/// most volts (`to_volts`, null for a fixed one). A battery's or a
/// variable supply's offer is said as fixed, by its most volts.
fn offers(root: &Root, folder: &str) -> Vec<Value> {
    let mut named: Vec<(u32, String)> = root
        .names(folder)
        .into_iter()
        .filter_map(|name| Some((name.split(':').next()?.parse().ok()?, name)))
        .collect();
    named.sort();
    named
        .iter()
        .filter_map(|(_, name)| {
            let at = |file: &str| thousandths(root, &format!("{folder}/{name}/{file}"));
            if name.ends_with(":programmable_supply") {
                return Some(json!({
                    "volts": at("minimum_voltage")?,
                    "to_volts": at("maximum_voltage"),
                    "amps": at("maximum_current"),
                }));
            }
            Some(json!({
                "volts": at("voltage").or_else(|| at("maximum_voltage"))?,
                "to_volts": null,
                "amps": at("maximum_current"),
            }))
        })
        .collect()
}

/// A USB-C port's roles, by its supply's number (`USBC000:001` is the
/// kernel's typec port0), and what's plugged into it.
fn port(root: &Root, name: &str) -> Value {
    let Some(n) = name
        .rsplit(':')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| *n > 0)
    else {
        return Value::Null;
    };
    let folder = format!("/sys/class/typec/port{}", n - 1);
    if !root.path(&folder).exists() {
        return Value::Null;
    }
    let at = |file: &str| root.said(&format!("{folder}/{file}"));
    let partner = format!("{folder}-partner");
    let plugged = root.path(&partner).exists().then(|| {
        let at = |file: &str| root.said(&format!("{partner}/{file}"));
        let offered = offers(
            root,
            &format!("{partner}/usb_power_delivery/source-capabilities"),
        );
        json!({
            "power_delivery": at("supports_usb_power_delivery").map(|yes| yes == "yes"),
            "power_delivery_revision": at("usb_power_delivery_revision").filter(|r| r != "0.0"),
            "accessory": at("accessory_mode").filter(|mode| mode != "none"),
            "source_capabilities": offered,
        })
    });
    json!({
        "port": n - 1,
        "power_role": chosen(at("power_role")).0,
        "data_role": chosen(at("data_role")).0,
        "operation_mode": at("power_operation_mode"),
        "power_delivery_revision": at("usb_power_delivery_revision").filter(|r| r != "0.0"),
        "typec_revision": at("usb_typec_revision"),
        "partner": plugged,
    })
}

/// One supply's record; `upower` what `UPower` last said of it, or null;
/// what it's set to as `settings` keeps it at `now`.
pub fn record(
    root: &Root,
    name: &str,
    upower: &Value,
    settings: &mut Settings,
    now: Instant,
) -> Value {
    let folder = format!("{SUPPLIES}/{name}");
    let set = settings.of(root, &folder, now);
    let at = |file: &str| root.said(&format!("{folder}/{file}"));
    let whole = |file: &str| root.number(&format!("{folder}/{file}"));
    let units = |file: &str| millionths(root, &folder, file).map(|v| rounded(v, 3));
    let volts = millionths(root, &folder, "voltage_now");
    let amps = millionths(root, &folder, "current_now");
    let watts = millionths(root, &folder, "power_now")
        .or_else(|| Some((volts? * amps?).abs()))
        .map(|w| rounded(w, 3));
    // Health against design, from charge where the battery counts it so,
    // else from energy.
    let health = ["charge", "energy"].iter().find_map(|unit| {
        let full = whole(&format!("{unit}_full"))?;
        let design = whole(&format!("{unit}_full_design")).filter(|d| *d > 0)?;
        Some(rounded(crate::float(full) / crate::float(design), 4))
    });
    let (charge_type, charge_types) = chosen(set.types);
    let (usb_type, usb_types) = chosen(at("usb_type"));
    // The kernel counts a battery's temperature in tenths of a degree.
    let temperature = settings
        .temperature(root, &folder, now)
        .and_then(|t| u32::try_from(t).ok())
        .map(|t| rounded(f64::from(t) / 10.0, 1));
    json!({
        "key": format!("supply:{name}"),
        "kind": "supply",
        "name": name,
        "type": at("type"),
        "online": whole("online").map(|o| o == 1),
        "present": whole("present").map(|p| p == 1),
        "status": at("status"),
        "scope": at("scope"),
        "capacity_pct": whole("capacity"),
        "capacity_level": at("capacity_level"),
        "charge_ah": units("charge_now"),
        "charge_full_ah": units("charge_full"),
        "charge_design_ah": units("charge_full_design"),
        "energy_wh": units("energy_now"),
        "energy_full_wh": units("energy_full"),
        "energy_design_wh": units("energy_full_design"),
        "health_ratio": health,
        "volts": volts.map(|v| rounded(v, 3)),
        "amps": amps.map(|a| rounded(a, 3)),
        "watts": watts,
        "min_design_volts": units("voltage_min_design"),
        "max_volts": units("voltage_max"),
        "min_volts": units("voltage_min"),
        "max_amps": units("current_max"),
        "cycles": whole("cycle_count"),
        "health": set.health,
        "technology": at("technology"),
        "charge_type": charge_type.or_else(|| at("charge_type")),
        "charge_types": charge_types,
        "limit_start_pct": set.start,
        "limit_stop_pct": set.stop,
        "limit_settable": limit_settable(root, &folder),
        "alarm": whole("alarm"),
        "manufacturer": at("manufacturer"),
        "model": at("model_name"),
        "serial": at("serial_number"),
        "made": set.made,
        "temperature_c": temperature,
        "usb_type": usb_type,
        "usb_types": usb_types,
        "typec": port(root, name),
        "upower": upower,
    })
}

/// The supplies the kernel lists, by name.
pub fn supplies(root: &Root) -> Vec<String> {
    root.names(SUPPLIES)
}

const STOP: &str = "charge_control_end_threshold";
const START: &str = "charge_control_start_threshold";

/// How far below its stop a battery with a start begins charging again,
/// as its firmware's own limit is set.
pub const RESTART_PCT: u32 = 5;

/// The files written to move a battery's limit to stop at `wanted`, each
/// with what it's given, in the order the kernel takes them: it refuses
/// a start at or above the stop, so a limit raised has its stop written
/// first, and one lowered its start. A battery with no start has only
/// its stop.
pub fn limit_writes(
    start: Option<u32>,
    stop: Option<u32>,
    wanted: u32,
) -> Vec<(&'static str, u32)> {
    let end = (STOP, wanted);
    if start.is_none() {
        return vec![end];
    }
    let begin = (START, wanted.saturating_sub(RESTART_PCT));
    if stop.is_none_or(|stop| wanted >= stop) {
        vec![end, begin]
    } else {
        vec![begin, end]
    }
}

/// Why a limit wasn't set: what was asked is no limit this machine has
/// (`bad-act`), or the kernel refused a write (`refused`), with what to
/// say of it.
#[derive(Debug, PartialEq, Eq)]
pub struct Unset {
    pub code: &'static str,
    pub message: String,
}

fn unset(code: &'static str, message: impl Into<String>) -> Unset {
    Unset {
        code,
        message: message.into(),
    }
}

/// Moves the limit of the supply named to stop charging at `wanted` per
/// cent, and start again `RESTART_PCT` below it where it can, as Power's
/// Charge Limit asks. A name that could reach outside the kernel's list
/// of supplies is refused before anything is written.
pub fn set_limit(root: &Root, name: &str, wanted: u32) -> Result<(), Unset> {
    let plain = !name.is_empty() && !name.starts_with('.') && !name.contains('/');
    let folder = format!("{SUPPLIES}/{name}");
    if !plain || !root.path(&format!("{folder}/{STOP}")).exists() {
        return Err(unset(
            "bad-act",
            format!("no supply {name:?} with a charge limit"),
        ));
    }
    if !(1..=100).contains(&wanted) {
        return Err(unset("bad-act", format!("no stop at {wanted} per cent")));
    }
    let figure = |file: &str| {
        root.said(&format!("{folder}/{file}"))
            .and_then(|said| said.parse().ok())
    };
    let start = root
        .path(&format!("{folder}/{START}"))
        .exists()
        .then(|| figure(START).unwrap_or(0));
    for (file, value) in limit_writes(start, figure(STOP), wanted) {
        std::fs::write(root.path(&format!("{folder}/{file}")), format!("{value}\n"))
            .map_err(|why| unset("refused", format!("{file}: {why}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_limit_is_settable_only_with_both_ends() {
        let root = std::env::temp_dir().join(format!("machine-limit-{}", std::process::id()));
        let folder = "/sys/class/power_supply/BAT0";
        let at = root.join(folder.trim_start_matches('/'));
        std::fs::create_dir_all(&at).unwrap();
        let held = Root::new(&root);
        assert_eq!(limit_settable(&held, folder), None, "no limit");
        std::fs::write(at.join("charge_control_end_threshold"), "80\n").unwrap();
        assert_eq!(limit_settable(&held, folder), Some(true), "a stop alone");
        let start = at.join("charge_control_start_threshold");
        std::fs::write(&start, "75\n").unwrap();
        assert_eq!(limit_settable(&held, folder), Some(true), "both ends");
        std::fs::set_permissions(&start, std::fs::Permissions::from_mode(0o444)).unwrap();
        let settable = limit_settable(&held, folder);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(settable, Some(false), "a start it can't write");
    }

    #[test]
    fn a_limit_is_written_in_the_order_the_kernel_takes_it() {
        let raised = limit_writes(Some(55), Some(60), 80);
        assert_eq!(raised, [(STOP, 80), (START, 75)], "its stop first");
        let lowered = limit_writes(Some(75), Some(80), 60);
        assert_eq!(lowered, [(START, 55), (STOP, 60)], "its start first");
        assert_eq!(limit_writes(None, Some(80), 60), [(STOP, 60)], "no start");
        assert_eq!(limit_writes(Some(0), None, 60), [(STOP, 60), (START, 55)]);
    }

    #[test]
    fn a_limit_set_writes_both_ends_and_refuses_a_name_outside() {
        let root = std::env::temp_dir().join(format!("machine-set-{}", std::process::id()));
        let at = root.join("sys/class/power_supply/BAT0");
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join(START), "75\n").unwrap();
        std::fs::write(at.join(STOP), "80\n").unwrap();
        let held = Root::new(&root);
        let set = set_limit(&held, "BAT0", 60);
        let said = |file: &str| std::fs::read_to_string(at.join(file)).unwrap();
        let ends = (said(START), said(STOP));
        let outside = set_limit(&held, "../BAT0", 60).map_err(|unset| unset.code);
        let over = set_limit(&held, "BAT0", 101).map_err(|unset| unset.code);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(set, Ok(()));
        assert_eq!(ends, ("55\n".into(), "60\n".into()));
        assert_eq!((outside, over), (Err("bad-act"), Err("bad-act")));
    }

    #[test]
    fn what_charging_is_set_to_is_read_once_a_minute_and_again_once_set() {
        let root = std::env::temp_dir().join(format!("machine-kept-{}", std::process::id()));
        let at = root.join("sys/class/power_supply/BAT0");
        std::fs::create_dir_all(&at).unwrap();
        let write = |types: &str, start: &str, stop: &str| {
            std::fs::write(at.join("charge_types"), format!("{types}\n")).unwrap();
            std::fs::write(at.join(START), format!("{start}\n")).unwrap();
            std::fs::write(at.join(STOP), format!("{stop}\n")).unwrap();
        };
        write("Fast [Standard]", "75", "80");
        let held = Root::new(&root);
        let mut settings = Settings::default();
        let first = Instant::now();
        let mut said = |at: Instant| {
            let kept = record(&held, "BAT0", &Value::Null, &mut settings, at);
            let figure = |name: &str| kept[name].clone();
            (
                figure("charge_type"),
                figure("limit_start_pct"),
                figure("limit_stop_pct"),
            )
        };
        let read = said(first);
        write("[Fast] Standard", "55", "60");
        let soon = said(first + Duration::from_secs(59));
        let later = said(first + SETTINGS_KEPT);
        write("Fast [Standard]", "75", "80");
        settings.forget("BAT0");
        let set = record(
            &held,
            "BAT0",
            &Value::Null,
            &mut settings,
            first + SETTINGS_KEPT,
        );
        std::fs::remove_dir_all(&root).unwrap();
        let was = (json!("Standard"), json!(75), json!(80));
        assert_eq!(read, was, "what the battery's files said");
        assert_eq!(
            soon, was,
            "read again within a minute, as firmware is asked each time"
        );
        let moved = (json!("Fast"), json!(55), json!(60));
        assert_eq!(later, moved, "kept past a minute");
        assert_eq!(
            set["limit_stop_pct"],
            json!(80),
            "kept past its limit being set"
        );
    }

    #[test]
    fn when_a_battery_was_made_and_its_health_are_read_once_a_minute() {
        let root = std::env::temp_dir().join(format!("machine-made-{}", std::process::id()));
        let at = root.join("sys/class/power_supply/BAT0");
        std::fs::create_dir_all(&at).unwrap();
        let write = |day: &str, health: &str| {
            std::fs::write(at.join("manufacture_year"), "2025\n").unwrap();
            std::fs::write(at.join("manufacture_month"), "3\n").unwrap();
            std::fs::write(at.join("manufacture_day"), format!("{day}\n")).unwrap();
            std::fs::write(at.join("health"), format!("{health}\n")).unwrap();
        };
        write("7", "Good");
        let held = Root::new(&root);
        let mut settings = Settings::default();
        let first = Instant::now();
        let mut said = |at: Instant| {
            let kept = record(&held, "BAT0", &Value::Null, &mut settings, at);
            (kept["made"].clone(), kept["health"].clone())
        };
        let read = said(first);
        write("8", "Overheat");
        let soon = said(first + Duration::from_secs(59));
        let later = said(first + SETTINGS_KEPT);
        std::fs::remove_dir_all(&root).unwrap();
        let was = (json!("2025-03-07"), json!("Good"));
        assert_eq!(read, was, "what the battery's files said");
        assert_eq!(
            soon, was,
            "read again within a minute, as firmware is asked each time"
        );
        let moved = (json!("2025-03-08"), json!("Overheat"));
        assert_eq!(later, moved, "kept past a minute");
    }

    /// A battery's temperature slow to read is read again only once kept
    /// `SLOW_KEPT`, as a slow zone's is: on Joel's laptop it's a call to
    /// the firmware, 6 to 54 ms each second Power is watched, where the
    /// battery's other figures answer in under 0.1 ms (2026-11-14). Which
    /// counts as slow is set here, as for zones.
    #[test]
    fn a_slow_battery_temperature_is_read_again_only_once_kept_a_while() {
        let root = std::env::temp_dir().join(format!("machine-temp-{}", std::process::id()));
        let at = root.join("sys/class/power_supply/BAT0");
        std::fs::create_dir_all(&at).unwrap();
        let write = |tenths: &str| std::fs::write(at.join("temp"), format!("{tenths}\n")).unwrap();
        write("370");
        let held = Root::new(&root);
        let said = |settings: &mut Settings, at: Instant| {
            record(&held, "BAT0", &Value::Null, settings, at)["temperature_c"].clone()
        };
        let mut slow = Settings {
            temperature_slow_from: Duration::ZERO,
            ..Settings::default()
        };
        let mut quick = Settings {
            temperature_slow_from: Duration::MAX,
            ..Settings::default()
        };
        let first = Instant::now();
        said(&mut slow, first);
        said(&mut quick, first);
        write("415");
        let soon = first + Duration::from_secs(1);
        let after = (said(&mut slow, soon), said(&mut quick, soon));
        let kept = said(&mut slow, first + heat::SLOW_KEPT);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            after,
            (json!(37.0), json!(41.5)),
            "the slow one kept, the quick read"
        );
        assert_eq!(kept, json!(41.5), "kept past SLOW_KEPT");
    }

    // A Dell's firmware read fails when the kernel finds no 64 KiB buffer
    // for it ("page allocation failure: order:4", six times on Joel's
    // laptop overnight, 2026-11-13). The file is still there; it just
    // answers nothing, as one this account can't read does here.
    #[test]
    fn a_firmware_read_that_fails_keeps_what_it_said_last() {
        let root = std::env::temp_dir().join(format!("machine-missed-{}", std::process::id()));
        let at = root.join("sys/class/power_supply/BAT0");
        std::fs::create_dir_all(&at).unwrap();
        let types = at.join("charge_types");
        let readable = |yes: bool| {
            let mode = if yes { 0o644 } else { 0o000 };
            std::fs::set_permissions(&types, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        std::fs::write(&types, "Fast [Standard]\n").unwrap();
        let held = Root::new(&root);
        let mut settings = Settings::default();
        let first = Instant::now();
        let mut said = |at: Instant| {
            let kept = record(&held, "BAT0", &Value::Null, &mut settings, at);
            (kept["charge_type"].clone(), kept["charge_types"].clone())
        };
        let read = said(first);
        readable(false);
        let failed = said(first + SETTINGS_KEPT);
        readable(true);
        std::fs::write(&types, "[Fast] Standard\n").unwrap();
        let again = said(first + SETTINGS_KEPT + SETTINGS_AGAIN);
        let next = first + SETTINGS_KEPT * 2 + SETTINGS_AGAIN;
        readable(false);
        said(next);
        said(next + SETTINGS_AGAIN);
        readable(true);
        std::fs::write(&types, "Fast [Standard]\n").unwrap();
        let twice = said(next + SETTINGS_AGAIN * 2);
        std::fs::remove_dir_all(&root).unwrap();
        let all = json!(["Fast", "Standard"]);
        assert_eq!(read, (json!("Standard"), all.clone()));
        assert_eq!(
            failed,
            (json!("Standard"), all.clone()),
            "what it said last, not nothing"
        );
        assert_eq!(
            again,
            (json!("Fast"), all.clone()),
            "read again soon, not a minute on"
        );
        assert_eq!(
            twice,
            (json!("Fast"), all),
            "answering nothing twice running waits a minute"
        );
    }

    #[test]
    fn a_list_marks_its_choice() {
        assert_eq!(
            chosen(Some("source [sink]".into())),
            (Some("sink".into()), vec!["source".into(), "sink".into()])
        );
        assert_eq!(
            chosen(Some("C [PD] PD_PPS".into())).0.as_deref(),
            Some("PD")
        );
        assert_eq!(
            chosen(Some("Fast".into())).0.as_deref(),
            Some("Fast"),
            "an older kernel's one word is the one in use"
        );
        assert_eq!(chosen(None), (None, vec![]));
    }
}
