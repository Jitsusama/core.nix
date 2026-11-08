//! The daemon over its socket, reading a machine of its own, a fixture
//! tree of /proc and /sys: hello says how it and its sources stand; a
//! watch says every record of its kinds at once, then, as a first read's
//! rates are since boot, reads again a moment later and says them as of
//! now; a disk taken out is said gone; nothing is read once nothing is
//! watched; the programs are ranked by their share and their memory; a
//! disk's file systems are found through what's mapped over it; and an
//! ask or kind it doesn't know is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A machine of two cores, one disk with an encrypted volume over its
/// second partition, a loop device, and two programs.
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
    process(root, 1, "systemd", 10, 100);
    process(root, 2, ".kitty-wrapped", 50, 1000);
    process(root, 3, ".kitty-wrapped", 30, 500);
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

fn process(root: &Path, pid: u32, name: &str, ticks: u64, pages: u64) {
    let folder = root.join(format!("proc/{pid}"));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("stat"),
        format!(
            "{pid} ({name}) S 1 1 1 0 -1 0 0 0 0 0 {ticks} 0 0 0 20 0 1 0 100 1000 {pages} 0\n"
        ),
    )
    .unwrap();
}

/// The daemon running on its own socket over its own machine.
struct Running {
    daemon: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Running {
    fn new(test: &str) -> Running {
        Running::through(test, &[])
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, launcher: &[&str]) -> Running {
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
        let daemon = command
            .arg(&socket)
            .arg(&root)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        until("the socket never came", || socket.exists());
        Running {
            daemon,
            socket,
            folder,
        }
    }

    fn root(&self) -> PathBuf {
        self.folder.join("root")
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Client {
            lines: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

struct Client {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// The answer to an ask, passing over what a watch pushed meanwhile.
    fn ask(&mut self, asked: &Value) -> Value {
        writeln!(self.stream, "{asked}").unwrap();
        loop {
            let said = self.next();
            if said["id"] == asked["id"] && said.get("changed").is_none() {
                return said;
            }
        }
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.lines.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    /// The next change a watch is told of the record with this key.
    fn changed(&mut self, key: &str) -> Value {
        loop {
            let said = self.next();
            let found = said["changed"]
                .as_array()
                .and_then(|records| records.iter().find(|record| record["key"] == key));
            if let Some(record) = found {
                return record.clone();
            }
        }
    }

    /// The next change saying keys went.
    fn gone(&mut self) -> Vec<Value> {
        loop {
            let said = self.next();
            if let Some(gone) = said["gone"].as_array() {
                return gone.clone();
            }
        }
    }
}

fn until(what: &str, held: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !held() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(20));
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
