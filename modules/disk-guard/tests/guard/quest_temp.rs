//! What a quest still at work left in its `tmp/` and beside it, as the
//! survey offers it.

use crate::discover::{dir, file};
use crate::survey::{DAY, config, find, now, offers, offers_sized, real, written_ago};
use disk_guard::activity::Process;
use disk_guard::plan::Class;
use disk_guard::survey::{Action, Sizes};

const HOUR: f64 = 3_600.0;

// A quest at work can leave its tmp/ for weeks: on one laptop 124 GiB of
// quest scratch lay in tmp/ folders, a quarter of it written to over a
// week before. What nothing has written to for six hours is offered, dated
// by its newest write, so the plan's week takes it; what's younger can't
// go in any band, and what a process works in stays whatever its age.
#[test]
fn what_a_quest_at_work_left_in_its_tmp_is_offered_by_its_last_write() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let tmp = dir(&workspaces.join("QEST-20200101-ABCDEF/tmp"));
    let aged = |name: &str, seconds: f64| {
        let lab = dir(&tmp.join(name));
        file(&lab.join("run.log"), "x");
        written_ago(&lab.join("run.log"), seconds);
        written_ago(&lab, seconds);
        lab
    };
    let left = aged("left-lab", 8.0 * DAY);
    let held = aged("held-lab", 8.0 * DAY);
    aged("young-lab", 3_600.0);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let working = Process {
        pid: 1,
        uid: unsafe { libc::geteuid() },
        cwd: Some(held.clone()),
        readable: true,
        ..Process::default()
    };
    let found = offers(&config, &[working], &|_| false);
    assert_eq!(
        found.len(),
        1,
        "only what nothing works in, unwritten for 6 h: {found:?}"
    );
    let offered = find(&found, &left);
    assert_eq!(offered.candidate.class, Class::QuestTemp);
    assert!(
        offered.candidate.idle > 7.9 * DAY,
        "dated by its last write: {}",
        offered.candidate.idle
    );
    assert_eq!(offered.action, Action::Entry { root: tmp.clone() });
}

// A build a quest linked into its tmp/ is a root of the store: on one
// laptop on 12 November, 51 such links held 4.3 GiB, nothing a build
// couldn't make again. It's offered as a root, so it goes by a root's
// windows; a link anywhere else stays an ordinary entry.
#[test]
fn a_link_into_the_store_in_a_quests_tmp_is_offered_as_a_root() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let tmp = dir(&workspaces.join("QEST-20200101-ABCDEF/tmp"));
    let linked = |name: &str, to: &str| {
        let link = tmp.join(name);
        std::os::unix::fs::symlink(to, &link).unwrap();
        written_ago(&link, 2.0 * DAY);
        link
    };
    let root = linked("result", "/nix/store/abc-app");
    let other = linked("notes", "/home/u/notes");
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let found = offers(&config, &[], &|_| false);
    assert_eq!(
        find(&found, &root).candidate.class,
        Class::QuestRoot,
        "a link into the store is a root"
    );
    assert_eq!(
        find(&found, &other).candidate.class,
        Class::QuestTemp,
        "a link elsewhere is an ordinary entry"
    );
}

// Beside tmp/, a quest at work keeps its labs, clones and runs, 53 GiB of
// them on one laptop on 11 November, 11 GiB unwritten for a week. Those
// are offered too, by their last write, but never tmp/ itself (its
// entries go by their own rule), the handover a leaving machine packs,
// or a folder of GC roots, whose links keep what running programs read.
#[test]
fn what_a_quest_at_work_left_beside_its_tmp_is_offered_but_what_it_keeps() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let workspace = dir(&workspaces.join("QEST-20200101-ABCDEF"));
    let aged = |name: &str, seconds: f64| {
        let folder = dir(&workspace.join(name));
        file(&folder.join("run.log"), "x");
        written_ago(&folder.join("run.log"), seconds);
        written_ago(&folder, seconds);
        folder
    };
    let left = aged("lab", 8.0 * DAY);
    let held = aged("runs", 8.0 * DAY);
    aged("clone", 3_600.0);
    aged("tmp", 8.0 * DAY);
    aged("handover", 8.0 * DAY);
    aged("gcroots-pi", 8.0 * DAY);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let working = Process {
        pid: 1,
        uid: unsafe { libc::geteuid() },
        cwd: Some(held.clone()),
        readable: true,
        ..Process::default()
    };
    let found = offers(&config, &[working], &|_| false);
    let paths: Vec<_> = found
        .iter()
        .filter(|offer| offer.candidate.class != Class::QuestTemp)
        .map(|offer| &offer.candidate.path)
        .collect();
    assert_eq!(
        paths,
        [&left],
        "only the lab, not tmp/, the handover, the roots, the young or the worked in"
    );
    let offered = find(&found, &left);
    assert_eq!(offered.candidate.class, Class::QuestScratch);
    assert_eq!(
        offered.action,
        Action::Entry {
            root: workspace.clone()
        }
    );
}

// On 12 November a laptop's guard spent 88% of a core on its passes once
// it read what quests left beside their tmp/: it judged each of 7,805
// entries against every process's open files before asking its age, and
// most were too young to offer anyway. Asking the age first, a survey of
// thousands of young entries among hundreds of busy processes takes no
// time; the other way round it's billions of comparisons.
#[test]
fn young_quest_scratch_is_never_judged_against_the_processes() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let workspace = dir(&workspaces.join("QEST-20200101-ABCDEF"));
    let tmp = dir(&workspace.join("tmp"));
    for n in 0..2_000 {
        file(&tmp.join(format!("run-{n}")), "x");
        file(&workspace.join(format!("lab-{n}")), "x");
    }
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let elsewhere = home.join("elsewhere");
    let busy: Vec<_> = (0..500)
        .map(|pid| Process {
            pid: pid + 10,
            uid: unsafe { libc::geteuid() },
            files: (0..1_000).map(|n| elsewhere.join(format!("{n}"))).collect(),
            readable: true,
            ..Process::default()
        })
        .collect();
    let started = std::time::Instant::now();
    let found = offers(&config, &busy, &|_| false);
    let took = started.elapsed();
    assert!(found.is_empty(), "nothing is old enough: {found:?}");
    assert!(
        took < std::time::Duration::from_secs(10),
        "a survey of young scratch took {took:?}, as if each entry were judged"
    );
}

// Old entries must be asked whether a process works in them, and on the
// same laptop a fresh survey still spent minutes of CPU doing it, one pass
// over every process's files per entry. Read once into an index, where
// processes work answers thousands of old entries at a lookup each.
#[test]
fn old_quest_scratch_is_judged_against_the_processes_once() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let workspace = dir(&workspaces.join("QEST-20200101-ABCDEF"));
    let tmp = dir(&workspace.join("tmp"));
    for n in 0..2_000 {
        for entry in [
            tmp.join(format!("run-{n}")),
            workspace.join(format!("lab-{n}")),
        ] {
            file(&entry, "x");
            written_ago(&entry, 8.0 * DAY);
        }
    }
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let elsewhere = home.join("elsewhere");
    let busy: Vec<_> = (0..500)
        .map(|pid| Process {
            pid: pid + 10,
            uid: unsafe { libc::geteuid() },
            files: (0..1_000).map(|n| elsewhere.join(format!("{n}"))).collect(),
            readable: true,
            ..Process::default()
        })
        .collect();
    let started = std::time::Instant::now();
    let found = offers(&config, &busy, &|_| false);
    let took = started.elapsed();
    assert_eq!(found.len(), 4_000, "every old entry is offered");
    assert!(
        took < std::time::Duration::from_secs(10),
        "a survey of old scratch took {took:?}, as if each entry were judged alone"
    );
}

// Every pass of a filling disk walked each old entry whole to size it
// again: 68,202 on one laptop, 178 GB read in four hours. An entry
// unwritten since an earlier survey keeps the size it found.
#[test]
fn old_scratch_is_sized_again_only_once_written_again() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let tmp = dir(&workspaces.join("QEST-20200101-ABCDEF/tmp"));
    let run = dir(&tmp.join("run"));
    file(&run.join("log"), &"x".repeat(20_000));
    written_ago(&run.join("log"), 8.0 * HOUR);
    written_ago(&run, 8.0 * HOUR);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        workspaces.display(),
        quests.display()
    ));
    let (found, sizes) = offers_sized(&config, now(), Sizes::default());
    let first = find(&found, &run).candidate.size;
    assert!(first > 0, "the run's log wasn't counted: {found:?}");
    assert_eq!(sizes.walked, 1, "the run wasn't sized once");
    let (found, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(sizes.walked, 1, "a run unwritten since was walked again");
    assert_eq!(
        find(&found, &run).candidate.size,
        first,
        "a run known unwritten since changed its size"
    );
    file(&run.join("more"), &"y".repeat(20_000));
    written_ago(&run.join("more"), 7.0 * HOUR);
    written_ago(&run, 7.0 * HOUR);
    let (found, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(sizes.walked, 2, "a run written since wasn't sized again");
    assert!(
        find(&found, &run).candidate.size > first,
        "a run written since kept its old size"
    );
}

/// The wall seconds of a local time as a Mac's `ps -o lstart` and the
/// harness's `startToken` give it, `Sun Nov 15 17:43:08 2026`.
fn lstart_wall(year: i32, month: i32, day: i32, [hour, minute, second]: [i32; 3]) -> u64 {
    // SAFETY: tm is plain data, zeroed is a valid value, and mktime reads
    // only the struct it's given.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    (tm.tm_year, tm.tm_mon, tm.tm_mday) = (year - 1900, month - 1, day);
    (tm.tm_hour, tm.tm_min, tm.tm_sec, tm.tm_isdst) = (hour, minute, second, -1);
    // SAFETY: as above.
    let seconds = unsafe { libc::mktime(&raw mut tm) };
    u64::try_from(seconds).unwrap()
}

// On 16 November a laptop's guard took 188 entries of a quest's tmp/ while
// its session worked in it, a script it had written that evening among
// them: a pass begun hours before judged each entry by its own age. A
// quest a live session works on keeps all it left, in tmp/ and beside it,
// whatever each entry's age; its browser profiles at rest still go, as
// their own lock says. A session lives while its process runs as the one
// that started then: by its start ticks where the harness keeps them, as
// on Linux, else by the start time it noted, as on a Mac. One whose
// process ended keeps nothing.
#[test]
fn a_quest_a_live_session_works_on_keeps_what_it_left_whatever_its_age() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    let workspaces = dir(&home.join("workspaces"));
    let sessions = dir(&home.join("sessions"));
    let left = |id: &str, process: &str| {
        file(
            &quests.join(format!("{id}/README.md")),
            &format!("---\nid: {id}\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n"),
        );
        file(
            &sessions.join(format!("{id}.json")),
            &format!(r#"{{"quest":"{id}","process":{{{process}}}}}"#),
        );
        let workspace = dir(&workspaces.join(id));
        let script = workspace.join("tmp/splice.sh");
        file(&script, "x");
        written_ago(&script, 8.0 * DAY);
        let lab = dir(&workspace.join("lab"));
        file(&lab.join("run.log"), "x");
        written_ago(&lab.join("run.log"), 8.0 * DAY);
        written_ago(&lab, 8.0 * DAY);
        [script, lab]
    };
    let token = lstart_wall(2026, 11, 15, [17, 43, 8]);
    let linux = left("QEST-20200101-LINUXA", r#""pid":41,"startTicks":500"#);
    let mac = left(
        "QEST-20200101-MACAAA",
        r#""pid":43,"startToken":"Sun Nov 15 17:43:08 2026""#,
    );
    let ended = left("QEST-20200101-ENDEDA", r#""pid":42,"startTicks":500"#);
    let reused = left("QEST-20200101-REUSED", r#""pid":44,"startTicks":500"#);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}",
            "sessions": "{}" }}"#,
        workspaces.display(),
        quests.display(),
        sessions.display()
    ));
    let running = |pid: u32, start_ticks: u64| Process {
        pid,
        uid: unsafe { libc::geteuid() },
        start_ticks,
        cwd: Some(home.clone()),
        readable: true,
        ..Process::default()
    };
    // The survey's boot counts 100 ticks a second from the epoch, so the
    // Mac's session started at its token's second, its ticks a little in.
    let processes = [
        running(41, 500),
        running(43, token * 100 + 40),
        running(44, 900),
    ];
    let found = offers(&config, &processes, &|_| false);
    let offered: Vec<_> = found.iter().map(|offer| &offer.candidate.path).collect();
    for path in linux.iter().chain(&mac) {
        assert!(
            !offered.contains(&path),
            "a live session's quest gave up {}: {offered:?}",
            path.display()
        );
    }
    for path in ended.iter().chain(&reused) {
        assert!(
            offered.contains(&path),
            "an ended session's quest kept {}: {offered:?}",
            path.display()
        );
    }
}

// Every machine's guard is now told where the harness keeps its session
// records, so on one it never ran on, with no such folder, no session
// lives and each quest's scratch still goes by its own age.
#[test]
fn with_no_sessions_folder_a_quest_scratch_goes_by_its_age() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let quests = dir(&home.join("quests"));
    let workspaces = dir(&home.join("workspaces"));
    let id = "QEST-20200101-NOHARN";
    file(
        &quests.join(format!("{id}/README.md")),
        &format!("---\nid: {id}\nstatus: active\nupdated: 2020-01-01\n---\n# At work\n"),
    );
    let script = dir(&workspaces.join(id)).join("tmp/splice.sh");
    file(&script, "x");
    written_ago(&script, 8.0 * DAY);
    let config = config(&format!(
        r#""workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}",
            "sessions": "{}" }}"#,
        workspaces.display(),
        quests.display(),
        home.join("no-sessions").display()
    ));
    let found = offers(&config, &[], &|_| false);
    assert!(
        found.iter().any(|offer| offer.candidate.path == script),
        "a week-old entry was kept with no sessions folder: {:?}",
        found
            .iter()
            .map(|offer| &offer.candidate.path)
            .collect::<Vec<_>>()
    );
}
