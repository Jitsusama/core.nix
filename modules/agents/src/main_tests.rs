use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

use super::*;

/// The daemon's own socket, served as `serve` serves it, refuses a
/// sprout from a process that isn't the shell's, this test's, asks
/// the node nothing, and keeps nothing of a start it names, nor takes
/// its word that one was told. Which are kept it answers anyone, from
/// the file as it was read before the socket listened.
#[test]
fn the_daemons_socket_takes_no_sprout_from_anyone_but_the_shell() {
    let folder = std::env::temp_dir().join(format!("agents-sprouts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for the sockets");
    // A node that would take any ask, so one that reached it shows.
    let at_node = folder.join("node.sock");
    let node = UnixListener::bind(&at_node).expect("a node's socket");
    node.set_nonblocking(true).expect("the node never waits");
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (told, heard) = mpsc::channel();
    let keeping = Keeping::read(&folder.join(starts::FILE), clock::now_ms());
    keeping
        .change(|kept| kept.begun("a-0", clock::now_ms()))
        .expect("a start kept");
    let stood = Arc::new(starts_stopped::Told::new(told.clone()));
    for_the_shell_alone(
        &shared,
        (Some(at_node), Wants::default()),
        keeping,
        told,
        (stood, Said::default()),
        Box::new(|_| None),
    );
    let socket = folder.join("agents.sock");
    let listener = serve::listen(&socket).expect("the daemon's socket");
    serve::accept(shared, listener);
    let mut asking = UnixStream::connect(&socket).expect("the daemon takes a connection");
    let ask = json!({"id": 1, "ask": sprout::ASK, "text": "https://github.com/o/r/issues/7",
        "start": "a-1"});
    writeln!(asking, "{ask}").expect("the sprout is asked");
    let ask = json!({"id": 2, "ask": starts::TOLD, "starts": ["a-0"]});
    writeln!(asking, "{ask}").expect("the told is asked");
    let ask = json!({"id": 3, "ask": starts::KEPT, "starts": ["a-9", "a-0", "a-1"]});
    writeln!(asking, "{ask}").expect("which are kept is asked");
    let mut reading = BufReader::new(&asking);
    let mut answer_to = |what: &str| {
        let mut line = String::new();
        reading.read_line(&mut line).expect(what);
        serde_json::from_str::<Value>(&line).expect("one line of JSON")
    };
    let mut answers = [
        answer_to("an answer"),
        answer_to("a second answer"),
        answer_to("a third answer"),
    ];
    // Each is answered on a thread of its own, so in any order.
    answers.sort_by_key(|one| one["id"].as_u64());
    let [answer, told_answer, kept_answer] = answers;
    assert_eq!(
        kept_answer["kept"],
        json!(["a-0"]),
        "kept, as the file says, with its told refused: {kept_answer}"
    );
    assert_eq!(
        told_answer["error"]["code"], "untold",
        "took a told: {told_answer}"
    );
    // Both are answered after all they tell the loop.
    assert!(
        heard.try_recv().is_err(),
        "the loop was told of another's start"
    );
    let why = answer["error"]["message"].as_str().unwrap_or_default();
    // The lent builders run Linux 6.1, where who asked can't be told;
    // there that refusal is what's held to.
    assert!(
        why == "only the launcher starts new work here" || why.contains("names no peer by pidfd"),
        "refused, saying why: {answer}"
    );
    assert!(
        node.accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "the node was never asked"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Let Them Start on the daemon's own socket, from a connection that
/// isn't the shell's (this test's), asks the node nothing, so no agent
/// lifts the stop on itself by asking the daemon.
#[test]
fn the_daemons_socket_lets_no_one_but_the_shell_lift_the_stop() {
    let folder = std::env::temp_dir().join(format!("agents-lift-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for the sockets");
    let at_node = folder.join("node.sock");
    let node = UnixListener::bind(&at_node).expect("a node's socket");
    node.set_nonblocking(true).expect("the node never waits");
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (told, heard) = mpsc::channel();
    let stood = Arc::new(starts_stopped::Told::new(told.clone()));
    let keeping = Keeping::read(&folder.join(starts::FILE), clock::now_ms());
    let words = Words::from_vocabulary(include_str!("../tests/words.json"))
        .expect("the launcher's words read");
    for_the_shell_alone(
        &shared,
        (Some(at_node), Wants::default()),
        keeping,
        told,
        (stood, words.starts),
        Box::new(|_| None),
    );
    let socket = folder.join("agents.sock");
    let listener = serve::listen(&socket).expect("the daemon's socket");
    serve::accept(shared, listener);
    let mut asking = UnixStream::connect(&socket).expect("the daemon takes a connection");
    writeln!(asking, "{}", json!({"id": 1, "ask": starts_stopped::ASK}))
        .expect("Let Them Start is asked");
    let mut line = String::new();
    BufReader::new(&asking)
        .read_line(&mut line)
        .expect("an answer");
    let answer: Value = serde_json::from_str(&line).expect("one line of JSON");
    assert_eq!(
        answer["error"]["message"], "agents start again only from your own Return in the launcher",
        "refused in its own words, whatever the kernel lets be told of who asked: {answer}"
    );
    assert!(
        node.accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "the node was never asked"
    );
    assert!(
        heard.try_recv().is_err(),
        "and nothing was told of the starts"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// What ASKER asks Joel, as the loop holds it, and Joel's answer to it.
fn asked_of_joel() -> (Wants, Value) {
    let wants = Wants::default();
    let want = json!({"kind": wants::KIND, "address": "ASKER",
        "answers": {"want": "question@2026-11-25T09:00:00.000Z"}});
    wants.hold([want].iter());
    let said = json!({"ask": wants::ASK, "address": "asker",
        "want": "question@2026-11-25T09:00:00.000Z", "value": "push"});
    (wants, said)
}

/// An answer on the daemon's own socket, from a connection that isn't
/// the shell's (this test's), asks the node nothing, so an agent can't
/// answer its own ask as Joel by asking the daemon (ISXEG8's
/// R-agent-answers-own-ask).
#[test]
fn the_daemons_socket_takes_no_answer_from_anyone_but_the_shell() {
    let folder = std::env::temp_dir().join(format!("agents-answer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for the sockets");
    let at_node = folder.join("node.sock");
    let node = UnixListener::bind(&at_node).expect("a node's socket");
    node.set_nonblocking(true).expect("the node never waits");
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (told, _heard) = mpsc::channel();
    let stood = Arc::new(starts_stopped::Told::new(told.clone()));
    let keeping = Keeping::read(&folder.join(starts::FILE), clock::now_ms());
    let (wants, mut said) = asked_of_joel();
    for_the_shell_alone(
        &shared,
        (Some(at_node), wants),
        keeping,
        told,
        (stood, Said::default()),
        Box::new(|_| None),
    );
    let socket = folder.join("agents.sock");
    let listener = serve::listen(&socket).expect("the daemon's socket");
    serve::accept(shared, listener);
    let mut asking = UnixStream::connect(&socket).expect("the daemon takes a connection");
    said["id"] = json!(1);
    writeln!(asking, "{said}").expect("the answer is asked");
    let mut line = String::new();
    BufReader::new(&asking)
        .read_line(&mut line)
        .expect("an answer");
    let answer: Value = serde_json::from_str(&line).expect("one line of JSON");
    assert_eq!(
        (&answer["error"]["code"], &answer["error"]["message"]),
        (&json!("refused"), &json!(ONLY_THE_SHELL)),
        "refused in its own words, whatever the kernel lets be told of who asked: {answer}"
    );
    assert!(
        node.accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "the node was never asked"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// An answer the shell asks reaches the node as Joel's answer to the
/// want he was shown, and one any other asks never does, whatever it
/// names.
#[test]
fn only_the_shells_answer_reaches_the_node() {
    let (wants, said) = asked_of_joel();
    let socket = std::env::temp_dir().join(format!("agents-answered-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
    listener
        .set_nonblocking(true)
        .expect("the node never waits");
    let refused = answered(
        Err("not the shell".to_owned()),
        &wants,
        &said,
        Some(&socket),
    );
    assert_eq!(
        refused,
        json!({"error": {"code": "refused", "message": ONLY_THE_SHELL}}),
        "another's answer is refused"
    );
    // Refused before it's judged, so one naming no want gets no word of
    // what the daemon holds.
    assert_eq!(
        answered(
            Err("not the shell".to_owned()),
            &wants,
            &json!({"ask": "answer", "address": "NOBODY"}),
            Some(&socket),
        ),
        json!({"error": {"code": "refused", "message": ONLY_THE_SHELL}}),
        "another's answer naming no want is refused alike"
    );
    assert!(
        listener
            .accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "the node was never asked for another's answer"
    );
    listener.set_nonblocking(false).expect("the node waits now");
    let node = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the answer asks the node");
        let mut answering = stream.try_clone().expect("a stream to answer on");
        let mut asks = BufReader::new(stream).lines();
        asks.next().expect("a hello").expect("a line");
        writeln!(answering, r#"{{"id": 1, "hello": {{}}}}"#).expect("the hello answered");
        let ask = asks.next().expect("a steer").expect("a line");
        writeln!(
            answering,
            r#"{{"id": 2, "steered": {{"machine": "skiff"}}}}"#
        )
        .expect("the steer answered");
        ask
    });
    let taken = answered(Ok(()), &wants, &said, Some(&socket));
    // Before the node's joined, so an answer that never reached it fails
    // here rather than waiting on it for good.
    assert!(
        taken.get("error").is_none(),
        "the shell's answer is taken: {taken}"
    );
    let ask: Value =
        serde_json::from_str(&node.join().expect("the node stood in")).expect("the steer is JSON");
    let _ = std::fs::remove_file(&socket);
    assert_eq!(
        taken,
        json!({"steered": {"machine": "skiff"}}),
        "the node's answer"
    );
    assert_eq!(
        (
            &ask["address"],
            &ask["steer"]["act"],
            &ask["steer"]["want"],
            &ask["steer"]["value"],
            &ask["steer"]["from"]
        ),
        (
            &json!("ASKER"),
            &json!("answer"),
            &json!("question@2026-11-25T09:00:00.000Z"),
            &json!("push"),
            &json!("joel")
        ),
        "Joel's answer to the want he was shown: {ask}"
    );
}

/// Places on the daemon's own socket, from a connection that isn't
/// the shell's (this test's), asks the node nothing, so no agent reads
/// the folders the node lists for Joel.
#[test]
fn the_daemons_socket_lists_no_places_for_anyone_but_the_shell() {
    let folder = std::env::temp_dir().join(format!("agents-places-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for the sockets");
    let at_node = folder.join("node.sock");
    let node = UnixListener::bind(&at_node).expect("a node's socket");
    node.set_nonblocking(true).expect("the node never waits");
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (told, _heard) = mpsc::channel();
    let stood = Arc::new(starts_stopped::Told::new(told.clone()));
    let keeping = Keeping::read(&folder.join(starts::FILE), clock::now_ms());
    for_the_shell_alone(
        &shared,
        (Some(at_node), Wants::default()),
        keeping,
        told,
        (stood, Said::default()),
        Box::new(|_| None),
    );
    let socket = folder.join("agents.sock");
    let listener = serve::listen(&socket).expect("the daemon's socket");
    serve::accept(shared, listener);
    let mut asking = UnixStream::connect(&socket).expect("the daemon takes a connection");
    writeln!(asking, "{}", json!({"id": 1, "ask": places::ASK})).expect("places are asked");
    let mut line = String::new();
    BufReader::new(&asking)
        .read_line(&mut line)
        .expect("an answer");
    let answer: Value = serde_json::from_str(&line).expect("one line of JSON");
    let why = answer["error"]["message"].as_str().unwrap_or_default();
    // The lent builders run Linux 6.1, where who asked can't be told;
    // there that's said as it is.
    assert!(
        answer["error"]["code"] == "unlisted"
            && (why == places::REFUSED || why.contains("names no peer by pidfd")),
        "refused in its own words: {answer}"
    );
    assert!(
        node.accept()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "the node was never asked"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Each of a row's acts, a line told, feedback and what the shell says
/// Joel saw, read, put off or forgot, on the daemon's own socket, from a
/// connection that isn't the shell's (this test's), is refused without
/// being done, so no agent steers, talks to or tells a session as him,
/// nor hides from him what he'd see, by asking the daemon (ISXEG8's
/// R-agents-acts-as-joel).
#[test]
fn the_daemons_socket_acts_as_joel_for_no_one_but_the_shell() {
    let folder = std::env::temp_dir().join(format!("agents-as-joel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder for the socket");
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (told, _heard) = mpsc::channel();
    let stood = Arc::new(starts_stopped::Told::new(told.clone()));
    let keeping = Keeping::read(&folder.join(starts::FILE), clock::now_ms());
    let (done, asked) = mpsc::channel();
    let done = std::sync::Mutex::new(done);
    for_the_shell_alone(
        &shared,
        (None, Wants::default()),
        keeping,
        told,
        (stood, Said::default()),
        Box::new(move |said| {
            let _ = done
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send(said["ask"].clone());
            Some(json!({"done": true}))
        }),
    );
    let socket = folder.join("agents.sock");
    let listener = serve::listen(&socket).expect("the daemon's socket");
    serve::accept(shared, listener);
    let asking = UnixStream::connect(&socket).expect("the daemon takes a connection");
    let mut reading = BufReader::new(&asking);
    for act in acts::ACTS {
        assert!(AS_JOEL.contains(&act), "{act} is named as Joel's");
    }
    for act in AS_JOEL {
        let ask = json!({"id": 1, "ask": act, "address": "P9R1KI", "text": "hi",
            "quest": "QEST-1-SKIFF", "words": "hi", "landing": "landing:s-p:o#1",
            "known": "may I?", "session": "s-1", "keys": ["k"], "sharing": false,
            "places": []});
        writeln!(&asking, "{ask}").expect("the act is asked");
        let mut line = String::new();
        reading.read_line(&mut line).expect("an answer");
        let answer: Value = serde_json::from_str(&line).expect("one line of JSON");
        assert_eq!(
            (&answer["error"]["code"], &answer["error"]["message"]),
            (&json!("refused"), &json!(NOT_AS_JOEL)),
            "{act} refused in its own words, whatever the kernel lets be told of who asked"
        );
    }
    assert!(
        asked.try_recv().is_err(),
        "none was done for a program that isn't the shell"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Every one of Joel's acts is the shell's alone, and each but feedback
/// is answered in the order the shell asks it, never apart, so a share's
/// end can't land before its start, nor an older set of his places after
/// a newer one; and each answered apart is the shell's alone too.
#[test]
fn joels_acts_are_the_shells_alone_and_answered_in_order() {
    for act in AS_JOEL {
        assert!(SHELLS_ALONE.contains(&act), "{act} is the shell's alone");
        assert_eq!(
            SLOW.contains(&act),
            act == "feedback",
            "{act} is answered in order, but feedback, which waits on its quest"
        );
    }
    for ask in SLOW {
        assert!(SHELLS_ALONE.contains(&ask), "{ask} is the shell's alone");
    }
}
