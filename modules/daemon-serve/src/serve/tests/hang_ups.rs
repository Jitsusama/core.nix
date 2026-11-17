//! The cases of a connection that stops reading: hung up on, by a change,
//! an ask, a slow ask or a pace, once what it's owed can't be queued, its
//! writer ended and the hang-up said once, while the rest are still told.

use std::io::Read;
use std::sync::mpsc::Sender;

use super::hang_up::{UNSAID, process};

use super::*;

#[test]
fn a_connection_no_longer_reading_is_hung_up_on_and_the_rest_told_on() {
    let (shared, mut client, folder) = served("stuck", Box::new(|_| None));
    let said = hearing(&shared);
    let (telling, told) = channel();
    shared.on_watches(Box::new(move || {
        let _ = telling.send(());
    }));
    let watch = json!({"id": 1, "ask": "watch", "kind": "row", "place": "agents"});
    // Asks for a watch, then never reads a line.
    let mut stuck = UnixStream::connect(folder.0.join("daemon.sock")).expect("it connects");
    writeln!(stuck, "{watch}").expect("the watch is asked");
    assert!(
        in_time(&told).is_some(),
        "told as the stuck one's watch begins"
    );
    client.ask(&watch);
    assert!(in_time(&told).is_some(), "told as the reading one's begins");
    // Lines of some size, so the socket's buffer fills within a few
    // dozen and the queue behind it within QUEUED more. The ceiling
    // leaves room for a send buffer of some 60 MB, far past any kernel's
    // default, and costs time only when the case fails.
    let long = "x".repeat(4096);
    let mut hung_up = None;
    for n in 1..=QUEUED * 16 {
        let name = format!("{n}{long}");
        shared.change(vec![row("agents", "O", &name)], json!({}));
        assert_eq!(
            client.heard()["changed"][0]["row"]["name"],
            json!(name),
            "the reading one heard each change"
        );
        if told.try_recv().is_ok() {
            hung_up = Some(n);
            break;
        }
    }
    let hung_up = hung_up.expect("the one no longer reading was never hung up on");
    said_once(&said, "a watch was told what moved");
    // Its watch's answer holds one of the queue's places, so with nothing
    // yet written the queue is full at QUEUED - 1 changes; a hang-up any
    // sooner came before it was. The socket's buffer isn't counted, so
    // this shows the hang-up isn't early, not that it's exactly on time.
    assert!(
        hung_up >= QUEUED,
        "hung up on at change {hung_up}, before its queue of {QUEUED} could fill"
    );
    stuck
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a timeout is set");
    let mut left = Vec::new();
    if let Err(why) = stuck.read_to_end(&mut left) {
        panic!("the stuck connection's reading half wasn't ended: {why}");
    }
    assert!(
        writeln!(stuck, "{watch}").is_err(),
        "the stuck connection's writing half wasn't ended"
    );
    shared.change(vec![row("agents", "O", "kelp")], json!({}));
    assert_eq!(
        client.heard()["changed"],
        json!([row("agents", "O", "kelp")]),
        "the reading one is told on"
    );
}

#[test]
fn a_connection_asking_without_reading_is_hung_up_on() {
    let (shared, _client, folder) = served("asking-unread", Box::new(|_| None));
    let said = hearing(&shared);
    shared.change(vec![row("agents", "O", &"x".repeat(LONG))], json!({}));
    let ask = json!({"id": 1, "ask": "get", "keys": ["row:agents:O"]});
    asked_without_reading(&folder, &ask);
    said_once(&said, "it was answered");
}

#[test]
fn a_connection_asking_slowly_without_reading_is_hung_up_on() {
    let own: Own =
        Box::new(|said| (said["ask"] == "share").then(|| json!({"to": "x".repeat(LONG)})));
    let (shared, _client, folder) = served("asking-slowly-unread", own);
    let said = hearing(&shared);
    shared.slow(&["share"]);
    asked_without_reading(&folder, &json!({"id": 1, "ask": "share"}));
    said_once(&said, "a slow ask was answered");
    // Slow asks still running may find the queue full too, after it.
    assert_eq!(
        said.recv_timeout(Duration::from_millis(200)).ok(),
        None,
        "the hang-up was said more than once"
    );
}

#[test]
fn a_paced_connection_no_longer_reading_is_hung_up_on_by_its_pace() {
    let (shared, _client, folder) = served("paced-unread", Box::new(|_| None));
    let said = hearing(&shared);
    let (telling, told) = channel();
    shared.on_watches(Box::new(move || {
        let _ = telling.send(());
    }));
    // The row is there before the watch, so each change after only moves
    // it: held back, and told by the pace alone, never by the change.
    shared.change(vec![row("agents", "O", "kelp")], json!({}));
    let watch = json!({"id": 1, "ask": "watch", "kind": "row", "every": 1});
    let mut stuck = UnixStream::connect(folder.0.join("daemon.sock")).expect("it connects");
    writeln!(stuck, "{watch}").expect("the watch is asked");
    assert!(in_time(&told).is_some(), "told as its watch begins");
    let long = "x".repeat(LONG);
    let mut hung_up = false;
    for n in 1..=QUEUED * 16 {
        shared.change(vec![row("agents", "O", &format!("{n}{long}"))], json!({}));
        // A little while each time, for its pace of a millisecond to tell it.
        if told.recv_timeout(Duration::from_millis(2)).is_ok() {
            hung_up = true;
            break;
        }
    }
    assert!(
        hung_up,
        "the paced one no longer reading was never hung up on"
    );
    stuck
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a timeout is set");
    drained(&mut stuck);
    assert!(!shared.watches("row", "agents"), "its watch went with it");
    said_once(&said, "a paced watch was told what it held back");
}

#[test]
fn a_hang_up_is_said_once_a_connection_and_never_of_one_that_went() {
    let shared = Shared::new(ROWS, Box::new(|_| None));
    let said = hearing(&shared);
    let (ours, _theirs) = UnixStream::pair().expect("a pair of sockets");
    let went = Arc::new(Peer::of(3, &ours));
    let stopped = Arc::new(Peer::of(4, &ours));
    let stuck = [
        (Arc::clone(&went), Unqueued::Gone),
        (Arc::clone(&stopped), Unqueued::Unread),
        (Arc::clone(&stopped), Unqueued::Unread),
    ];
    shared.hang_up(&mut lock(&shared), &stuck, "a watch was told what moved");
    // Its slow asks find it stopped reading after, and the other gone.
    shared.say_once(&stopped, Unqueued::Unread, "a slow ask was answered");
    shared.say_once(&went, Unqueued::Gone, "it was answered");
    // Lines are written in turn, so what came before this one is all said.
    let last = Peer::of(5, &ours);
    shared.say_once(&last, Unqueued::Unread, "it was answered");
    let mut heard = Vec::new();
    while let Some(line) = in_time(&said) {
        if line.contains("connection 5 ") {
            break;
        }
        heard.push(line);
    }
    assert_eq!(
        heard,
        [format!(
            "agents: hung up on connection 4 ({}), which left {QUEUED} lines unread as a watch \
             was told what moved",
            this_process()
        )],
        "said once of the one that stopped reading, by whichever path found it first, and \
         never of the one whose writer ended"
    );
}

#[test]
fn a_line_is_unqueued_as_unread_once_its_queue_fills_and_gone_once_its_writer_ends() {
    let (told, lines) = mpsc::sync_channel(1);
    assert_eq!(queue(&told, &json!(1)), Ok(()), "the first is queued");
    assert_eq!(
        queue(&told, &json!(2)),
        Err(Unqueued::Unread),
        "its queue is full"
    );
    drop(lines);
    assert_eq!(
        queue(&told, &json!(3)),
        Err(Unqueued::Gone),
        "its writer ended"
    );
}

#[test]
fn hang_ups_past_what_waits_to_be_written_are_counted_never_waited_on() {
    let (opening, open) = channel();
    let (writing, written) = channel();
    let saying = Saying::to(
        "agents",
        Shut {
            open: Some(open),
            writing,
        },
    );
    saying.say("first".to_owned());
    // The first is being written, and its write waits for the case.
    assert!(in_time(&written).is_some(), "the first was never written");
    let (done, said) = channel();
    std::thread::spawn(move || {
        for n in 0..UNSAID + 3 {
            saying.say(format!("line {n}"));
        }
        let _ = done.send(saying);
    });
    let saying = in_time(&said).expect("saying waited on the journal");
    opening.send(()).expect("the write is let go");
    let mut out = String::new();
    while out.lines().count() < UNSAID + 2 {
        let bytes = in_time(&written).expect("fewer lines were written than waited");
        out.push_str(&String::from_utf8(bytes).expect("lines are words"));
    }
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[..2],
        [
            "first",
            "agents: 3 more hang-ups went unsaid while the journal was behind"
        ],
        "the three past the queue were counted and said after the next line written"
    );
    let waited: Vec<String> = (0..UNSAID).map(|n| format!("line {n}")).collect();
    assert_eq!(
        lines[2..],
        waited,
        "the lines that waited were each written once, in turn, and the count said once"
    );
    drop(saying);
}

#[test]
fn a_process_is_named_on_one_line_by_what_could_be_read_of_it() {
    assert_eq!(
        process(Some(7), Some("x\nagents: ok\n")),
        "pid 7, x\\nagents: ok",
        "a name it set itself can't start a line of its own"
    );
    assert_eq!(
        process(Some(7), None),
        "pid 7",
        "a name that couldn't be read"
    );
    assert_eq!(
        process(None, None),
        "an unknown process",
        "a pid that couldn't be"
    );
}

/// A journal that takes nothing until it's opened, as one stalled does,
/// handing each write on once it is.
struct Shut {
    open: Option<Receiver<()>>,
    writing: Sender<Vec<u8>>,
}

impl Write for Shut {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let _ = self.writing.send(Vec::new());
        if let Some(open) = self.open.take() {
            let _ = open.recv();
        }
        let _ = self.writing.send(bytes.to_vec());
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// This test's process, as a hang-up names the one at a connection's
/// other end, since each case connects from it.
fn this_process() -> String {
    let name = std::fs::read_to_string("/proc/self/comm").expect("its name is read");
    format!("pid {}, {}", std::process::id(), name.trim_end())
}

/// Each line the daemon says of a hang-up, written where the case hears
/// it rather than to standard error.
fn hearing(shared: &Shared) -> Receiver<String> {
    let (saying, said) = channel();
    shared.hear_hang_ups(Heard {
        saying,
        part: String::new(),
    });
    said
}

/// A journal that hands each whole line it's written on to a case.
struct Heard {
    saying: Sender<String>,
    part: String,
}

impl Write for Heard {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.part.push_str(&String::from_utf8_lossy(bytes));
        while let Some(end) = self.part.find('\n') {
            let line: String = self.part.drain(..=end).collect();
            let _ = self.saying.send(line.trim_end_matches('\n').to_owned());
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// That the hang-up of the second connection, the client `served`
/// connects being the first, was said as `how` it was told, naming this
/// test's process at its other end. That it's said only once is the unit
/// case's to show, which needs no race to.
fn said_once(said: &Receiver<String>, how: &str) {
    let line = format!(
        "agents: hung up on connection 2 ({}), which left {QUEUED} lines unread as {how}",
        this_process()
    );
    assert_eq!(in_time(said), Some(line), "the hang-up was said");
}

/// How long an answer the hang-up cases ask for: long enough that the
/// socket's buffer (Linux's `net.core.wmem_default`, some 200 KB) holds a
/// dozen or so, far fewer than QUEUED, unless a host raises it past 4 MB.
const LONG: usize = 16384;

/// Asks `ask`, whose answer is long, over and over without reading, until
/// the daemon hangs up, then reads what's left.
fn asked_without_reading(folder: &Folder, ask: &Value) {
    let mut stuck = UnixStream::connect(folder.0.join("daemon.sock")).expect("it connects");
    let patience = Some(Duration::from_secs(10));
    stuck.set_write_timeout(patience).expect("a timeout is set");
    stuck.set_read_timeout(patience).expect("a timeout is set");
    // Each answer is LONG, so the socket's buffer fills within a dozen or
    // so and the queue behind it within QUEUED more. Past that it hangs up, and
    // the next ask written finds it gone; left open, its buffer fills too
    // and the write waits out its timeout. Asked for a while rather than a
    // number of times, since slow asks are answered on threads of their own,
    // which on a busy host may fall behind the asking.
    let began = Instant::now();
    let mut ended = None;
    while ended.is_none() && began.elapsed() < Duration::from_secs(30) {
        ended = writeln!(stuck, "{ask}").err();
    }
    let ended = ended.expect("the daemon read every ask for 30 s and never hung up");
    // A broken pipe once it's shut, or a reset once it's closed with asks
    // still unread; a wait that timed out means it was left open.
    assert!(
        matches!(
            ended.kind(),
            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
        ),
        "the one asking without reading was left open: {ended}"
    );
    drained(&mut stuck);
}

/// Reads all a connection hung up on has left to read: only what its
/// socket's buffer held, since the answers queued behind it go with the
/// writer, which ends. More than QUEUED whole answers means the writer
/// went on writing them.
fn drained(stuck: &mut UnixStream) {
    use std::io::ErrorKind::{ConnectionReset, TimedOut, WouldBlock};
    let mut left = Vec::new();
    let mut chunk = [0; LONG];
    loop {
        match stuck.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => left.extend_from_slice(&chunk[..read]),
            Err(why) if why.kind() == ConnectionReset => break,
            Err(why) if matches!(why.kind(), WouldBlock | TimedOut) => {
                panic!("the connection was left open: its read waited out its timeout")
            }
            Err(why) => panic!("the connection hung up on wasn't ended: {why}"),
        }
    }
    // Whole answers only, each ended by its newline.
    let answers = left.split(|byte| *byte == b'\n').count() - 1;
    assert!(
        answers < QUEUED,
        "{answers} answers were read, more than a socket's buffer holds, so its writer \
         went on writing past the hang-up (unless net.core.wmem_default is past 4 MB)"
    );
}
