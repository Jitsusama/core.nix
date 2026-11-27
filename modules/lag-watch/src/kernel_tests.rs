use super::*;
use std::collections::BTreeMap;

#[test]
fn a_pressure_file_gives_its_totals() {
    let text = "some avg10=1.20 avg60=0.80 avg300=0.50 total=123456\n\
                full avg10=0.10 avg60=0.00 avg300=0.00 total=789\n";
    assert_eq!(
        stall(text),
        Stall {
            some: 123_456,
            full: 789
        }
    );
    assert_eq!(
        stall("some avg10=0.00 avg60=0.00 avg300=0.00 total=5\n").full,
        0
    );
}

#[test]
fn a_keyed_file_gives_the_field_named_and_not_one_it_starts() {
    let text = "workingset_refault_anon 12\nworkingset_refault_anon_x 99\nanon 4096\n";
    assert_eq!(field(text, "workingset_refault_anon"), 12);
    assert_eq!(field(text, "anon"), 4096);
    assert_eq!(field(text, "file"), 0);
}

#[test]
fn a_thread_is_asleep_unless_its_stat_says_it_runs_or_is_ready() {
    assert!(
        asleep("7 (Chrome_ChildIOT) S 1 7 7 0 -1\n"),
        "a sleeping thread counted as ready"
    );
    assert!(
        asleep("7 (a (b) c) D 1 7 7 0 -1\n"),
        "a thread in disk sleep, named with brackets, counted as ready"
    );
    assert!(
        !asleep("7 (CrBrowserMain) R 1 7 7 0 -1\n"),
        "a ready thread counted as asleep"
    );
    assert!(!asleep(""), "an unread stat counted as asleep");
}

#[test]
fn a_schedstat_gives_run_wait_and_slices() {
    assert_eq!(
        sched("1621278039012 163383489014 6999273\n"),
        Some(Sched {
            run_ns: 1_621_278_039_012,
            wait_ns: 163_383_489_014,
            slices: 6_999_273,
            fair_ns: None,
            owed: (0, 0)
        })
    );
    assert_eq!(sched(""), None);
}

#[test]
fn a_sched_file_gives_the_time_blocked_only_while_it_is_kept() {
    let kept = "Chrome_ChildIOT (7, #threads: 30)\n\
                ----------------------------------------\n\
                se.sum_exec_runtime          :          1.019739\n\
                stats.sum_sleep_runtime      :       9000.000000\n\
                stats.sum_block_runtime      :        842.500125\n\
                stats.block_max              :        300.000000\n";
    assert_eq!(
        blocked(kept),
        Some(842_500_125),
        "the time blocked wasn't read from its line"
    );
    let unkept = "cat (7, #threads: 1)\n\
                  ----------------------------------------\n\
                  se.sum_exec_runtime          :          1.019739\n";
    assert_eq!(blocked(unkept), None, "a file without it read as blocked");
}

#[test]
fn a_sched_file_gives_the_wait_for_a_cpu_never_the_wait_for_a_disk() {
    // In the kernel's own order, from a live file: every key beside the
    // wait's begins or ends as its does, so a looser match takes another.
    let kept = "slack (7, #threads: 30)\n\
                ----------------------------------------\n\
                wait_start                   :          0.000000\n\
                wait_max                     :         18.364358\n\
                wait_sum                     :         12.250000\n\
                wait_count                   :             32714\n\
                iowait_sum                   :        900.000000\n\
                iowait_count                 :              4164\n";
    assert_eq!(
        waited(kept),
        Some(12_250_000),
        "the wait wasn't read from its line"
    );
    let only_io = "slack (7, #threads: 30)\n\
                   ----------------------------------------\n\
                   iowait_sum                   :        900.000000\n";
    assert_eq!(
        waited(only_io),
        None,
        "a wait for a disk read as one for a CPU"
    );
}

#[test]
fn a_thread_s_own_name_never_reads_as_a_count() {
    let forged = "wait_sum: 9000000\n (7, #threads: 1)\n\
                  ----------------------------------------\n\
                  wait_sum                     :         12.250000\n\
                  stats.sum_block_runtime      :          3.000000\n";
    assert_eq!(
        waited(forged),
        Some(12_250_000),
        "a line in the thread's name was read as its wait"
    );
    // Names a thread can really give itself, at most 15 bytes, that open
    // with an empty line or a short run of dashes and end their own line,
    // so the kernel's " (7, #threads: 1)" falls on the next: only the dash
    // row's length tells them from the kernel's own.
    for name in ["\nwait_sum:9e9\n", "-\nwait_sum:9e9\n", "---\nwait_sum:9\n"] {
        assert!(name.len() <= NAME_MOST, "{name:?} is longer than a name can be");
        let forged = format!(
            "{name} (7, #threads: 1)\n\
             ----------------------------------------\n\
             wait_sum                     :         12.250000\n"
        );
        assert_eq!(
            waited(&forged),
            Some(12_250_000),
            "the name {name:?} was read as the thread's wait"
        );
    }
    let no_header = "wait_sum : 2.000000\nstats.sum_block_runtime : 3.000000\n";
    assert_eq!(
        waited(no_header),
        None,
        "a file without its header was read"
    );
    assert_eq!(
        blocked(no_header),
        None,
        "a file without its header was read"
    );
}

#[test]
fn a_name_cut_short_of_utf8_leaves_its_counts_readable() {
    let root = std::env::temp_dir().join(format!("lag-watch-name-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let sched = root.join("sched");
    // A name of two-byte letters the kernel cut after a first byte.
    let mut bytes = b"caf\xc3 (7, #threads: 1)\n".to_vec();
    bytes.extend_from_slice(
        b"----------------------------------------\n\
          wait_sum                     :         12.250000\n\
          stats.sum_block_runtime      :          3.000000\n",
    );
    fs::write(&sched, bytes).unwrap();
    let read = text(&sched);
    let _ = fs::remove_dir_all(&root);
    assert_eq!(waited(&read), Some(12_250_000), "the wait was lost to the name");
    assert_eq!(blocked(&read), Some(3_000_000), "the time blocked was lost to the name");
}

#[test]
fn a_thread_s_fair_count_is_read_beside_its_schedstat() {
    let root = std::env::temp_dir().join(format!("lag-watch-wait-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let kept = root.join("7/task/7/schedstat");
    write(&kept, "5000000 999000000 40\n");
    write(
        &kept.with_file_name("sched"),
        "t (7, #threads: 1)\n----------------\nwait_sum : 2.000000\n",
    );
    assert_eq!(
        read_sched(&kept),
        Some(Sched {
            run_ns: 5_000_000,
            wait_ns: 999_000_000,
            slices: 40,
            fair_ns: Some(2_000_000),
            owed: (0, 0)
        }),
        "the fair count wasn't read beside the schedstat"
    );
    let unkept = root.join("8/task/8/schedstat");
    write(&unkept, "5000000 3000000 40\n");
    assert_eq!(
        read_sched(&unkept).map(|s| (s.wait_ns, s.fair_ns)),
        Some((3_000_000, None)),
        "a thread with no fair count lost its wait"
    );
    assert_eq!(read_sched(&root.join("9/task/9/schedstat")), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_wait_is_the_lesser_of_the_two_counts() {
    let at = |wait_ns, fair_ns| Sched {
        wait_ns,
        fair_ns,
        ..Sched::default()
    };
    let then = at(1_000_000_000, Some(500_000_000));
    assert_eq!(
        at(1_999_000_000, Some(500_400_000)).waited_since(&then),
        400_000,
        "a sleep the schedstat counted read as a wait"
    );
    assert_eq!(
        at(1_000_125_000, Some(500_310_000)).waited_since(&then),
        125_000,
        "a sleep the fair count counted read as a wait"
    );
    assert_eq!(
        at(1_003_000_000, None).waited_since(&at(1_000_000_000, None)),
        3_000_000,
        "a thread with no fair count lost its wait"
    );
    assert_eq!(
        at(1_003_000_000, Some(100)).waited_since(&then),
        3_000_000,
        "a fair count that went back wasn't left for the schedstat's"
    );
}

/// Reads `reads` in turn, each crediting from the one before, and
/// gives what each window between them waited.
fn windows(reads: &[(u64, u64)]) -> Vec<u64> {
    let mut old = Sched {
        wait_ns: reads[0].0,
        fair_ns: Some(reads[0].1),
        ..Sched::default()
    };
    let mut waited = Vec::new();
    for &(wait_ns, fair) in &reads[1..] {
        let mut now = Sched {
            wait_ns,
            fair_ns: Some(fair),
            ..Sched::default()
        };
        waited.push(now.waited_since(&old));
        now.credit_since(&old);
        old = now;
    }
    waited
}

#[test]
fn a_wait_the_two_count_either_side_of_a_read_is_counted_once_both_have() {
    // A 500 ms wait that moved CPUs 200 ms in: the schedstat books
    // those 200 ms at the move, before the read; the fair count books
    // all 500 at the end, after it.
    assert_eq!(
        windows(&[(0, 0), (200, 0), (500, 500)]),
        [0, 500],
        "a wait split across a read was lost"
    );
    // A wait that ends between the two files' reads: the fair count
    // has it at the read, the schedstat only at the next.
    assert_eq!(
        windows(&[(0, 0), (0, 300), (300, 300)]),
        [0, 300],
        "a wait ending between the files was lost"
    );
}

#[test]
fn growth_that_pays_what_is_owed_is_never_owed_again() {
    // The fair count's 500 pays the schedstat's 200 owed, and only its
    // other 300 is owed on; the schedstat's 500 after it can then book
    // no more than those 300, so the three windows count the lesser
    // total, 500, never 700.
    assert_eq!(
        windows(&[(0, 0), (200, 0), (200, 500), (700, 500)]),
        [0, 200, 300],
        "what paid a debt was owed again"
    );
}

#[test]
fn a_fair_count_that_went_back_lets_go_of_what_was_owed() {
    // The schedstat books a 4 s sleep and the fair count then goes back,
    // as when schedstats is turned off and on: what was owed goes with
    // it, so the fair count's growth after it pays nothing.
    assert_eq!(
        windows(&[(0, 0), (4_000, 10), (4_000, 5), (4_000, 4_005)]),
        [10, 0, 0],
        "a sleep owed survived the fair count going back"
    );
}

#[test]
fn a_sleep_one_count_books_is_let_go_after_a_window() {
    // The schedstat books a 4 s sleep; the fair count never matches it,
    // so it's let go and later waits are counted as they come.
    assert_eq!(
        windows(&[(0, 0), (4_000, 10), (4_020, 30), (4_050, 60)]),
        [10, 20, 30],
        "a sleep one count booked passed as a wait"
    );
    assert_eq!(
        windows(&[(0, 0), (10, 300), (30, 320), (60, 350)]),
        [10, 20, 30],
        "a sleep the fair count booked passed as a wait"
    );
    // Let go means let go: when the fair count later runs ahead with a
    // sleep of its own, the schedstat's old 4 s isn't there to pair with.
    assert_eq!(
        windows(&[(0, 0), (4_000, 10), (4_020, 30), (4_040, 530)]),
        [10, 20, 20],
        "a sleep one count booked was kept past a window"
    );
}

#[test]
fn a_sleep_one_count_booked_lends_the_other_s_excess_only_beside_it() {
    // A real wait of 20 a window, the fair count 10 over it each window
    // (a sleeper queued before it leaves), and one 4 s sleep the
    // schedstat books. The fair count's excess is counted only in the
    // windows either side of the sleep, never after.
    assert_eq!(
        windows(&[
            (0, 0),
            (20, 30),
            (4_040, 60),
            (4_060, 90),
            (4_080, 120),
            (4_100, 150)
        ]),
        [20, 40, 30, 20, 20],
        "the fair count's excess went on being counted after a sleep"
    );
}

/// Writes `text` to a stand-in kernel file, making its folders.
pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn the_watched_threads_are_found_by_their_names() {
    let root = std::env::temp_dir().join(format!("lag-watch-proc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(&root.join("10/comm"), "niri\n");
    write(&root.join("10/schedstat"), "1 2 3\n");
    write(&root.join("20/comm"), "pipewire\n");
    write(&root.join("20/task/20/comm"), "pipewire\n");
    write(&root.join("20/task/21/comm"), "data-loop.0\n");
    write(&root.join("30/comm"), "chrome\n");
    write(&root.join("self/comm"), "niri\n");
    let found = seen(&root, None, &root.join("tuple.service"));
    assert_eq!(
        found,
        [
            self::found("niri", "niri", root.join("10/schedstat"), false),
            self::found(
                "pipewire",
                "data-loop.0",
                root.join("20/task/21/schedstat"),
                false
            )
        ]
    );
}

#[test]
fn only_the_niri_drawing_the_desktop_is_watched_once_it_is_known() {
    let root = std::env::temp_dir().join(format!("lag-watch-nested-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(&root.join("10/comm"), "niri\n");
    write(&root.join("40/comm"), "niri\n");
    write(&root.join("50/comm"), "niri\n");
    let tuple = root.join("tuple.service");
    assert_eq!(
        seen(&root, Some(40), &tuple),
        [found("niri", "niri", root.join("40/schedstat"), false)]
    );
    assert_eq!(seen(&root, None, &tuple).len(), 3);
}

/// The account the tests' processes are Joel's in.
pub const ACCOUNT: &str = "/user.slice/user-1000.slice/user@1000.service";

/// A process named `comm`, in the scope of an app Joel opened.
pub fn opened(proc: &Path, pid: &str, comm: &str) {
    write(&proc.join(pid).join("comm"), &format!("{comm}\n"));
    let scope = format!("{ACCOUNT}/app.slice/app-niri-a-70.scope");
    write(&proc.join(pid).join("cgroup"), &format!("0::{scope}\n"));
}

/// The threads found in a first look, with no app's process known yet.
fn seen(proc: &Path, compositor: Option<u32>, unit: &Path) -> Vec<Thread> {
    threads(proc, compositor, unit, &mut Apps::of_account(ACCOUNT))
}

#[test]
fn a_thread_s_nice_and_start_are_read_past_a_name_holding_spaces_and_brackets() {
    let stat = "72 (chrome (a) b) S 70 1 1 0 -1 4194560 1 0 0 0 5 6 0 0 25 5 30 0 99\n";
    assert_eq!(nice(stat), Some(5));
    assert_eq!(started(stat), Some(99));
    assert_eq!(
        nice("72 (chrome) S 70 1 1 0 -1 4194560 1 0 0 0 5 6 0 0 12 -8"),
        Some(-8)
    );
    assert_eq!(nice("72 (chrome) S 70"), None);
    assert_eq!(started("72 (chrome) S 70"), None);
    assert_eq!(nice(""), None);
}

fn found(label: &'static str, name: &str, schedstat: PathBuf, call: bool) -> Thread {
    Thread {
        label,
        name: name.to_string(),
        schedstat,
        call,
    }
}

#[test]
fn a_call_s_threads_are_found_while_it_carries_one() {
    let root = std::env::temp_dir().join(format!("lag-watch-call-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let proc = root.join("proc");
    write(&root.join("tuple.service/cgroup.procs"), "60\n");
    write(&proc.join("60/comm"), ".tuple-wrapped\n");
    write(&proc.join("60/task/60/comm"), ".tuple-wrapped\n");
    write(&proc.join("60/task/61/comm"), "Signaling\n");
    write(&proc.join("60/task/62/comm"), "AudioEncoder\n");
    opened(&proc, "30", "chrome");
    write(&proc.join("30/task/31/comm"), "WebRTC_Network\n");
    write(&proc.join("30/task/32/comm"), "ThreadPoolAudio\n");
    write(&proc.join("30/task/33/comm"), "AudioOutputDevi\n");
    write(&proc.join("30/task/34/comm"), "ThreadPoolForeg\n");
    let task = |pid: &str, tid: &str| proc.join(format!("{pid}/task/{tid}/schedstat"));
    // A tab with a peer connection playing sound is no call of
    // Chrome's until a page records the microphone, in any of its
    // processes.
    let chrome = |proc: &Path| -> Vec<Thread> {
        seen(proc, None, &root.join("tuple.service"))
            .into_iter()
            .filter(|t| t.label == "chrome")
            .collect()
    };
    assert_eq!(chrome(&proc), []);
    // Nor while only a Chrome Joel didn't open records one, a test's.
    write(&proc.join("50/comm"), "chrome\n");
    write(
        &proc.join("50/cgroup"),
        &format!("0::{ACCOUNT}/app.slice/call-under-load.service\n"),
    );
    write(&proc.join("50/task/51/comm"), "AudioInputDevic\n");
    assert_eq!(chrome(&proc), []);
    // Nor an agent's, in the jobs slice.
    write(&proc.join("55/comm"), "chrome\n");
    write(
        &proc.join("55/cgroup"),
        &format!("0::{ACCOUNT}/jobs.slice/jobs-work.slice/work-1-2.scope\n"),
    );
    write(&proc.join("55/task/56/comm"), "AudioInputDevic\n");
    assert_eq!(chrome(&proc), []);
    // A Chrome Joel started from a terminal carries his call, though
    // its drawing isn't read.
    write(&proc.join("45/comm"), "chrome\n");
    write(
        &proc.join("45/cgroup"),
        &format!("0::{ACCOUNT}/app.slice/kitty-70-0.scope\n"),
    );
    write(&proc.join("45/task/46/comm"), "WebRTC_Network\n");
    opened(&proc, "40", "chrome");
    write(&proc.join("40/task/41/comm"), "AudioInputDevic\n");
    assert_eq!(
        seen(&proc, None, &root.join("tuple.service")),
        [
            found("chrome", "AudioInputDevic", task("40", "41"), true),
            found("chrome", "AudioOutputDevi", task("30", "33"), false),
            found("chrome", "WebRTC_Network", task("30", "31"), false),
            found("chrome", "WebRTC_Network", task("45", "46"), false),
            found("tuple", ".tuple-wrapped", task("60", "60"), false),
            found("tuple", "AudioEncoder", task("60", "62"), true),
            found("tuple", "Signaling", task("60", "61"), false),
        ]
    );
    // Out of a call, neither has a thread worth reading.
    fs::remove_file(proc.join("60/task/62/comm")).unwrap();
    fs::remove_file(proc.join("40/task/41/comm")).unwrap();
    assert_eq!(seen(&proc, None, &root.join("tuple.service")), []);
    // Nor without Tuple at all.
    assert_eq!(seen(&proc, None, &root.join("gone.service")), []);
}

#[test]
fn a_call_s_threads_count_only_while_one_carrying_it_is_read() {
    let at = |n: &str| PathBuf::from(format!("/proc/{n}/schedstat"));
    let mut read = BTreeMap::new();
    read.insert(
        found("tuple", "Signaling", at("61"), false),
        Sched::default(),
    );
    read.insert(found("niri", "niri", at("10"), false), Sched::default());
    read.insert(
        found("chrome", "AudioThread", at("31"), true),
        Sched::default(),
    );
    while_in_a_call(&mut read);
    let labels: Vec<&str> = read.keys().map(|t| t.label).collect();
    assert_eq!(labels, ["chrome", "niri"]);
}

// A stand-in for Tuple's daemon: this test's own process, whose thread
// names itself as WebRTC's encoder does (prctl's PR_SET_NAME, which
// std's thread names are set by), read from the kernel's own /proc.
#[test]
fn a_stand_in_s_named_threads_are_read_from_the_kernel() {
    let root = std::env::temp_dir().join(format!("lag-watch-stand-in-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(
        &root.join("tuple.service/cgroup.procs"),
        &format!("{}\n", std::process::id()),
    );
    let (named, go) = std::sync::mpsc::channel();
    let (stop, stopped) = std::sync::mpsc::channel::<()>();
    let encoder = std::thread::Builder::new()
        .name("AudioEncoder".into())
        .spawn(move || {
            named.send(()).unwrap();
            let _ = stopped.recv();
        })
        .unwrap();
    go.recv().unwrap();
    let found = seen(Path::new("/proc"), None, &root.join("tuple.service"));
    let carrying = found
        .iter()
        .find(|t| t.label == "tuple" && t.name == "AudioEncoder")
        .expect("the encoder is found by its name");
    let read = sched(&text(&carrying.schedstat));
    stop.send(()).unwrap();
    encoder.join().unwrap();
    assert!(carrying.call);
    assert!(read.is_some_and(|s| s.slices > 0), "{read:?}");
    assert!(found.iter().filter(|t| t.label == "tuple").count() > 1);
}

#[test]
fn a_cgroup_is_read_whole_or_not_at_all() {
    let root = std::env::temp_dir().join(format!("lag-watch-cg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(
        &root.join("app.slice/cpu.pressure"),
        "some avg10=0 avg60=0 avg300=0 total=10\n",
    );
    write(&root.join("app.slice/memory.swap.current"), "4096\n");
    write(
        &root.join("app.slice/memory.stat"),
        "workingset_refault_anon 3\nworkingset_refault_file 4\n",
    );
    write(
        &root.join("app.slice/memory.events"),
        "low 0\nhigh 0\nmax 0\noom 2\noom_kill 1\noom_group_kill 0\n",
    );
    let g = group(&root.join("app.slice")).unwrap();
    assert_eq!((g.cpu.some, g.swap, g.refaults, g.killed), (10, 4096, 7, 1));
    assert_eq!(group(&root.join("gone.slice")), None);
}
