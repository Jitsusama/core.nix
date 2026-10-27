use disk_guard::band::BtrfsSpace;
use disk_guard::linux::{
    Commits, boot, btrfs_allocation, commits, committed, patiently, power, pressure, space, tick,
};
use disk_guard::moments::{Power, Stall};
use std::cell::Cell;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

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

/// A laptop's supplies as one shows them: a charger and a USB-C source for
/// the system, its battery, and a mouse's battery.
fn supplies(root: &Path, charger: &str, usb: &str, capacity: &str) {
    text(root, "AC/type", "Mains\n");
    text(root, "AC/online", charger);
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
            on_charger: true,
            battery: Some(95)
        }
    );
    supplies(root.path(), "0\n", "1\n", "40\n");
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            on_charger: true,
            battery: Some(40)
        }
    );
    supplies(root.path(), "0\n", "0\n", "40\n");
    assert_eq!(
        power(root.path()).unwrap(),
        Power {
            on_charger: false,
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
            on_charger: true,
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
    crate::assert_exactly(booted.wall, 1_760_000_000.0);
    let ticks = u64::try_from(unsafe { libc::sysconf(libc::_SC_CLK_TCK) }).unwrap();
    crate::assert_exactly(booted.ticks, disk_guard::units::float(ticks));
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

const PATIENCE: Duration = Duration::from_millis(100);

#[test]
fn a_commit_in_time_is_all_settling_takes() {
    let stuck = Arc::new(AtomicBool::new(false));
    let fell_back = Cell::new(false);
    patiently(
        &stuck,
        PATIENCE,
        || Ok(()),
        || {
            fell_back.set(true);
            Ok(())
        },
    )
    .unwrap();
    assert!(!fell_back.get());
    let failing = patiently(&stuck, PATIENCE, || Err(io::Error::other("no")), || Ok(()));
    assert_eq!(failing.unwrap_err().to_string(), "no");
}

#[test]
fn a_commit_that_hangs_is_given_up_on_for_the_fallback() {
    let stuck = Arc::new(AtomicBool::new(false));
    let (release, released) = mpsc::channel::<()>();
    let (done, finished) = mpsc::channel::<()>();
    let started = Instant::now();
    patiently(
        &stuck,
        PATIENCE,
        move || {
            // Hangs until the fallback runs, as a commit nobody starts does.
            let _ = released.recv_timeout(Duration::from_secs(5));
            done.send(()).unwrap();
            Ok(())
        },
        || {
            release.send(()).unwrap();
            Ok(())
        },
    )
    .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    finished.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while stuck.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!stuck.load(Ordering::SeqCst), "the hung commit came back");
}

#[test]
fn while_a_commit_is_stuck_no_other_is_begun() {
    let stuck = Arc::new(AtomicBool::new(true));
    let (beginning, begun) = mpsc::channel::<()>();
    let fell_back = Cell::new(false);
    let started = Instant::now();
    patiently(
        &stuck,
        PATIENCE,
        move || {
            beginning.send(()).unwrap();
            Ok(())
        },
        || {
            fell_back.set(true);
            Ok(())
        },
    )
    .unwrap();
    assert!(fell_back.get());
    assert!(
        started.elapsed() < PATIENCE,
        "no waiting on a commit known stuck"
    );
    assert!(begun.recv_timeout(Duration::from_millis(200)).is_err());
}

#[test]
fn commits_are_read_from_the_filesystems_stats() {
    let dir = tempfile::tempdir().unwrap();
    let stats = dir.path().join("commit_stats");
    let shown = |pending: u64| {
        format!(
            "commits 1106\ncur_commit_ms {pending}\nlast_commit_ms 251\nmax_commit_ms 3019\n\
             total_commit_ms 182474\n"
        )
    };
    fs::write(&stats, shown(0)).unwrap();
    assert_eq!(
        commits(&stats).unwrap(),
        Commits {
            done: 1106,
            underway: false
        }
    );
    fs::write(&stats, shown(161)).unwrap();
    assert!(commits(&stats).unwrap().underway);
    fs::write(&stats, "cur_commit_ms 0\n").unwrap();
    assert!(commits(&stats).is_err(), "no count is no guess");
}

/// Gives `counts` one after another, keeping the last, as a filesystem's
/// stats read on each look would.
fn reading<const N: usize>(counts: [Commits; N]) -> impl FnMut() -> io::Result<Commits> {
    let mut counts = counts.into_iter();
    let mut last = None;
    move || {
        last = counts.next().or(last);
        last.ok_or_else(|| io::Error::other("nothing to read"))
    }
}

const LOOK: Duration = Duration::from_millis(1);
const LONGEST: Duration = Duration::from_secs(5);

fn at(done: u64, underway: bool) -> Commits {
    Commits { done, underway }
}

#[test]
fn settling_waits_for_the_next_commit() {
    let looks = Cell::new(0);
    let mut read = reading([at(10, false), at(10, false), at(10, true), at(11, false)]);
    let looked = || {
        looks.set(looks.get() + 1);
        read()
    };
    committed(looked, || false, LOOK, LONGEST).unwrap();
    assert_eq!(looks.get(), 4, "done once the count moved");
}

#[test]
fn a_commit_under_way_as_settling_starts_is_not_the_one_waited_for() {
    // It may have closed before what was deleted, so the one after it is.
    let looks = Cell::new(0);
    let mut read = reading([at(10, true), at(11, false), at(11, true), at(12, false)]);
    let looked = || {
        looks.set(looks.get() + 1);
        read()
    };
    committed(looked, || false, LOOK, LONGEST).unwrap();
    assert_eq!(looks.get(), 4);
}

#[test]
fn a_stop_ends_a_wait_for_a_commit_that_never_comes() {
    // A wait in the kernel held a stop 23 s at a switch; this one ends at
    // the next look.
    let looks = Cell::new(0);
    let started = Instant::now();
    let waited = committed(
        reading([at(10, false)]),
        || {
            looks.set(looks.get() + 1);
            looks.get() > 3
        },
        LOOK,
        LONGEST,
    );
    assert_eq!(waited.unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert!(started.elapsed() < LONGEST / 2, "{:?}", started.elapsed());
}

#[test]
fn a_commit_that_never_comes_is_waited_for_no_longer_than_the_longest() {
    // Nothing dirty, nothing to commit: the frees show at the next commit.
    let started = Instant::now();
    committed(
        reading([at(10, false)]),
        || false,
        LOOK,
        Duration::from_millis(50),
    )
    .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_count_that_cant_be_read_fails_the_settle() {
    let failed = committed(reading([]), || false, LOOK, LONGEST);
    assert_eq!(failed.unwrap_err().to_string(), "nothing to read");
}
