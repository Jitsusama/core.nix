//! Gives a session found in jobs a scope of its own beside the windows
//! again.
//!
//! A session's holder, mycelium's `holder` and the pi it runs, is Joel's
//! window whoever opened it, so its scope is never taken for a stray
//! (`strays` in src/main.rs). A session that reached jobs anyway stays
//! there for good, at jobs' idle weight: one moved before that rule, or
//! one started from inside jobs and split into a work scope there. On 14
//! November 37 sessions' pis had sat in jobs since a move the evening
//! before. The busiest waited 1.5 to 2.9 ms for a CPU for each ms they
//! ran, against 0.1 to 0.4 once given their scopes back, and Joel's keys
//! in two of them were the morning's slowest, p99 554 and 1,733 ms.
//!
//! So every [`EVERY`] passes, a holder found anywhere under jobs goes,
//! with each process it started that `--keep` names, into a scope of its
//! own in `--sessions`' slice, named as mycelium names one: the first
//! `--weigh` prefix and the holder's id. What those processes started
//! stays in jobs, since it's the session's work. Nothing ends: systemd
//! moves the processes into the scope it makes, and the sorter weighs and
//! holds it from then on as any session's.

use crate::Sample;

/// How many of the sorter's passes go by between looks for sessions in
/// jobs: a look reads every process there.
pub const EVERY: u32 = 5;

/// A session holder's id, from its command line: what follows `--id`
/// after the word `holder`, before any `--` that starts what it runs. An
/// id that couldn't name a unit is none.
pub fn holder_id(cmdline: &[u8]) -> Option<String> {
    let words: Vec<&[u8]> = cmdline.split(|byte| *byte == 0).collect();
    let at = words.iter().position(|word| *word == b"holder")?;
    let own: Vec<&[u8]> = words[at..]
        .iter()
        .take_while(|word| **word != b"--")
        .copied()
        .collect();
    let id = own.windows(2).find(|pair| pair[0] == b"--id")?[1];
    let named = |byte: &u8| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_';
    (!id.is_empty() && id.iter().all(named)).then(|| String::from_utf8_lossy(id).into_owned())
}

/// The sessions among `procs`, the processes under jobs: each holder's id,
/// with its own pid first and then those of the processes it started whose
/// names `keep` holds. A holder is a process `keep` names whose command
/// line, as `cmdline` reads it, says it holds a session.
pub fn found(
    procs: &[Sample],
    keep: &[String],
    cmdline: impl Fn(u32) -> Option<Vec<u8>>,
) -> Vec<(String, Vec<u32>)> {
    procs
        .iter()
        .filter(|proc| keep.contains(&proc.comm))
        .filter_map(|holder| {
            let id = holder_id(&cmdline(holder.pid)?)?;
            let mut pids = vec![holder.pid];
            pids.extend(
                procs
                    .iter()
                    .filter(|proc| proc.ppid == holder.pid && keep.contains(&proc.comm))
                    .map(|proc| proc.pid),
            );
            Some((id, pids))
        })
        .collect()
}

/// Whether a session's scope already holds its holder `id`, from the
/// command lines of the processes in it: then one in jobs saying it holds
/// `id` is another, and moving it in would have it held as Joel's pi is.
pub fn held_already(cmdlines: impl IntoIterator<Item = Vec<u8>>, id: &str) -> bool {
    cmdlines
        .into_iter()
        .any(|cmdline| holder_id(&cmdline).as_deref() == Some(id))
}

/// What busctl is given to start the scope `name` in `slice` around
/// `pids`, as mycelium starts a session's: collected once it ends or
/// fails, and stopped as any session's is.
pub fn scope_call(name: &str, slice: &str, pids: &[u32]) -> Vec<String> {
    let mut args: Vec<String> = [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "StartTransientUnit",
        "ssa(sv)a(sa(sv))",
        name,
        "fail",
        "4",
        "Slice",
        "s",
        slice,
        "Description",
        "s",
        "A session, given its scope back from jobs",
        "CollectMode",
        "s",
        "inactive-or-failed",
        "PIDs",
        "au",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect();
    args.push(pids.len().to_string());
    args.extend(pids.iter().map(u32::to_string));
    args.push("0".into());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[&str]) -> Vec<u8> {
        words.join("\0").into_bytes()
    }

    fn proc(own: u32, parent: u32, comm: &str) -> Sample {
        Sample {
            pid: own,
            ppid: parent,
            pgrp: own,
            comm: comm.into(),
            cpu: 0,
            start: 0,
        }
    }

    #[test]
    fn a_holders_id_is_read_from_its_command_line() {
        let holder = line(&[
            "/nix/store/x-mycelium/bin/mycelium",
            "--realm",
            "work",
            "holder",
            "--id",
            "09b57f72",
            "--folder",
            "/home/j/world",
            "--",
            "pi",
            "--session-id",
            "01a1",
        ]);
        assert_eq!(holder_id(&holder).as_deref(), Some("09b57f72"));
    }

    #[test]
    fn only_a_holder_has_an_id() {
        let attach = line(&["mycelium", "attach", "--id", "09b57f72"]);
        let after = line(&["mycelium", "holder", "--", "pi", "--id", "x"]);
        let unsafe_id = line(&["mycelium", "holder", "--id", "../x"]);
        let bare = line(&["mycelium", "holder", "--id"]);
        for (what, cmdline) in [
            ("an attach", attach),
            ("an --id after --", after),
            ("an id with a slash", unsafe_id),
            ("a missing id", bare),
        ] {
            assert_eq!(
                holder_id(&cmdline),
                None,
                "{what} was read as a holder's id"
            );
        }
    }

    #[test]
    fn a_session_is_its_holder_and_the_kept_processes_it_started() {
        let keep = ["pi".to_owned(), "mycelium".to_owned()];
        let procs = [
            proc(10, 1, "mycelium"),
            proc(11, 10, "pi"),
            proc(12, 11, "bash"),
            proc(13, 11, "pi"),
            proc(20, 1, "mycelium"),
            proc(30, 1, "cargo"),
        ];
        let cmdline = |pid| match pid {
            10 => Some(line(&["mycelium", "holder", "--id", "ab12", "--", "pi"])),
            20 => Some(line(&["mycelium", "attach", "--id", "ab12"])),
            30 => Some(line(&["cargo", "holder", "--id", "cd34"])),
            _ => Some(Vec::new()),
        };
        assert_eq!(
            found(&procs, &keep, cmdline),
            [("ab12".to_owned(), vec![10, 11])],
            "a session took in its work, a subagent's pi, an attach or a process nothing keeps"
        );
    }

    #[test]
    fn a_session_whose_holder_ended_is_none() {
        let keep = ["mycelium".to_owned()];
        let procs = [proc(10, 1, "mycelium")];
        assert_eq!(found(&procs, &keep, |_| None), []);
    }

    #[test]
    fn a_scope_holding_its_holder_already_takes_no_other() {
        let holder = line(&["mycelium", "holder", "--id", "ab12", "--", "pi"]);
        let pi = line(&["pi", "--session", "s"]);
        assert!(
            held_already([pi.clone(), holder.clone()], "ab12"),
            "its holder is there"
        );
        assert!(!held_already([pi], "ab12"), "its holder isn't there");
        assert!(!held_already([holder], "cd34"), "another session's holder");
        assert!(!held_already(Vec::new(), "ab12"), "no scope");
    }

    #[test]
    fn a_sessions_scope_is_asked_for_in_its_slice_around_its_processes() {
        let call = scope_call("mycelium-pi-ab12.scope", "agents.slice", &[10, 11]);
        let at = |word: &str| call.iter().position(|arg| arg == word).unwrap();
        assert_eq!(call[at("StartTransientUnit") + 2], "mycelium-pi-ab12.scope");
        assert_eq!(call[at("Slice") + 2], "agents.slice");
        assert_eq!(call[at("PIDs") + 2..], ["2", "10", "11", "0"]);
    }
}
