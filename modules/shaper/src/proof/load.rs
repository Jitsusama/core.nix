//! The agents' load a proof puts on the link, and how it ends: at the
//! proof's word, or on its own once the proof is gone, so a proof stopped
//! any way leaves nothing uploading.

use super::stop::halted;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// The uploads of the window under way, held unreaped until they're
/// ended, so each one's pid, and so its group, is still its own.
static LOADS: Mutex<Vec<Child>> = Mutex::new(Vec::new());

/// One upload's loop, posting 400 MB to the server, its first argument,
/// again as each ends, while its proof lives, as [`guarded_for`] asks.
/// One that fails waits a second, rather than spinning on a dead server.
fn script(pid: u32, began: &str) -> String {
    guarded_for(
        pid,
        began,
        "while alive; do head -c 400000000 /dev/zero \
         | curl -s -g -o /dev/null -X POST -T - --max-time 120 \"$1/__up\" || sleep 1; done",
    )
}

/// Runs the shell `body` while the process `pid` that began at `began`
/// lives. A proof killed outright can't stop its scopes, which outlive it,
/// so a watcher beside `body` asks after it every second and kills the
/// whole of the scope's process group once it's gone, so what it ran
/// leaves the link within about two seconds of its proof. `body` may ask
/// `alive` itself, as [`alive_for`] defines it. The scope's shell must
/// lead its own group, as [`scoped`] makes it.
///
/// [`scoped`]: super::scoped
fn guarded_for(pid: u32, began: &str, body: &str) -> String {
    format!(
        "{}; (while alive; do sleep 1; done; kill 0) >/dev/null 2>&1 & watch=$!; \
         {body}; kill $watch 2>/dev/null",
        alive_for(pid, began)
    )
}

/// A shell function `alive`, saying whether the process `pid` that began
/// at `began` still runs. It asks by the start in `/proc/<pid>/stat` too,
/// since the pid may be given to another process, and a zombie, killed
/// but not yet reaped, counts as gone, as does one stopped, since it
/// can't stop what it started; a start that couldn't be read, or isn't a
/// number, matches nothing. It reads with the shell's own builtins, since
/// every scope asks every second while the proof measures.
fn alive_for(pid: u32, began: &str) -> String {
    let began = if began.bytes().all(|b| b.is_ascii_digit()) {
        began
    } else {
        ""
    };
    format!(
        "alive() {{ read -r s 2>/dev/null </proc/{pid}/stat || return 1; set -- ${{s##*) }}; \
         case \"$1 ${{20}}\" in [!ZXT]' {began}') ;; *) return 1;; esac; }}"
    )
}

/// Runs the shell `body` while this proof lives, as [`guarded_for`] says.
pub fn guarded(body: &str) -> String {
    guarded_for(std::process::id(), &began(), body)
}

/// Runs the shell `body` once the process `pid` that began at `began` is
/// gone, as [`alive_for`] tells it, asking every second.
pub fn once_gone_for(pid: u32, began: &str, body: &str) -> String {
    format!(
        "{}; while alive; do sleep 1; done; {body}",
        alive_for(pid, began)
    )
}

/// Whether nothing of the group `group` runs, for the tests here and
/// the probes': a member killed may stay a zombie a while, until
/// whoever inherits it reaps it.
#[cfg(test)]
pub fn group_gone(group: u32) -> bool {
    let group = group.to_string();
    std::fs::read_dir("/proc")
        .expect("/proc")
        .flatten()
        .all(|entry| {
            let stat = std::fs::read_to_string(entry.path().join("stat")).unwrap_or_default();
            let fields: Vec<&str> = stat
                .rsplit_once(") ")
                .map_or(Vec::new(), |(_, rest)| rest.split(' ').collect());
            !matches!(fields.as_slice(), [state, _, pgrp, ..] if *pgrp == group && *state != "Z")
        })
}

/// Ends the whole of `child`'s process group, which it leads, as
/// [`scoped`] makes it. An unreaped child's pid can't be given to
/// another process, so the group is only ever its own.
///
/// [`scoped`]: super::scoped
pub fn end_group(child: &Child) {
    if let Ok(group) = libc::pid_t::try_from(child.id()) {
        // SAFETY: kill(2) on the group of a child still unreaped.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
}

/// When this process began, as `/proc/self/stat` says, in clock ticks
/// since boot: the field after the command's name and eighteen more.
pub fn began() -> String {
    began_of("self")
}

/// When the process `pid` began, as [`began`] reads it.
pub fn began_of(pid: &str) -> String {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| Some(s.rsplit_once(") ")?.1.split(' ').nth(19)?.to_string()))
        .unwrap_or_default()
}

/// Starts the agents' uploads, from jobs.slice, and waits until systemd
/// has each one's scope, five seconds pass or the window's `stop`, so the
/// load is on before the first block counts.
pub fn start(uploads: u32, server: &str, stop: &AtomicBool) {
    let script = script(std::process::id(), &began());
    let mut loads = LOADS.lock().unwrap_or_else(PoisonError::into_inner);
    for n in 0..uploads {
        let command = ["sh", "-c", &script, "sh", server];
        if let Ok(load) = super::scoped("jobs.slice", &format!("load{n}"), &command)
            .stdout(Stdio::null())
            .spawn()
        {
            loads.push(load);
        }
    }
    drop(loads);
    let asked = Instant::now();
    let mut running = scopes("load*");
    while running < uploads as usize && asked.elapsed() < Duration::from_secs(5) && !halted(stop) {
        thread::sleep(Duration::from_millis(100));
        running = scopes("load*");
    }
    // Said, since a block under a lighter load than asked compares the
    // shaper with the router under less than the agents' traffic.
    if running < uploads as usize && !halted(stop) {
        super::stop::tell(&format!(
            "only {running} of {uploads} uploads started within 5 s; measuring on"
        ));
    }
}

/// How many of this proof's scopes named `name` systemd has.
fn scopes(name: &str) -> usize {
    Command::new("systemctl")
        .args(["--user", "list-units", "--no-legend", "--plain"])
        .arg(format!("shaper-proof-{name}-{}.scope", std::process::id()))
        .stderr(Stdio::null())
        .output()
        .map_or(0, |o| String::from_utf8_lossy(&o.stdout).lines().count())
}

/// Ends every upload this proof started, the whole of each one's group,
/// whether or not systemd has its scope yet, and reaps it; then stops
/// every scope of the proof's by name, for anything else left.
pub fn stop_scopes() {
    let loads = std::mem::take(&mut *LOADS.lock().unwrap_or_else(PoisonError::into_inner));
    for mut load in loads {
        end_group(&load);
        let _ = load.wait();
    }
    let _ = Command::new("systemctl")
        .args([
            "--user",
            "stop",
            &format!("shaper-proof-*-{}.scope", std::process::id()),
        ])
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `script`, given a server nothing answers on, ends by itself
    /// within `within`; it's killed if not.
    fn ends_within(script: &str, within: Duration) -> bool {
        ends_within_on(script, "http://127.0.0.1:1", within)
    }

    /// Whether `script`, uploading to `server`, ends by itself within
    /// `within`, the whole of its process group with it, as a scope's
    /// would; the group is killed if not, leaving nothing behind.
    fn ends_within_on(script: &str, server: &str, within: Duration) -> bool {
        use std::os::unix::process::CommandExt;
        let mut sh = Command::new("sh")
            .args(["-c", script, "sh", server])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("a shell");
        let group = libc::pid_t::try_from(sh.id()).expect("a pid");
        let asked = Instant::now();
        while asked.elapsed() < within {
            let _ = sh.try_wait();
            if group_gone(sh.id()) {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        // SAFETY: the group is this test's own shell's.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
        let _ = sh.wait();
        false
    }

    /// A proof that ends at once and is reaped: its pid and start.
    fn a_proof_gone() -> (u32, String) {
        let mut gone = Command::new("sleep").arg("0.2").spawn().expect("sleep");
        let (pid, began) = (gone.id(), began_of(&gone.id().to_string()));
        gone.wait().expect("sleep to end");
        (pid, began)
    }

    /// A proof that lives for `seconds`, reaped once it ends, since a
    /// zombie keeps its stat: its pid and start.
    fn a_proof_for(seconds: &str) -> (u32, String) {
        let mut proof = Command::new("sleep").arg(seconds).spawn().expect("sleep");
        let (pid, began) = (proof.id(), began_of(&proof.id().to_string()));
        thread::spawn(move || proof.wait());
        (pid, began)
    }

    #[test]
    fn a_load_ends_once_its_proof_is_gone() {
        let (pid, began_then) = a_proof_gone();
        let (me, began) = (std::process::id(), began());
        assert!(!began.is_empty(), "no start read from /proc/self/stat");
        assert!(
            ends_within(&script(pid, &began_then), Duration::from_secs(3)),
            "uploading on after its proof ended"
        );
        // The first process's start, read as this one's is: a pid given
        // to another process is told apart by it.
        let other = began_of("1");
        assert!(
            !other.is_empty() && other != began,
            "no other start to tell apart"
        );
        assert!(
            ends_within(&script(me, &other), Duration::from_secs(3)),
            "uploading on for another process given its proof's pid"
        );
        assert!(
            ends_within(&script(me, ""), Duration::from_secs(3)),
            "uploading on for a proof whose start couldn't be read"
        );
        assert!(
            !ends_within(&script(me, &began), Duration::from_secs(1)),
            "ended while its proof runs"
        );
    }

    #[test]
    fn an_upload_under_way_ends_soon_after_its_proof() {
        // Without curl the loop would end with its proof between
        // uploads, and prove nothing of one under way.
        assert!(
            Command::new("curl")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success()),
            "no curl to upload with"
        );
        // A server that takes the connection and never answers holds
        // curl until its own limit, as a slow link would.
        let held = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let server = format!("http://{}", held.local_addr().expect("its address"));
        let (pid, began) = a_proof_for("1");
        assert!(
            ends_within_on(&script(pid, &began), &server, Duration::from_secs(4)),
            "its upload ran on after its proof ended"
        );
    }

    #[test]
    fn a_killed_proof_not_yet_reaped_counts_as_gone() {
        let mut zombie = Command::new("true").spawn().expect("true");
        let (pid, began) = (zombie.id(), began_of(&zombie.id().to_string()));
        // Long enough for it to end, and stay a zombie, unreaped.
        thread::sleep(Duration::from_millis(300));
        let ended = ends_within(&script(pid, &began), Duration::from_secs(3));
        let _ = zombie.wait();
        assert!(ended, "uploading on for a proof killed but not yet reaped");
    }

    #[test]
    fn a_stopped_proof_counts_as_gone() {
        use std::os::unix::process::CommandExt;
        // In a group of its own, where the Ctrl-Z test's waking can't
        // reach it.
        let mut proof = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .expect("sleep");
        let (pid, began) = (proof.id(), began_of(&proof.id().to_string()));
        let group = libc::pid_t::try_from(pid).expect("a pid");
        // SAFETY: stopping, then ending, this test's own sleep.
        unsafe {
            libc::kill(group, libc::SIGSTOP);
        }
        thread::sleep(Duration::from_millis(200));
        let ended = ends_within(&script(pid, &began), Duration::from_secs(3));
        let _ = proof.kill();
        let _ = proof.wait();
        assert!(
            ended,
            "uploading on for a proof stopped, as Ctrl-Z or SIGSTOP leaves it"
        );
        assert!(
            ends_within(&script(std::process::id(), "1 2"), Duration::from_secs(3)),
            "took a start that isn't a number"
        );
    }

    #[test]
    fn a_live_proofs_probe_runs_to_its_end() {
        use std::os::unix::process::CommandExt;
        let probe = guarded_for(std::process::id(), &began(), "sleep 1.5; echo done");
        // In a group of its own, so a guard that took this proof for gone
        // fails the test with its `kill 0`, rather than ending cargo.
        let out = Command::new("sh")
            .args(["-c", &probe])
            .process_group(0)
            .output()
            .expect("a shell");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "done\n",
            "a probe cut short while its proof lives"
        );
    }

    #[test]
    fn what_waits_for_a_proof_runs_once_it_is_gone() {
        let (pid, began) = a_proof_for("1");
        let asked = Instant::now();
        let out = Command::new("sh")
            .args(["-c", &once_gone_for(pid, &began, "echo after")])
            .output()
            .expect("a shell");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "after\n");
        assert!(
            asked.elapsed() >= Duration::from_millis(800),
            "ran while its proof lived, after {:?}",
            asked.elapsed()
        );
    }

    #[test]
    fn whatever_a_scope_runs_ends_soon_after_its_proof() {
        let (pid, then) = a_proof_for("1");
        assert!(
            ends_within(&guarded_for(pid, &then, "sleep 30"), Duration::from_secs(4)),
            "a probe ran on after its proof ended"
        );
        let (me, began) = (std::process::id(), began());
        assert!(
            ends_within(
                &guarded_for(me, &began, "sleep 0.2"),
                Duration::from_secs(3)
            ),
            "a probe's scope outlived what it ran"
        );
    }
}
