//! The daemon over its socket, reading a machine of its own, a fixture
//! tree of /proc and /sys: hello says how it and its sources stand; a
//! watch says every record of its kinds at once, then, as a first read's
//! rates are since boot, reads again a moment later and says them as of
//! now; a disk taken out is said gone; nothing is read once nothing is
//! watched; the programs are ranked by their share and their memory; a
//! disk's file systems are found through what's mapped over it; the
//! brightness and warmth the backlight program steers the screen to are
//! told as it writes them, as are Keep Awake's locks and the machine's
//! chosen profile; who has each camera is told as its file is; the
//! keyboards, caps lock and a board's layer are told as their watchers
//! write them; a charge limit set is told before it's answered; Heat's
//! rows are made for a watch of their place and told as they move; and
//! an ask, act or kind it doesn't know is refused.
//!
//! Long because every case runs the one daemon over the one fixture
//! machine built here, which a second file of cases would have to copy.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use daemon_client::{Client, Daemon};
use serde_json::{Value, json};

/// A machine of two cores, one disk with an encrypted volume over its
/// second partition, a loop device, and two programs.
#[expect(
    clippy::too_many_lines,
    reason = "a fixture machine reads best whole, a file a line, as the kernel lays it out"
)]
fn machine(root: &Path) {
    let write = |path: &str, text: &str| {
        let file = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    let link = |path: &str, to: &str| {
        let file = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(to, file).unwrap();
    };
    write("/proc/stat", "");
    stat(root, 100, 1000);
    write(
        "/proc/cpuinfo",
        "processor\t: 0\nvendor_id\t: GenuineIntel\ncpu family\t: 6\nmodel\t\t: 204\nmodel name\t: Intel(R) Core(TM) Ultra X7 358H\nstepping\t: 2\nmicrocode\t: 0x11c\n\nprocessor\t: 1\n",
    );
    write("/proc/loadavg", "1.50 1.25 1.00 3/900 12345\n");
    write("/proc/uptime", "1000.00 1500.00\n");
    write(
        "/proc/pressure/cpu",
        "some avg10=12.50 avg60=6.00 avg300=3.00 total=5000\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
    );
    write(
        "/proc/pressure/memory",
        "some avg10=1.00 avg60=0.50 avg300=0.25 total=900\nfull avg10=0.50 avg60=0.25 avg300=0.10 total=400\n",
    );
    write(
        "/proc/pressure/io",
        "some avg10=2.00 avg60=1.00 avg300=0.50 total=700\nfull avg10=1.00 avg60=0.50 avg300=0.25 total=300\n",
    );
    write(
        "/proc/meminfo",
        "MemTotal:       16000000 kB\nMemFree:         2000000 kB\nMemAvailable:    8000000 kB\nActive(anon):    3000000 kB\nZswap:            100000 kB\nZswapped:         400000 kB\nHugePages_Total:       0\n",
    );
    write(
        "/proc/vmstat",
        "pgfault 2000000\npgmajfault 1000\npswpin 0\npswpout 0\noom_kill 2\n",
    );
    write("/proc/sys/kernel/random/boot_id", "b007\n");
    write("/proc/sys/kernel/osrelease", "7.2.0\n");
    write("/proc/sys/kernel/version", "#1 SMP PREEMPT_DYNAMIC\n");
    for (n, core, max) in [(0, 0, "4700000"), (1, 16, "3700000")] {
        let cpu = format!("/sys/devices/system/cpu/cpu{n}");
        write(&format!("{cpu}/topology/core_id"), &format!("{core}\n"));
        write(&format!("{cpu}/cpufreq/cpuinfo_max_freq"), max);
        write(&format!("{cpu}/cpufreq/cpuinfo_min_freq"), "400000");
        write(&format!("{cpu}/cpufreq/scaling_cur_freq"), "2000000");
        write(&format!("{cpu}/cpufreq/scaling_driver"), "intel_pstate");
        write(&format!("{cpu}/cpufreq/scaling_governor"), "powersave");
        write(
            &format!("{cpu}/cpufreq/energy_performance_preference"),
            "balance_performance",
        );
        write(&format!("{cpu}/cache/index3/level"), "3");
        write(&format!("{cpu}/cache/index3/type"), "Unified");
        write(&format!("{cpu}/cache/index3/size"), "18432K");
        write(&format!("{cpu}/cache/index3/shared_cpu_list"), "0-1");
        write(&format!("{cpu}/cache/index3/id"), "0");
    }
    write("/sys/devices/cpu_core/cpus", "0\n");
    write("/sys/devices/system/cpu/intel_pstate/no_turbo", "0\n");
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/name",
        "coretemp\n",
    );
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/temp1_label",
        "Package id 0\n",
    );
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/temp1_input",
        "79000\n",
    );
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/temp1_max",
        "100000\n",
    );
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/temp2_label",
        "Core 16\n",
    );
    write(
        "/sys/devices/platform/coretemp.0/hwmon/hwmon1/temp2_input",
        "73000\n",
    );
    link(
        "/sys/class/hwmon/hwmon1",
        "../../devices/platform/coretemp.0/hwmon/hwmon1",
    );
    // The disk, its controller on a PCIe link, and its sensors.
    let controller = "/sys/devices/pci0000:00/0000:01:00.0/nvme/nvme0";
    write(
        &format!("{controller}/model"),
        "PVC10 SK hynix 1024GB          \n",
    );
    write(&format!("{controller}/firmware_rev"), "41002131\n");
    write(&format!("{controller}/transport"), "pcie\n");
    write(
        "/sys/devices/pci0000:00/0000:01:00.0/current_link_speed",
        "16.0 GT/s PCIe\n",
    );
    write(
        "/sys/devices/pci0000:00/0000:01:00.0/current_link_width",
        "4\n",
    );
    link(&format!("{controller}/device"), "../../../0000:01:00.0");
    write(&format!("{controller}/hwmon0/name"), "nvme\n");
    write(&format!("{controller}/hwmon0/temp1_label"), "Composite\n");
    write(&format!("{controller}/hwmon0/temp1_input"), "52850\n");
    write(&format!("{controller}/hwmon0/temp1_max"), "83000\n");
    write(&format!("{controller}/hwmon0/temp2_input"), "68850\n");
    write(&format!("{controller}/hwmon0/temp2_max"), "65261850\n");
    link(&format!("{controller}/hwmon0/device"), "../../nvme0");
    link(
        "/sys/class/hwmon/hwmon0",
        "../../devices/pci0000:00/0000:01:00.0/nvme/nvme0/hwmon0",
    );
    let disk = "/sys/block/nvme0n1";
    write(&format!("{disk}/size"), "2000409264\n");
    write(
        &format!("{disk}/queue/scheduler"),
        "[none] mq-deadline kyber\n",
    );
    write(&format!("{disk}/queue/nr_requests"), "1023\n");
    write(&format!("{disk}/queue/logical_block_size"), "512\n");
    write(&format!("{disk}/queue/physical_block_size"), "512\n");
    write(&format!("{disk}/queue/rotational"), "0\n");
    write(&format!("{disk}/removable"), "0\n");
    link(
        &format!("{disk}/device"),
        "../../devices/pci0000:00/0000:01:00.0/nvme/nvme0",
    );
    write(&format!("{disk}/nvme0n1p1/dev"), "259:1\n");
    write(&format!("{disk}/nvme0n1p2/holders/dm-0"), "");
    disk_stat(root, 1000, 2000);
    write("/sys/block/dm-0/dm/name", "system\n");
    write(
        "/sys/block/loop0/stat",
        "1 0 2 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n",
    );
    std::fs::create_dir_all(root.join("mnt/system")).unwrap();
    std::fs::create_dir_all(root.join("mnt/boot")).unwrap();
    write(
        "/proc/self/mountinfo",
        "1 0 0:34 / /mnt/system rw - btrfs /dev/mapper/system rw\n\
         2 1 0:34 /home /mnt/system/home rw - btrfs /dev/mapper/system rw\n\
         3 1 259:1 / /mnt/boot rw - vfat /dev/nvme0n1p1 rw\n\
         4 1 7:0 / /mnt/loop rw - ext4 /dev/loop0 rw\n",
    );
    process(root, 1, 0, "systemd", 10, 100);
    process(root, 2, 1, ".kitty-wrapped", 50, 1000);
    process(root, 3, 2, ".kitty-wrapped", 30, 500);
    // The session's runtime folder, and follow.sh's, with what it chose.
    std::fs::create_dir_all(root.join("run/user/1000")).unwrap();
    write("/run/power-profile/wanted", "performance\n");
}

/// The processors' counters, `busy` ticks of `all` on each core.
fn stat(root: &Path, busy: u64, all: u64) {
    let idle = all - busy;
    let line = |name: &str, scale: u64| {
        format!("{name} {} 0 0 {} 0 0 0 0 0 0\n", busy * scale, idle * scale)
    };
    let text = format!(
        "{}{}{}ctxt {}\nbtime 1700000000\nprocesses {}\nprocs_running 3\nprocs_blocked 1\nintr {} 0\nsoftirq {} 0\n",
        line("cpu", 2),
        line("cpu0", 1),
        line("cpu1", 1),
        all * 10,
        all,
        all * 20,
        all * 5
    );
    std::fs::write(root.join("proc/stat"), text).unwrap();
}

fn disk_stat(root: &Path, reads: u64, sectors: u64) {
    std::fs::write(
        root.join("sys/block/nvme0n1/stat"),
        format!("{reads} 0 {sectors} {reads} 0 0 0 0 2 500 900 0 0 0 0 0 0\n"),
    )
    .unwrap();
}

fn process(root: &Path, pid: u32, parent: u32, name: &str, ticks: u64, pages: u64) {
    let folder = root.join(format!("proc/{pid}"));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("stat"),
        format!(
            "{pid} ({name}) S {parent} 1 1 0 -1 0 0 0 0 0 {ticks} 0 0 0 20 0 1 0 100 1000 {pages} 0\n"
        ),
    )
    .unwrap();
}

/// The daemon running on its own socket over its own machine.
struct Running {
    daemon: Daemon,
}

impl Running {
    fn new(test: &str) -> Running {
        Running::through(test, &[])
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, launcher: &[&str]) -> Running {
        Running::started(test, launcher, None)
    }

    /// Handed the launcher's `words`, and a comma and a full stop for its
    /// numbers.
    fn worded(test: &str, words: &Value) -> Running {
        Running::started(test, &[], Some(words))
    }

    fn started(test: &str, launcher: &[&str], words: Option<&Value>) -> Running {
        let folder = std::env::temp_dir().join(format!("machine-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let root = folder.join("root");
        machine(&root);
        let socket = folder.join("daemons").join("machine.sock");
        std::fs::create_dir_all(folder.join("daemons")).unwrap();
        let program = env!("CARGO_BIN_EXE_machine");
        let mut command = match launcher.split_first() {
            Some((first, arguments)) => {
                let mut command = Command::new(first);
                for argument in arguments {
                    command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
                }
                command.arg(program);
                command
            }
            None => Command::new(program),
        };
        // The session's folders, as paths on the machine under its root.
        command.arg(&socket).arg(&root);
        if let Some(words) = words {
            let (vocabulary, numbers) = (folder.join("words.json"), folder.join("numbers.json"));
            std::fs::write(&vocabulary, words.to_string()).unwrap();
            std::fs::write(&numbers, r#"{"group": ",", "point": "."}"#).unwrap();
            command
                .arg("--words")
                .arg(&vocabulary)
                .arg("--numbers")
                .arg(&numbers);
        }
        command
            .env("XDG_RUNTIME_DIR", "/run/user/1000")
            .env_remove("XDG_STATE_HOME")
            .env("HOME", "/home/joel")
            .stderr(Stdio::null());
        Running {
            daemon: Daemon::start(&mut command, socket, folder),
        }
    }

    fn root(&self) -> PathBuf {
        self.daemon.folder().join("root")
    }

    fn client(&self) -> Client {
        self.daemon.client()
    }
}

fn record<'a>(records: &'a Value, key: &str) -> &'a Value {
    records
        .as_array()
        .and_then(|records| records.iter().find(|record| record["key"] == key))
        .unwrap_or_else(|| panic!("no {key} in {records}"))
}

#[test]
fn hello_says_it_answers_on_the_socket_systemd_hands_it() {
    let running = Running::through("hello", &["systemd-socket-activate", "--listen={socket}"]);
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "hello", "version": 1}));
    assert_eq!(said["domain"], "machine");
    assert_eq!(said["version"], 1);
    assert_eq!(said["state"], "answering");
    assert_eq!(said["sources"]["kernel"]["state"], "answering");
}

#[test]
fn a_watch_says_every_record_at_once_then_its_rates_as_of_now() {
    let running = Running::new("watch");
    let mut client = running.client();
    let said = client.ask(
        &json!({"id": "all", "ask": "watch", "kind": ["processor", "memory", "disk", "boot"]}),
    );
    let records = &said["records"];
    let processor = record(records, "processor:0");
    assert_eq!(processor["since"], "boot", "a first read is since boot");
    assert_eq!(processor["busy_ratio"], json!(0.1));
    assert_eq!(processor["model"], "Intel Core Ultra X7 358H");
    assert_eq!(processor["guest"], false, "its flags name no hypervisor");
    assert_eq!(processor["cores"][0]["kind"], "performance");
    assert_eq!(processor["cores"][1]["kind"], "efficient");
    assert_eq!(processor["cores"][1]["temperature_c"], json!(73.0));
    assert_eq!(processor["temperature"]["temperature_c"], json!(79.0));
    assert_eq!(processor["pressure"]["some"]["avg10_pct"], json!(12.5));
    assert_eq!(processor["caches"][0]["size_b"], json!(18_874_368));
    assert_eq!(processor["caches"][0]["cpus"], json!([0, 1]));
    assert_eq!(
        processor["caches"][0]["count"],
        json!(1),
        "one L3 both share"
    );
    assert_eq!(processor["caches"][0]["shared_by"], json!(2));
    assert_eq!(
        processor["switches_per_s"],
        json!(10.0),
        "10000 switches over 1000 s"
    );
    let memory = record(records, "memory:main");
    assert_eq!(memory["used_b"], json!(8_000_000u64 * 1024));
    assert_eq!(memory["info_b"]["Zswapped"], json!(400_000u64 * 1024));
    assert_eq!(memory["oom_kills"], json!(2));
    assert_eq!(memory["faults_per_s"], json!(2000.0));
    let boot = record(records, "boot:b007");
    assert_eq!(boot["booted_at_s"], json!(1_700_000_000u64));
    assert_eq!(boot["kernel"], "7.2.0");
    // Half the processors were busy since the first read, and the next
    // read, a moment later, says so as of now.
    stat(&running.root(), 600, 2000);
    let processor = client.changed("processor:0");
    assert_eq!(processor["since"], "now");
    assert_eq!(processor["busy_ratio"], json!(0.5));
    assert_eq!(processor["time"]["user"], json!(0.5));
}

#[test]
fn a_disk_says_its_rates_sensors_link_and_file_systems() {
    let running = Running::new("disk");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "list", "kind": "disk"}));
    let records = said["records"].as_array().unwrap();
    assert_eq!(records.len(), 1, "a loop device isn't a disk: {records:?}");
    let disk = &records[0];
    assert_eq!(disk["key"], "disk:nvme0n1");
    assert_eq!(disk["model"], "PVC10 SK hynix 1024GB");
    assert_eq!(disk["firmware"], "41002131");
    assert_eq!(disk["size_b"], json!(2_000_409_264u64 * 512));
    assert_eq!(disk["scheduler"], "none");
    assert_eq!(disk["link"]["speed"], "16.0 GT/s PCIe");
    assert_eq!(disk["link"]["width"], json!(4));
    assert_eq!(disk["reads_per_s"], json!(1.0), "1000 reads over 1000 s");
    assert_eq!(disk["read_latency_ms"], json!(1.0));
    assert_eq!(disk["in_flight"], json!(2));
    let sensors = disk["temperatures"].as_array().unwrap();
    assert_eq!(sensors[0]["label"], "Composite");
    assert_eq!(sensors[0]["high_c"], json!(83.0));
    assert_eq!(
        sensors[1]["high_c"],
        Value::Null,
        "a threshold out of reach is none"
    );
    let mounts: Vec<&Value> = disk["file_systems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|fs| &fs["mount"])
        .collect();
    assert_eq!(mounts, [&json!("/mnt/system"), &json!("/mnt/boot")]);
    assert_eq!(disk["file_systems"][0]["device"], "dm-0");
    assert!(disk["file_systems"][0]["size_b"].as_u64().is_some());
}

/// A fan chip, a USB-C port's chip, a thermal zone and its trip points,
/// two kinds of cooling device, and the processors' throttle counts.
fn heat(root: &Path) {
    let write = |path: &str, text: &str| {
        let file = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    let fans = "/sys/class/hwmon/hwmon5";
    write(&format!("{fans}/name"), "dell_smm\n");
    write(&format!("{fans}/fan1_label"), "CPU Fan\n");
    write(&format!("{fans}/fan1_input"), "2400\n");
    write(&format!("{fans}/fan1_max"), "5900\n");
    write(&format!("{fans}/fan1_target"), "2400\n");
    write(&format!("{fans}/pwm1"), "128\n");
    write(&format!("{fans}/pwm1_enable"), "2\n");
    write(&format!("{fans}/fan2_input"), "0\n");
    write(&format!("{fans}/temp1_input"), "47000\n");
    let port = "/sys/class/hwmon/hwmon7";
    write(&format!("{port}/name"), "ucsi_source_psy_USBC000:001\n");
    write(&format!("{port}/in0_input"), "20000\n");
    write(&format!("{port}/in0_max"), "20000\n");
    write(&format!("{port}/curr1_input"), "2250\n");
    let zone = "/sys/class/thermal/thermal_zone6";
    write(&format!("{zone}/type"), "TCPU\n");
    write(&format!("{zone}/temp"), "78500\n");
    write(&format!("{zone}/policy"), "user_space\n");
    write(&format!("{zone}/mode"), "enabled\n");
    write(&format!("{zone}/trip_point_0_type"), "critical\n");
    write(&format!("{zone}/trip_point_0_temp"), "110000\n");
    write(&format!("{zone}/trip_point_0_hyst"), "0\n");
    write(&format!("{zone}/trip_point_1_type"), "passive\n");
    write(&format!("{zone}/trip_point_1_temp"), "-274000\n");
    for (n, kind, state, most) in [
        (0, "Processor", 0, 3),
        (1, "Processor", 1, 3),
        (16, "dell-smm-fan1", 2, 2),
    ] {
        let device = format!("/sys/class/thermal/cooling_device{n}");
        write(&format!("{device}/type"), &format!("{kind}\n"));
        write(&format!("{device}/cur_state"), &format!("{state}\n"));
        write(&format!("{device}/max_state"), &format!("{most}\n"));
    }
    for (n, cores, package) in [(0, 4, 1161), (1, 2, 1100)] {
        let at = format!("/sys/devices/system/cpu/cpu{n}/thermal_throttle");
        write(&format!("{at}/core_throttle_count"), &format!("{cores}\n"));
        write(&format!("{at}/core_throttle_total_time_ms"), "42\n");
        write(&format!("{at}/core_throttle_max_time_ms"), "18\n");
        write(
            &format!("{at}/package_throttle_count"),
            &format!("{package}\n"),
        );
        write(&format!("{at}/package_throttle_total_time_ms"), "8699\n");
        write(&format!("{at}/package_throttle_max_time_ms"), "129\n");
    }
}

#[test]
fn the_sensors_zones_and_cooling_say_every_reading() {
    let running = Running::new("heat");
    heat(&running.root());
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "list", "kind": ["sensor", "zone", "cooling"]}));
    let records = &said["records"];
    let core = record(records, "sensor:coretemp");
    assert_eq!(core["part"], "processor");
    assert_eq!(core["temperatures"][0]["label"], "Package id 0");
    assert_eq!(core["temperatures"][0]["high_c"], json!(100.0));
    let fans = record(records, "sensor:dell_smm");
    assert_eq!(fans["part"], "fans");
    let fan = &fans["fans"][0];
    assert_eq!(fan["label"], "CPU Fan");
    assert_eq!(fan["rpm"], json!(2400));
    assert_eq!(fan["max_rpm"], json!(5900));
    assert_eq!(fan["target_rpm"], json!(2400));
    assert_eq!(fan["power_ratio"], json!(0.502), "128 of 255");
    assert_eq!(fan["mode"], "automatic");
    assert_eq!(fans["fans"][1]["rpm"], json!(0), "a stopped fan says so");
    assert_eq!(fans["fans"][1]["label"], Value::Null, "no label made up");
    let port = record(records, "sensor:ucsi_source_psy_USBC000:001");
    assert_eq!(port["part"], "port");
    assert_eq!(port["voltages"][0]["volts"], json!(20.0));
    assert_eq!(port["voltages"][0]["max_volts"], json!(20.0));
    assert_eq!(port["currents"][0]["amps"], json!(2.25));
    let zone = record(records, "zone:TCPU");
    assert_eq!(zone["temperature_c"], json!(78.5));
    assert_eq!(zone["policy"], "user_space");
    let trips = zone["trips"].as_array().unwrap();
    assert_eq!(trips.len(), 1, "a trip point at absolute zero is unset");
    assert_eq!(trips[0]["type"], "critical");
    assert_eq!(trips[0]["temperature_c"], json!(110.0));
    let processors = record(records, "cooling:Processor");
    assert_eq!(processors["count"], json!(2));
    assert_eq!(processors["states"], json!([0, 1]));
    assert_eq!(processors["max_state"], json!(3));
    assert_eq!(
        record(records, "cooling:dell-smm-fan1")["states"],
        json!([2])
    );
    let processor = client.ask(&json!({"id": 2, "ask": "get", "keys": ["processor:0"]}));
    let throttled = &processor["records"][0]["throttled"];
    assert_eq!(throttled["cores_count"], json!(6), "each core counted once");
    assert_eq!(throttled["cores"][1]["core"], json!(16));
    assert_eq!(throttled["cores"][1]["count"], json!(2));
    assert_eq!(
        throttled["package_count"],
        json!(1161),
        "the package's most"
    );
    assert_eq!(throttled["package_ms"], json!(8699));
    assert_eq!(throttled["package_longest_ms"], json!(129));
}

#[test]
fn heats_rows_are_made_for_its_watch_and_told_as_they_move() {
    let words = json!({ "things": [
        { "id": "heat", "name": "Heat", "glyph": "heat", "said": {
            "hottest": "{degrees} at the {part}", "stopped": "fans stopped",
            "turning": "a fan turning", "turnings": "{count} fans turning" } },
        { "id": "heat:processor", "name": "Processor" },
        { "id": "heat:cpu-fan", "name": "Processor Fan" }
    ] });
    let running = Running::worded("rows", &words);
    heat(&running.root());
    let mut client = running.client();
    let said = client
        .ask(&json!({"id": "heat", "ask": "watch", "kind": ["row", "head"], "place": "heat"}));
    let records = &said["records"];
    let head = &record(records, "head:heat")["head"];
    assert_eq!(head["said"], "79.0 \u{b0}C at the processor, a fan turning");
    assert_eq!(head["graph"]["shape"], "bar");
    let processor = &record(records, "row:heat:processor:0")["row"];
    assert_eq!(processor["name"], "Processor");
    assert_eq!(processor["side"][0]["said"], "79.0 \u{b0}C");
    assert_eq!(
        record(records, "row:heat:fan:cpu-fan:1")["row"]["name"],
        "Processor Fan"
    );
    assert!(
        records
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["kind"] == "row" || record["kind"] == "head"),
        "only Heat's rows and headline: {records}"
    );
    // Another place's rows are another daemon's.
    let mail = client.ask(&json!({"id": "mail", "ask": "watch", "kind": "row", "place": "mail"}));
    assert_eq!(mail["records"], json!([]));
    // As the processor warms, its row is told again within a read.
    std::fs::write(
        running
            .root()
            .join("sys/devices/platform/coretemp.0/hwmon/hwmon1/temp1_input"),
        "85000\n",
    )
    .unwrap();
    let warmer = client.changed("row:heat:processor:0");
    assert_eq!(warmer["row"]["side"][0]["said"], "85.0 \u{b0}C");
}

/// A battery, a charger and a USB-C port with a charger in it, and the
/// kernel's counts of the machine's sleeps.
fn power(root: &Path) {
    let write = |path: &str, text: &str| {
        let file = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    let battery = "/sys/class/power_supply/BAT0";
    for (file, text) in [
        ("type", "Battery"),
        ("status", "Discharging"),
        ("present", "1"),
        ("capacity", "80"),
        ("charge_now", "4000000"),
        ("charge_full", "4600000"),
        ("charge_full_design", "5000000"),
        ("voltage_now", "16000000"),
        ("current_now", "1500000"),
        ("cycle_count", "312"),
        ("charge_control_start_threshold", "75"),
        ("charge_control_end_threshold", "80"),
        ("charge_types", "[Standard] Fast Adaptive"),
        ("manufacture_year", "2025"),
        ("manufacture_month", "3"),
        ("manufacture_day", "9"),
        ("model_name", "DELL 8RJWW5A"),
        ("temp", "312"),
    ] {
        write(&format!("{battery}/{file}"), &format!("{text}\n"));
    }
    let port = "/sys/class/power_supply/ucsi-source-psy-USBC000:001";
    for (file, text) in [
        ("type", "USB"),
        ("online", "1"),
        ("usb_type", "C [PD] PD_PPS"),
        ("voltage_now", "20000000"),
        ("current_max", "2250000"),
    ] {
        write(&format!("{port}/{file}"), &format!("{text}\n"));
    }
    write("/sys/class/typec/port0/power_role", "source [sink]\n");
    write("/sys/class/typec/port0/data_role", "host [device]\n");
    write(
        "/sys/class/typec/port0/usb_power_delivery_revision",
        "3.0\n",
    );
    write(
        "/sys/class/typec/port0-partner/supports_usb_power_delivery",
        "yes\n",
    );
    let offered = "/sys/class/typec/port0-partner/usb_power_delivery/source-capabilities";
    write(&format!("{offered}/1:fixed_supply/voltage"), "5000mV\n");
    write(
        &format!("{offered}/1:fixed_supply/maximum_current"),
        "3000mA\n",
    );
    write(
        &format!("{offered}/10:programmable_supply/minimum_voltage"),
        "3300mV\n",
    );
    write(
        &format!("{offered}/10:programmable_supply/maximum_voltage"),
        "21000mV\n",
    );
    write(
        &format!("{offered}/10:programmable_supply/maximum_current"),
        "5000mA\n",
    );
    write(&format!("{offered}/2:fixed_supply/voltage"), "20000mV\n");
    write(
        &format!("{offered}/2:fixed_supply/maximum_current"),
        "2250mA\n",
    );
    write("/sys/power/mem_sleep", "[s2idle]\n");
    write("/sys/power/suspend_stats/success", "7\n");
    write("/sys/power/suspend_stats/fail", "2\n");
    write("/sys/power/suspend_stats/failed_freeze", "2\n");
    write("/sys/power/suspend_stats/last_failed_errno", "-16\n");
    write("/sys/power/suspend_stats/max_hw_sleep", "523986009990\n");
}

#[test]
fn a_supply_says_its_charge_health_and_port_and_sleep_its_steps() {
    let running = Running::new("power");
    power(&running.root());
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "list", "kind": ["supply", "sleep", "awake"]}));
    let records = &said["records"];
    let battery = record(records, "supply:BAT0");
    assert_eq!(battery["capacity_pct"], json!(80));
    assert_eq!(battery["charge_ah"], json!(4.0));
    assert_eq!(battery["health_ratio"], json!(0.92), "4.6 of 5.0 Ah");
    assert_eq!(battery["watts"], json!(24.0), "16 V at 1.5 A");
    assert_eq!(battery["cycles"], json!(312));
    assert_eq!(battery["limit_stop_pct"], json!(80));
    assert_eq!(battery["limit_settable"], json!(true));
    assert_eq!(battery["charge_type"], "Standard");
    assert_eq!(battery["made"], "2025-03-09");
    assert_eq!(battery["temperature_c"], json!(31.2));
    assert_eq!(battery["upower"], Value::Null, "no UPower in a test");
    let port = record(records, "supply:ucsi-source-psy-USBC000:001");
    assert_eq!(port["usb_type"], "PD");
    assert_eq!(port["volts"], json!(20.0));
    assert_eq!(port["max_amps"], json!(2.25));
    assert_eq!(port["typec"]["port"], json!(0));
    assert_eq!(port["typec"]["power_role"], "sink");
    assert_eq!(port["typec"]["power_delivery_revision"], "3.0");
    assert_eq!(port["typec"]["partner"]["power_delivery"], json!(true));
    assert_eq!(
        port["typec"]["partner"]["source_capabilities"],
        json!([
            {"volts": 5.0, "to_volts": null, "amps": 3.0},
            {"volts": 20.0, "to_volts": null, "amps": 2.25},
            {"volts": 3.3, "to_volts": 21.0, "amps": 5.0}
        ]),
        "in the kernel's order, 10 after 2"
    );
    let sleep = record(records, "sleep:main");
    assert_eq!(sleep["mode"], "s2idle");
    assert_eq!(sleep["successes"], json!(7));
    assert_eq!(sleep["failed_at"]["freeze"], json!(2));
    assert_eq!(sleep["last_failed_errno"], json!(-16));
    assert_eq!(
        sleep["hardware_sleep_counts_to_us"],
        json!(523_986_009_990u64)
    );
    assert_eq!(sleep["logind"], Value::Null, "logind's is awake:main");
    assert_eq!(record(records, "awake:main")["kind"], "awake");
}

#[test]
fn a_charge_limit_set_is_told_before_its_answer() {
    let running = Running::new("limit");
    power(&running.root());
    let mut client = running.client();
    client.ask(&json!({"id": "power", "ask": "watch", "kind": "supply"}));
    client.send(
        &json!({"id": 2, "ask": "act", "act": "charge-limit", "supply": "BAT0",
        "stop_pct": 60}),
    );
    // Read again before it's answered, so Power's row moves as the act
    // does rather than at the next second's read.
    let told = client.heard();
    let battery = told["changed"]
        .as_array()
        .and_then(|records| records.iter().find(|record| record["key"] == "supply:BAT0"));
    assert_eq!(
        battery.map(|record| &record["limit_stop_pct"]),
        Some(&json!(60)),
        "{told}"
    );
    assert_eq!(
        battery.map(|record| &record["limit_start_pct"]),
        Some(&json!(55))
    );
    assert_eq!(client.heard(), json!({"id": 2, "done": true}));
    let refused = client.ask(&json!({"id": 3, "ask": "act", "act": "charge-limit",
        "supply": "../BAT0", "stop_pct": 60}));
    assert_eq!(refused["error"]["code"], "bad-act");
    let unknown = client.ask(&json!({"id": 4, "ask": "act", "act": "overclock"}));
    assert_eq!(unknown["error"]["code"], "unknown-act");
}

// Only the units the shell may restart are, refused before the bus is
// reached, so a unit outside the list is never restarted, and one not
// named says so.
#[test]
fn a_unit_the_shell_may_not_restart_is_refused() {
    let running = Running::new("units");
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "act", "act": "restart-unit",
        "unit": "sshd.service"}));
    assert_eq!(refused["error"]["code"], "not-allowed", "{refused}");
    let unnamed = client.ask(&json!({"id": 2, "ask": "act", "act": "restart-unit"}));
    assert_eq!(unnamed["error"]["code"], "bad-act", "{unnamed}");
}

#[test]
fn powers_rows_are_made_for_its_watch_and_move_with_its_limit() {
    let words = json!({ "things": [
        { "id": "power", "name": "Battery", "said": { "battery": "on battery" } },
        { "id": "power:charge", "name": "Charge" },
        { "id": "power:limit", "name": "Charge Limit", "said": {
            "both": "stops at {stop}%, starts below {start}%" } },
        { "id": "power:limit:stop", "name": "Stop at {n}%", "said": { "chosen": "chosen" } }
    ] });
    let running = Running::worded("power-rows", &words);
    power(&running.root());
    let mut client = running.client();
    let said = client
        .ask(&json!({"id": "power", "ask": "watch", "kind": ["row", "head"], "place": "power"}));
    let records = &said["records"];
    let head = &record(records, "head:power")["head"];
    assert_eq!(head["said"], "80% \u{b7} on battery");
    assert_eq!(
        record(records, "row:power:charge")["row"]["side"][0]["said"],
        "80%"
    );
    let limit = &record(records, "row:power:limit")["row"];
    assert_eq!(limit["side"][0]["said"], "stops at 80%, starts below 75%");
    assert!(
        records
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["key"]
                .as_str()
                .is_some_and(|key| key.contains(":power"))),
        "only Power's rows and headline: {records}"
    );
    // A limit set is told on its row before the act is answered.
    client.send(
        &json!({"id": 2, "ask": "act", "act": "charge-limit", "supply": "BAT0",
        "stop_pct": 60}),
    );
    let moved = client.changed("row:power:limit");
    assert_eq!(
        moved["row"]["side"][0]["said"],
        "stops at 60%, starts below 55%"
    );
    // A row that goes sends only itself: the rows after it keep their
    // orders, where ordering siblings by where they stood sent every one
    // after it again (FEH1HE, 2026-11-10).
    std::fs::remove_file(
        running
            .root()
            .join("sys/class/power_supply/BAT0/cycle_count"),
    )
    .expect("the cycles taken away");
    let mut told = Vec::new();
    loop {
        let said = client.heard();
        told.extend(
            said["changed"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|record| record["key"].as_str().map(str::to_owned)),
        );
        if said["gone"]
            .as_array()
            .is_some_and(|gone| gone.contains(&json!("row:power:cycles")))
        {
            break;
        }
    }
    let after = ["type", "temperature", "made"].map(|id| format!("row:power:{id}"));
    assert!(
        !told.iter().any(|key| after.contains(key)),
        "no row after the one gone told again: {told:?}"
    );
}

#[test]
fn the_screens_light_and_the_rooms_are_said_as_they_move() {
    let running = Running::new("light");
    let root = running.root();
    let write = |path: &str, text: &str| {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    write("sys/class/backlight/panel/brightness", "60\n");
    write("sys/class/backlight/panel/max_brightness", "100\n");
    write("sys/bus/iio/devices/iio:device0/name", "als\n");
    write(
        "sys/bus/iio/devices/iio:device0/in_illuminance_raw",
        "400\n",
    );
    write(
        "sys/bus/iio/devices/iio:device0/in_illuminance_scale",
        "0.5\n",
    );
    let mut client = running.client();
    let said = client.ask(&json!({"id": "all", "ask": "watch", "kind": ["backlight", "light"]}));
    let records = &said["records"];
    assert_eq!(record(records, "backlight:panel")["brightness"], json!(60));
    // The sensor is read on its own, so the backlight's answer never
    // waits on it; what it hears is told as soon as it's read.
    assert_eq!(client.changed("light:iio:device0")["lux"], json!(200.0));
    // As the brightness keys set it, the watch is told within a read.
    write("sys/class/backlight/panel/brightness", "30\n");
    assert_eq!(client.changed("backlight:panel")["brightness"], json!(30));
    // And as the room darkens, within the sensor's next read.
    write("sys/bus/iio/devices/iio:device0/in_illuminance_raw", "40\n");
    assert_eq!(client.changed("light:iio:device0")["lux"], json!(20.0));
}

#[test]
fn the_brightness_and_warmth_steered_are_told_as_theyre_written() {
    let running = Running::new("steered");
    let root = running.root();
    // Each written whole, beside it and then renamed, as the backlight
    // program writes them.
    let write = |path: &str, text: &str| {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let writing = file.with_extension("writing");
        std::fs::write(&writing, text).unwrap();
        std::fs::rename(writing, file).unwrap();
    };
    write(
        "run/user/1000/backlight-now",
        "level\t0.380\nlux\t28\nasks\t0.950\nfollowing\tyes\n",
    );
    write(
        "run/user/1000/backlight-warmth",
        r#"{"kelvin": 6500, "followsTheRoom": false, "nightLight": false}"#,
    );
    let mut client = running.client();
    let said = client.ask(&json!({"id": "all", "ask": "watch", "kind": ["brightness", "warmth"]}));
    let records = &said["records"];
    assert_eq!(
        record(records, "brightness:built-in")["level_ratio"],
        json!(0.38)
    );
    assert_eq!(
        record(records, "warmth:built-in")["night_light"],
        json!(false)
    );
    // Turned on in the launcher, the night light is told as it's written;
    // nothing else reads these again while they're watched.
    write(
        "run/user/1000/backlight-warmth",
        r#"{"kelvin": 6500, "followsTheRoom": false, "nightLight": true}"#,
    );
    assert_eq!(
        client.changed("warmth:built-in")["night_light"],
        json!(true)
    );
    // A curve first learned makes its folder, which is watched from then.
    write(
        "home/joel/.local/state/backlight/learned",
        "6 0.377\n28 0.950\n",
    );
    let curve = &client.changed("brightness:built-in")["curve"];
    assert_eq!(curve[1], json!({"lux": 28.0, "level_ratio": 0.95}));
}

#[test]
fn who_has_each_camera_is_told_as_its_file_is_written() {
    let running = Running::new("camera");
    let folder = running.root().join("run/camera-holders");
    // Each written beside it under the writer's pid, then renamed, as the
    // holders programs write them.
    let write = |name: &str, text: &str| {
        std::fs::create_dir_all(&folder).unwrap();
        let writing = folder.join(format!("{name}.4120"));
        std::fs::write(&writing, text).unwrap();
        std::fs::rename(writing, folder.join(name)).unwrap();
    };
    let mut client = running.client();
    // No folder yet, as before the first camera is plugged in.
    let said = client.ask(&json!({"id": "all", "ask": "watch", "kind": "camera"}));
    assert_eq!(said["records"], json!([]));
    write("obs", "");
    assert_eq!(client.changed("camera:obs")["programs"], json!([]));
    write("obs", "zoom\n");
    assert_eq!(client.changed("camera:obs")["programs"], json!(["zoom"]));
    // Unplugged, its file goes, and so does the camera.
    std::fs::remove_file(folder.join("obs")).unwrap();
    assert_eq!(client.gone(), [json!("camera:obs")]);
}

#[test]
fn keep_awakes_locks_and_the_chosen_profile_are_told_as_theyre_written() {
    let running = Running::new("kept");
    let root = running.root();
    let mut client = running.client();
    let said = client.ask(&json!({"id": "all", "ask": "watch", "kind": ["kept", "profile"]}));
    let records = &said["records"];
    assert_eq!(record(records, "kept:idle")["held"], json!(false));
    assert_eq!(record(records, "kept:lid")["held"], json!(false));
    // No power-profiles-daemon answers here, but the machine's choice is
    // said all the same.
    assert_eq!(record(records, "profile:main")["active"], Value::Null);
    assert_eq!(record(records, "profile:main")["wanted"], "performance");
    // Kept awake from the launcher, its service leaves its file, which
    // is told as it's made, and as it goes.
    std::fs::write(root.join("run/user/1000/kept-awake"), "").unwrap();
    assert_eq!(client.changed("kept:idle")["held"], json!(true));
    std::fs::remove_file(root.join("run/user/1000/kept-awake")).unwrap();
    assert_eq!(client.changed("kept:idle")["held"], json!(false));
    // On battery, follow.sh writes its new choice in place.
    std::fs::write(root.join("run/power-profile/wanted"), "balanced\n").unwrap();
    assert_eq!(client.changed("profile:main")["wanted"], "balanced");
}

#[test]
fn the_keyboards_caps_lock_and_a_boards_layer_are_told_as_their_watchers_write() {
    let running = Running::new("keyboards");
    let root = running.root();
    let lights = root.join("run/keyboard-lights");
    let runtime = root.join("run/user/1000");
    std::fs::create_dir_all(root.join("proc/bus/input")).unwrap();
    std::fs::create_dir_all(&runtime).unwrap();
    let board = "I: Bus=0003 Vendor=3297 Product=4975 Version=0111\n\
        N: Name=\"ZSA Technology Labs ErgoDox EZ\"\n\
        P: Phys=usb-0000:00:14.0-2/input0\n\
        S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/input/input12\n\
        U: Uniq=\n\
        H: Handlers=sysrq kbd leds event12\n\
        B: LED=1f\n";
    let mut client = running.client();
    // Nothing yet from either watcher, as on a machine just booted.
    let said = client.ask(&json!({
        "id": "all", "ask": "watch", "kind": ["keyboards", "capslock", "layer"],
    }));
    assert_eq!(
        record(&said["records"], "keyboards:connected")["listed"],
        json!([])
    );
    assert_eq!(record(&said["records"], "capslock:keyboard")["on"], false);
    // A board plugged in: the kernel lists it, and the watcher's list,
    // written beside and renamed over, says it holds one more.
    std::fs::write(root.join("proc/bus/input/devices"), board).unwrap();
    std::fs::create_dir_all(&lights).unwrap();
    std::fs::write(lights.join("keyboards.new"), "12\n").unwrap();
    std::fs::rename(lights.join("keyboards.new"), lights.join("keyboards")).unwrap();
    let listed = &client.changed("keyboards:connected")["listed"];
    assert_eq!(listed[0]["vendor"], "3297");
    // Caps lock, written in place as the watcher writes it.
    std::fs::write(lights.join("capslock"), "1\n").unwrap();
    assert_eq!(client.changed("capslock:keyboard")["on"], true);
    std::fs::write(runtime.join("keyboard-layer"), "Numb locked\n").unwrap();
    let layer = client.changed("layer:keyboard");
    assert_eq!(
        (&layer["name"], &layer["locked"]),
        (&json!("Numb"), &json!(true))
    );
    // Unplugged, the board's watcher says "-", and there's no layer.
    std::fs::write(runtime.join("keyboard-layer"), "-\n").unwrap();
    assert_eq!(client.gone(), [json!("layer:keyboard")]);
}

#[test]
fn what_key_counts_tallied_is_told_as_it_writes_its_counts_again() {
    let running = Running::new("counts");
    let folder = running.root().join("var/lib/key-counts");
    let mut client = running.client();
    let said = client.ask(&json!({"id": "all", "ask": "watch", "kind": "counts"}));
    assert_eq!(said["records"], json!([]), "nothing counted yet");
    // Written beside and renamed over, as key-counts writes it.
    let write = |presses: u32| {
        std::fs::create_dir_all(&folder).unwrap();
        let text = format!(
            "board\tlayer\tmodifiers\tkey\tpresses\tholds\n\
             ergodox\tcoal\t-\te\t{presses}\t0\n"
        );
        std::fs::write(folder.join("counts.new"), text).unwrap();
        std::fs::rename(folder.join("counts.new"), folder.join("counts.tsv")).unwrap();
    };
    write(12);
    assert_eq!(client.changed("counts:ergodox")["presses"], 12);
    write(13);
    assert_eq!(client.changed("counts:ergodox")["presses"], 13);
}

#[test]
fn a_disk_taken_out_is_said_gone() {
    let running = Running::new("gone");
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "disk"}));
    std::fs::remove_dir_all(running.root().join("sys/block/nvme0n1")).unwrap();
    assert_eq!(client.gone(), [json!("disk:nvme0n1")]);
}

#[test]
fn programs_are_ranked_by_their_share_and_their_memory() {
    let running = Running::new("using");
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "list", "kind": "using"}));
    let records = &said["records"];
    let first = record(records, "using:cpu:1");
    assert_eq!(first["name"], "kitty");
    assert_eq!(first["processes"], json!(2));
    assert_eq!(
        first["share_ratio"],
        json!(0.04),
        "80 of 2000 ticks since boot"
    );
    assert_eq!(record(records, "using:cpu:2")["name"], "systemd");
    let held = record(records, "using:memory:1");
    assert_eq!(held["name"], "kitty");
    assert_eq!(held["resident_b"].as_u64().unwrap() % 1500, 0, "1500 pages");
}

#[test]
fn a_tree_is_watched_by_its_pid_and_counts_what_it_started() {
    let running = Running::new("tree");
    let mut client = running.client();
    let said = client.ask(&json!({"id": "t", "ask": "watch",
        "keys": ["tree:2", "tree:3", "tree:99"]}));
    let records = &said["records"];
    assert_eq!(
        records.as_array().unwrap().len(),
        2,
        "no process 99: {records}"
    );
    // The kitty the first started is a tree of its own too, and the first's
    // pids say it holds it, so a drawer summing both counts it once.
    assert_eq!(record(records, "tree:3")["pids"], json!([3]));
    assert_eq!(record(records, "tree:2")["pids"], json!([2, 3]));
    let tree = record(records, "tree:2");
    assert_eq!(tree["name"], "kitty");
    assert_eq!(
        tree["processes"],
        json!(2),
        "kitty and the kitty it started"
    );
    assert_eq!(
        tree["share_ratio"],
        json!(0.04),
        "80 of 2000 ticks since boot"
    );
    // A shell started under the second kitty joins the first's tree.
    process(&running.root(), 4, 3, "zsh", 40, 100);
    let tree = loop {
        let tree = client.changed("tree:2");
        if tree["processes"] == json!(3) {
            break tree;
        }
    };
    assert_eq!(tree["pids"], json!([2, 3, 4]));
    assert_eq!(tree["threads"], json!(3));
    assert_eq!(tree["resident_b"].as_u64().unwrap() % 1600, 0, "1600 pages");
    std::fs::remove_dir_all(running.root().join("proc/2")).unwrap();
    assert_eq!(client.gone(), [json!("tree:2")]);
}

#[test]
fn nothing_is_read_once_nothing_is_watched() {
    let running = Running::new("idle");
    let mut client = running.client();
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "processor"}));
    client.changed("processor:0");
    client.ask(&json!({"id": "all", "ask": "unwatch"}));
    let reads = |client: &mut Client| {
        client.ask(&json!({"id": "h", "ask": "health"}))["took_us"]
            .as_array()
            .unwrap()
            .len()
    };
    let before = reads(&mut client);
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(reads(&mut client), before, "it read with nothing watched");
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused() {
    let running = Running::new("refused");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "search"}));
    assert_eq!(said["error"]["code"], "unknown-ask");
    let said = client.ask(&json!({"id": 2, "ask": "watch", "kind": "person"}));
    assert_eq!(said["error"]["code"], "unknown-kind");
}
