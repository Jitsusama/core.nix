//! The tally's tests: what a build is said to have read and written, and
//! who is said to have asked for it.

use super::*;

#[test]
fn reads_what_a_build_wrote_and_read_on_every_disk() {
    let stat = "259:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n\
                8:0 rbytes=5 wbytes=10 rios=1 wios=1 dbytes=0 dios=0\n";
    assert_eq!(parse_io_stat(stat, |_| true), (105, 2058));
    assert_eq!(parse_io_stat("", |_| true), (0, 0));
}

#[test]
fn keeps_what_a_build_wrote_when_its_counters_start_again() {
    let mut tally = Tally::new(1);
    tally.count(Some((10, 1 << 20)));
    tally.count(Some((10, 2 << 20)));
    // The daemon restarts: its io controller is taken back, so the
    // build's io.stat goes, and comes back counting from nothing.
    tally.count(None);
    tally.count(Some((0, 64 << 10)));
    assert_eq!((tally.read, tally.written), (10, (2 << 20) + (64 << 10)));
    // Taken back and handed again between two looks.
    tally.count(Some((0, 4 << 10)));
    tally.count(Some((5, 8 << 10)));
    assert_eq!((tally.read, tally.written), (15, (2 << 20) + (72 << 10)));
}

#[test]
fn counts_a_write_once_however_many_layers_it_passes() {
    // As a laptop's build.slice reads: zram, then the disk, then the
    // LUKS mapper on it, holding the disk's bytes again.
    let stat = "253:0 rbytes=7 wbytes=70 rios=1 wios=1 dbytes=0 dios=0\n\
                259:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n\
                254:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n";
    assert_eq!(parse_io_stat(stat, |device| device == "259:0"), (100, 2048));
}

#[test]
fn says_sizes_as_a_person_reads_them() {
    assert_eq!(size(0), "0 B");
    assert_eq!(size(1023), "1023 B");
    assert_eq!(size(1536), "1.5 KiB");
    assert_eq!(size(64 << 20), "64.0 MiB");
    assert_eq!(size(3 << 30), "3.0 GiB");
    assert_eq!(size(5 << 40), "5.0 TiB");
}

#[test]
fn names_a_client_by_its_command() {
    assert_eq!(
        command_of(b"/run/current-system/sw/bin/nix-build\x00--store\x00daemon\x00/etc/a.nix\x00"),
        "nix-build --store daemon /etc/a.nix"
    );
    let long = [b"nix\x00build\x00".as_slice(), &[b'x'; 400]].concat();
    let said = command_of(&long);
    assert!(said.starts_with("nix build xxx"), "{said}");
    assert!(said.ends_with("..."), "{said}");
    assert!(said.chars().count() <= 120, "{said}");
}

#[test]
fn says_which_quest_asked_for_a_build_and_what_it_wrote() {
    let mut tally = Tally {
        cgroup: 1,
        read: 0,
        written: 1 << 30,
        counter: (0, 1 << 30),
        unwritten: 0,
        seen: true,
        asked: Some(Asker {
            command: "nix build .#checks".into(),
            quest: Some("qestabc".into()),
        }),
    };
    assert_eq!(
        told(&tally).as_deref(),
        Some("A build for `nix build .#checks`, for quest qestabc, wrote 1.0 GiB and read 0 B.")
    );
    tally.asked = Some(Asker {
        command: "nix-build x".into(),
        quest: None,
    });
    assert_eq!(
        told(&tally).as_deref(),
        Some("A build for `nix-build x`, for no quest, wrote 1.0 GiB and read 0 B.")
    );
    tally.asked = None;
    assert_eq!(
        told(&tally).as_deref(),
        Some("A build whose client couldn't be found wrote 1.0 GiB and read 0 B.")
    );
    tally.unwritten = 64 << 20;
    assert_eq!(
        told(&tally).as_deref(),
        Some(
            "A build whose client couldn't be found wrote 1.0 GiB and read 0 B, \
             leaving up to 64.0 MiB more in memory to write."
        )
    );
}

#[test]
fn a_build_gone_before_a_look_says_so_and_one_that_wrote_nothing_says_nothing() {
    let mut tally = Tally {
        cgroup: 1,
        read: 4096,
        written: 1 << 20,
        counter: (4096, 1 << 20),
        unwritten: 0,
        seen: false,
        asked: None,
    };
    assert_eq!(
        told(&tally).as_deref(),
        Some("A build too short to see who asked for it wrote 1.0 MiB and read 4.0 KiB.")
    );
    tally.written = 0;
    assert_eq!(told(&tally), None);
    tally.unwritten = 4096;
    assert!(
        told(&tally).is_some(),
        "a build that left writes in memory said nothing"
    );
}

#[test]
fn counts_what_a_build_left_to_write() {
    let stat = "anon 4096\nfile 9000\nfile_dirty 3000\nfile_writeback 500\n\
                file_mapped 0\n";
    assert_eq!(parse_unwritten(stat), 3500);
    assert_eq!(parse_unwritten(""), 0);
}

#[test]
fn names_the_quest_of_its_slice() {
    assert_eq!(
        quest_of("/user.slice/jobs.slice/jobs-work.slice/jobs-work-qestabc.slice"),
        Some("qestabc")
    );
    assert_eq!(quest_of("/user.slice/jobs.slice/jobs-work.slice"), None);
}
