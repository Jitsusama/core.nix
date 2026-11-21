//! What the machine says each second, read from the kernel's files under a
//! root that is `/` on a machine and a stand-in in the tests.
//!
//! Long because each source's quirks, a counter that wraps or starts again,
//! two fans' drivers loading in either order, a battery's sign, need a case
//! of their own over one stand-in machine, and those cases are half of it.
//!
//! Each source is found by what it names itself, never by its number,
//! since hwmon and thermal zones are numbered in the order their drivers
//! load. The recorder starts before udev has loaded every driver, so a
//! source missing at the start is looked for again every ten seconds. A
//! counter, the energy drawn, the time tasks waited and the time spent at
//! rest, is said as what it grew by a second since the record before, so a
//! record stands alone however late it was read; the first record of a run
//! has none of those, and neither has the first after a sleep, whose energy
//! counters went on through it or started again while the waits and rests
//! stood still.
//!
//! The fans, the battery and the chargers answer through the firmware, and
//! on the XPS 14 `dell_smm`'s fans through a system management call that
//! holds every processor, about 3 ms a fan idle and more loaded. So the fans
//! come from another driver where one gives them, as `dell_ddv` does from
//! its cache, and all three are read every fifth second, after the
//! counters so their time isn't counted in the counters' second. The
//! seconds between carry the last reading, at most four seconds old, and
//! each record says whether it read them. A record written on the way to
//! a stop or a sleep carries them without asking, and the next asks. Fans
//! first found only on `dell_smm` are looked for again, since udev loads
//! the two drivers in no set order.

use crate::record::{self, Battery, Record, Stalls};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

/// A powercap zone's energy counter, and where it wraps.
#[derive(Debug)]
struct Energy {
    now: PathBuf,
    range: u64,
}

/// The files the machine's readings come from.
#[derive(Debug)]
pub struct Machine {
    root: PathBuf,
    package: Option<Energy>,
    platform: Option<Energy>,
    temperature: Option<PathBuf>,
    fans: Vec<PathBuf>,
    battery: Option<PathBuf>,
    supplies: Vec<PathBuf>,
    /// Each rest state's time file, on every processor.
    rests: Vec<Vec<PathBuf>>,
    /// Each rest state's name, as the first processor gives it.
    pub rest_names: Vec<String>,
    /// How many processors rest.
    pub processors: u16,
}

fn text(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
}

fn number<T: FromStr>(path: &Path) -> Option<T> {
    text(path)?.parse().ok()
}

/// The folders in `folder` whose names start with `prefix` and end in a
/// number, in that number's order.
fn numbered(folder: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<(u32, PathBuf)> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            let number = name.strip_prefix(prefix)?.parse().ok()?;
            Some((number, entry.path()))
        })
        .collect();
    found.sort_by_key(|(number, _)| *number);
    found.into_iter().map(|(_, path)| path).collect()
}

impl Machine {
    /// The machine under `root`, its sources found by name.
    pub fn under(root: &Path) -> Self {
        let class = root.join("sys/class");
        let zone = |named: &str| {
            let folder = numbered(&class.join("powercap"), "intel-rapl:")
                .into_iter()
                .find(|zone| text(&zone.join("name")).as_deref() == Some(named))?;
            Some(Energy {
                range: number(&folder.join("max_energy_range_uj"))?,
                now: folder.join("energy_uj"),
            })
        };
        let temperature = numbered(&class.join("thermal"), "thermal_zone")
            .into_iter()
            .find(|zone| text(&zone.join("type")).as_deref() == Some("x86_pkg_temp"))
            .map(|zone| zone.join("temp"));
        let with_fans: Vec<PathBuf> = numbered(&class.join("hwmon"), "hwmon")
            .into_iter()
            .filter(|hwmon| hwmon.join("fan1_input").exists())
            .collect();
        let fans = with_fans
            .iter()
            .find(|hwmon| text(&hwmon.join("name")).as_deref() != Some(HOLDS_EVERY_PROCESSOR))
            .or_else(|| with_fans.first())
            .map(|hwmon| {
                (1..=record::FANS)
                    .map(|fan| hwmon.join(format!("fan{fan}_input")))
                    .filter(|fan| fan.exists())
                    .collect()
            })
            .unwrap_or_default();
        let mut supplies = Vec::new();
        let mut battery = None;
        let mut named: Vec<PathBuf> = fs::read_dir(class.join("power_supply"))
            .into_iter()
            .flatten()
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();
        named.sort();
        for supply in named {
            // A mouse's or a headset's battery says its scope is a device.
            let of_a_device = text(&supply.join("scope")).as_deref() == Some("Device");
            match text(&supply.join("type")).as_deref() {
                Some("Battery") if !of_a_device && battery.is_none() => battery = Some(supply),
                Some("Mains" | "USB") if !of_a_device => supplies.push(supply.join("online")),
                _ => {}
            }
        }
        let processors: Vec<PathBuf> = numbered(&root.join("sys/devices/system/cpu"), "cpu")
            .into_iter()
            .map(|cpu| cpu.join("cpuidle"))
            .filter(|cpuidle| cpuidle.is_dir())
            .collect();
        let states = processors
            .first()
            .map(|first| numbered(first, "state"))
            .unwrap_or_default();
        let rest_names = states
            .iter()
            .take(record::RESTS)
            .map(|state| text(&state.join("name")).unwrap_or_default())
            .collect();
        let rests = (0..states.len().min(record::RESTS))
            .map(|state| {
                processors
                    .iter()
                    .map(|cpuidle| cpuidle.join(format!("state{state}/time")))
                    .collect()
            })
            .collect();
        Self {
            root: root.to_owned(),
            package: zone("package-0"),
            platform: zone("psys"),
            temperature,
            fans,
            battery,
            supplies,
            rests,
            rest_names,
            processors: u16::try_from(processors.len()).unwrap_or(u16::MAX),
        }
    }

    fn proc(&self, name: &str) -> PathBuf {
        self.root.join("proc").join(name)
    }

    /// Whether the fans are read through the driver that holds every
    /// processor, as they are when it loaded before the one with a cache.
    fn fans_hold_every_processor(&self) -> bool {
        self.fans
            .first()
            .and_then(|fan| text(&fan.parent()?.join("name")))
            .as_deref()
            == Some(HOLDS_EVERY_PROCESSOR)
    }
}

/// The counters a record is the growth of since the last.
#[derive(Debug, Clone, Copy)]
struct Counters {
    since_boot_ms: u64,
    package_uj: Option<u64>,
    platform_uj: Option<u64>,
    stalls: [Option<u64>; 5],
    rests: [Option<u64>; record::RESTS],
}

/// The fans' driver whose every read is a system management call, holding
/// every processor while it runs (modules/machine/src/heat.rs), read only
/// where no other driver gives the fans.
const HOLDS_EVERY_PROCESSOR: &str = "dell_smm";
/// How many reads apart a missing source is looked for again.
const FIND_AGAIN: u64 = 10;
/// The longest gap across which an energy counter that fell is taken to
/// have wrapped, in milliseconds: past it the counter more likely started
/// again, as it does when a hibernation powers the package off.
const WRAPS_WITHIN_MS: u64 = 10_000;
/// The most a wrapped counter may say was drawn, in milliwatts: more than
/// any laptop's package or platform draws, so a fall that would say more
/// is a counter that started again.
const MOST_MW: u64 = 500_000;
/// How many reads apart the firmware's readings are taken.
const SLOW: u64 = 5;

/// What the firmware answers, read every few seconds.
#[derive(Debug, Clone, Copy, Default)]
struct Slow {
    fans: [Option<u16>; record::FANS],
    battery_mv: Option<u32>,
    battery_ma: Option<i32>,
    battery_percent: Option<u8>,
    battery: Battery,
    outside_power: Option<bool>,
}

/// Reads the machine each second, keeping the counters it read last.
pub struct Sampler {
    machine: Machine,
    last: Option<Counters>,
    slow: Slow,
    reads: u64,
    /// Whether a quick read skipped the firmware, so the next read asks it
    /// however soon its turn would come.
    firmware_owed: bool,
}

/// A pressure file's total for its `some` and `full` lines, in
/// microseconds.
fn pressure(path: &Path) -> [Option<u64>; 2] {
    let text = text(path).unwrap_or_default();
    let total = |kind: &str| {
        text.lines()
            .find(|line| line.starts_with(kind))?
            .split_whitespace()
            .find_map(|field| field.strip_prefix("total="))?
            .parse()
            .ok()
    };
    [total("some"), total("full")]
}

/// A field of /proc/meminfo, in KiB.
fn meminfo(text: &str, field: &str) -> Option<u64> {
    text.lines()
        .find_map(|line| line.strip_prefix(field)?.strip_prefix(':'))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// A load the kernel writes with two places, as 0.52, in hundredths.
fn hundredths(load: &str) -> Option<u32> {
    let (whole, places) = load.split_once('.').unwrap_or((load, "00"));
    let places = places.get(..2)?;
    let whole: u32 = whole.parse().ok()?;
    whole.checked_mul(100)?.checked_add(places.parse().ok()?)
}

fn small<T: TryFrom<u64>>(value: u64) -> Option<T> {
    T::try_from(value).ok()
}

/// What a counter grew by a second over `elapsed_ms`, saturating rather
/// than wrapping its type; none where either reading is missing, it went
/// backwards or no time passed.
fn grew(now: Option<u64>, then: Option<u64>, elapsed_ms: Option<u64>) -> Option<u32> {
    let grown = now?.checked_sub(then?)?;
    let a_second = grown.saturating_mul(1_000).checked_div(elapsed_ms?)?;
    Some(u32::try_from(a_second).unwrap_or(u32::MAX - 1))
}

/// What an energy counter drew over `elapsed_ms`, in milliwatts, allowing
/// for it wrapping past its range; none where it fell in a way no wrap
/// explains, as when it started again.
fn drawn(now: Option<u64>, then: Option<u64>, range: u64, elapsed_ms: u64) -> Option<u32> {
    let (now, then) = (now?, then?);
    let wrapped = now < then;
    let used = if wrapped {
        now + range.checked_sub(then)?
    } else {
        now - then
    };
    // A microjoule a millisecond is a milliwatt.
    let milliwatts = used.checked_div(elapsed_ms)?;
    if wrapped && (elapsed_ms > WRAPS_WITHIN_MS || milliwatts > MOST_MW) {
        return None;
    }
    Some(u32::try_from(milliwatts).unwrap_or(u32::MAX - 1))
}

/// The time since boot, sleep included, in milliseconds.
pub fn since_boot_ms() -> u64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: it writes the clock's reading into a timespec this owns.
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &raw mut now) } != 0 {
        return 0;
    }
    let seconds = u64::try_from(now.tv_sec).unwrap_or(0);
    let millis = u64::try_from(now.tv_nsec).unwrap_or(0) / 1_000_000;
    seconds * 1_000 + millis
}

impl Sampler {
    /// A sampler of `machine`, whose first record has nothing to grow from.
    pub fn of(machine: Machine) -> Self {
        Self {
            machine,
            last: None,
            slow: Slow::default(),
            reads: 0,
            firmware_owed: false,
        }
    }

    /// Looks again for each source not yet found, keeping those that were,
    /// and for fans that hold no processor while those found so far do,
    /// since udev loads both fans' drivers in whichever order they come.
    fn find_again(&mut self) {
        let machine = &mut self.machine;
        let costly_fans = machine.fans_hold_every_processor();
        let missing = machine.package.is_none()
            || machine.platform.is_none()
            || machine.temperature.is_none()
            || machine.fans.is_empty()
            || costly_fans
            || machine.battery.is_none()
            || machine.supplies.is_empty();
        if !missing {
            return;
        }
        let found = Machine::under(&machine.root);
        machine.package = machine.package.take().or(found.package);
        machine.platform = machine.platform.take().or(found.platform);
        machine.temperature = machine.temperature.take().or(found.temperature);
        machine.battery = machine.battery.take().or(found.battery);
        if machine.fans.is_empty() || (costly_fans && !found.fans.is_empty()) {
            machine.fans = found.fans;
        }
        if machine.supplies.is_empty() {
            machine.supplies = found.supplies;
        }
    }

    /// Starts again after a sleep: the counters grew through it, s2idle's
    /// draw and rest that the processors' own counters leave out among it,
    /// so the first record after has no growth, as a run's first hasn't,
    /// and asks the firmware.
    pub fn woke(&mut self) {
        self.last = None;
        self.firmware_owed = true;
    }

    /// This second's record, read at `since_boot_ms` into the boot.
    pub fn read(&mut self, since_boot_ms: u64) -> Record {
        self.read_then(since_boot_ms, true)
    }

    /// This second's record, carrying the firmware's last readings rather
    /// than asking it, as a record written on the way to a stop or a sleep
    /// does so as not to hold either up.
    pub fn read_quickly(&mut self, since_boot_ms: u64) -> Record {
        self.read_then(since_boot_ms, false)
    }

    fn read_then(&mut self, since_boot_ms: u64, firmware: bool) -> Record {
        if self.reads > 0 && self.reads.is_multiple_of(FIND_AGAIN) {
            self.find_again();
        }
        let firmware_due = firmware && (self.firmware_owed || self.reads.is_multiple_of(SLOW));
        // A quick read always owes one, so the first after a stop or a
        // sleep reads what the firmware says now, not what it said before.
        self.firmware_owed = !firmware;
        self.reads += 1;
        let mut record = self.counted(since_boot_ms);
        if firmware_due {
            self.slow = self.slow();
        }
        record.fans = self.slow.fans;
        record.battery_mv = self.slow.battery_mv;
        record.battery_ma = self.slow.battery_ma;
        record.battery_percent = self.slow.battery_percent;
        record.battery = self.slow.battery;
        record.outside_power = self.slow.outside_power;
        record.firmware_read = firmware_due;
        record
    }

    /// What the kernel holds: the counters' growth, heat, load and memory.
    fn counted(&mut self, since_boot_ms: u64) -> Record {
        let machine = &self.machine;
        let energy = |zone: &Option<Energy>| zone.as_ref().and_then(|zone| number(&zone.now));
        let [cpu_some, _] = pressure(&machine.proc("pressure/cpu"));
        let [memory_some, memory_full] = pressure(&machine.proc("pressure/memory"));
        let [io_some, io_full] = pressure(&machine.proc("pressure/io"));
        let mut rests = [None; record::RESTS];
        for (state, times) in machine.rests.iter().enumerate() {
            rests[state] = times
                .iter()
                .map(|time| number::<u64>(time))
                .sum::<Option<u64>>();
        }
        let now = Counters {
            since_boot_ms,
            package_uj: energy(&machine.package),
            platform_uj: energy(&machine.platform),
            stalls: [cpu_some, memory_some, memory_full, io_some, io_full],
            rests,
        };
        let last = self.last.replace(now);
        let elapsed_ms = last.map(|last| since_boot_ms.saturating_sub(last.since_boot_ms));
        let draw = |zone: &Option<Energy>, now: Option<u64>, then: Option<u64>| {
            drawn(now, then, zone.as_ref()?.range, elapsed_ms?)
        };
        let then = |pick: fn(&Counters) -> Option<u64>| last.as_ref().and_then(pick);
        let stall = |at: usize| grew(now.stalls[at], last.as_ref()?.stalls[at], elapsed_ms);
        let mut record = Record {
            at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|since| i64::try_from(since.as_millis()).ok())
                .unwrap_or(0),
            since_boot_ms,
            package_mw: draw(
                &machine.package,
                now.package_uj,
                then(|last| last.package_uj),
            ),
            platform_mw: draw(
                &machine.platform,
                now.platform_uj,
                then(|last| last.platform_uj),
            ),
            package_mc: machine.temperature.as_deref().and_then(number),
            stalls: Stalls {
                cpu_some: stall(0),
                memory_some: stall(1),
                memory_full: stall(2),
                io_some: stall(3),
                io_full: stall(4),
            },
            ..Record::default()
        };
        for (rest, (now, then)) in record.rests.iter_mut().zip(
            now.rests
                .iter()
                .zip(last.map_or([None; record::RESTS], |last| last.rests)),
        ) {
            *rest = grew(*now, then, elapsed_ms);
        }
        self.loads(&mut record);
        self.memory(&mut record);
        record
    }

    fn loads(&self, record: &mut Record) {
        let text = text(&self.machine.proc("loadavg")).unwrap_or_default();
        let mut fields = text.split_whitespace();
        record.load = fields.next().and_then(hundredths);
        record.tasks = fields.nth(2).and_then(|tasks| {
            let (running, all) = tasks.split_once('/')?;
            let cap = |count: &str| u16::try_from(count.parse::<u64>().ok()?.min(65_534)).ok();
            cap(running).zip(cap(all))
        });
    }

    fn memory(&self, record: &mut Record) {
        let text = text(&self.machine.proc("meminfo")).unwrap_or_default();
        record.available_kib = meminfo(&text, "MemAvailable").and_then(small);
        record.swapped_kib = meminfo(&text, "SwapTotal")
            .zip(meminfo(&text, "SwapFree"))
            .and_then(|(total, free)| small(total.saturating_sub(free)));
    }

    /// What the fans, the battery and the chargers say now.
    fn slow(&self) -> Slow {
        let mut slow = Slow::default();
        for (fan, path) in slow.fans.iter_mut().zip(&self.machine.fans) {
            *fan = number::<u64>(path).and_then(small);
        }
        if let Some(battery) = &self.machine.battery {
            let microvolts = number::<u64>(&battery.join("voltage_now"));
            slow.battery_mv = microvolts.and_then(|microvolts| small(microvolts / 1_000));
            slow.battery = text(&battery.join("status"))
                .map_or(Battery::Unsaid, |status| Battery::of_status(&status));
            // A battery whose firmware counts in watts gives its power, not
            // its current; the current is worked out from it and the
            // voltage. Drivers sign the current differently, an ACPI
            // battery's never, so it's taken as its size and said drawn
            // from the battery when the battery says it's discharging.
            let from_power = || {
                let microwatts = number::<i64>(&battery.join("power_now"))?;
                let microvolts = i64::try_from(microvolts?).ok()?;
                microwatts.checked_mul(1_000)?.checked_div(microvolts)
            };
            slow.battery_ma = number::<i64>(&battery.join("current_now"))
                .map(|microamps| microamps / 1_000)
                .or_else(from_power)
                .map(|milliamps| {
                    if slow.battery == Battery::Discharging {
                        -milliamps.abs()
                    } else {
                        milliamps.abs()
                    }
                })
                .and_then(|milliamps| i32::try_from(milliamps).ok());
            slow.battery_percent = number(&battery.join("capacity"));
        }
        let online: Vec<bool> = self
            .machine
            .supplies
            .iter()
            .filter_map(|online| number::<u8>(online))
            .map(|online| online == 1)
            .collect();
        slow.outside_power = (!online.is_empty()).then(|| online.contains(&true));
        slow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_energy_counter_that_wrapped_is_read_across_its_range() {
        assert_eq!(drawn(Some(1_000), Some(0), 10_000, 1_000), Some(1));
        assert_eq!(drawn(Some(500), Some(9_500), 10_000, 100), Some(10));
        assert_eq!(drawn(Some(500), None, 10_000, 100), None);
        assert_eq!(drawn(Some(500), Some(400), 10_000, 0), None);
    }

    #[test]
    fn an_energy_counter_that_started_again_draws_nothing() {
        let range = 262_143_328_850;
        // After a hibernation of five minutes, near its bottom.
        assert_eq!(
            drawn(Some(1_000), Some(9_000_000), range, 300_000),
            None,
            "a counter that started again across a long gap was read as a wrap"
        );
        // Within a second, far from its top.
        assert_eq!(
            drawn(Some(1_000), Some(9_000_000), range, 1_000),
            None,
            "a counter that started again was read as a wrap of hundreds of kW"
        );
        // After an hour's hibernation, where a wrap would read a likely 73 W.
        assert_eq!(
            drawn(Some(1_000), Some(9_000_000), range, 3_600_000),
            None,
            "a counter that started again an hour before was read as a wrap"
        );
        // A wrap nine seconds apart, as a late read may be, still counts.
        assert_eq!(
            drawn(Some(40_000_000), Some(range - 50_000_000), range, 9_000),
            Some(10_000),
            "a wrap within ten seconds was refused"
        );
    }

    #[test]
    fn a_load_is_read_in_hundredths() {
        assert_eq!(hundredths("0.52"), Some(52));
        assert_eq!(hundredths("12.07"), Some(1_207));
        assert_eq!(hundredths("3"), Some(300));
        assert_eq!(hundredths("x.52"), None);
    }

    #[test]
    fn a_counter_that_went_back_says_nothing_rather_than_a_wrapped_number() {
        let a_second = Some(1_000);
        assert_eq!(grew(Some(10), Some(4), a_second), Some(6));
        assert_eq!(grew(Some(4), Some(10), a_second), None);
        assert_eq!(grew(Some(4), None, a_second), None);
        assert_eq!(grew(Some(10), Some(4), Some(0)), None, "no time passed");
    }

    #[test]
    fn a_counter_is_said_a_second_however_far_apart_it_was_read() {
        assert_eq!(
            grew(Some(600), Some(0), Some(3_000)),
            Some(200),
            "read late"
        );
        assert_eq!(
            grew(Some(600), Some(0), Some(500)),
            Some(1_200),
            "read early"
        );
    }

    #[test]
    fn a_source_found_late_is_read_from_then_on() {
        let root = a_machine("late");
        let zone = root.join("sys/class/thermal/thermal_zone7");
        fs::remove_dir_all(&zone).expect("the package's heat taken away");
        let mut sampler = Sampler::of(Machine::under(&root));
        let reads: Vec<Option<i32>> = (0..12)
            .map(|second| {
                if second == 3 {
                    put(
                        &root,
                        "sys/class/thermal/thermal_zone7/type",
                        "x86_pkg_temp\n",
                    );
                    put(&root, "sys/class/thermal/thermal_zone7/temp", "71000\n");
                }
                sampler.read(1_000 * (second + 1)).package_mc
            })
            .collect();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(reads[9], None, "a source was looked for every second");
        assert_eq!(
            reads[10],
            Some(71_000),
            "a source found late was never read"
        );
    }

    #[test]
    fn the_firmwares_readings_are_taken_every_fifth_second_and_carried_between() {
        let root = a_machine("slow");
        let mut sampler = Sampler::of(Machine::under(&root));
        let fans: Vec<Option<u16>> = (0..6)
            .map(|second| {
                put(
                    &root,
                    "sys/class/hwmon/hwmon4/fan1_input",
                    &format!("{second}\n"),
                );
                sampler.read(1_000 * (second + 1)).fans[0]
            })
            .collect();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            fans,
            [Some(0), Some(0), Some(0), Some(0), Some(0), Some(5)],
            "the fans were read more often than every fifth second"
        );
    }

    #[test]
    fn each_record_says_whether_it_read_the_firmware() {
        let root = a_machine("fresh");
        let mut sampler = Sampler::of(Machine::under(&root));
        let read: Vec<bool> = (0..10)
            .map(|second| sampler.read(1_000 * (second + 1)).firmware_read)
            .collect();
        // The tenth read is the firmware's turn, which a quick one passes.
        put(&root, "sys/class/hwmon/hwmon4/fan1_input", "3100\n");
        let quick = sampler.read_quickly(11_000);
        let after = sampler.read(12_000);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            read,
            [
                true, false, false, false, false, true, false, false, false, false
            ]
        );
        assert!(
            !quick.firmware_read && quick.fans[0] == Some(2_400),
            "a quick read asked the firmware, or carried nothing"
        );
        assert!(
            after.firmware_read && after.fans[0] == Some(3_100),
            "the read after a quick one didn't ask the firmware"
        );
    }

    #[test]
    fn the_first_record_after_a_sleep_says_no_growth_across_it() {
        let root = a_machine("woke");
        let mut slept = Sampler::of(Machine::under(&root));
        let mut awake = Sampler::of(Machine::under(&root));
        slept.read_quickly(1_000);
        awake.read(1_000);
        // Counters that grew while it slept, over a gap the hour's rule
        // doesn't already leave out.
        a_second_later(&root);
        slept.woke();
        let woken = slept.read(3_000);
        let unslept = awake.read(3_000);
        let _ = fs::remove_dir_all(&root);
        assert!(
            unslept.package_mw.is_some() && unslept.stalls.cpu_some.is_some(),
            "a sampler that never slept read no growth, so this proves nothing"
        );
        assert!(
            woken.package_mw.is_none() && woken.stalls.cpu_some.is_none(),
            "the first record after a sleep spread what grew over the whole sleep"
        );
        assert!(
            woken.firmware_read,
            "the first record after a sleep didn't ask the firmware"
        );
    }

    #[test]
    fn fans_found_first_on_the_driver_that_holds_every_processor_move_off_it() {
        let root = a_machine("smm-first");
        let cached = root.join("sys/class/hwmon/hwmon4");
        fs::remove_dir_all(&cached).expect("the cached driver not loaded yet");
        let mut sampler = Sampler::of(Machine::under(&root));
        for second in 0..11 {
            if second == 3 {
                put(&root, "sys/class/hwmon/hwmon4/name", "dell_ddv\n");
                put(&root, "sys/class/hwmon/hwmon4/fan1_input", "2400\n");
            }
            sampler.read(1_000 * (second + 1));
        }
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            sampler.machine.fans.first().and_then(|fan| fan.parent()),
            Some(cached.as_path()),
            "the fans stayed on system management calls once another driver gave them"
        );
    }

    #[test]
    fn the_fans_come_from_a_driver_that_holds_no_processor_where_one_gives_them() {
        let root = a_machine("fans");
        let smm = Machine::under(&root).fans;
        fs::remove_dir_all(root.join("sys/class/hwmon/hwmon4")).expect("the other driver gone");
        let only_smm = Machine::under(&root).fans;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            smm.first().and_then(|fan| fan.parent()),
            Some(root.join("sys/class/hwmon/hwmon4").as_path()),
            "the fans were read through system management calls"
        );
        assert_eq!(
            only_smm.first().and_then(|fan| fan.parent()),
            Some(root.join("sys/class/hwmon/hwmon2").as_path()),
            "fans only one driver gives weren't read"
        );
    }

    #[test]
    fn a_battery_is_said_drawn_from_only_when_it_says_so() {
        let root = a_machine("signs");
        // An ACPI battery gives its current unsigned.
        let discharging = Sampler::of(Machine::under(&root)).read(1_000).battery_ma;
        put(&root, "sys/class/power_supply/BAT0/status", "Charging\n");
        put(
            &root,
            "sys/class/power_supply/BAT0/current_now",
            "-900000\n",
        );
        let charging = Sampler::of(Machine::under(&root)).read(1_000).battery_ma;
        fs::remove_file(root.join("sys/class/power_supply/BAT0/current_now"))
            .expect("its current taken away");
        put(&root, "sys/class/power_supply/BAT0/power_now", "16100000\n");
        let charging_in_watts = Sampler::of(Machine::under(&root)).read(1_000).battery_ma;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            (discharging, charging, charging_in_watts),
            (Some(-900), Some(900), Some(1_000))
        );
    }

    #[test]
    fn a_battery_that_gives_its_power_gives_its_current_from_it() {
        let root = a_machine("watts");
        let battery = root.join("sys/class/power_supply/BAT0");
        fs::remove_file(battery.join("current_now")).expect("its current taken away");
        put(&root, "sys/class/power_supply/BAT0/power_now", "16100000\n");
        let record = Sampler::of(Machine::under(&root)).read(1_000);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            record.battery_ma,
            Some(-1_000),
            "16.1 W at 16.1 V, discharging, wasn't an amp drawn"
        );
    }

    #[test]
    fn a_pressure_files_totals_are_read_from_its_lines() {
        let folder =
            std::env::temp_dir().join(format!("last-seconds-pressure-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("memory");
        fs::write(
            &file,
            "some avg10=0.00 avg60=0.00 avg300=0.00 total=1149083191\n\
             full avg10=0.00 avg60=0.00 avg300=0.00 total=575258820\n",
        )
        .expect("a pressure file");
        assert_eq!(pressure(&file), [Some(1_149_083_191), Some(575_258_820)]);
        let _ = fs::remove_dir_all(&folder);
    }

    /// Writes `text` at `path` under `root`, making its folders.
    fn put(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a file's folder")).expect("its folders");
        fs::write(&path, text).expect("a stand-in file");
    }

    /// A stand-in machine in a folder of its own, named so tests running
    /// at once don't share one.
    fn a_machine(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("last-seconds-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        // The zones are numbered against their names, so a number can't
        // stand for one.
        put(&root, "sys/class/powercap/intel-rapl:0/name", "psys\n");
        put(
            &root,
            "sys/class/powercap/intel-rapl:0/energy_uj",
            "1000000\n",
        );
        put(
            &root,
            "sys/class/powercap/intel-rapl:0/max_energy_range_uj",
            "10000000\n",
        );
        put(&root, "sys/class/powercap/intel-rapl:1/name", "package-0\n");
        put(
            &root,
            "sys/class/powercap/intel-rapl:1/energy_uj",
            "9900000\n",
        );
        put(
            &root,
            "sys/class/powercap/intel-rapl:1/max_energy_range_uj",
            "10000000\n",
        );
        put(&root, "sys/class/thermal/thermal_zone0/type", "acpitz\n");
        put(&root, "sys/class/thermal/thermal_zone0/temp", "30000\n");
        put(
            &root,
            "sys/class/thermal/thermal_zone7/type",
            "x86_pkg_temp\n",
        );
        put(&root, "sys/class/thermal/thermal_zone7/temp", "71000\n");
        put(&root, "sys/class/hwmon/hwmon1/temp1_input", "40000\n");
        // The same fans twice, as the XPS 14 gives them, the driver that
        // holds every processor numbered first.
        put(&root, "sys/class/hwmon/hwmon2/name", "dell_smm\n");
        put(&root, "sys/class/hwmon/hwmon2/fan1_input", "2410\n");
        put(&root, "sys/class/hwmon/hwmon4/name", "dell_ddv\n");
        put(&root, "sys/class/hwmon/hwmon4/fan1_input", "2400\n");
        put(&root, "sys/class/hwmon/hwmon4/fan2_input", "2350\n");
        put(&root, "sys/class/power_supply/AC/type", "Mains\n");
        put(&root, "sys/class/power_supply/AC/online", "0\n");
        put(&root, "sys/class/power_supply/BAT0/type", "Battery\n");
        put(
            &root,
            "sys/class/power_supply/BAT0/voltage_now",
            "16100000\n",
        );
        put(&root, "sys/class/power_supply/BAT0/current_now", "900000\n");
        put(&root, "sys/class/power_supply/BAT0/capacity", "63\n");
        put(&root, "sys/class/power_supply/BAT0/status", "Discharging\n");
        // A mouse's battery, sorted ahead of the laptop's, is no battery
        // of the machine's.
        put(&root, "sys/class/power_supply/AAA-mouse/type", "Battery\n");
        put(&root, "sys/class/power_supply/AAA-mouse/scope", "Device\n");
        put(&root, "sys/class/power_supply/AAA-mouse/capacity", "5\n");
        for cpu in ["cpu0", "cpu1"] {
            let at = format!("sys/devices/system/cpu/{cpu}/cpuidle");
            put(&root, &format!("{at}/state0/name"), "POLL\n");
            put(&root, &format!("{at}/state0/time"), "100\n");
            put(&root, &format!("{at}/state1/name"), "C10\n");
            put(&root, &format!("{at}/state1/time"), "5000000\n");
        }
        put(&root, "sys/devices/system/cpu/cpufreq/boost", "1\n");
        put(&root, "proc/loadavg", "1.52 0.90 0.40 3/1201 4242\n");
        put(
            &root,
            "proc/meminfo",
            "MemAvailable: 20000000 kB\nSwapTotal: 8000 kB\nSwapFree: 6000 kB\n",
        );
        put(
            &root,
            "proc/pressure/cpu",
            "some avg10=0 avg60=0 avg300=0 total=1000\n",
        );

        root
    }

    /// Moves the stand-in's counters on by a second.
    fn a_second_later(root: &Path) {
        put(
            root,
            "sys/class/powercap/intel-rapl:0/energy_uj",
            "3000000\n",
        );
        put(
            root,
            "sys/class/powercap/intel-rapl:1/energy_uj",
            "400000\n",
        );
        put(
            root,
            "proc/pressure/cpu",
            "some avg10=0 avg60=0 avg300=0 total=251000\n",
        );
        for cpu in ["cpu0", "cpu1"] {
            let at = format!("sys/devices/system/cpu/{cpu}/cpuidle");
            put(root, &format!("{at}/state1/time"), "5800000\n");
        }
    }

    /// What a sampler of a stand-in machine reads, once and a second later.
    fn two_records(name: &str) -> (Sampler, Record, Record) {
        let root = a_machine(name);
        let mut sampler = Sampler::of(Machine::under(&root));
        let first = sampler.read(1_000);
        a_second_later(&root);
        let second = sampler.read(2_000);
        let _ = fs::remove_dir_all(&root);
        (sampler, first, second)
    }

    #[test]
    fn each_source_is_found_by_its_name_and_read_as_what_grew() {
        let (sampler, first, second) = two_records("names");
        assert_eq!(
            sampler.machine.rest_names,
            ["POLL", "C10"],
            "the rest states were misnamed"
        );
        assert_eq!(
            sampler.machine.processors, 2,
            "a folder beside the processors was counted"
        );
        assert_eq!(first.package_mw, None, "a first record drew from nothing");
        assert_eq!(
            first.rests,
            [None; record::RESTS],
            "a first record rested from nothing"
        );
        assert_eq!(
            second.rests[..2],
            [Some(0), Some(1_600_000)],
            "the rest times grew wrong"
        );
        assert_eq!(
            second.platform_mw,
            Some(2_000),
            "psys was read as another zone"
        );
        assert_eq!(
            second.package_mw,
            Some(500),
            "the package's counter wrapped wrong"
        );
        assert_eq!(
            second.package_mc,
            Some(71_000),
            "another zone was read as the package"
        );
        assert_eq!(
            second.fans,
            [Some(2_400), Some(2_350), None, None],
            "the fans were misread"
        );
    }

    #[test]
    fn the_load_waits_memory_and_battery_are_read_as_they_stand() {
        let (_, _, second) = two_records("load");
        assert_eq!(second.load, Some(152), "the load was misread");
        assert_eq!(second.tasks, Some((3, 1_201)), "the tasks were misread");
        assert_eq!(
            second.stalls.cpu_some,
            Some(250_000),
            "the processor's wait was misread"
        );
        assert_eq!(
            second.stalls.io_some, None,
            "a missing pressure file read as a wait"
        );
        assert_eq!(
            second.available_kib,
            Some(20_000_000),
            "the memory left was misread"
        );
        assert_eq!(
            second.swapped_kib,
            Some(2_000),
            "the swap in use was misread"
        );
        assert_eq!(
            second.battery_mv,
            Some(16_100),
            "the mouse's battery was read"
        );
        assert_eq!(
            second.battery_ma,
            Some(-900),
            "the battery's current was misread"
        );
        assert_eq!(
            second.battery_percent,
            Some(63),
            "the mouse's battery was read"
        );
        assert_eq!(
            second.battery,
            Battery::Discharging,
            "the battery's state was misread"
        );
        assert_eq!(second.outside_power, Some(false), "the charger was misread");
    }

    #[test]
    fn meminfos_fields_are_found_by_their_whole_name() {
        let text = "MemTotal: 64000000 kB\nMemAvailable:   20123456 kB\nSwapTotal: 8 kB\n";
        assert_eq!(meminfo(text, "MemAvailable"), Some(20_123_456));
        assert_eq!(meminfo(text, "Mem"), None);
    }
}
