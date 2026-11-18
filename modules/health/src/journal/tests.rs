//! The journal's lines, case by case: what each says and where it may
//! say it from.

use super::*;

#[test]
fn a_warning_is_its_programs() {
    let line = r#"{"__REALTIME_TIMESTAMP":"1794450621321755","PRIORITY":"4",
        "SYSLOG_IDENTIFIER":"pebble-lights","_SYSTEMD_USER_UNIT":"pebble-lights.service",
        "MESSAGE":"light 3 answered in 812 ms"}"#
        .replace('\n', "");
    let entry = parse(&line).expect("a journal line is an entry");
    assert_eq!(
        entry.thing, "pebble-lights",
        "the identifier names the thing"
    );
    assert_eq!(entry.priority, 4, "the priority is read");
    assert_eq!(entry.at, 1_794_450_621_321_755, "the time is read");
    assert_eq!(
        entry.ran_in.as_deref(),
        Some("pebble-lights.service"),
        "its unit"
    );
    assert!(
        !entry.restart && !entry.kernel,
        "it's neither a restart nor the kernel's"
    );
}

#[test]
fn systemd_speaking_of_a_unit_is_the_units() {
    let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
        "_SYSTEMD_CGROUP":"/user.slice/user-1000.slice/user@1000.service/init.scope",
        "USER_UNIT":"merge-net.service","MESSAGE_ID":"5eb03494b6584870a536b337290809b3",
        "MESSAGE":"merge-net.service: Scheduled restart job, restart counter is at 1."}"#
        .replace('\n', "");
    let entry = parse(&line).expect("a journal line is an entry");
    assert_eq!(
        entry.thing, "merge-net.service",
        "the unit spoken of is the thing"
    );
    assert!(entry.restart, "systemd's restart message is a restart");
}

/// His account's manager's cgroup, below which his slices sit.
const ACCOUNT: &str = "/user.slice/user-1000.slice/user@1000.service";

/// A line of `fields` written from `cgroup`, or from none when it's empty.
fn from(cgroup: &str, fields: &str) -> Entry {
    let cgroup = if cgroup.is_empty() {
        String::new()
    } else {
        format!(r#""_SYSTEMD_CGROUP":"{cgroup}","#)
    };
    let line = format!(r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6",{cgroup}{fields}}}"#);
    parse(&line).expect("a journal line is an entry")
}

#[test]
fn the_launcher_is_told_ready_only_by_his_shell() {
    let work = format!("{ACCOUNT}/jobs.slice/jobs-work.slice/work-9-1.scope");
    let ready = r#""SYSLOG_IDENTIFIER":"quickshell",
        "MESSAGE":"launcher: home kept as the shell started""#
        .replace('\n', "");
    let ready = ready.as_str();
    let shell = format!("{ACCOUNT}/session.slice/quickshell.service");
    assert_eq!(
        from(&shell, ready).mark,
        Some(Mark::Ready),
        "his shell tells the launcher ready"
    );
    assert_eq!(from(&work, ready).mark, None, "a lab's shell doesn't");
    let beside = format!("{ACCOUNT}/session.slice/run-u9.service");
    assert_eq!(
        from(&beside, ready).mark,
        None,
        "nor a unit a job starts in his session"
    );
    let timed = format!("{ACCOUNT}/jobs.slice/jobs-timed.slice/timed.service");
    assert_eq!(
        from(&timed, ready).mark,
        None,
        "nor one the timed work runs"
    );
    let agents = format!("{ACCOUNT}/agents.slice/sessions-kitty.service");
    assert_eq!(
        from(&agents, ready).mark,
        None,
        "nor one an agent's terminal starts, before its work is moved"
    );
    assert_eq!(from("", ready).mark, None, "nor a line from nowhere");
}

#[test]
fn systemds_news_is_taken_only_from_where_systemd_writes_it() {
    let restart = format!(
        r#""SYSLOG_IDENTIFIER":"systemd","USER_UNIT":"quickshell.service",
        "MESSAGE_ID":"{RESTART_SCHEDULED}",
        "MESSAGE":"quickshell.service: Scheduled restart job.""#
    )
    .replace('\n', "");
    let told = from(&format!("{ACCOUNT}/init.scope"), &restart);
    assert!(
        told.restart && told.thing == "quickshell.service",
        "the account's manager tells a restart of a unit"
    );
    assert_eq!(told.origin, Origin::Systemd, "and is told apart as systemd");
    assert!(
        from("/init.scope", &restart).restart,
        "and so does the machine's"
    );
    for elsewhere in [
        format!("{ACCOUNT}/jobs.slice/jobs.service"),
        format!("{ACCOUNT}/jobs.slice/jobs-work.slice/jobs-work-delta.slice/work-9-1.scope"),
        format!("{ACCOUNT}/jobs.slice/jobs-timed.slice/timed.service"),
        format!("{ACCOUNT}/session.slice/run-u9.service"),
        format!("{ACCOUNT}/app.slice/init.scope"),
        format!("{ACCOUNT}/jobs.slice/jobs-timed.slice/timed.service{ACCOUNT}/init.scope"),
        format!("{ACCOUNT}/init.scope/below"),
        "/user.slice/user-1000.slice/user@1001.service/init.scope".to_owned(),
        String::new(),
    ] {
        let forged = from(&elsewhere, &restart);
        assert!(
            !forged.restart,
            "a restart of his shell said from {elsewhere:?} is made up"
        );
        assert_eq!(
            forged.thing, "systemd",
            "and it doesn't speak as systemd of his unit from {elsewhere:?}"
        );
    }
    let work = format!("{ACCOUNT}/jobs.slice/jobs-work.slice/work-9-1.scope");
    let stopping = from(
        &work,
        &format!(
            r#""SYSLOG_IDENTIFIER":"quickshell.service",
            "MESSAGE_ID":"{UNIT_STOPPING}","MESSAGE":"Stopping""#
        )
        .replace('\n', ""),
    );
    assert_eq!(
        (stopping.thing.as_str(), stopping.job),
        ("quickshell.service", None),
        "a job named as his unit can't say it's stopping"
    );
    let started = from(
        &work,
        &format!(
            r#""MESSAGE_ID":"{UNIT_STARTED}","UNIT":"systemd-ask-password-plymouth.service",
            "MESSAGE":"Started.""#
        )
        .replace('\n', ""),
    );
    assert_eq!(
        (started.mark, started.job),
        (None, None),
        "nor that a unit started, the password's asking among them"
    );
    let boot = format!(
        r#""MESSAGE_ID":"{STARTUP_FINISHED}","KERNEL_USEC":"1",
        "MESSAGE":"Startup finished.""#
    )
    .replace('\n', "");
    assert_eq!(from(&work, &boot).mark, None, "nor how long the boot took");
    let down = format!(r#""MESSAGE_ID":"{SHUTTING_DOWN}","MESSAGE":"System is rebooting.""#);
    // Spelled as journald writes it, not from the constant under test.
    let logind = "/system.slice/systemd-logind.service";
    assert_eq!(
        from(logind, &down).mark,
        Some(Mark::Stopping),
        "logind tells the machine going down"
    );
    assert_eq!(from(&work, &down).mark, None, "and nobody else does");
}

#[test]
fn a_line_says_where_it_was_written() {
    let origin = |cgroup: &str| {
        let line =
            format!(r#"{{"__REALTIME_TIMESTAMP":"1","_SYSTEMD_CGROUP":"{cgroup}","MESSAGE":"m"}}"#);
        parse(&line).expect("a journal line is an entry").origin
    };
    assert_eq!(
        origin("/init.scope"),
        Origin::Systemd,
        "the machine's manager"
    );
    assert_eq!(
        origin(&format!("{ACCOUNT}/session.slice/quickshell.service")),
        Origin::Shell,
        "his shell"
    );
    for elsewhere in [
        "session.slice/run-u9.service",
        "agents.slice/mycelium-pi-1.scope",
        "app.slice/windows.service",
        "background.slice/pi-packages.service",
    ] {
        assert_eq!(
            origin(&format!("{ACCOUNT}/{elsewhere}")),
            Origin::Elsewhere,
            "{elsewhere} is neither systemd, his shell nor the jobs"
        );
    }
    assert_eq!(
        origin(&format!(
            "{ACCOUNT}/jobs.slice/jobs-awaited.slice/awaited.service"
        )),
        Origin::Jobs,
        "work he waits on"
    );
    assert_eq!(
        origin(&format!("{ACCOUNT}/app.slice/session.slice/x.scope")),
        Origin::Elsewhere,
        "a slice counts only where the manager puts it"
    );
    assert_eq!(origin(""), Origin::Unknown, "a line with no cgroup");
}

#[test]
fn a_crash_in_work_he_waits_on_is_his() {
    let crash = |slice: &str| {
        let line = format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"2",
            "SYSLOG_IDENTIFIER":"systemd-coredump","COREDUMP_COMM":"cargo",
            "COREDUMP_CGROUP":"/user.slice/user-1000.slice/user@1000.service/{slice}",
            "MESSAGE":"Process 1 (cargo) of user 1000 dumped core."}}"#
        )
        .replace('\n', "");
        parse(&line).expect("a crash is an entry").whose
    };
    assert_eq!(
        crash("jobs.slice/jobs-awaited.slice/awaited.service"),
        Some(Whose::His),
        "what he waits on at a terminal is his"
    );
    assert_eq!(
        crash("jobs.slice/jobs-timed.slice/keeps.service"),
        Some(Whose::Agents),
        "the machine's timed work isn't"
    );
    assert_eq!(
        crash("session.slice/quickshell.service"),
        Some(Whose::His),
        "his session is his"
    );
    assert_eq!(
        crash("jobs.slice/app.slice-lab/x.scope"),
        Some(Whose::Agents),
        "a slice is matched whole, not by its name's end"
    );
    assert_eq!(
        crash("jobs.slice/jobs-timed.slice/timed.service/app.slice/x.scope"),
        Some(Whose::Agents),
        "a slice counts only where the manager puts it, not one a unit made below itself"
    );
}

#[test]
fn a_crash_is_taken_only_as_systemd_coredump_tells_it() {
    let from = |cgroup: &str| {
        let line = format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"2",
            "SYSLOG_IDENTIFIER":"systemd-coredump","COREDUMP_COMM":"kitty",
            "COREDUMP_CGROUP":"/user.slice/user-1000.slice/user@1000.service/app.slice/k.scope",
            {cgroup}"MESSAGE":"Process 1 (kitty) of user 1000 dumped core."}}"#
        )
        .replace('\n', "");
        parse(&line).expect("a crash is an entry").crashed
    };
    // Spelled as journald writes it, not from the constant under test, with
    // JSON escaping the slice's one backslash.
    let reporter = concat!(
        r#""_SYSTEMD_CGROUP":"/system.slice/system-systemd\\x2dcoredump.slice/"#,
        r#"systemd-coredump@9.service","#
    );
    assert_eq!(
        from(reporter).as_deref(),
        Some("kitty"),
        "systemd-coredump tells a crash"
    );
    assert_eq!(
        from("").as_deref(),
        Some("kitty"),
        "and its report may come with no cgroup, gone before journald looked"
    );
    assert_eq!(
        from(r#""_SYSTEMD_CGROUP":"/","#).as_deref(),
        Some("kitty"),
        "or from the root cgroup, should journald write it"
    );
    for elsewhere in [
        "/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-timed.slice/t.service",
        "/user.slice/user-1000.slice/user@1000.service/session.slice/run-u9.service",
        "/system.slice/systemd-coredump-lab.service",
    ] {
        assert_eq!(
            from(&format!(r#""_SYSTEMD_CGROUP":"{elsewhere}","#)),
            None,
            "a crash said from {elsewhere} is only a line"
        );
    }
}

#[test]
fn a_crash_says_whose_program_it_was() {
    let his = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"2",
        "SYSLOG_IDENTIFIER":"systemd-coredump","COREDUMP_COMM":"1password",
        "COREDUMP_CGROUP":"/user.slice/user-1000.slice/user@1000.service/app.slice/a.scope",
        "MESSAGE":"Process 4169227 (1password) of user 1000 dumped core."}"#
        .replace('\n', "");
    let agents = his
        .replace("1password", "chrome")
        .replace("/app.slice/", "/jobs.slice/jobs-timed.slice/");
    let his = parse(&his).expect("a crash is an entry");
    let agents = parse(&agents).expect("a crash is an entry");
    assert_eq!(
        his.crashed.as_deref(),
        Some("1password"),
        "the program is named"
    );
    assert_eq!(his.whose, Some(Whose::His), "a program in his slice is his");
    assert_eq!(
        agents.whose,
        Some(Whose::Agents),
        "a job's program is agents'"
    );
}

#[test]
fn a_line_says_whether_an_agents_command_wrote_it() {
    let from = |slice: &str| {
        let line = format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","SYSLOG_IDENTIFIER":"tec",
            "_SYSTEMD_CGROUP":"/user.slice/user-1000.slice/user@1000.service/{slice}",
            "MESSAGE":"could not reach the daemon"}}"#
        )
        .replace('\n', "");
        parse(&line).expect("a journal line is an entry").agents
    };
    assert!(
        !from("app.slice/app-niri-kitty-1.scope"),
        "a line from his own terminal is his"
    );
    assert!(
        from("jobs.slice/jobs-work.slice/jobs-work-delta.slice/work-9-1.scope"),
        "a line from an agent's command is agents'"
    );
    assert!(
        from("jobs.slice/jobs.service"),
        "a line from an agent's shell is agents'"
    );
    assert!(
        !from("jobs.slice/jobs-awaited.slice/awaited.service"),
        "what he waits on at a terminal is his"
    );
    assert!(
        !from("jobs.slice/jobs-timed.slice/keeps.service"),
        "the machine's timed work is the machine's"
    );
    assert!(
        !from("jobs.slice/sccache.service"),
        "the compile cache every build reads is the machine's"
    );
    assert!(
        !from("agents.slice/sessions-kitty.service"),
        "the terminal he reads sessions in, and the agents themselves, answer to him"
    );
    assert!(
        !from("app.slice/notjobs.slice/x.scope"),
        "a slice is matched whole, not by its name's end"
    );
}

#[test]
fn a_message_in_bytes_is_read_as_far_as_it_goes() {
    let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","_TRANSPORT":"kernel",
        "SYSLOG_IDENTIFIER":"kernel","MESSAGE":[110,118,109,101,255]}"#
        .replace('\n', "");
    let entry = parse(&line).expect("a kernel line is an entry");
    assert!(
        entry.message.starts_with("nvme"),
        "the bytes are read: {}",
        entry.message
    );
    assert!(entry.kernel, "the kernel's transport is the kernel's");
}

#[test]
fn a_service_stopping_or_started_is_followed_whatever_its_description_says() {
    // systemd says a service with a description by it, as in "Stopping
    // Quickshell, which draws the launcher...", so a follower that
    // greps the message for a unit's name hears no restart of one.
    let stopping = format!("MESSAGE_ID={UNIT_STOPPING}");
    let follower = followed()
        .into_iter()
        .find(|asked| asked.contains(&stopping))
        .expect("a service stopping is followed");
    assert_eq!(
        follower,
        [stopping, format!("MESSAGE_ID={UNIT_STARTED}")],
        "a service's stop and start are followed only where the message names its unit"
    );
}

#[test]
fn every_follower_asks_for_every_boot_last() {
    // journalctl follows only this boot unless asked otherwise, so the
    // day a reboot cut in two was counted from the reboot alone: on 17
    // November, 4834 warnings before it went uncounted. `-k` and `-b`
    // choose a boot too, so every boot is asked for after them, which
    // holds wherever the last choice wins. They count however they're
    // grouped, and a few are tried here whatever the followers hold
    // today.
    let chooses_a_boot = |arg: &str| {
        (arg.starts_with('-') && !arg.starts_with("--") && arg.contains(['k', 'b']))
            || arg == "--dmesg"
            || arg.starts_with("--boot")
            || arg.starts_with("--this-boot")
    };
    let mut asked_for = followed();
    asked_for.extend([
        vec!["-k".to_owned()],
        vec!["-kp".to_owned(), "notice..info".to_owned()],
        vec!["-b".to_owned()],
    ]);
    for asked in asked_for {
        assert!(
            !asked.iter().any(|arg| arg.starts_with("_BOOT_ID=")),
            "a follower matches one boot: {asked:?}"
        );
        for journalctl in [
            reading(&asked, 1, 2),
            following(&asked, None, 1),
            following(&asked, Some("s=1"), 1),
        ] {
            let args = words(&journalctl);
            assert_eq!(
                args.iter()
                    .rev()
                    .find(|arg| chooses_a_boot(arg))
                    .map(String::as_str),
                Some("--boot=all"),
                "a follower reads only one boot: {args:?}"
            );
        }
    }
}

fn words(journalctl: &std::process::Command) -> Vec<String> {
    journalctl
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_history_reads_one_day_to_one_moment_and_its_follower_follows_on() {
    let asked = ["-p".to_owned(), "warning".to_owned()];
    let has = |words: &[String], word: &str| words.iter().any(|said| said == word);
    let starts = |words: &[String]| {
        words
            .iter()
            .filter(|word| word.starts_with("--since") || word.starts_with("--after-cursor"))
            .count()
    };
    let history = words(&reading(&asked, 1_763_442_000, 1_763_460_000));
    assert!(
        has(&history, "--since=@1763442000")
            && has(&history, "--until=@1763460000")
            && !has(&history, "--follow")
            && starts(&history) == 1,
        "a history doesn't read from the day's start it was given to the moment and end: \
         {history:?}"
    );
    let after = words(&following(&asked, Some("s=abc;i=2"), 1_763_442_000));
    assert!(
        has(&after, "--follow")
            && has(&after, "--after-cursor=s=abc;i=2")
            && starts(&after) == 1
            && !after.iter().any(|word| word.starts_with("--until")),
        "a follower doesn't follow on from its history's cursor alone: {after:?}"
    );
    let empty = words(&following(&asked, None, 1_763_442_000));
    assert!(
        has(&empty, "--follow") && has(&empty, "--since=@1763442000") && starts(&empty) == 1,
        "a follower whose history had no lines doesn't start from its history's day: {empty:?}"
    );
}

#[test]
fn a_service_stopping_or_started_is_its_job_and_a_scope_is_none() {
    let line = |id: &str, unit: &str| {
        format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
            "_SYSTEMD_CGROUP":"/user.slice/user-1000.slice/user@1000.service/init.scope",
            "MESSAGE_ID":"{id}","USER_UNIT":"{unit}","MESSAGE":"Started {unit}"}}"#
        )
        .replace('\n', "")
    };
    let job = |id, unit| parse(&line(id, unit)).expect("systemd's line is an entry");
    let stopping = job(UNIT_STOPPING, "google.service");
    assert_eq!(
        (stopping.thing.as_str(), stopping.job),
        ("google.service", Some(Job::Stopping)),
        "a service stopping wasn't its job"
    );
    assert_eq!(
        job(UNIT_STARTED, "google.service").job,
        Some(Job::Started),
        "a service started wasn't its job"
    );
    assert_eq!(
        job(UNIT_STARTED, "work-1-2.scope").job,
        None,
        "a scope started was taken for a service's"
    );
}

#[test]
fn a_boot_says_how_long_each_stage_took() {
    let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
        "_BOOT_ID":"382e94","__MONOTONIC_TIMESTAMP":"55725000","_SYSTEMD_CGROUP":"/init.scope",
        "MESSAGE_ID":"b07a249cd024414a82dd00cd181378ff","KERNEL_USEC":"432199",
        "INITRD_USEC":"24362468","USERSPACE_USEC":"20671351",
        "MESSAGE":"Startup finished in 9.709s (firmware) + 549ms (loader) + 432ms (kernel) + 24.362s (initrd) + 20.671s (userspace) = 55.725s."}"#
        .replace('\n', "");
    let entry = parse(&line).expect("a boot's line is an entry");
    assert_eq!(
        entry.mark,
        Some(Mark::Started {
            firmware: 9_709_000,
            loader: 549_000,
            kernel: 432_199,
            initrd: 24_362_468,
            userspace: 20_671_351,
        }),
        "each stage is read"
    );
    assert_eq!(
        (entry.boot.as_str(), entry.since_boot),
        ("382e94", 55_725_000),
        "its boot and when in it"
    );
    let user = line.replace(r#""KERNEL_USEC":"432199","#, "");
    assert_eq!(
        parse(&user).expect("an entry").mark,
        None,
        "an account's manager starting isn't the machine's boot"
    );
}

#[test]
fn the_password_the_launcher_and_a_shutdown_are_marked() {
    let unit = |name: &str| {
        format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","_SYSTEMD_CGROUP":"/init.scope",
            "MESSAGE_ID":"{UNIT_STARTED}","UNIT":"{name}","MESSAGE":"Finished."}}"#
        )
        .replace('\n', "")
    };
    let mark = |line: &str| parse(line).expect("an entry").mark;
    assert_eq!(
        mark(&unit("systemd-ask-password-plymouth.service")),
        Some(Mark::Asked),
        "Plymouth asking is the password asked"
    );
    assert_eq!(
        mark(&unit("systemd-cryptsetup@system.service")),
        Some(Mark::Opened),
        "any disk opening"
    );
    assert_eq!(
        mark(&unit("nix-daemon.service")),
        None,
        "other units aren't marks"
    );
    // The shutdown is logind's to tell; the launcher's lines are the
    // shell's, from his session.
    let said = |message: &str, id: &str| {
        let from = if id.is_empty() {
            "/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service"
        } else {
            LOGIND
        };
        format!(
            r#"{{"__REALTIME_TIMESTAMP":"1","_SYSTEMD_CGROUP":"{from}",
            "MESSAGE_ID":"{id}","MESSAGE":"{message}"}}"#
        )
        .replace('\n', "")
    };
    assert_eq!(
        mark(&said(
            "launcher: home and 7 places kept as the shell started, 7054 rows made in 213 ms",
            ""
        )),
        Some(Mark::Ready),
        "the launcher's rows made is it ready"
    );
    assert_eq!(
        mark(&said(
            "launcher: home kept as the shell started, 7989 rows made in 35 ms",
            ""
        )),
        Some(Mark::Ready),
        "the launcher's rows made with no places kept is it ready"
    );
    assert_eq!(
        mark(&said(
            "System is rebooting.",
            "98268866d1d54a499c4e98921d93bc40"
        )),
        Some(Mark::Stopping),
        "logind's rebooting is the shutdown asked"
    );
}

#[test]
fn a_line_that_isnt_an_entry_is_none() {
    assert_eq!(parse("-- No entries --"), None, "words aren't an entry");
    assert_eq!(
        parse(r#"{"MESSAGE":"no time"}"#),
        None,
        "an entry needs its time"
    );
}
