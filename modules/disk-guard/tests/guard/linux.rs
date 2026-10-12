use disk_guard::band::BtrfsSpace;
use disk_guard::linux::{boot, btrfs_allocation, power, pressure, settle, space, tick};
use disk_guard::moments::{Power, Stall};
use std::fs;
use std::path::Path;
use std::process::Command;

const GIB: u64 = 1 << 30;
const MIB: u64 = 1 << 20;

fn write(root: &Path, path: &str, value: u64) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("{value}\n")).unwrap();
}

/// A filesystem's sysfs directory as Btrfs lays it out, on one 1000 GiB
/// device: 400 GiB of data chunks, 8 GiB of metadata kept twice (dup), and
/// a system chunk.
fn sysfs(root: &Path) {
    write(root, "allocation/global_rsv_size", 512 * MIB);
    write(root, "allocation/data/total_bytes", 400 * GIB);
    write(root, "allocation/data/disk_total", 400 * GIB);
    write(root, "allocation/data/chunk_size", 10 * GIB);
    write(root, "allocation/metadata/total_bytes", 8 * GIB);
    write(root, "allocation/metadata/disk_total", 16 * GIB);
    write(root, "allocation/metadata/chunk_size", GIB);
    write(root, "allocation/metadata/bytes_used", 7 * GIB);
    write(root, "allocation/metadata/bytes_reserved", 100 * MIB);
    write(root, "allocation/metadata/bytes_pinned", 20 * MIB);
    write(root, "allocation/metadata/bytes_readonly", 4 * MIB);
    write(root, "allocation/metadata/bytes_may_use", 3 * GIB);
    write(root, "allocation/system/disk_total", 64 * MIB);
    write(root, "devices/dm-0/size", 1000 * GIB / 512);
}

#[test]
fn allocation_is_read_from_the_filesystems_sysfs() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    write(root.path(), "allocation/metadata/bytes_zone_unusable", 0);
    let space = btrfs_allocation(root.path()).unwrap();
    assert_eq!(
        space,
        BtrfsSpace {
            unallocated: 1000 * GIB - 400 * GIB - 16 * GIB - 64 * MIB,
            metadata_chunk: 2 * GIB,
            // Reservations that may yet be given back (bytes_may_use) are
            // Btrfs overcommitting, and count as free.
            metadata_free: GIB - 124 * MIB,
            global_reserve: 512 * MIB,
        }
    );
}

#[test]
fn an_older_kernel_without_zone_counts_reads_the_same() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    assert_eq!(
        btrfs_allocation(root.path()).unwrap().metadata_free,
        GIB - 124 * MIB
    );
}

#[test]
fn a_filesystem_over_two_devices_counts_both() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    write(root.path(), "devices/sdb/size", 500 * GIB / 512);
    let unallocated = 1500 * GIB - 400 * GIB - 16 * GIB - 64 * MIB;
    assert_eq!(
        btrfs_allocation(root.path()).unwrap().unallocated,
        unallocated
    );
}

/// What a command says about `path`, its output trimmed.
fn says(program: &str, args: &[&str], path: &Path) -> Option<String> {
    let output = Command::new(program).args(args).arg(path).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn space_reads_the_filesystem_a_path_is_on() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let space = space(dir.path()).unwrap();
    assert!(space.total > 0 && space.free <= space.total, "{space:?}");
    let blocks = says("df", &["--output=size,avail", "-B1"], dir.path());
    if let Some(Some(line)) = blocks.map(|text| text.lines().last().map(str::to_owned)) {
        let numbers: Vec<u64> = line
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        assert_eq!(space.total, numbers[0]);
        // Other writers move free space between the two readings.
        assert!(
            space.free.abs_diff(numbers[1]) < 1 << 30,
            "{} against {}",
            space.free,
            numbers[1]
        );
    }
    let btrfs = says("findmnt", &["-no", "FSTYPE", "-T"], dir.path()).as_deref() == Some("btrfs");
    if btrfs && Path::new("/sys/fs/btrfs").exists() {
        let allocation = space.btrfs.expect("Btrfs without its allocation");
        assert!(allocation.unallocated < space.total * 2 && allocation.global_reserve > 0);
    } else if !btrfs {
        assert_eq!(space.btrfs, None);
    }
}

#[test]
fn settling_shows_what_a_delete_freed() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let file = dir.path().join("noise");
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let data: Vec<u8> = (0..256 * MIB)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    // The filesystem is shared with whatever else the machine is writing,
    // which can use up some of the freed space between the two readings,
    // so the best of three tries counts. Without settling, none shows.
    let mut best = 0;
    for _ in 0..3 {
        // What the guard deletes has long been written out, as this has
        // once synced; data still in memory is accounted for differently.
        fs::write(&file, &data).unwrap();
        fs::File::open(&file).unwrap().sync_all().unwrap();
        settle(dir.path()).unwrap();
        let before = space(dir.path()).unwrap().free;
        fs::remove_file(&file).unwrap();
        settle(dir.path()).unwrap();
        best = best.max(space(dir.path()).unwrap().free.saturating_sub(before));
        if best >= 200 * MIB {
            break;
        }
    }
    assert!(best >= 200 * MIB, "only {} MiB showed as freed", best / MIB);
}

#[test]
fn a_tick_reads_the_clocks_and_the_boot() {
    let first = tick().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = tick().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    assert!((second.wall - now.as_secs_f64()).abs() < 1.0);
    assert!(second.monotonic - first.monotonic >= 0.02);
    assert!(first.monotonic > 0.0 && first.suspended >= 0.0);
    assert_eq!(first.boot, second.boot);
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap();
    assert_eq!(first.boot, boot.trim());
}

fn text(root: &Path, path: &str, value: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

#[test]
fn pressure_is_the_last_ten_seconds_of_each_stall() {
    let root = tempfile::tempdir().unwrap();
    let io = "some avg10=12.50 avg60=3.00 avg300=0.82 total=136859772\n\
              full avg10=4.25 avg60=1.00 avg300=0.19 total=60570158\n";
    text(root.path(), "io", io);
    text(
        root.path(),
        "memory",
        &io.replace("12.50", "0.50").replace("4.25", "0.00"),
    );
    // CPU pressure had no full line before Linux 5.13.
    text(
        root.path(),
        "cpu",
        "some avg10=30.00 avg60=0.00 avg300=0.00 total=1\n",
    );
    let pressure = pressure(root.path()).unwrap();
    assert_eq!(
        pressure.io,
        Stall {
            some: 12.5,
            full: 4.25
        }
    );
    assert_eq!(
        pressure.memory,
        Stall {
            some: 0.5,
            full: 0.0
        }
    );
    assert_eq!(
        pressure.cpu,
        Stall {
            some: 30.0,
            full: 0.0
        }
    );
}

#[test]
fn the_live_pressure_reads() {
    if Path::new("/proc/pressure/io").exists() {
        let pressure = pressure(Path::new("/proc/pressure")).unwrap();
        assert!((0.0..=100.0).contains(&pressure.io.some));
    }
}

/// A laptop's supplies as one shows them: mains and a USB-C source for
/// the system, its battery, and a mouse's battery.
fn supplies(root: &Path, mains: &str, usb: &str, capacity: &str) {
    text(root, "AC/type", "Mains\n");
    text(root, "AC/online", mains);
    text(root, "ucsi-source-psy-USBC000:001/type", "USB\n");
    text(root, "ucsi-source-psy-USBC000:001/scope", "System\n");
    text(root, "ucsi-source-psy-USBC000:001/online", usb);
    text(root, "BAT0/type", "Battery\n");
    text(root, "BAT0/capacity", capacity);
    text(root, "BAT0/status", "Discharging\n");
    text(root, "hidpp_battery_0/type", "Battery\n");
    text(root, "hidpp_battery_0/scope", "Device\n");
    text(root, "hidpp_battery_0/capacity", "5\n");
}

#[test]
fn power_comes_from_any_system_supply_and_the_systems_battery() {
    let root = tempfile::tempdir().unwrap();
    supplies(root.path(), "1\n", "0\n", "95\n");
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            mains: true,
            battery: Some(95)
        }
    );
    supplies(root.path(), "0\n", "1\n", "40\n");
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            mains: true,
            battery: Some(40)
        }
    );
    supplies(root.path(), "0\n", "0\n", "40\n");
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            mains: false,
            battery: Some(40)
        }
    );
}

#[test]
fn a_machine_with_no_supplies_is_on_mains() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            mains: true,
            battery: None
        }
    );
}

#[test]
fn a_missing_count_is_an_error_rather_than_a_guess() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    fs::remove_file(root.path().join("allocation/metadata/bytes_used")).unwrap();
    assert!(btrfs_allocation(root.path()).is_err());
}

#[test]
fn the_boot_is_dated_from_the_kernels_statistics() {
    let root = tempfile::tempdir().unwrap();
    let stat = root.path().join("stat");
    fs::write(
        &stat,
        "cpu  1 2 3\nintr 9\nbtime 1760000000\nprocesses 77\n",
    )
    .unwrap();
    let booted = boot(&stat).unwrap();
    assert_eq!(booted.wall, 1_760_000_000.0);
    assert_eq!(booted.ticks, unsafe { libc::sysconf(libc::_SC_CLK_TCK) }
        as f64);
    fs::write(&stat, "cpu  1 2 3\n").unwrap();
    assert!(boot(&stat).is_err());
}

#[test]
fn this_machines_boot_is_in_the_past() {
    let booted = boot(Path::new("/proc/stat")).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    assert!(booted.wall > 0.0 && booted.wall < now);
}
