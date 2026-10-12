use disk_guard::activity::Process;
use disk_guard::socket::{Request, Server, ask};
use disk_guard::vouch::Answer;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const SECOND: Duration = Duration::from_secs(1);

fn told() -> Answer {
    Answer {
        vouched: vec![(300, 77)],
        touching: vec![Process {
            pid: 300,
            start_ticks: 77,
            cwd: Some(PathBuf::from("/home/u/src/app")),
            readable: true,
            ..Default::default()
        }],
    }
}

/// A server answering `connections` connections in a thread, reporting
/// who asked and for what.
fn serving(
    every: Duration,
    connections: usize,
) -> (tempfile::TempDir, PathBuf, mpsc::Receiver<(u32, Request)>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let mut server = Server::bind(&path, every).unwrap();
    let (sender, asked) = mpsc::channel();
    std::thread::spawn(move || {
        for _ in 0..connections {
            server
                .serve(&mut |uid, request| {
                    sender.send((uid, request)).unwrap();
                    Ok(told())
                })
                .unwrap();
        }
    });
    (dir, path, asked)
}

fn roots() -> Request {
    Request::Processes {
        roots: vec![PathBuf::from("/home/u/src")],
    }
}

#[test]
fn a_request_is_answered_and_its_asker_known() {
    let (_dir, path, asked) = serving(Duration::ZERO, 1);
    assert_eq!(ask(&path, &roots(), SECOND).unwrap(), told());
    let (uid, request) = asked.recv().unwrap();
    assert_eq!(uid, unsafe { libc::geteuid() });
    assert_eq!(request, roots());
}

#[test]
fn an_asker_may_ask_once_a_second() {
    let (_dir, path, _asked) = serving(Duration::from_secs(60), 2);
    assert!(ask(&path, &roots(), SECOND).is_ok());
    let refused = ask(&path, &roots(), SECOND).unwrap_err();
    assert!(refused.to_string().contains("too soon"), "{refused}");
}

#[test]
fn a_request_that_wont_read_is_refused_and_the_server_goes_on() {
    let (_dir, path, _asked) = serving(Duration::ZERO, 3);
    let mut stream = UnixStream::connect(&path).unwrap();
    stream.write_all(b"{ not json }\n").unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    assert!(reply.contains("error"), "{reply}");
    let mut endless = UnixStream::connect(&path).unwrap();
    endless.write_all(&vec![b'x'; 200_000]).unwrap_or(());
    drop(endless);
    assert_eq!(ask(&path, &roots(), SECOND).unwrap(), told());
}

#[test]
fn an_asker_that_says_nothing_cant_hold_the_server() {
    let (_dir, path, _asked) = serving(Duration::ZERO, 2);
    let _silent = UnixStream::connect(&path).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        ask(&path, &roots(), Duration::from_secs(5)).unwrap(),
        told()
    );
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn no_server_is_an_error_at_once() {
    let dir = tempfile::tempdir().unwrap();
    assert!(ask(&dir.path().join("socket"), &roots(), SECOND).is_err());
}

#[test]
fn a_stale_socket_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    drop(Server::bind(&path, Duration::ZERO).unwrap());
    assert!(Server::bind(&path, Duration::ZERO).is_ok());
}
