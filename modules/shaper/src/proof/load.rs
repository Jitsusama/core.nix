//! The agents' load a proof puts on the link, and how it ends: at the
//! proof's word, or on its own once the proof is gone, so a proof stopped
//! any way leaves nothing uploading.

use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// One upload's loop, posting 400 MB to the server, its first argument,
/// again as each ends, while its proof lives, as [`guarded_for`] asks.
/// One that fails waits a second, rather than spinning on a dead server.
fn script(pid: u32, began: &str) -> String {
    guarded_for(
        pid,
        began,
        "while alive; do head -c 400000000 /dev/zero \
         | curl -s -o /dev/null -X POST -T - --max-time 120 \"$1/__up\" || sleep 1; done",
    )
}

/// Runs the shell `body` while the process `pid` that began at `began`
/// lives. A proof killed outright can't stop its scopes, which outlive it,
/// so a watcher beside `body` asks after it every second and kills the
/// whole of the scope's process group once it's gone, so what it ran
/// leaves the link within about two seconds of its proof. It asks by the
/// start in `/proc/<pid>/stat` too, since the pid may be given to another
/// process, and a zombie, killed but not yet reaped, counts as gone; a
/// start that couldn't be read matches nothing. `body` may ask `alive`
/// itself. The scope's shell must lead its own group, as [`scoped`]
/// makes it.
///
/// [`scoped`]: super::scoped
fn guarded_for(pid: u32, began: &str, body: &str) -> String {
    format!(
        "alive() {{ case \"$(sed 's/.*) //' /proc/{pid}/stat 2>/dev/null | cut -d' ' -f1,20)\" \
         in [!ZX]' {began}') ;; *) return 1;; esac; }}; \
         (while alive; do sleep 1; done; kill 0) >/dev/null 2>&1 & watch=$!; \
         {body}; kill $watch 2>/dev/null"
    )
}

/// Runs the shell `body` while this proof lives, as [`guarded_for`] says.
pub fn guarded(body: &str) -> String {
    guarded_for(std::process::id(), &began(), body)
}

/// When this process began, as `/proc/self/stat` says, in clock ticks
/// since boot: the field after the command's name and eighteen more.
fn began() -> String {
    began_of("self")
}

/// When the process `pid` began, as [`began`] reads it.
fn began_of(pid: &str) -> String {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| Some(s.rsplit_once(") ")?.1.split(' ').nth(19)?.to_string()))
        .unwrap_or_default()
}

/// Starts the agents' uploads, from jobs.slice, and waits until systemd
/// has each one's scope or five seconds pass, since a scope stopped
/// before systemd has it would upload on.
pub fn start(uploads: u32, server: &str) {
    let script = script(std::process::id(), &began());
    for n in 0..uploads {
        let command = ["sh", "-c", &script, "sh", server];
        let _ = super::scoped("jobs.slice", &format!("load{n}"), &command)
            .stdout(Stdio::null())
            .spawn();
    }
    let asked = Instant::now();
    while scopes("load*") < uploads as usize && asked.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(100));
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

/// Stops every scope this proof started.
pub fn stop_scopes() {
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
        // SAFETY: kill(2) with signal 0 only asks whether the group is.
        let gone = || unsafe { libc::kill(-group, 0) } != 0;
        let asked = Instant::now();
        while asked.elapsed() < within {
            let _ = sh.try_wait();
            if gone() {
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
