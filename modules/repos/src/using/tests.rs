use std::fs;
use std::os::unix::fs::symlink;

use super::*;

const CHECKOUT: &str = "/home/someone/src/core.nix";
const INSIDE: &str = "/home/someone/src/core.nix/.trees/fix";
const TREE: &str = "/home/someone/trees/core.nix-fix";

fn folders() -> Vec<PathBuf> {
    [CHECKOUT, INSIDE, TREE].map(PathBuf::from).to_vec()
}

fn process(pid: u32, folder: &str, started: u64, ticks: u64, memory: u64) -> Process {
    Process {
        pid,
        parent: 0,
        folder: Some(PathBuf::from(folder)),
        started,
        own: ticks,
        children: 0,
        memory: Some(memory),
    }
}

/// A process `parent` started, in the tree, since tick 10, with its own
/// ticks and those of the children it has waited on.
fn child(pid: u32, parent: u32, own: u64, children: u64) -> Process {
    Process {
        parent,
        children,
        ..process(pid, TREE, 10, own, 0)
    }
}

/// The same, working in no folder the reader can place.
fn unplaced(pid: u32, parent: u32, own: u64, children: u64) -> Process {
    Process {
        folder: None,
        memory: None,
        ..child(pid, parent, own, children)
    }
}

/// A reading read whole, as much waking time since the first as since boot.
fn reading(at: u64, processes: Vec<Process>) -> Sample {
    Sample {
        at,
        awake: Some(at),
        processes,
    }
}

#[test]
fn a_process_is_placed_in_the_deepest_folder_holding_it() {
    let folders = folders();
    let at = |folder: &str| placed(&folders, Path::new(folder)).map(Path::to_path_buf);
    assert_eq!(
        at("/home/someone/src/core.nix/modules/repos"),
        Some(CHECKOUT.into())
    );
    assert_eq!(
        at("/home/someone/src/core.nix/.trees/fix/modules"),
        Some(INSIDE.into())
    );
    assert_eq!(at(TREE), Some(TREE.into()), "a tree's own folder is in it");
    assert_eq!(
        at("/home/someone/src/core.nix-old"),
        None,
        "a name sharing letters isn't inside"
    );
    assert_eq!(at("/home/someone"), None);
}

#[test]
fn each_folder_sums_its_processes_memory() {
    let now = reading(
        1_600,
        vec![
            process(1, CHECKOUT, 10, 0, 2_000),
            process(2, CHECKOUT, 10, 0, 3_000),
            process(3, TREE, 10, 0, 5_000),
        ],
    );
    let using = using(None, &now);
    assert_eq!(
        using.get(Path::new(CHECKOUT)),
        Some(&Use {
            processes: 2,
            memory: 5_000,
            cpu: None
        })
    );
    assert_eq!(
        using.get(Path::new(TREE)),
        Some(&Use {
            processes: 1,
            memory: 5_000,
            cpu: None
        })
    );
    assert_eq!(
        using.get(Path::new(INSIDE)),
        None,
        "a folder nothing works in isn't said"
    );
}

#[test]
fn processor_time_is_a_share_of_one_core_between_two_readings() {
    let before = reading(
        1_000,
        vec![process(1, TREE, 10, 100, 0), process(3, TREE, 20, 50, 0)],
    );
    let now = reading(
        1_600,
        vec![
            // Worked 300 ticks of the 600 between: half a core.
            process(1, TREE, 10, 400, 0),
            // Started since the first reading: its 150 ticks are all since.
            process(2, TREE, 1_200, 150, 0),
            // The same id, a later process: counted from its start.
            process(3, TREE, 1_300, 30, 0),
            // Older than the first reading but not in it, so it moved in:
            // nothing says how much it worked here.
            process(4, TREE, 500, 900, 0),
        ],
    );
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some((300.0 + 150.0 + 30.0) / 600.0));
}

#[test]
fn a_child_counted_as_it_worked_is_not_counted_again_once_waited_on() {
    // The kernel moves a reaped child's whole time into its parent's
    // cutime, though each reading that saw it counted it already: Joel's
    // zsh would read as eight cores the minute a long cargo ended.
    let before = reading(1_000, vec![child(1, 0, 10, 0), child(2, 1, 48_000, 0)]);
    let now = reading(7_000, vec![child(1, 0, 10, 48_600)]);
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some(600.0 / 6_000.0), "only cargo's last 600 ticks");
}

#[test]
fn a_child_out_of_every_folder_is_taken_off_only_once_waited_on() {
    // A shell that ran a long build cds out of every tree, or ends and
    // isn't yet waited on: its parent hasn't its time yet, so nothing comes
    // off the parent's own work, and all of it comes off once it's reaped.
    let before = reading(1_000, vec![child(1, 0, 0, 0), child(2, 1, 30_000, 0)]);
    let out = reading(7_000, vec![child(1, 0, 600, 0), unplaced(2, 1, 30_100, 0)]);
    let reaped = reading(13_000, vec![child(1, 0, 600, 30_200)]);
    let share = |before, now| using(Some(before), now)[Path::new(TREE)].cpu;
    assert_eq!(
        share(&before, &out),
        Some(600.0 / 6_000.0),
        "while it's out"
    );
    assert_eq!(
        share(&out, &reaped),
        Some(100.0 / 6_000.0),
        "once it's reaped, only its last 100 ticks"
    );
}

#[test]
fn a_child_the_reader_cannot_look_in_is_taken_off_once_waited_on() {
    // sudo is setuid, so its working folder can't be read, but its stat
    // can: Joel's rebuild under it lands in zsh's time when it ends.
    let before = reading(
        1_000,
        vec![
            child(1, 0, 0, 0),
            unplaced(2, 1, 10, 0),
            unplaced(3, 2, 12_000, 0),
        ],
    );
    let now = reading(7_000, vec![child(1, 0, 0, 12_100)]);
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(
        cpu,
        Some(90.0 / 6_000.0),
        "only the rebuild's last 90 ticks"
    );
}

#[test]
fn a_child_waited_on_elsewhere_leaves_its_ancestors_own_work() {
    // A killed shell's cargo is handed to a subreaper, which waits on it:
    // node's children's time never grew, and its own work still counts.
    let before = reading(1_000, vec![child(1, 0, 0, 0), child(2, 1, 30_000, 0)]);
    let now = reading(7_000, vec![child(1, 0, 600, 0)]);
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some(600.0 / 6_000.0));
}

#[test]
fn a_grandchild_gone_with_its_parent_is_taken_off_the_one_still_working() {
    // bash waited on cargo, which had waited on a test that ran across the
    // first reading; node, still working, now holds all three's time.
    let before = reading(
        1_000,
        vec![
            child(1, 0, 0, 0),
            child(2, 1, 1_000, 0),
            child(3, 2, 2_000, 0),
        ],
    );
    let now = reading(2_000, vec![child(1, 0, 0, 3_200)]);
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some(200.0 / 1_000.0));
}

#[test]
fn a_suspend_between_readings_measures_the_waking_time() {
    // The time since boot runs on while asleep, and processor time doesn't.
    let before = reading(1_000, vec![process(1, TREE, 10, 100, 0)]);
    let now = Sample {
        at: 1_000 + 60 * 60 * TICKS,
        ..reading(1_600, vec![process(1, TREE, 10, 400, 0)])
    };
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some(300.0 / 600.0));
}

#[test]
fn a_reading_held_up_measures_no_share_either_side() {
    // A reading held up past EVEN read the processes after the hold late.
    let before = reading(1_000, vec![process(1, TREE, 10, 100, 0)]);
    let held = Sample {
        awake: None,
        ..reading(1_600, vec![process(1, TREE, 10, 400, 0)])
    };
    let after = reading(2_200, vec![process(1, TREE, 10, 700, 0)]);
    assert!(!measured(Some(&before), &held), "the one held up");
    assert!(!measured(Some(&held), &after), "the one after it");
    assert_eq!(using(Some(&held), &after)[Path::new(TREE)].cpu, None);
}

fn proc_with(name: &str) -> PathBuf {
    let proc = std::env::temp_dir().join(format!("repos-using-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&proc);
    fs::create_dir_all(&proc).expect("a folder to stand for /proc");
    fs::write(proc.join("uptime"), "16.25 40.00\n").expect("uptime");
    proc
}

fn pid(proc: &Path, pid: u32, cwd: Option<&str>, stat: &str, rollup: Option<&str>) {
    let folder = proc.join(pid.to_string());
    fs::create_dir_all(&folder).expect("a process's folder");
    if let Some(cwd) = cwd {
        symlink(cwd, folder.join("cwd")).expect("its working folder");
    }
    fs::write(folder.join("stat"), stat).expect("its stat");
    if let Some(rollup) = rollup {
        fs::write(folder.join("smaps_rollup"), rollup).expect("its memory");
    }
}

/// A `stat` line: the id, the name in brackets, then fields 3 to 22, with
/// `utime` and `stime` at 14 and 15 and `starttime` at 22.
fn stat(pid: u32, name: &str, utime: u64, stime: u64, started: u64) -> String {
    format!(
        "{pid} ({name}) R 1 1 1 0 -1 4194560 100 0 0 0 {utime} {stime} \
         0 0 20 0 1 0 {started} 1000 10"
    )
}

#[test]
fn a_reading_keeps_the_processes_working_in_a_folder() {
    let proc = proc_with("keeps");
    let rollup = "56447141e000-7ffe6ac26000 ---p 00000000 00:00 0 [rollup]\n\
                  Rss:                5036 kB\n\
                  Pss:                 894 kB\n";
    pid(
        &proc,
        41,
        Some("/home/someone/trees/core.nix-fix/modules"),
        &stat(41, "cargo", 70, 30, 900),
        Some(rollup),
    );
    // A name with a bracket and a space in it, read from its last bracket.
    pid(&proc, 42, Some(TREE), &stat(42, "a) b", 5, 5, 1_000), None);
    pid(
        &proc,
        43,
        Some("/home/someone/elsewhere"),
        &stat(43, "zsh", 1, 1, 10),
        Some(rollup),
    );
    // One the reader may not look in has no working folder to read.
    pid(&proc, 44, None, &stat(44, "sshd", 1, 1, 10), None);
    fs::create_dir_all(proc.join("self")).expect("a folder that isn't a process");

    let mut sample = sample(&proc, &folders());
    sample.processes.sort_by_key(|process| process.pid);
    assert_eq!(sample.at, 1_625, "16.25 s since boot, in ticks");
    assert!(sample.awake.is_some(), "a quick walk read as held up");
    let read = |pid, folder: Option<&str>, started, own, memory| Process {
        pid,
        parent: 1,
        folder: folder.map(PathBuf::from),
        started,
        own,
        children: 0,
        memory,
    };
    assert_eq!(
        sample.processes,
        [
            read(41, Some(TREE), 900, 100, Some(894 * 1024)),
            read(42, Some(TREE), 1_000, 10, None),
            // Placed nowhere, so its memory is never read, but its time is
            // read, as a child's may land in a parent that is placed.
            read(43, None, 10, 2, None),
            read(44, None, 10, 2, None),
        ]
    );
    let _ = fs::remove_dir_all(&proc);
}

#[test]
fn a_process_counts_the_children_it_has_waited_on() {
    // A build's compilers start and end between two readings; the cargo
    // that waited on them holds their time, in fields 16 and 17.
    let proc = proc_with("children");
    let cargo = "41 (cargo) S 1 1 1 0 -1 4194560 100 0 0 0 70 30 \
                 4000 1000 20 0 1 0 900 1000 10";
    pid(&proc, 41, Some(TREE), cargo, None);
    let sample = sample(&proc, &folders());
    assert_eq!(
        sample
            .processes
            .iter()
            .map(|process| (process.own, process.children))
            .collect::<Vec<_>>(),
        [(100, 5_000)],
        "its own 100 ticks and its children's 5,000"
    );
    let _ = fs::remove_dir_all(&proc);
}

#[test]
fn a_process_in_a_removed_folder_works_nowhere() {
    // The kernel says a working folder that was removed with " (deleted)"
    // after it, which would place it in the checkout holding that tree.
    let proc = proc_with("deleted");
    let gone = format!("{CHECKOUT}/.trees/gone (deleted)");
    pid(
        &proc,
        41,
        Some(&gone),
        &stat(41, "cargo", 70, 30, 900),
        None,
    );
    let folders: Vec<_> = sample(&proc, &folders())
        .processes
        .into_iter()
        .map(|process| process.folder)
        .collect();
    assert_eq!(folders, [None]);
    let _ = fs::remove_dir_all(&proc);
}

#[test]
fn a_walk_past_even_is_read_as_held_up() {
    let proc = proc_with("held");
    pid(&proc, 41, Some(TREE), &stat(41, "cargo", 70, 30, 900), None);
    let walked = |took: u64| {
        let mut times = [500, 500 + took].into_iter();
        sample_by(&proc, &folders(), || times.next().unwrap_or(u64::MAX)).awake
    };
    assert_eq!(walked(EVEN), Some(500), "a walk of EVEN is whole");
    assert_eq!(walked(EVEN + 1), None, "a walk past EVEN is held up");
    let _ = fs::remove_dir_all(&proc);
}

#[test]
fn readings_far_apart_measure_no_share() {
    // A share is of one recent span, not of the time since one long gone.
    let before = reading(1_000, vec![process(1, TREE, 10, 100, 0)]);
    let late = reading(1_000 + LONGEST + 1, vec![process(1, TREE, 10, 400, 0)]);
    assert!(
        !measured(Some(&before), &late),
        "readings past LONGEST apart"
    );
    assert_eq!(using(Some(&before), &late)[Path::new(TREE)].cpu, None);
    let soon = reading(1_000 + LONGEST, late.processes);
    assert!(measured(Some(&before), &soon), "readings LONGEST apart");
    assert!(!measured(Some(&before), &before), "readings no time apart");
    assert!(!measured(None, &soon), "a first reading");
}

#[test]
fn nothing_is_read_where_there_is_no_proc() {
    let gone = std::env::temp_dir().join(format!("repos-using-{}-none", std::process::id()));
    assert_eq!(sample(&gone, &folders()), Sample::default());
}
