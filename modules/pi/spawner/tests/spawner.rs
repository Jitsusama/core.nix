//! The spawner as pi drives it: requests in on stdin, frames out on
//! stdout, with the real program started for each test.

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// A spawner started for one test, with its two ends.
struct Spawner {
    child: Child,
    input: Option<ChildStdin>,
    output: ChildStdout,
}

/// One frame as the spawner sent it.
#[derive(Debug)]
struct Frame {
    kind: u8,
    id: u32,
    payload: Vec<u8>,
}

impl Spawner {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pi-spawner"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the spawner starts");
        let input = child.stdin.take();
        let output = child.stdout.take().expect("its stdout");
        Spawner {
            child,
            input,
            output,
        }
    }

    fn ask(&mut self, fields: &[&[u8]]) {
        let mut body = Vec::new();
        for field in fields {
            body.extend_from_slice(field);
            body.push(0);
        }
        let length = u32::try_from(body.len()).expect("a short request");
        let input = self.input.as_mut().expect("stdin still open");
        input.write_all(&length.to_le_bytes()).expect("writes");
        input.write_all(&body).expect("writes");
        input.flush().expect("flushes");
    }

    /// Run a shell command, with the test's own `PATH` beside `env` so it
    /// finds `sleep` wherever the test runs.
    fn run(&mut self, id: &str, cwd: &str, args: &[&str], env: &[&str]) {
        let path = format!("PATH={}", std::env::var("PATH").unwrap_or_default());
        let env: Vec<&str> = env.iter().copied().chain([path.as_str()]).collect();
        let arguments = args.len().to_string();
        let variables = env.len().to_string();
        let mut fields: Vec<&[u8]> = vec![b"run", id.as_bytes(), b"/bin/sh", cwd.as_bytes()];
        fields.push(arguments.as_bytes());
        fields.extend(args.iter().map(|arg| arg.as_bytes()));
        fields.push(variables.as_bytes());
        fields.extend(env.iter().map(|pair| pair.as_bytes()));
        self.ask(&fields);
    }

    fn next(&mut self) -> Frame {
        let mut head = [0u8; 9];
        self.output.read_exact(&mut head).expect("a frame");
        let id = u32::from_le_bytes(head[1..5].try_into().expect("four bytes"));
        let length = u32::from_le_bytes(head[5..9].try_into().expect("four bytes"));
        let mut payload = vec![0u8; usize::try_from(length).expect("fits")];
        self.output.read_exact(&mut payload).expect("its payload");
        Frame {
            kind: head[0],
            id,
            payload,
        }
    }

    /// Every frame up to the command's exit and both its streams' ends.
    fn until_done(&mut self) -> Vec<Frame> {
        let mut frames = Vec::new();
        let (mut exited, mut closed) = (false, 0);
        while !(exited && closed == 2) {
            let frame = self.next();
            exited |= frame.kind == b'x';
            closed += usize::from(frame.kind == b'c');
            frames.push(frame);
        }
        frames
    }
}

impl Drop for Spawner {
    fn drop(&mut self) {
        // Killed, it leaves no command of a test running past the test.
        if self.child.kill().is_ok() {
            let _ = self.child.wait();
        }
    }
}

fn output(frames: &[Frame], stream: u8) -> String {
    let bytes: Vec<u8> = frames
        .iter()
        .filter(|frame| frame.kind == b'o' && frame.payload[0] == stream)
        .flat_map(|frame| frame.payload[1..].to_vec())
        .collect();
    String::from_utf8(bytes).expect("text")
}

fn exit_of(frames: &[Frame]) -> (i32, i32) {
    let frame = frames
        .iter()
        .find(|frame| frame.kind == b'x')
        .expect("an exit");
    (
        i32::from_le_bytes(frame.payload[0..4].try_into().expect("four bytes")),
        i32::from_le_bytes(frame.payload[4..8].try_into().expect("four bytes")),
    )
}

fn pid_of(frame: &Frame) -> i32 {
    assert_eq!(frame.kind, b'p', "the first frame names the process");
    i32::from_le_bytes(frame.payload[..4].try_into().expect("four bytes"))
}

/// Whether a process still runs: there, and not a zombie left for a
/// parent that may never reap it, as in a build's sandbox.
fn alive(pid: i32) -> bool {
    // SAFETY: signal 0 only asks whether the target exists.
    let there = unsafe { libc::kill(pid, 0) == 0 };
    let zombie = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            let after = stat.rfind(')')?;
            stat[after + 1..]
                .split_whitespace()
                .next()
                .map(|state| state == "Z")
        })
        .unwrap_or(false);
    there && !zombie
}

#[test]
fn passes_on_both_streams_and_the_exit_code() {
    let mut spawner = Spawner::start();
    spawner.run("7", "/", &["-c", "echo out; echo err >&2; exit 3"], &[]);

    let frames = spawner.until_done();

    assert!(
        frames.iter().all(|frame| frame.id == 7),
        "every frame carries the request's id, 7"
    );
    assert_eq!(output(&frames, 1), "out\n");
    assert_eq!(output(&frames, 2), "err\n");
    assert_eq!(exit_of(&frames), (3, 0));
}

#[test]
fn runs_in_the_folder_with_only_the_variables_given() {
    let folder = std::env::temp_dir().canonicalize().expect("a folder");
    let folder = folder.to_str().expect("a plain path");
    let mut spawner = Spawner::start();
    spawner.run(
        "1",
        folder,
        &["-c", "pwd -P; echo \"$ONLY/${HOME:-none}/$SPACED\""],
        &["ONLY=given", "SPACED=a b=c"],
    );

    let frames = spawner.until_done();

    assert_eq!(output(&frames, 1), format!("{folder}\ngiven/none/a b=c\n"));
}

#[test]
fn starts_each_command_in_a_group_of_its_own() {
    let mut spawner = Spawner::start();
    spawner.run("2", "/", &["-c", "sleep 30 & wait"], &[]);
    let pid = pid_of(&spawner.next());

    // SAFETY: the group is this test's own command and what it started.
    let signalled = unsafe { libc::kill(-pid, libc::SIGKILL) };

    assert_eq!(signalled, 0, "the command leads a group of its own");
    let frames = spawner.until_done();
    assert_eq!(exit_of(&frames), (-1, libc::SIGKILL));
}

#[test]
fn says_why_a_command_could_not_start() {
    let mut spawner = Spawner::start();
    spawner.run("3", "/no/such/folder", &["-c", "true"], &[]);

    let frame = spawner.next();

    assert_eq!((frame.kind, frame.id), (b'e', 3));
    assert!(!frame.payload.is_empty(), "the failure says why");
}

#[test]
fn sends_nothing_more_of_a_dropped_command_but_its_exit() {
    let mut spawner = Spawner::start();
    spawner.run("4", "/", &["-c", "sleep 0.3; echo late"], &[]);
    pid_of(&spawner.next());

    spawner.ask(&[b"drop", b"4"]);

    let frame = spawner.next();
    assert_eq!(frame.kind, b'x', "only the exit follows a drop");
}

#[test]
fn kills_every_group_still_running_when_pi_goes() {
    let mut spawner = Spawner::start();
    spawner.run("5", "/", &["-c", "sleep 30 & wait"], &[]);
    let pid = pid_of(&spawner.next());
    assert!(alive(pid), "the command runs while pi is there");

    drop(spawner.input.take());

    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!alive(pid), "the command is gone once stdin ends");
}

#[test]
#[cfg(target_os = "linux")]
fn keeps_its_requests_from_other_processes_of_the_same_user() {
    let mut spawner = Spawner::start();
    let pid = spawner.child.id();
    // Answering a request, it has passed its start.
    spawner.run("6", "/", &["-c", "true"], &[]);
    spawner.until_done();

    let opened = std::fs::read_link(format!("/proc/{pid}/fd/0"));

    assert_eq!(
        opened.map_err(|error| error.kind()),
        Err(std::io::ErrorKind::PermissionDenied),
        "another process of the same user can't reach the spawner's stdin through /proc"
    );
}
