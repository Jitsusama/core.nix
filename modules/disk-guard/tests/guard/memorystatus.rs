use disk_guard::memorystatus::room;
use disk_guard::moments::{MemoryLevel, MemoryRoom};

#[test]
fn each_level_the_kernel_gives_reads_as_its_own() {
    for (level, read) in [
        (1, MemoryLevel::Normal),
        (2, MemoryLevel::Warning),
        (4, MemoryLevel::Critical),
    ] {
        assert_eq!(
            room(35, level),
            Some(MemoryRoom {
                free_percent: 35,
                level: read
            }),
            "level {level}"
        );
    }
}

#[test]
fn a_value_the_kernel_never_gives_reads_as_nothing() {
    assert_eq!(room(35, 0), None, "no level 0");
    assert_eq!(room(35, 3), None, "no level 3");
    assert_eq!(room(-1, 1), None, "no share under nothing");
    assert_eq!(room(101, 1), None, "no share past all");
    assert!(room(0, 4).is_some(), "nothing free is a share it gives");
    assert!(room(100, 1).is_some(), "all free is a share it gives");
}
