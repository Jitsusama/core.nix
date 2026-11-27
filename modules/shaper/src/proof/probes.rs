//! Joel's probes in each block, run from app.slice as his own programs
//! are: an upload and a download, each repeated, a fresh connection each
//! second, and the call's packets; and how each ends: on its own, at a
//! stop, or once it has run well past its block.

use super::load::{end_group, guarded};
use std::io::Read;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

/// How long past its block a probe may run before it's ended. Each asks
/// its last request with no more than the block's time left, or ten
/// seconds for a fresh connection, so one still going this late is held
/// by something and would only hold the proof.
pub const GRACE: Duration = Duration::from_secs(10);

/// A shell loop asking `request` again until `seconds` have passed, and
/// waiting `every` after each. A server that refused one, its status 400
/// or more, is given a second before the next: asked again at once, it
/// went on refusing for most of an hour. The time is read with `date`,
/// not bash's `$SECONDS`, since a plainer `sh` such as dash leaves that
/// empty and the loop asked nothing.
pub fn repeated(seconds: &str, request: &str, every: &str) -> String {
    format!(
        "end=$(($(date +%s) + {seconds})); while [ \"$(date +%s)\" -lt \"$end\" ]; do \
         a=$({request}); \
         echo \"$a\"; case $a in [45]*) sleep 1;; esac; {every} done"
    )
}

/// The shell of each probe of a block of `seconds`, in order: the
/// upload, the download and the fresh connections, each given the server
/// as its first argument, and the call, given this program's own path.
/// Neither is read as shell, and each ends with its proof.
pub fn scripts(seconds: u64) -> [String; 4] {
    let s = seconds.to_string();
    // Each says each answer's status beside what it measured, so a
    // server refusing Joel's requests is told from a shaper slowing them,
    // and the upload is repeated as the download is, since a server may
    // end one early. The download comes in pieces of 5 MB, since the
    // server refuses one of 10 MB or more once it has sent many.
    let up = repeated(
        &s,
        "head -c 4000000000 /dev/zero | curl -s -g -o /dev/null -X POST -T - \
         --max-time $((end - $(date +%s) + 1)) -w '%{http_code} %{size_upload}\\n' \
         \"$1/__up\"",
        "",
    );
    let down = repeated(
        &s,
        "curl -s -g -o /dev/null --max-time $((end - $(date +%s) + 1)) \
         -w '%{http_code} %{size_download}\\n' \"$1/__down?bytes=5000000\"",
        "",
    );
    let first = repeated(
        &s,
        "curl -s -g -o /dev/null --max-time 10 -w '%{http_code} %{time_starttransfer}\\n' \
         \"$1/__down?bytes=0\"",
        // A fresh connection a second, as a page's would be.
        "sleep 1;",
    );
    [guarded(&up), guarded(&down), guarded(&first), call(seconds)]
}

/// The call's shell for `seconds`, given this program's own path.
pub fn call(seconds: u64) -> String {
    guarded(&format!("\"$1\" proof-call {seconds}"))
}

/// Waits for `probe` to end and gives what it said, ending the whole of
/// its group once `halted` says to stop, or once `by` passes. What it
/// says is read as it comes, so a probe saying much can't fill the pipe
/// and stall.
pub fn finish(mut probe: Child, by: Instant, halted: impl Fn() -> bool) -> String {
    let said = probe.stdout.take().map(|mut out| {
        thread::spawn(move || {
            let mut text = String::new();
            let _ = out.read_to_string(&mut text);
            text
        })
    });
    loop {
        match probe.try_wait() {
            Ok(None) if halted() || Instant::now() >= by => {
                end_group(&probe);
                let _ = probe.wait();
                break;
            }
            // A stop is heard within a tenth of a second.
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Ok(Some(_)) | Err(_) => break,
        }
    }
    said.and_then(|s| s.join().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    #[test]
    fn a_refused_request_waits_before_it_is_asked_again() {
        let refused = repeated("2", "echo '429 0'", "");
        let out = Command::new("sh")
            .args(["-c", &refused])
            .output()
            .expect("a shell");
        let asked = String::from_utf8_lossy(&out.stdout).lines().count();
        assert!(
            (1..=3).contains(&asked),
            "asked {asked} times in two seconds a server refused"
        );
        // Answered, a probe waits only what it asks to between requests.
        let answered = repeated("2", "echo '200 5'", "sleep 0.4;");
        let out = Command::new("sh")
            .args(["-c", &answered])
            .output()
            .expect("a shell");
        let asked = String::from_utf8_lossy(&out.stdout).lines().count();
        assert!(
            (2..=6).contains(&asked),
            "asked {asked} times, not every 0.4 s"
        );
    }

    /// A server answering every request at once with nothing, and where.
    fn answering() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let at = format!("http://{}", listener.local_addr().expect("its address"));
        thread::spawn(move || {
            for mut asked in listener.incoming().flatten() {
                let mut heard = [0; 4096];
                let _ = asked.read(&mut heard);
                let _ = asked.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        at
    }

    /// What the shell `script` says, given `first` as its first argument,
    /// in a group of its own, so a watcher's `kill 0` can't reach cargo.
    fn says(script: &str, first: &str) -> String {
        let out = Command::new("sh")
            .args(["-c", script, "sh", first])
            .process_group(0)
            .output()
            .expect("a shell");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn each_probe_asks_what_it_is_given() {
        let server = answering();
        let [_, down, first, call] = scripts(1);
        for (name, script) in [("download", &down), ("fresh connection", &first)] {
            let said = says(script, &server);
            assert!(
                !said.is_empty() && said.lines().all(|l| l.starts_with("200 ")),
                "the {name} didn't ask the server it was given: {said:?}"
            );
        }
        // The call is this program's own, given as its first argument.
        assert_eq!(says(&call, "echo"), "proof-call 1\n");
        // A server given as shell stays a server.
        let said = says(&first, "http://127.0.0.1:1/'; echo broke; '");
        assert!(!said.contains("broke"), "ran the server as shell");
    }

    /// A probe of `script` in a group of its own, as a scope's is.
    fn probe(script: &str) -> Child {
        Command::new("sh")
            .args(["-c", script])
            .stdout(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("a shell")
    }

    /// Whether nothing of the group `group` runs within a second: a
    /// member just killed may still be dying on a busy machine.
    fn gone(group: u32) -> bool {
        let asked = Instant::now();
        while !super::super::load::group_gone(group) {
            if asked.elapsed() > Duration::from_secs(1) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
    }

    #[test]
    fn a_probe_ends_at_a_stop_or_well_past_its_block() {
        let far = Instant::now() + Duration::from_secs(30);
        let ended = probe("echo said; sleep 30 & sleep 30");
        let group = ended.id();
        let began = Instant::now();
        // Stopped once it has had time to say its word.
        let stop = Duration::from_millis(300);
        let said = finish(ended, far, || began.elapsed() > stop);
        let took = began.elapsed();
        assert!(
            took < stop + Duration::from_secs(1) && gone(group),
            "a stop left a probe running {:?}",
            began.elapsed()
        );
        assert_eq!(said, "said\n", "a stopped probe's word lost");
        let late = probe("sleep 30 & sleep 30");
        let group = late.id();
        let began = Instant::now();
        finish(late, began + Duration::from_millis(300), || false);
        let took = began.elapsed();
        assert!(
            took < Duration::from_secs(2) && gone(group),
            "a probe held past its block ran on"
        );
        // One that ends by itself is waited for, and its word kept.
        let done = finish(probe("sleep 0.3; echo done"), far, || false);
        assert_eq!(done, "done\n");
    }
}
