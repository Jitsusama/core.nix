use disk_guard::clock::Tick;
use disk_guard::disks::{self, Disks, Reading, Watch, Wear};
use std::cell::RefCell;
use std::rc::Rc;

/// A kernel's `stat` line for a disk that has written `sectors`.
fn stat(sectors: u64) -> String {
    format!(
        "557333479 1870035 27838072019 1247833829 312637040 12980134 {sectors} 1332444195 0 36876858 2585922169 9860189 0 3944338832 3099218 1039778 2544926\n"
    )
}

#[test]
fn a_disks_stat_says_what_it_wrote_since_boot_in_bytes() {
    assert_eq!(
        disks::written_since_boot(&stat(8_945_884_093)),
        Some(8_945_884_093 * 512)
    );
    assert_eq!(disks::written_since_boot("1 2 3"), None, "too short");
    assert_eq!(disks::written_since_boot(""), None);
}

/// An NVMe health log whose wear is `used` percent of its rating, having
/// written `units` thousands of 512-byte blocks in its life.
fn health(used: u8, units: u128) -> Vec<u8> {
    let mut log = vec![0u8; 512];
    log[5] = used;
    // Data units read, which isn't what wears it, so it must be left out.
    log[32..48].copy_from_slice(&7u128.to_le_bytes());
    log[48..64].copy_from_slice(&units.to_le_bytes());
    log
}

#[test]
fn an_nvme_health_log_says_its_wear_and_what_it_wrote_in_its_life() {
    assert_eq!(
        disks::health(&health(2, 80_000_000)),
        Some(Wear {
            used: 2,
            written: 80_000_000 * 512_000,
        })
    );
    assert_eq!(disks::health(&[0u8; 63]), None, "a log cut short");
}

/// Disks a test sets, counting how often their health was asked for.
#[derive(Clone, Default)]
struct Set {
    readings: Rc<RefCell<Vec<Reading>>>,
    asked: Rc<RefCell<Vec<bool>>>,
}

impl Disks for Set {
    fn read(&mut self, health: bool) -> Vec<Reading> {
        self.asked.borrow_mut().push(health);
        self.readings
            .borrow()
            .iter()
            .map(|reading| Reading {
                wear: if health { reading.wear } else { None },
                ..reading.clone()
            })
            .collect()
    }
}

impl Set {
    fn wrote(&self, written: u64, wear: Option<Wear>) {
        *self.readings.borrow_mut() = vec![Reading {
            name: "nvme0n1".into(),
            written,
            wear,
        }];
    }
}

/// A tick `awake` seconds into a boot, having slept `suspended` more.
fn at(awake: f64, suspended: f64) -> Tick {
    Tick {
        wall: 1e9 + awake + suspended,
        monotonic: awake,
        suspended,
        boot: "test".into(),
    }
}

const GB: u64 = 1_000_000_000;

/// Whether a rate is there and within a byte an hour of `rate`.
fn near(said: Option<f64>, rate: f64) -> bool {
    said.is_some_and(|said| (said - rate).abs() < 1.0)
}

#[test]
fn the_rate_is_what_the_last_awake_hour_wrote_once_ten_minutes_are_watched() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(100 * GB, None);
    watch.observe(&at(7200.0, 0.0));
    set.wrote(101 * GB, None);
    watch.observe(&at(7500.0, 0.0));
    let status = watch.status();
    assert_eq!(status[0].name, "nvme0n1");
    assert_eq!(status[0].written_this_boot, 101 * GB);
    assert_eq!(
        status[0].written_per_hour, None,
        "five minutes isn't enough"
    );

    set.wrote(104 * GB, None);
    watch.observe(&at(9000.0, 0.0));
    // 4 GB over 30 minutes awake.
    let said = watch.status()[0].written_per_hour;
    assert!(near(said, 8.0 * 1e9), "{said:?}");

    // An hour on, the first readings have gone from the rate: 30 GB over
    // the hour from 9,000 s on.
    set.wrote(134 * GB, None);
    watch.observe(&at(12_600.0, 0.0));
    let said = watch.status()[0].written_per_hour;
    assert!(near(said, 30.0 * 1e9), "{said:?}");
}

#[test]
fn health_is_asked_once_an_awake_hour_and_kept_between() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    let worn = Some(Wear {
        used: 2,
        written: 40_000 * GB,
    });
    set.wrote(100 * GB, worn);
    for awake in [3600.0, 3660.0, 5000.0, 7200.0, 7260.0] {
        watch.observe(&at(awake, 0.0));
    }
    assert_eq!(*set.asked.borrow(), [true, false, false, true, false]);
    assert_eq!(watch.status()[0].used_percent, Some(2));
    assert_eq!(watch.status()[0].written_in_life, Some(40_000 * GB));

    // A clock that went back is a new one, so it asks again.
    watch.observe(&at(100.0, 0.0));
    assert_eq!(set.asked.borrow().last(), Some(&true));
}

#[test]
fn a_disk_that_stops_answering_for_its_health_says_none_at_the_next_ask() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(
        100 * GB,
        Some(Wear {
            used: 2,
            written: 40_000 * GB,
        }),
    );
    watch.observe(&at(3600.0, 0.0));
    set.wrote(101 * GB, None);
    watch.observe(&at(5000.0, 0.0));
    assert_eq!(watch.status()[0].used_percent, Some(2), "kept between asks");
    watch.observe(&at(7200.0, 0.0));
    assert_eq!(
        watch.status()[0].used_percent,
        None,
        "the ask went unanswered"
    );
    assert_eq!(watch.status()[0].written_in_life, None);
}

#[test]
fn a_disk_that_goes_is_no_longer_said() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(100 * GB, None);
    watch.observe(&at(3600.0, 0.0));
    set.readings.borrow_mut().clear();
    watch.observe(&at(3700.0, 0.0));
    assert_eq!(watch.status(), []);
}

#[test]
fn its_rated_wear_comes_at_the_pace_since_boot_slept_time_and_all() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    // 2% worn by 40 TB: 20 TB a percent, 1,960 TB to its rating. Written
    // 1 TB in a boot of 4 h awake and 6 h asleep is 100 GB an hour.
    set.wrote(
        1000 * GB,
        Some(Wear {
            used: 2,
            written: 40_000 * GB,
        }),
    );
    watch.observe(&at(4.0 * 3600.0, 6.0 * 3600.0));
    let in_hours = watch.status()[0].rated_wear_in.unwrap() / 3600.0;
    assert!((in_hours - 19_600.0).abs() < 1e-6, "{in_hours}");
}

#[test]
fn no_rated_wear_is_said_until_it_can_be() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    let unworn = Wear {
        used: 0,
        written: 100 * GB,
    };
    set.wrote(10 * GB, Some(unworn));
    watch.observe(&at(7200.0, 0.0));
    assert_eq!(watch.status()[0].rated_wear_in, None, "nothing worn yet");

    // Under an hour into a boot says nothing of its pace.
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(10 * GB, Some(Wear { used: 3, ..unworn }));
    watch.observe(&at(600.0, 0.0));
    assert_eq!(watch.status()[0].rated_wear_in, None);

    // Nothing written this boot has no pace.
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(0, Some(Wear { used: 3, ..unworn }));
    watch.observe(&at(7200.0, 0.0));
    assert_eq!(watch.status()[0].rated_wear_in, None);

    // Past its rating, its wear says so and there's no time left to say.
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(
        10 * GB,
        Some(Wear {
            used: 104,
            ..unworn
        }),
    );
    watch.observe(&at(7200.0, 0.0));
    assert_eq!(watch.status()[0].used_percent, Some(104));
    assert_eq!(watch.status()[0].rated_wear_in, None);
}

#[test]
fn a_count_that_goes_back_starts_the_rate_again() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    set.wrote(100 * GB, None);
    watch.observe(&at(7200.0, 0.0));
    set.wrote(GB, None);
    watch.observe(&at(8400.0, 0.0));
    assert_eq!(watch.status()[0].written_per_hour, None);
}

/// A count that goes back is another disk under the same name, whose
/// wear isn't the last one's: it's dropped, and asked at the next step.
#[test]
fn a_count_that_goes_back_drops_the_wear_and_asks_again() {
    let set = Set::default();
    let mut watch = Watch::new(Box::new(set.clone()));
    let worn = Some(Wear {
        used: 9,
        written: 400 * GB,
    });
    set.wrote(100 * GB, worn);
    watch.observe(&at(3600.0, 0.0));
    set.wrote(GB, worn);
    watch.observe(&at(3700.0, 0.0));
    assert_eq!(watch.status()[0].used_percent, None);
    watch.observe(&at(3800.0, 0.0));
    assert_eq!(*set.asked.borrow(), [true, false, true]);
    assert_eq!(watch.status()[0].used_percent, Some(9));
}

#[cfg(target_os = "linux")]
#[test]
fn only_a_disk_with_a_device_is_read_and_one_whose_health_cant_be_read_has_none() {
    let root = tempfile::tempdir().unwrap();
    let block = root.path().join("block");
    let entry = |name: &str, device: Option<&str>, sectors: u64| {
        let at = block.join(name);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("stat"), stat(sectors)).unwrap();
        std::fs::write(at.join("ro"), "0\n").unwrap();
        std::fs::write(at.join("size"), "2000409264\n").unwrap();
        if let Some(device) = device {
            std::os::unix::fs::symlink(device, at.join("device")).unwrap();
        }
        at
    };
    entry("loop0", None, 9);
    entry("dm-0", None, 9);
    entry("nvme0n1", Some("../../nvme0"), 1000);
    entry("sda", Some("../../../0:0:0:0"), 2000);
    // An empty card reader's slot, and an eMMC's read-only boot area.
    let empty = entry("sdb", Some("../../../1:0:0:0"), 0);
    std::fs::write(empty.join("size"), "0\n").unwrap();
    let boot = entry("mmcblk0boot0", Some("../../mmc0:0001"), 0);
    std::fs::write(boot.join("ro"), "1\n").unwrap();
    // The controller a health log is asked of, a plain file here, which
    // refuses the ask as a disk that isn't NVMe would.
    let dev = root.path().join("dev");
    std::fs::create_dir_all(&dev).unwrap();
    std::fs::write(dev.join("nvme0"), "").unwrap();
    let mut sysfs = disks::Sysfs::new(block, dev);
    let read = sysfs.read(true);
    assert_eq!(
        read,
        [
            Reading {
                name: "nvme0n1".into(),
                written: 1000 * 512,
                wear: None,
            },
            Reading {
                name: "sda".into(),
                written: 2000 * 512,
                wear: None,
            },
        ]
    );
}

/// Each NVMe disk's health is asked of its own controller: the one its
/// `device` names, or under native multipath the first its subsystem
/// holds. Nothing else is asked.
#[cfg(target_os = "linux")]
#[test]
fn each_nvme_disks_health_is_asked_of_its_own_controller() {
    let root = tempfile::tempdir().unwrap();
    let block = root.path().join("block");
    for (name, device) in [
        ("nvme0n1", "../../nvme0"),
        ("nvme1n1", "../../nvme-subsys1"),
        ("sda", "../../../0:0:0:0"),
    ] {
        let at = block.join(name);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join("stat"), stat(8)).unwrap();
        std::os::unix::fs::symlink(device, at.join("device")).unwrap();
    }
    // A subsystem holds its controllers, and its own files beside them.
    let subsystem = root.path().join("nvme-subsys1");
    for held in ["nvme2", "nvme1", "nvme1c1n1"] {
        std::fs::create_dir_all(subsystem.join(held)).unwrap();
    }
    std::fs::write(subsystem.join("subsysnqn"), "nqn\n").unwrap();
    let dev = root.path().join("dev");
    let asked = Rc::new(RefCell::new(Vec::new()));
    let log = health(2, 80_000_000);
    let mut sysfs = disks::Sysfs::asking(block, dev.clone(), {
        let asked = Rc::clone(&asked);
        Box::new(move |controller| {
            asked.borrow_mut().push(controller.to_owned());
            Some(log.clone())
        })
    });
    let read = sysfs.read(true);
    // The kernel lists its disks in no order of its own.
    asked.borrow_mut().sort();
    assert_eq!(*asked.borrow(), [dev.join("nvme0"), dev.join("nvme1")]);
    let worn = Some(Wear {
        used: 2,
        written: 80_000_000 * 512_000,
    });
    let wear: Vec<_> = read.iter().map(|reading| reading.wear).collect();
    assert_eq!(wear, [worn, worn, None]);

    asked.borrow_mut().clear();
    sysfs.read(false);
    assert!(asked.borrow().is_empty(), "health asked unasked");
}
