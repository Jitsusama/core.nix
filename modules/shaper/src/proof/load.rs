//! The agents' load a proof puts on the link, and how it ends: at the
//! proof's word, or on its own once the proof is gone, so a proof stopped
//! any way leaves nothing uploading.

use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// One upload's loop, posting 400 MB to the server, its first argument,
/// again as each ends, while the process `pid` that began at `began` (its
/// start in `/proc/<pid>/stat`) lives. A proof killed outright can't stop
/// its scopes, which outlive it, so the loop asks after it by its start
/// too, since its pid may be given to another process. It asks before
/// each upload, and a watcher asks every second during one, ending it, so
/// the load leaves the link within about two seconds of its proof.
fn script(pid: u32, began: &str) -> String {
    format!(
        "alive() {{ [ \"$(sed 's/.*) //' /proc/{pid}/stat 2>/dev/null | cut -d' ' -f20)\" \
         = '{began}' ]; }}; \
         while alive; do head -c 400000000 /dev/zero \
         | curl -s -o /dev/null -X POST -T - --max-time 120 \"$1/__up\" & up=$!; \
         (while alive; do sleep 1; done; kill $up 2>/dev/null) & watch=$!; \
         wait $up; kill $watch 2>/dev/null; done"
    )
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
    /// `within`; it's killed if not.
    fn ends_within_on(script: &str, server: &str, within: Duration) -> bool {
        let mut sh = Command::new("sh")
            .args(["-c", script, "sh", server])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a shell");
        let asked = Instant::now();
        while asked.elapsed() < within {
            if sh.try_wait().ok().flatten().is_some() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = sh.kill();
        let _ = sh.wait();
        false
    }

    #[test]
    fn a_load_ends_once_its_proof_is_gone() {
        let mut gone = Command::new("true").spawn().expect("true");
        let pid = gone.id();
        gone.wait().expect("true to end");
        let (me, began) = (std::process::id(), began());
        assert!(!began.is_empty(), "no start read from /proc/self/stat");
        assert!(
            ends_within(&script(pid, &began), Duration::from_secs(3)),
            "uploading on after its proof ended"
        );
        assert!(
            ends_within(&script(me, "1"), Duration::from_secs(3)),
            "uploading on for another process given its proof's pid"
        );
        assert!(
            !ends_within(&script(me, &began), Duration::from_secs(1)),
            "ended while its proof runs"
        );
    }

    #[test]
    fn an_upload_under_way_ends_soon_after_its_proof() {
        // A server that takes the connection and never answers holds
        // curl until its own limit, as a slow link would.
        let held = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let server = format!("http://{}", held.local_addr().expect("its address"));
        let mut proof = Command::new("sleep").arg("1").spawn().expect("sleep");
        let (pid, began) = (proof.id(), began_of(&proof.id().to_string()));
        // Reaped once it ends, since a zombie keeps its stat.
        let reaper = thread::spawn(move || proof.wait());
        assert!(
            ends_within_on(&script(pid, &began), &server, Duration::from_secs(4)),
            "its upload ran on after its proof ended"
        );
        let _ = reaper.join();
    }
}
