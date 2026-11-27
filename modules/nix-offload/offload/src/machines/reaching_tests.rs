//! The tests of whether ssh reaches a machine nearby, and of what it said
//! kept a minute.

use super::*;
use crate::testing::executable;
use std::time::SystemTime;

#[test]
fn ssh_asked_of_a_machine_nearby_never_asks_and_gives_up_in_15_s() {
    let args = reaching_args("-mac");
    let options: Vec<&str> = args
        .windows(2)
        .filter(|pair| pair[0] == "-o")
        .map(|pair| pair[1].as_str())
        .collect();
    for option in [
        "BatchMode=yes",
        "StrictHostKeyChecking=yes",
        "ConnectTimeout=15",
        "ServerAliveInterval=5",
        "ServerAliveCountMax=2",
        "UpdateHostKeys=no",
        "CheckHostIP=no",
    ] {
        assert!(options.contains(&option), "{option} in {args:?}");
    }
    assert_eq!(
        options
            .iter()
            .filter(|option| option.starts_with("ConnectTimeout="))
            .count(),
        1,
        "ssh is given one connection timeout: {args:?}"
    );
    assert_eq!(
        args[args.len() - 3..],
        ["--", "-mac", "true"],
        "it runs true there, the host never read as an option"
    );
}

#[test]
fn ssh_reaching_a_machine_nearby_says_whether_it_ran_there() {
    let folder = tempfile::tempdir().unwrap();
    let ssh = folder.path().join("ssh");
    let run = |ssh: &Path| ssh_reaches(&ssh.display().to_string(), "mac", REACHING);
    executable(&ssh, "#!/bin/sh\nexit 0\n");
    assert_eq!(
        run(&ssh),
        Reached::Ran,
        "an ssh that ran true there is nearby"
    );
    executable(
        &ssh,
        "#!/bin/sh\necho 'debug1: first' >&2\necho 'Host key verification failed.' >&2\n\
         exit 255\n",
    );
    assert_eq!(
        run(&ssh),
        Reached::Refused("Host key verification failed.".to_owned()),
        "a refused key is said apart from a machine away"
    );
    executable(
        &ssh,
        "#!/bin/sh\necho '@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@' >&2\n\
         echo 'Offending key in known_hosts:3' >&2\nexit 255\n",
    );
    assert_eq!(
        run(&ssh),
        Reached::Refused("Offending key in known_hosts:3".to_owned()),
        "a changed key's banner alone is a refused key, said with ssh's last line"
    );
    executable(
        &ssh,
        "#!/bin/sh\necho 'ssh: connect to host mac port 22: Connection refused' >&2\n\
         exit 255\n",
    );
    assert_eq!(
        run(&ssh),
        Reached::Away("ssh: connect to host mac port 22: Connection refused".to_owned()),
        "a machine away is said with ssh's last line"
    );
    let missing = folder.path().join("missing");
    assert!(
        matches!(run(&missing), Reached::Away(why) if why.contains("couldn't run")),
        "no ssh to ask is away"
    );
}

// An ssh that never answers is given up on at its deadline, and what
// it started, here a sleep holding its standard error open, ends with
// it, so nothing outlives the ask or holds its read open.
#[test]
fn ssh_that_never_answers_is_ended_with_what_it_started() {
    let folder = tempfile::tempdir().unwrap();
    let ssh = folder.path().join("ssh");
    let pid = folder.path().join("pid");
    executable(
        &ssh,
        &format!(
            "#!/bin/sh\nsleep 30 &\necho $! > '{}'\nsleep 30\n",
            pid.display()
        ),
    );
    // Two seconds, so even a loaded machine's sh writes the pid first.
    let began = Instant::now();
    let reached = ssh_reaches(&ssh.display().to_string(), "mac", Duration::from_secs(2));
    assert!(
        began.elapsed() < Duration::from_secs(6),
        "the ask outlasted its deadline: {:?}",
        began.elapsed()
    );
    assert!(
        matches!(&reached, Reached::Away(why) if why.contains("didn't answer")),
        "an ssh that never answered isn't away: {reached:?}"
    );
    let started = fs::read_to_string(&pid).unwrap();
    assert!(
        ended(started.trim()),
        "what ssh started outlived it: {started}"
    );

    // One that ends at once, leaving a child holding its standard
    // error, is read without waiting on that child.
    executable(
        &ssh,
        &format!(
            "#!/bin/sh\nsleep 30 &\necho $! > '{}'\necho gone >&2\nexit 1\n",
            pid.display()
        ),
    );
    let began = Instant::now();
    let reached = ssh_reaches(&ssh.display().to_string(), "mac", REACHING);
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "a child holding ssh's standard error held the ask: {:?}",
        began.elapsed()
    );
    assert_eq!(
        reached,
        Reached::Away("gone".to_owned()),
        "ssh's last line is lost"
    );
    let started = fs::read_to_string(&pid).unwrap();
    assert!(
        ended(started.trim()),
        "what ssh started outlived it: {started}"
    );
}

// One that refuses a key and ends, leaving a proxy in a session of
// its own holding its standard error, which the group's end can't
// reach, is said as soon as that error goes quiet.
#[test]
fn ssh_refusing_a_key_is_heard_though_a_proxy_holds_its_error_open() {
    let folder = tempfile::tempdir().unwrap();
    let ssh = folder.path().join("ssh");
    let pid = folder.path().join("pid");
    executable(
        &ssh,
        &format!(
            "#!/bin/sh\nsetsid sleep 30 &\necho $! > '{}'\n\
             echo 'Host key verification failed.' >&2\nexit 255\n",
            pid.display()
        ),
    );
    let began = Instant::now();
    let reached = ssh_reaches(&ssh.display().to_string(), "mac", REACHING);
    let took = began.elapsed();
    let started = fs::read_to_string(&pid)
        .ok()
        .and_then(|started| started.trim().parse::<libc::pid_t>().ok());
    // SAFETY: kill only signals the sleep this test started.
    let was_running =
        started.is_some_and(|started| unsafe { libc::kill(started, libc::SIGKILL) } == 0);
    assert!(
        was_running,
        "the proxy never ran, or had ended, so nothing held ssh's error open: {started:?}"
    );
    assert!(
        took < Duration::from_secs(5),
        "a proxy holding ssh's error held the ask {took:?}"
    );
    assert_eq!(
        reached,
        Reached::Refused("Host key verification failed.".to_owned()),
        "a refused key behind a proxy is lost"
    );
}

/// Whether process `pid` has ended, or is left only to be reaped,
/// within two seconds: gone, as signalling it says, or a zombie.
fn ended(pid: &str) -> bool {
    let Ok(pid) = pid.parse::<libc::pid_t>() else {
        return false;
    };
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(2) {
        // SAFETY: signal 0 only asks whether the process is there.
        let gone = unsafe { libc::kill(pid, 0) } != 0
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
        let zombie = fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .as_deref()
            .and_then(|stat| stat.rsplit_once(") "))
            .is_some_and(|(_, after)| after.starts_with('Z'));
        if gone || zombie {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

/// What ssh says of a machine, `reaches` or not, with why not.
fn said(reaches: bool) -> Said {
    Said {
        reaches,
        why: if reaches {
            String::new()
        } else {
            "ssh doesn't reach mac either: gone".to_owned()
        },
    }
}

/// A run's own memo, empty, as each run starts with.
fn a_run() -> Memo {
    Mutex::new(Vec::new())
}

/// Ages `file` past what's believed.
fn aged(file: &Path) {
    fs::File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(SystemTime::now() - REACHED_FOR - Duration::from_secs(1))
        .unwrap();
}

// What ssh said is kept a minute, in a file a machine in the runtime
// folder, so the runs a run starts ask once, a machine away carrying
// why; past that it's asked again, and a name can't reach outside the
// folder.
#[test]
fn what_ssh_said_of_a_machine_is_kept_a_minute() {
    let folder = tempfile::tempdir().unwrap();
    let kept = Some(folder.path());
    let asked = std::cell::Cell::new(0);
    let ask = |reaches: bool| {
        asked.set(asked.get() + 1);
        said(reaches)
    };
    assert!(
        reached_lately(&a_run(), kept, "mac", || ask(true)),
        "a machine ssh reached wasn't said to be reached"
    );
    assert!(
        reached_lately(&a_run(), kept, "mac", || ask(false)),
        "what another run's ssh said a moment ago wasn't read"
    );
    assert_eq!(asked.get(), 1, "ssh was asked again within the minute");
    let file = folder.path().join("mac");
    assert_eq!(fs::read_to_string(&file).unwrap(), "reaches\n");

    aged(&file);
    assert!(
        !reached_lately(&a_run(), kept, "mac", || ask(false)),
        "what ssh said over a minute ago was believed"
    );
    assert_eq!(asked.get(), 2, "ssh wasn't asked again past the minute");
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "away\nssh doesn't reach mac either: gone\n"
    );
    let later = a_run();
    assert!(
        !reached_lately(&later, kept, "mac", || ask(true)),
        "a machine away a moment ago was read as reached"
    );
    assert_eq!(asked.get(), 2, "ssh was asked again within the minute");
    assert_eq!(
        later.lock().unwrap()[0].2,
        said(false),
        "why it's away wasn't read back"
    );

    assert!(
        reached_lately(&a_run(), kept, "mac.local", || ask(true)),
        "mac.local was read as mac"
    );
    assert_eq!(asked.get(), 3, "mac.local beside mac wasn't asked of");
    fs::write(&file, "maybe\n").unwrap();
    assert!(
        reached_lately(&a_run(), kept, "mac", || ask(true)),
        "a garbled file was believed"
    );
    assert_eq!(asked.get(), 4, "a garbled file wasn't asked past");
    let mut names: Vec<String> = fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["mac", "mac.local"], "a draft was left behind");
    assert_eq!(reached_name("../mac.local"), "_._mac.local");
}

// A folder that can't be written, or none, leaves each run asking
// once, from its own memo, rather than at every look.
#[test]
fn with_nowhere_to_keep_it_each_run_asks_once() {
    let folder = tempfile::tempdir().unwrap();
    let not_a_folder = folder.path().join("a-file");
    fs::write(&not_a_folder, "").unwrap();
    for kept in [Some(not_a_folder.as_path()), None] {
        let asked = std::cell::Cell::new(0);
        let ask = || {
            asked.set(asked.get() + 1);
            said(false)
        };
        let run = a_run();
        for _ in 0..3 {
            assert!(
                !reached_lately(&run, kept, "mac", ask),
                "away read as reached"
            );
        }
        assert_eq!(
            asked.get(),
            1,
            "a run asked again within the minute: {kept:?}"
        );
        reached_lately(&a_run(), kept, "mac", ask);
        assert_eq!(
            asked.get(),
            2,
            "another run read what can't be kept: {kept:?}"
        );
    }
}

// A machine forgotten is asked of afresh by the run that forgot it,
// and what ssh said of it leaves the folder, though another machine's
// stays.
#[test]
fn a_machine_forgotten_is_asked_of_again() {
    let folder = tempfile::tempdir().unwrap();
    let kept = Some(folder.path());
    let asked = std::cell::Cell::new(0);
    let ask = |reaches: bool| {
        asked.set(asked.get() + 1);
        said(reaches)
    };
    let run = a_run();
    assert!(
        reached_lately(&run, kept, "mac", || ask(true)),
        "a machine ssh reaches read as unreached"
    );
    assert!(
        reached_lately(&run, kept, "mac.local", || ask(true)),
        "a second machine ssh reaches read as unreached"
    );
    forget_reached_in(&run, kept, "mac");
    assert!(
        !folder.path().join("mac").exists(),
        "what ssh said was left in the folder"
    );
    assert!(
        folder.path().join("mac.local").exists(),
        "another machine was forgotten"
    );
    assert!(
        !reached_lately(&run, kept, "mac", || ask(false)),
        "a forgotten machine was read as reached"
    );
    assert_eq!(asked.get(), 3, "a forgotten machine wasn't asked of again");
}
