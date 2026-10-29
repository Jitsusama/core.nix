use disk_guard::activity::Process;
use disk_guard::band::Band;
use disk_guard::socket::{Reply, Request, Responder, Server, ask};
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
/// who asked and for what. A collect is handed over unanswered, for the
/// test to answer when it likes; anything else is answered at once.
fn serving(
    every: Duration,
    connections: usize,
) -> (
    tempfile::TempDir,
    PathBuf,
    mpsc::Receiver<(u32, Request)>,
    mpsc::Receiver<Responder>,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let mut server = Server::bind(&path, every).unwrap();
    let (sender, asked) = mpsc::channel();
    let (handing, handed) = mpsc::channel();
    std::thread::spawn(move || {
        for _ in 0..connections {
            server
                .serve(&mut |uid, request, responder| {
                    sender.send((uid, request.clone())).unwrap();
                    match request {
                        Request::Processes { .. } => responder.send(&Reply::Answer(told())),
                        Request::Watching { paths } => responder.send(&Reply::Collects(paths)),
                        Request::Collect { .. } => handing.send(responder).unwrap(),
                        Request::Footprint { paths } => {
                            responder.send(&Reply::Footprints(vec![Some(4096); paths.len()]));
                        }
                    }
                })
                .unwrap();
        }
    });
    (dir, path, asked, handed)
}

fn roots() -> Request {
    Request::Processes {
        roots: vec![PathBuf::from("/home/u/src")],
    }
}

#[test]
fn a_request_is_answered_and_its_asker_known() {
    let (_dir, path, asked, _) = serving(Duration::ZERO, 1);
    assert_eq!(ask(&path, &roots(), SECOND).unwrap(), Reply::Answer(told()));
    let (uid, request) = asked.recv().unwrap();
    assert_eq!(uid, unsafe { libc::geteuid() });
    assert_eq!(request, roots());
}

#[test]
fn an_asker_may_ask_once_a_second() {
    let (_dir, path, _asked, _) = serving(Duration::from_secs(60), 2);
    assert!(ask(&path, &roots(), SECOND).is_ok());
    let refused = ask(&path, &roots(), SECOND).unwrap_err();
    assert!(refused.to_string().contains("too soon"), "{refused}");
}

#[test]
fn each_kind_of_request_waits_its_own_turn() {
    let (_dir, path, _asked, _) = serving(Duration::from_secs(60), 3);
    let watching = Request::Watching {
        paths: vec![PathBuf::from("/home")],
    };
    assert!(ask(&path, &roots(), SECOND).is_ok());
    assert_eq!(
        ask(&path, &watching, SECOND).unwrap(),
        Reply::Collects(vec![PathBuf::from("/home")]),
        "a heartbeat isn't held up by asking about processes"
    );
    assert!(
        ask(&path, &watching, SECOND).is_err(),
        "but has its own turn"
    );
}

#[test]
fn a_reply_can_come_later_while_others_are_answered() {
    let (_dir, path, _asked, handed) = serving(Duration::ZERO, 2);
    let collecting = path.clone();
    let collected = std::thread::spawn(move || {
        let collect = Request::Collect {
            path: PathBuf::from("/home"),
            band: Band::Urgent,
        };
        ask(&collecting, &collect, Duration::from_secs(10))
    });
    let waiting = handed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(ask(&path, &roots(), SECOND).unwrap(), Reply::Answer(told()));
    assert!(!collected.is_finished(), "still waiting for its reply");
    waiting.send(&Reply::Collected("reached".to_owned()));
    assert_eq!(
        collected.join().unwrap().unwrap(),
        Reply::Collected("reached".to_owned())
    );
}

#[test]
fn a_request_that_wont_read_is_refused_and_the_server_goes_on() {
    let (_dir, path, _asked, _) = serving(Duration::ZERO, 3);
    let mut stream = UnixStream::connect(&path).unwrap();
    stream.write_all(b"{ not json }\n").unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    assert!(reply.contains("error"), "{reply}");
    let mut endless = UnixStream::connect(&path).unwrap();
    endless.write_all(&vec![b'x'; 200_000]).unwrap_or(());
    drop(endless);
    assert_eq!(ask(&path, &roots(), SECOND).unwrap(), Reply::Answer(told()));
}

#[test]
fn an_asker_that_says_nothing_cant_hold_the_server() {
    let (_dir, path, _asked, _) = serving(Duration::ZERO, 2);
    let _silent = UnixStream::connect(&path).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        ask(&path, &roots(), Duration::from_secs(5)).unwrap(),
        Reply::Answer(told())
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
