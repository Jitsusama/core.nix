//! The cases of a connection that stops reading: hung up on, by a change,
//! an ask, a slow ask or a pace, once what it's owed can't be queued, its
//! writer ended, while the rest are still told.

use std::io::Read;

use super::*;

#[test]
fn a_connection_no_longer_reading_is_hung_up_on_and_the_rest_told_on() {
    let (shared, mut client, folder) = served("stuck", Box::new(|_| None));
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
    shared.change(vec![row("agents", "O", &"x".repeat(LONG))], json!({}));
    let ask = json!({"id": 1, "ask": "get", "keys": ["row:agents:O"]});
    asked_without_reading(&folder, &ask);
}

#[test]
fn a_connection_asking_slowly_without_reading_is_hung_up_on() {
    let own: Own =
        Box::new(|said| (said["ask"] == "share").then(|| json!({"to": "x".repeat(LONG)})));
    let (shared, _client, folder) = served("asking-slowly-unread", own);
    shared.slow(&["share"]);
    asked_without_reading(&folder, &json!({"id": 1, "ask": "share"}));
}

#[test]
fn a_paced_connection_no_longer_reading_is_hung_up_on_by_its_pace() {
    let (shared, _client, folder) = served("paced-unread", Box::new(|_| None));
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
