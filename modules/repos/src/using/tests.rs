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
        folder: PathBuf::from(folder),
        started,
        ticks,
        memory: Some(memory),
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
    let now = Sample {
        at: 1_600,
        processes: vec![
            process(1, CHECKOUT, 10, 0, 2_000),
            process(2, CHECKOUT, 10, 0, 3_000),
            process(3, TREE, 10, 0, 5_000),
        ],
    };
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
    let before = Sample {
        at: 1_000,
        processes: vec![process(1, TREE, 10, 100, 0), process(3, TREE, 20, 50, 0)],
    };
    let now = Sample {
        at: 1_600,
        processes: vec![
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
    };
    let cpu = using(Some(&before), &now)[Path::new(TREE)].cpu;
    assert_eq!(cpu, Some((300.0 + 150.0 + 30.0) / 600.0));
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
    assert_eq!(
        sample.processes,
        [
            Process {
                pid: 41,
                folder: TREE.into(),
                started: 900,
                ticks: 100,
                memory: Some(894 * 1024),
            },
            Process {
                pid: 42,
                folder: TREE.into(),
                started: 1_000,
                ticks: 10,
                memory: None
            },
        ]
    );
    let _ = fs::remove_dir_all(&proc);
}

#[test]
fn nothing_is_read_where_there_is_no_proc() {
    let gone = std::env::temp_dir().join(format!("repos-using-{}-none", std::process::id()));
    assert_eq!(sample(&gone, &folders()), Sample::default());
}
