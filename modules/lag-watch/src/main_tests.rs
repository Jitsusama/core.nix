//! main.rs's tests, of the day's name, the status, the account and the
//! compositor, and of the snapshot over a forged /proc tree.

use super::*;

#[test]
fn a_day_is_named_in_utc() {
    assert_eq!(day(0), "1970-01-01");
    // 19:26 UTC on 23 October 2026, and 23:00, which is the 24th.
    assert_eq!(day(1_792_783_560), "2026-10-23");
    assert_eq!(day(1_792_800_000), "2026-10-24");
    assert_eq!(day(951_782_400), "2000-02-29");
}

// The status folder only ever holds the status and the folder it's
// written in, and that's left empty.
#[test]
fn the_status_is_written_aside() {
    let dir = std::env::temp_dir().join(format!("lag-watch-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    write_aside(&dir, "status.json", "{}").unwrap();
    write_aside(&dir, "status.json", "{\"a\":1}").unwrap();
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, [".writing", "status.json"]);
    assert_eq!(fs::read_dir(dir.join(".writing")).unwrap().count(), 0);
    assert_eq!(
        fs::read_to_string(dir.join("status.json")).unwrap(),
        "{\"a\":1}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_tab_its_app_lowered_is_counted_and_kept_only_as_where_the_next_window_starts() {
    let dir = std::env::temp_dir().join(format!("lag-watch-lowered-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let thread = |label: &'static str, pid: &str, tid: &str, nice: i64| {
        let at = dir.join(format!("{pid}/task/{tid}"));
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
        let stat = format!("{tid} (chrome) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 {nice} 30 0\n");
        fs::write(at.join("stat"), &stat).unwrap();
        if pid == tid {
            fs::write(dir.join(pid).join("stat"), &stat).unwrap();
        }
        kernel::Thread {
            label,
            name: "chrome".into(),
            schedstat: at.join("schedstat"),
            call: false,
        }
    };
    // A hidden tab: its main thread lowered, its compositor raised to
    // draw, as Chromium leaves them. A tab shown, its main thread at
    // Chromium's own nice. A tab whose stat is gone, ending. And niri,
    // whose nice is never asked.
    let hidden = thread("chrome-ui", "72", "72", 5);
    let hidden_drawing = thread("chrome-ui", "72", "73", -8);
    let shown = thread("chrome-ui", "75", "75", 0);
    let shown_drawing = thread("chrome-ui", "75", "76", -8);
    let ending = thread("slack-ui", "78", "79", -8);
    let niri = thread("niri", "74", "74", 5);
    let s = snapshot(
        &dir,
        &dir,
        // The compositor first, so its own nice is never what's asked.
        &[
            hidden_drawing.clone(),
            hidden.clone(),
            shown.clone(),
            shown_drawing.clone(),
            ending.clone(),
            niri.clone(),
        ],
        BTreeMap::new(),
    );
    let read: Vec<&kernel::Thread> = s.threads.keys().collect();
    assert_eq!(read, [&shown, &shown_drawing, &niri, &ending]);
    let resting: Vec<&kernel::Thread> = s.resting.keys().collect();
    assert_eq!(resting, [&hidden, &hidden_drawing], "the hidden tab rests");
    assert_eq!(
        s.lowered,
        BTreeMap::from([("chrome-ui".to_string(), 1)]),
        "one hidden tab, counted once"
    );
    // Shown again, the app raises the tab, and the next window reads it.
    let stat = "72 (chrome) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0\n";
    fs::write(dir.join("72/stat"), stat).unwrap();
    let s = snapshot(
        &dir,
        &dir,
        std::slice::from_ref(&hidden_drawing),
        BTreeMap::new(),
    );
    assert_eq!(s.threads.keys().collect::<Vec<_>>(), [&hidden_drawing]);
    assert!(s.lowered.is_empty(), "{:?}", s.lowered);
    assert!(s.resting.is_empty(), "{:?}", s.resting);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_app_thread_s_time_blocked_is_read_shown_or_hidden_and_no_one_else_s() {
    let dir = std::env::temp_dir().join(format!("lag-watch-blocked-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    // What each thread's sched keeps, in milliseconds: its time blocked and its fair
    // scheduler's wait, each unlike the other so neither is read for it.
    let thread = |label: &'static str, pid: &str, nice: i64, kept: Option<(&str, &str)>| {
        let at = dir.join(format!("{pid}/task/{pid}"));
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
        let stat = format!("{pid} (slack) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 {nice} 30 0\n");
        fs::write(at.join("stat"), &stat).unwrap();
        fs::write(dir.join(pid).join("stat"), &stat).unwrap();
        if let Some((blocked, waited)) = kept {
            let sched = format!(
                "slack ({pid}, #threads: 9)\n----------------\n\
                 wait_sum : {waited}\nstats.sum_block_runtime : {blocked}\n"
            );
            fs::write(at.join("sched"), sched).unwrap();
        }
        kernel::Thread {
            label,
            name: "slack".into(),
            schedstat: at.join("schedstat"),
            call: false,
        }
    };
    let shown = thread("slack-ui", "81", 0, Some(("12.500000", "40.000000")));
    let hidden = thread("chrome-ui", "82", 5, Some(("3.000000", "6.000000")));
    let unkept = thread("slack-ui", "83", 0, None);
    let niri = thread("niri", "84", 0, Some(("7.000000", "9.000000")));
    let s = snapshot(
        &dir,
        &dir,
        &[shown.clone(), hidden.clone(), unkept, niri],
        BTreeMap::new(),
    );
    // The sched read once for the time blocked gives the fair count too, to
    // a thread read and to one resting alike, so each window takes the
    // lesser of the two counts for both.
    assert_eq!(
        s.threads.get(&shown).and_then(|sched| sched.fair_ns),
        Some(40_000_000),
        "the shown thread's fair count wasn't read from its sched: {:?}",
        s.threads
    );
    assert_eq!(
        s.resting.get(&hidden).and_then(|sched| sched.fair_ns),
        Some(6_000_000),
        "the resting thread's fair count wasn't read from its sched: {:?}",
        s.resting
    );
    assert_eq!(
        s.blocked,
        BTreeMap::from([(shown, 12_500_000), (hidden, 3_000_000)]),
        "only the apps' threads, shown or hidden, had their time blocked read"
    );
    fs::remove_dir_all(&dir).unwrap();
}

// The kernel cuts a name at 15 bytes, byte by byte, so a name in letters of
// more than one byte can end partway through one, and the process's stat is
// then not UTF-8. Its nice, past the name, must still be read, or a hidden
// tab's sleeps would be counted as waits.
#[test]
fn a_lowered_tab_whose_name_the_kernel_cut_mid_letter_still_rests() {
    let dir = std::env::temp_dir().join(format!("lag-watch-cut-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    // "chrome ", two letters of three bytes, and the first two of a third.
    let mut stat = b"72 (chrome \xe2\x98\x95\xe2\x98\x95\xe2\x98".to_vec();
    stat.extend_from_slice(b") S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 5 30 0\n");
    assert!(
        std::str::from_utf8(&stat).is_err(),
        "the forged stat is UTF-8, so it tests nothing"
    );
    fs::create_dir_all(dir.join("72")).unwrap();
    fs::write(dir.join("72/stat"), stat).unwrap();
    let threads: Vec<kernel::Thread> = ["72", "73"]
        .into_iter()
        .map(|tid| {
            let at = dir.join(format!("72/task/{tid}"));
            fs::create_dir_all(&at).unwrap();
            fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
            kernel::Thread {
                label: "chrome-ui",
                name: "chrome".into(),
                schedstat: at.join("schedstat"),
                call: false,
            }
        })
        .collect();
    let s = snapshot(&dir, &dir, &threads, BTreeMap::new());
    assert!(
        s.threads.is_empty(),
        "the tab whose name was cut was read: {:?}",
        s.threads
    );
    assert_eq!(
        s.resting.keys().collect::<Vec<_>>(),
        threads.iter().collect::<Vec<_>>(),
        "the tab's threads didn't rest"
    );
    assert_eq!(
        s.lowered,
        BTreeMap::from([("chrome-ui".to_string(), 1)]),
        "the tab's nice wasn't read past its cut name"
    );
    fs::remove_dir_all(&dir).unwrap();
}

// A thread that ended between the minute's looks leaves no schedstat, and
// its sched is never opened, so nothing it left behind is read as a count.
#[test]
fn a_thread_whose_schedstat_is_gone_gives_nothing_from_its_sched() {
    let dir = std::env::temp_dir().join(format!("lag-watch-gone-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let at = dir.join("85/task/85");
    fs::create_dir_all(&at).unwrap();
    fs::write(
        dir.join("85/stat"),
        "85 (slack) S 70 1 1 0 -1 0 1 0 0 0 5 6 0 0 20 0 30 0\n",
    )
    .unwrap();
    fs::write(
        at.join("sched"),
        "slack (85, #threads: 9)\n----------------\n\
         wait_sum : 40.000000\nstats.sum_block_runtime : 12.500000\n",
    )
    .unwrap();
    let gone = kernel::Thread {
        label: "slack-ui",
        name: "slack".into(),
        schedstat: at.join("schedstat"),
        call: false,
    };
    let s = snapshot(&dir, &dir, &[gone], BTreeMap::new());
    assert!(
        s.threads.is_empty() && s.resting.is_empty(),
        "a thread without a schedstat was read: {:?} {:?}",
        s.threads,
        s.resting
    );
    assert!(
        s.blocked.is_empty(),
        "the sched of a thread without a schedstat was read: {:?}",
        s.blocked
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn each_sched_read_without_its_dash_row_is_counted() {
    let dir = std::env::temp_dir().join(format!("lag-watch-rowless-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let thread = |tid: &str, sched: Option<&str>| {
        let at = dir.join(format!("74/task/{tid}"));
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join("schedstat"), "10 20 3\n").unwrap();
        if let Some(text) = sched {
            fs::write(at.join("sched"), text).unwrap();
        }
        kernel::Thread {
            label: "niri",
            name: "niri".into(),
            schedstat: at.join("schedstat"),
            call: false,
        }
    };
    let rowless = "niri (74, #threads: 9)\nwait_sum : 4.000000\n";
    let whole = "niri (75, #threads: 9)\n----------------\nwait_sum : 4.000000\n";
    let threads = [
        thread("74", Some(rowless)),
        thread("75", Some(whole)),
        thread("76", None),
    ];
    let s = snapshot(&dir, &dir, &threads, BTreeMap::new());
    assert_eq!(
        s.scheds_without_dash_row,
        1,
        "only the sched read without its dash row counts, not one whole or one not read"
    );
    let fair: Vec<Option<u64>> = s.threads.values().map(|t| t.fair_ns).collect();
    assert_eq!(
        fair,
        [None, Some(4_000_000), None],
        "the fair counts weren't each read as their sched allows"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_hidden_tab_is_left_out_on_the_paths_the_watch_finds_its_threads_by() {
    use kernel::tests::{ACCOUNT, opened, write};
    let proc = std::env::temp_dir().join(format!("lag-watch-found-{}", std::process::id()));
    let _ = fs::remove_dir_all(&proc);
    // A renderer whose main thread Chrome lowered, as it does a hidden
    // tab's, found as the watch finds it, not as a test builds it.
    opened(&proc, "72", "chrome");
    write(
        &proc.join("72/stat"),
        "72 (chrome) S 1 1 1 0 -1 0 1 0 0 0 5 6 0 0 25 5 30 0 99\n",
    );
    for (tid, comm) in [("72", "chrome"), ("73", "Compositor")] {
        write(
            &proc.join(format!("72/task/{tid}/comm")),
            &format!("{comm}\n"),
        );
        write(&proc.join(format!("72/task/{tid}/schedstat")), "10 20 3\n");
    }
    let mut apps = apps::Apps::of_account(ACCOUNT);
    let threads = kernel::threads(&proc, None, &proc.join("tuple.service"), &mut apps);
    assert_eq!(threads.len(), 2, "the renderer's two threads: {threads:?}");
    let s = snapshot(&proc, &proc, &threads, apps.unwatched());
    assert!(
        s.threads.is_empty(),
        "a hidden tab was read: {:?}",
        s.threads
    );
    assert_eq!(s.lowered, BTreeMap::from([("chrome-ui".to_string(), 1)]));
    fs::remove_dir_all(&proc).unwrap();
}

/// A snapshot at `at` seconds holding one Slack thread, read, with its
/// schedstat's wait, its fair count and its slices.
fn reading(at: f64, wait_ns: u64, fair_ns: u64, slices: u64) -> Snapshot {
    let mut s = Snapshot {
        at,
        ..Snapshot::default()
    };
    s.threads.insert(
        slack(),
        kernel::Sched {
            run_ns: 0,
            wait_ns,
            slices,
            fair_ns: Some(fair_ns),
            owed: (0, 0),
        },
    );
    s
}

/// The Slack thread these windows hold.
fn slack() -> kernel::Thread {
    kernel::Thread {
        label: "slack-ui",
        name: "slack".into(),
        schedstat: "/proc/81/task/81/schedstat".into(),
        call: false,
    }
}

/// The window from `then` to `now`, as the watch closes it, with niri's
/// answers, its end and the window before left out as none.
fn closed(then: &Snapshot, now: &mut Snapshot) -> Window {
    close_window(then, now, Answers::default(), 0, "gen", None)
}

/// The window's mean wait a turn for Slack's threads, in microseconds.
fn slack_waited(w: &Window) -> f64 {
    w.waits["slack-ui"].mean_us
}

// The schedstat books a 5 ms wait by the first window's read, the fair
// count only after it: the first window has only one count's, so counts
// none, and the second takes what the first was owed.
#[test]
fn a_wait_one_count_booked_before_the_read_and_one_after_is_counted_once_in_the_next() {
    let start = reading(0.0, 0, 0, 0);
    let mut first = reading(10.0, 5_000_000, 0, 1000);
    let w = closed(&start, &mut first);
    assert!(
        slack_waited(&w).abs() < 1e-9,
        "the wait only the schedstat had was counted at once: {:?}",
        w.waits
    );
    let mut second = reading(20.0, 5_000_000, 5_000_000, 2000);
    let w = closed(&first, &mut second);
    assert!(
        (slack_waited(&w) - 5.0).abs() < 1e-9,
        "the wait owed from the window before wasn't counted once: {:?}",
        w.waits
    );
}

// The minute's look takes a fresh snapshot, owed nothing, as the next
// window's start: a thread still read, and one hidden since, keep what
// the window that ended was owed, and the next window counts it.
#[test]
fn what_is_owed_survives_the_minute_s_look_read_or_resting() {
    let start = reading(0.0, 0, 0, 0);
    let mut ended = reading(10.0, 5_000_000, 0, 1000);
    closed(&start, &mut ended);
    let mut fresh = reading(10.0, 5_000_000, 0, 1000);
    look_again(&mut fresh, &ended);
    assert_eq!(
        fresh.threads[&slack()].owed,
        ended.threads[&slack()].owed,
        "a thread still read lost what it was owed"
    );
    let mut next = reading(20.0, 5_000_000, 5_000_000, 2000);
    let read = closed(&fresh, &mut next);
    assert!(
        (slack_waited(&read) - 5.0).abs() < 1e-9,
        "a thread still read lost its wait to the minute's look: {:?}",
        read.waits
    );
    // Hidden between the window's read and the look, shown again by the next.
    let mut fresh = Snapshot {
        at: 10.0,
        resting: reading(10.0, 5_000_000, 0, 1000).threads,
        ..Snapshot::default()
    };
    look_again(&mut fresh, &ended);
    let mut next = reading(20.0, 5_000_000, 5_000_000, 2000);
    let shown = closed(&fresh, &mut next);
    assert!(
        (slack_waited(&shown) - 5.0).abs() < 1e-9,
        "a thread resting at the look lost its wait: {:?}",
        shown.waits
    );
}

#[test]
fn the_account_is_the_manager_in_the_watch_s_own_cgroup() {
    let own = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/lag-watch.service\n";
    assert_eq!(
        manager(own),
        Some("/user.slice/user-1000.slice/user@1000.service")
    );
    // As the apps take it, a process of Joel's is found by it.
    let apps = apps::Apps::of_account(manager(own).unwrap());
    let scope = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-a.scope\n";
    assert!(
        apps.launched_by_joel(scope),
        "Joel's app isn't found by the account"
    );
    assert_eq!(manager("0::/system.slice/lag-watch.service\n"), None);
    assert_eq!(
        account(Path::new("/sys/fs/cgroup"), manager(own).unwrap()),
        Path::new("/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service")
    );
}

#[test]
fn the_compositor_is_the_niri_its_socket_names() {
    assert_eq!(
        socket_pid(Path::new("/run/user/1000/niri.wayland-1.2005.sock")),
        Some(2005)
    );
    assert_eq!(socket_pid(Path::new("/run/user/1000/niri.sock")), None);
    assert_eq!(socket_pid(Path::new("/run/user/1000/other")), None);
}
