//! vm-first's tests of how it reads threads, the daemon and its builds, and
//! how it weighs and schedules them, beside it as the tally's are.

use super::*;

// Only Linux has a /proc to read.
#[cfg(target_os = "linux")]
#[test]
fn a_process_is_read_through_its_open_folder_until_it_is_gone() {
    let own = Proc::open(std::process::id()).expect("this process's folder wouldn't open");
    assert_eq!(
        (own.read("cmdline"), own.owner()),
        (
            fs::read("/proc/self/cmdline").ok(),
            fs::metadata("/proc/self").ok().map(|meta| meta.uid())
        ),
        "a process's own files or owner weren't read through its folder"
    );
    let mut child = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("sleep wouldn't start");
    let gone = Proc::open(child.id()).expect("the child's folder wouldn't open");
    child.kill().expect("the child wouldn't stop");
    child.wait().expect("the child wasn't reaped");
    assert_eq!(
        gone.read("cmdline"),
        None,
        "a process reaped was still read, as its pid's next process could be"
    );
}

// Only Linux has a /proc to read.
#[cfg(target_os = "linux")]
#[test]
fn a_process_named_is_opened_only_if_it_started_no_later_than_its_namer() {
    let started = |proc: &Proc| parse_started(&proc.text("stat").expect("no stat"));
    let own = Proc::open(std::process::id()).expect("this process's folder wouldn't open");
    let own_at = started(&own).expect("this process's start wouldn't read");
    // Start times count clock ticks of 10 ms, so a child made within one
    // of this process starting would start in the same tick; made 30 ms
    // after it, the child starts at least a tick later.
    std::thread::sleep(std::time::Duration::from_millis(30));
    let mut child = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("sleep wouldn't start");
    let child_at = Proc::open(child.id()).as_ref().and_then(started);
    let younger = Proc::open_older(child.id(), own_at).is_some();
    let itself = Proc::open_older(std::process::id(), own_at).is_some();
    let parent = Proc::open_older(std::os::unix::process::parent_id(), own_at).is_some();
    child.kill().expect("the child wouldn't stop");
    child.wait().expect("the child wasn't reaped");
    assert!(
        child_at.is_some_and(|at| at > own_at),
        "the child didn't start a tick later, so the test proves nothing"
    );
    assert_eq!(
        (younger, itself, parent),
        (false, true, true),
        "a process started later than the one naming it was opened, or one started in \
         its tick or before wasn't"
    );
}

#[test]
fn a_build_s_first_process_counts_only_while_in_its_cgroup_or_one_under_it() {
    let build = Path::new("/sys/fs/cgroup/build.slice/nix-daemon.service/nix-build-a.slice");
    let daemon = "0::/build.slice/nix-daemon.service";
    // A cgroup whose name only starts with the build's is another's, and
    // only the unified hierarchy's line says where a process sits.
    for (said, within) in [
        (format!("{daemon}/nix-build-a.slice\n"), true),
        (format!("{daemon}/nix-build-a.slice/payload\n"), true),
        (format!("{daemon}/nix-build-ab.slice\n"), false),
        (format!("{daemon}/nix-build-a.slice-x\n"), false),
        (format!("{daemon}/nix-build-a.sliced\n"), false),
        ("0::/user.slice\n".to_owned(), false),
        (
            "1:cpu:/build.slice/nix-daemon.service/nix-build-a.slice\n0::/user.slice\n".to_owned(),
            false,
        ),
    ] {
        assert_eq!(
            in_cgroup(&said, build),
            within,
            "{said:?} was read as in the build's cgroup when it isn't, or out when it is"
        );
    }
}

#[test]
fn reads_when_a_process_started_past_a_name_with_spaces() {
    let stat = "4242 (a b) c) S 1 4242 4242 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 98765 1 2";
    assert_eq!(
        (parse_started(stat), parse_started("4242 (cc1) S 1")),
        (Some(98765), None),
        "a start wasn't read past a name with spaces, or was read from too few fields"
    );
    // A name that isn't UTF-8, decoded with stand-ins, still leaves the
    // fields past it to read; that Proc::text decodes it so is pinned on
    // Linux, below.
    let lossy = [
        b"4242 (a\xff\xfe) S 1 4242 4242 0 -1 4194560".as_slice(),
        b" 100 0 0 0 1 2 0 0 20 0 1 0 98765 1 2",
    ]
    .concat();
    let lossy = String::from_utf8_lossy(&lossy);
    assert_eq!(
        (parse_started(&lossy), parse_ppid(&lossy)),
        (Some(98765), Some(1)),
        "a name that isn't UTF-8 lost the fields past it"
    );
}

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn no_flags_watch_the_daemon_for_every_timed_program() {
    let settings = settings(&[]).expect("no flags should be its service's settings");
    assert_eq!(
        settings.cgroup,
        PathBuf::from("/sys/fs/cgroup/build.slice/nix-daemon.service"),
        "the daemon's own cgroup should be watched"
    );
    assert_eq!(
        settings.names.len(),
        TIMED.len(),
        "every timed program should be named"
    );
}

#[test]
fn named_programs_take_the_place_of_the_timed_ones() {
    let settings = settings(&words("--name niri-* --nice -5 --every 250"))
        .expect("each flag here should be taken");
    assert_eq!(
        settings.names,
        ["niri-*"],
        "only the named program should be brought ahead"
    );
    assert_eq!(settings.nice, -5, "the nice asked for should be kept");
    assert_eq!(
        settings.every,
        Duration::from_millis(250),
        "it should look as often as asked"
    );
}

#[test]
fn a_flag_asked_wrongly_is_a_usage_failure() {
    for line in [
        "--nice",
        "--nice -40",
        "--weight 0",
        "--every soon",
        "--quick",
        "--cgroup sys/fs/cgroup/build.slice",
        "--cgroup /sys/fs/cgroup/../build.slice",
        "--cgroup /run/build.slice",
        "--cgroup /sys/fs/cgroupx/build.slice",
        "--cgroup /sys/fs/cgroup",
        "--cgroup /sys/fs/cgroup/",
    ] {
        let exit = settings(&words(line)).err().map(|failure| failure.exit);
        assert_eq!(
            exit,
            Some(Exit::Usage),
            "{line} should be refused as asked wrongly"
        );
    }
}

fn defaults() -> Settings {
    Settings {
        cgroup: PathBuf::new(),
        names: TIMED.iter().map(|&name| name.into()).collect(),
        nice: -10,
        weight: 1000,
        every: Duration::from_millis(1000),
    }
}

/// A stat line with the given name, nice and policy, the other fields
/// as a sleeping process has them.
fn stat(comm: &str, nice: i32, policy: i32) -> String {
    let mut fields = vec!["0".to_string(); 52 - 2];
    fields[0] = "S".into();
    fields[19 - 3] = nice.to_string();
    fields[41 - 3] = policy.to_string();
    format!("4242 ({comm}) {}", fields.join(" "))
}

#[test]
fn hands_builds_the_controllers_the_daemon_has_and_has_not_handed() {
    assert_eq!(
        to_enable("cpuset cpu io memory hugetlb pids\n", "\n").as_deref(),
        Some("+cpu +io +memory +pids")
    );
    // As a daemon whose unit delegates no controllers is given them, without cpu.
    assert_eq!(
        to_enable("io memory pids\n", "memory\n").as_deref(),
        Some("+io +pids")
    );
    assert_eq!(
        to_enable("cpu io memory pids\n", "cpu io memory pids\n"),
        None
    );
    assert_eq!(to_enable("", ""), None);
}

#[test]
fn knows_the_daemon_from_its_workers() {
    assert!(
        is_daemon(b"/nix/store/abc-nix/bin/nix-daemon\x00--daemon\x00"),
        "the daemon run by its store path wasn't known"
    );
    assert!(
        is_daemon(b"nix-daemon\x00--daemon\x00"),
        "the daemon run by its bare name wasn't known"
    );
    // A worker, which writes its client's pid over --daemon.
    assert!(
        !is_daemon(b"nix-daemon\x00307605\x00"),
        "a worker was taken for the daemon"
    );
    assert!(
        !is_daemon(b"bash\x00--daemon\x00"),
        "another program given --daemon was taken for the daemon"
    );
    assert!(
        !is_daemon(b""),
        "an empty command line was taken for the daemon"
    );
}

#[test]
fn the_daemon_is_aside_only_once_it_has_moved_itself() {
    let workers_left = |pid: &str| match pid {
        "696" | "702" => b"nix-daemon\x00307605\x00".to_vec(),
        _ => Vec::new(),
    };
    // A restarted daemon's workers stay behind in its cgroup, the new
    // daemon not yet among them.
    assert!(
        !daemon_aside("696\n702\n", workers_left),
        "workers left behind were taken for the daemon moved aside"
    );
    assert!(
        !daemon_aside("", workers_left),
        "an empty cgroup was taken for the daemon moved aside"
    );

    let moved = |pid: &str| match pid {
        "966" => b"nix-daemon\x00--daemon\x00".to_vec(),
        other => workers_left(other),
    };
    assert!(
        daemon_aside("696\n702\n966\n", moved),
        "the daemon among its workers wasn't seen to have moved aside"
    );
}

#[test]
fn finds_the_builds_cgroups_and_nothing_else() {
    let root = std::env::temp_dir().join(format!("vm-first-{}", std::process::id()));
    for dir in ["nix-daemon", "nix-build-uid-872415232", "nix-build-pid-7-0"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    fs::write(root.join("nix-build-file"), "").unwrap();

    let mut found = build_cgroups(&root);
    found.sort();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(
        found,
        vec![
            root.join("nix-build-pid-7-0"),
            root.join("nix-build-uid-872415232")
        ]
    );
}

#[test]
fn a_vms_build_is_weighed_for_cpu_and_disk() {
    assert_eq!(
        weight_files(1000),
        [
            ("cpu.weight", "1000".to_string()),
            ("io.weight", "default 1000".to_string())
        ]
    );
}

#[test]
fn a_daemon_worker_names_its_client() {
    // Nix writes the client's pid over "--daemon", padding with NULs.
    assert_eq!(
        client_of(b"nix-daemon\x00307605\x00\x00\x00", 0),
        Some(307_605)
    );
    assert_eq!(
        client_of(b"/nix/store/x-nix/bin/nix-daemon\x0042\x00", 0),
        Some(42)
    );
    assert_eq!(client_of(b"nix-daemon\x00--daemon\x00", 0), None);
    assert_eq!(client_of(b"bash\x00123\x00", 0), None);
    assert_eq!(client_of(b"", 0), None);
    // A build's process, run as a build user, naming itself a worker.
    assert_eq!(
        client_of(b"nix-daemon\x00307605\x00", 30_001),
        None,
        "a build's own process was taken for a daemon worker"
    );
}

#[test]
fn reads_a_parent() {
    let mut line = stat("cc1", 0, 3);
    line = line.replacen(" S 0 ", " S 307960 ", 1);
    assert_eq!(parse_ppid(&line), Some(307_960));
    assert_eq!(parse_ppid("4242 (cc1)"), None);
}

#[test]
fn finds_the_quest_a_client_works_for() {
    let quest = concat!(
        "/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice",
        "/jobs-work-qest20261024abc123.slice"
    );
    assert_eq!(
        quest_slice(&format!("0::{quest}/work-307590-4656305.scope\n")),
        Some(quest)
    );
    assert_eq!(
        quest_slice(concat!(
            "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice",
            "/work-1-2.scope\n"
        )),
        None
    );
    assert_eq!(
        quest_slice("0::/system.slice/compiler-build.service\n"),
        None
    );
}

#[test]
fn a_tiered_quests_build_is_weighed_by_its_tier() {
    let tier = concat!(
        "/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice",
        "/jobs-work-driving.slice"
    );
    assert_eq!(
        quest_slice(&format!(
            "0::{tier}/jobs-work-driving-qest20261024abc123.slice/work-1-2.scope\n"
        )),
        Some(tier),
        "a tiered quest's build wasn't weighed by its tier, which ranks it among every quest"
    );
}

#[test]
fn a_quests_build_stays_below_a_vms() {
    assert_eq!(quest_weight(400, 1000), 400);
    assert_eq!(quest_weight(10_000, 1000), 999);
    assert_eq!(quest_weight(0, 1000), 1);
}

#[test]
fn reads_nice_and_policy() {
    let line = stat("qemu-system-x86", 0, 3);
    assert_eq!(parse_stat(&line), Some(Scheduling { nice: 0, policy: 3 }));
}

#[test]
fn reads_past_a_name_holding_parentheses() {
    let line = stat("a) 1 2 (b", -10, 0);
    assert_eq!(
        parse_stat(&line),
        Some(Scheduling {
            nice: -10,
            policy: 0
        })
    );
}

#[test]
fn a_short_line_reads_as_nothing() {
    assert_eq!(parse_stat("4242 (qemu) S 1 2"), None);
}

#[test]
fn knows_a_timed_test_by_what_runs_in_it_wrapped_or_not() {
    let names = defaults().names;
    for comm in [
        "qemu-system-x86\n",
        ".qemu-system-x8\n",
        "qemu-kvm\n",
        "nixos-test-driv\n",
        ".nixos-test-dri\n",
        "systemd-nspawn\n",
        "niri\n",
        ".niri-wrapped\n",
        "sway\n",
        ".sway-wrapped\n",
    ] {
        assert!(is_timed(comm, &names), "{comm}");
    }
    // A niri build's own test programs, which cargo names after the
    // crate and a hash, are a compiler's work, not a compositor.
    for comm in [
        "cc1\n",
        "vde_switch\n",
        "rustc\n",
        "python3.14\n",
        "bash\n",
        "niri-3f2a1b9c0d\n",
        "niri_config-ab1\n",
        "niri_ipc-9e8d7c\n",
        "swaybg\n",
    ] {
        assert!(!is_timed(comm, &names), "{comm}");
    }
}

#[test]
fn finds_every_process_under_a_build_its_containers_too() {
    let root = std::env::temp_dir().join(format!("vm-first-under-{}", std::process::id()));
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("init.scope")).unwrap();
    fs::write(root.join("cgroup.procs"), "10\n11\n").unwrap();
    fs::write(payload.join("cgroup.procs"), "").unwrap();
    fs::write(payload.join("init.scope/cgroup.procs"), "30\n").unwrap();

    let mut found = pids_under(&root);
    found.sort();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(found, vec!["10", "11", "30"]);
}

#[test]
fn a_builds_first_process_is_its_own_else_the_nearest_under_it() {
    let root = std::env::temp_dir().join(format!("vm-first-first-{}", std::process::id()));
    // `c` sorts first, though APFS lists `m` first by its names' hashes,
    // and tmpfs by `m` made last; then, emptied, `c` holds its process a
    // cgroup further down than `m`.
    let (c, deep, m) = (root.join("c"), root.join("c/deep"), root.join("m"));
    fs::create_dir_all(&deep).expect("the temp folder takes a folder");
    fs::create_dir_all(&m).expect("the temp folder takes a folder");
    let procs = |cgroup: &Path, pids: &str| {
        fs::write(cgroup.join("cgroup.procs"), pids).expect("the temp folder takes a file");
    };
    procs(&root, "10\n11\n");
    procs(&c, "30\n");
    procs(&deep, "50\n");
    procs(&m, "40\n");
    let own = first_process(&root);
    procs(&root, "");
    let sorted = first_process(&root);
    procs(&c, "");
    let nearest = first_process(&root);
    procs(&m, "");
    let deeper = first_process(&root);
    procs(&deep, "");
    let none = first_process(&root);
    fs::remove_dir_all(&root).expect("the temp folder gives its folder back");
    assert_eq!(
        (own, sorted, nearest, deeper, none),
        (Some(10), Some(30), Some(40), Some(50), None),
        "a build's first process wasn't its own, else the nearest under it, by name"
    );
}

#[test]
fn a_cgroup_under_the_cgroup_root_is_kept_as_asked() {
    for asked in [
        "/sys/fs/cgroup/build.slice",
        "/sys/fs/cgroup/build.slice/",
        "/sys/fs/cgroup/./build.slice",
    ] {
        let kept = settings(&words(&format!("--cgroup {asked}")))
            .ok()
            .map(|settings| settings.cgroup);
        let within = kept
            .as_deref()
            .is_some_and(|kept| in_cgroup("0::/build.slice/x\n", kept));
        assert_eq!(
            (kept.as_deref(), within),
            (Some(Path::new(asked)), true),
            "{asked} wasn't kept, or a process under it wasn't read as in it"
        );
    }
}

// Only Linux reads a folder's files through its open descriptor.
#[cfg(target_os = "linux")]
#[test]
fn a_stat_whose_name_isnt_utf8_is_read_as_text_past_it() {
    let folder = std::env::temp_dir().join(format!("vm-first-stat-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("the temp folder takes a folder");
    let stat = [
        b"4242 (a\xff\xfe) S 1 4242 4242 0 -1 4194560".as_slice(),
        b" 100 0 0 0 1 2 0 0 20 0 1 0 98765 1 2",
    ]
    .concat();
    fs::write(folder.join("stat"), stat).expect("the temp folder takes a file");
    let text = fs::File::open(&folder)
        .ok()
        .and_then(|opened| Proc(opened).text("stat"));
    fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
    assert_eq!(
        text.as_deref()
            .map(|text| (parse_started(text), parse_ppid(text))),
        Some((Some(98765), Some(1))),
        "a stat whose name isn't UTF-8 wasn't read as text past it"
    );
}

#[test]
fn an_idle_or_batch_thread_or_a_nice_one_is_behind() {
    let settings = defaults();
    // Policy 3 is SCHED_BATCH, 5 SCHED_IDLE and 0 SCHED_OTHER.
    for (nice, policy, what) in [
        (0, 3, "a batch thread"),
        (0, 5, "an idle thread"),
        (0, 0, "a thread at another nice"),
        (-10, 3, "a batch thread at the nice asked for"),
    ] {
        assert!(
            behind(&Scheduling { nice, policy }, &settings),
            "{what} wasn't behind"
        );
    }
    assert!(
        !behind(
            &Scheduling {
                nice: -10,
                policy: 0
            },
            &settings
        ),
        "a thread already ahead was behind"
    );
}
