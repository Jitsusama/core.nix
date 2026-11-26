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
    // Started again with more written since than before, its read alone
    // reading less.
    tally.count(Some((2, 16 << 10)));
    assert_eq!(
        (tally.read, tally.written),
        (17, (2 << 20) + (88 << 10)),
        "a read counter alone reading less wasn't taken for one started again"
    );
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
    assert_eq!(
        said.chars().count(),
        COMMAND_LENGTH,
        "a long command wasn't cut to its length: {said}"
    );
    assert_eq!(
        command_of(b"nix\x00build\x00x\nA build for `y`\x1b[2J\x00a\\nb\x00"),
        "nix build x\\nA build for \\`y\\`\\u{1b}[2J a\\\\nb",
        "a control character, a backtick or a backslash in a client's command wasn't escaped"
    );
    assert_eq!(
        command_of("a\u{2028}b\u{202e}c\u{2067}d\u{200f}e".as_bytes()),
        "a\\u{2028}b\\u{202e}c\\u{2067}d\\u{200f}e",
        "a line separator or a mark turning text round wasn't escaped"
    );
}

#[test]
fn every_separator_and_mark_turning_text_round_is_unsafe_and_its_neighbours_safe() {
    let unsafe_ones = ['\u{061c}', '\u{200e}', '\u{200f}']
        .into_iter()
        .chain('\u{2028}'..='\u{202e}')
        .chain('\u{2066}'..='\u{2069}');
    for c in unsafe_ones {
        assert!(unsafe_to_say(c), "{c:?} was said as it is");
    }
    for c in [
        '\u{061b}', '\u{061d}', '\u{200d}', '\u{2010}', '\u{2027}', '\u{202f}', '\u{2065}',
        '\u{206a}', 'é',
    ] {
        assert!(!unsafe_to_say(c), "{c:?} was escaped");
    }
}

#[test]
fn a_long_command_is_cut_between_escapes() {
    // "nix build " and 106 x's are 116 characters, and the newline's escape
    // would end past the 117 a cut command keeps.
    let long = [
        b"nix\x00build\x00".as_slice(),
        &[b'x'; 106],
        b"\n",
        &[b'y'; 40],
    ]
    .concat();
    assert_eq!(
        command_of(&long),
        format!("nix build {}...", "x".repeat(106)),
        "a long command was cut inside an escape"
    );
}

#[test]
fn the_asker_is_said_from_its_clients_command_and_cgroup() {
    let said = asker(
        b"nix\x00build\x00",
        b"0::/jobs.slice/jobs-work.slice/jobs-work-q\\\xffb.slice/x.scope\n",
    );
    assert_eq!(
        (said.command, said.quest),
        ("nix build".to_owned(), Some("q\\\\\u{fffd}b".to_owned())),
        "the quest wasn't said safely, or a cgroup not UTF-8 lost it"
    );
}

#[test]
fn says_a_quest_as_safely_as_a_command() {
    assert_eq!(
        quest_said("0::/user.slice/jobs.slice/jobs-work.slice/jobs-work-a`\t\\b.slice/x.scope\n"),
        Some("a\\`\\t\\\\b".to_owned()),
        "a quest's name wasn't escaped as a command is"
    );
    assert_eq!(
        quest_said("0::/user.slice/jobs.slice/jobs-work.slice/x.scope\n"),
        None,
        "a process in no quest's slice was given one"
    );
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
    assert_eq!(
        told(&tally).as_deref(),
        Some(
            "A build too short to see who asked for it wrote 0 B and read 4.0 KiB, \
             leaving up to 4.0 KiB more in memory to write."
        ),
        "a build that left writes in memory said other than what it left"
    );
}

#[test]
fn a_build_whose_processes_moved_into_a_cgroup_under_it_stays_seen_once_they_go() {
    let build = std::env::temp_dir().join(format!("vm-first-moved-{}", std::process::id()));
    let payload = build.join("payload");
    fs::create_dir_all(&payload).expect("the temp folder takes a folder");
    fs::write(build.join("cgroup.procs"), "").expect("the temp folder takes a file");
    // A pid past any Linux gives, so no client is found through it.
    fs::write(payload.join("cgroup.procs"), "4194305\n").expect("the temp folder takes a file");
    let mut tallies = HashMap::new();
    let seen = |tallies: &mut HashMap<PathBuf, Tally>| {
        tally(tallies, std::slice::from_ref(&build));
        tallies.get(&build).map(|tally| tally.seen)
    };
    let held = seen(&mut tallies);
    // Its process gone, with no client found, the build is looked over
    // again, and still held one once.
    fs::write(payload.join("cgroup.procs"), "").expect("the temp folder takes a file");
    let after = seen(&mut tallies);
    fs::remove_dir_all(&build).expect("the temp folder gives its folder back");
    assert_eq!(
        (held, after),
        (Some(true), Some(true)),
        "a build whose processes moved under it wasn't seen to hold one, or was \
         forgotten to have once its process went"
    );
}

#[test]
fn a_build_is_known_by_its_cgroup_made_again_and_forgotten_once_gone() {
    let build = std::env::temp_dir().join(format!("vm-first-tally-{}", std::process::id()));
    let before = build.with_extension("before");
    fs::create_dir_all(&build).expect("the temp folder takes a folder");
    let mut tallies = HashMap::new();
    let known = |tallies: &HashMap<PathBuf, Tally>| {
        tallies.get(&build).map(|tally| {
            (
                tally.cgroup,
                tally.seen,
                tally.asked.is_some(),
                tally.written,
            )
        })
    };
    tally(&mut tallies, std::slice::from_ref(&build));
    let first = known(&tallies);
    let first_inode = fs::metadata(&build).map(|meta| meta.ino()).ok();
    // What the first build counted and saw must stay with it, so a tally
    // taken over by the build after it would be told apart.
    if let Some(first) = tallies.get_mut(&build) {
        first.written = 1 << 20;
        first.seen = true;
    }
    // Nix makes the next build of the same user in a cgroup of the same
    // name; made while the first stands, it can't take the first's inode.
    fs::rename(&build, &before).expect("the temp folder renames a folder");
    fs::create_dir(&build).expect("the temp folder takes a folder");
    fs::remove_dir(&before).expect("the temp folder gives its folder back");
    tally(&mut tallies, std::slice::from_ref(&build));
    let again = known(&tallies);
    let again_inode = fs::metadata(&build).map(|meta| meta.ino()).ok();
    fs::remove_dir(&build).expect("the temp folder gives its folder back");
    tally(&mut tallies, &[]);
    assert_eq!(
        first,
        first_inode.map(|inode| (inode, false, false, 0)),
        "a build with no process wasn't known by its cgroup's inode, unseen and unasked"
    );
    assert_eq!(
        again,
        again_inode.map(|inode| (inode, false, false, 0)),
        "a cgroup made again was taken for the build before it, or its tally for the new one"
    );
    assert!(tallies.is_empty(), "a build gone was still tallied");
}

#[test]
fn counts_what_a_build_left_to_write() {
    let stat = "anon 4096\nfile 9000\nfile_dirty 3000\nfile_writeback 500\n\
                file_mapped 0\n";
    assert_eq!(parse_unwritten(stat), 3500);
    assert_eq!(parse_unwritten(""), 0);
}

#[test]
fn names_the_quest_of_its_slice_in_a_tier_or_none() {
    let work = "0::/user.slice/jobs.slice/jobs-work.slice";
    let said = |slices: &str| quest_said(&format!("{work}{slices}/work-7-8.scope\n"));
    assert_eq!(
        [
            said("/jobs-work-qestabc.slice"),
            said("/jobs-work-first.slice/jobs-work-first-qestabc.slice"),
            said("/jobs-work-unranked.slice/jobs-work-unranked-qestabc.slice"),
            said("/jobs-work-unranked.slice"),
            said(""),
        ],
        [
            Some("qestabc".to_owned()),
            Some("qestabc".to_owned()),
            Some("qestabc".to_owned()),
            None,
            None
        ],
        "a quest in its tier was said as its tier, or work for none given a quest"
    );
}
