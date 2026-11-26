//! vm-first's tests of how it reads threads, the daemon and its builds, and
//! how it weighs and schedules them, beside it as the tally's are.

use super::*;

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
        client_of(b"nix-daemon\x00307605\x00\x00\x00"),
        Some(307_605)
    );
    assert_eq!(
        client_of(b"/nix/store/x-nix/bin/nix-daemon\x0042\x00"),
        Some(42)
    );
    assert_eq!(client_of(b"nix-daemon\x00--daemon\x00"), None);
    assert_eq!(client_of(b"bash\x00123\x00"), None);
    assert_eq!(client_of(b""), None);
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
    let quest = "/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/jobs-work-qest20261024abc123.slice";
    assert_eq!(
        quest_slice(&format!("0::{quest}/work-307590-4656305.scope\n")),
        Some(quest)
    );
    assert_eq!(
        quest_slice(
            "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/work-1-2.scope\n"
        ),
        None
    );
    assert_eq!(
        quest_slice("0::/system.slice/compiler-build.service\n"),
        None
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
