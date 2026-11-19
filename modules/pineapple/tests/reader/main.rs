//! pineapple-slack against a stand-in for Slack on this machine: a stop
//! said in a thread about a pull request reaches that pull request and is
//! kept as a shadow record; a tag asking to stop without the phrase is
//! written down and stops nothing; of the would-be replies, only the first
//! fresh stop in a thread's goes through, and the switch holds even that
//! back; only Slack's reading methods are asked; a restart judges
//! nothing twice; and an alert for Joel's phone finds his own IM and, in
//! shadow, is only written down. The reader let go live is in live.rs,
//! with a keyring that hangs in keyring.rs, telling Joel in For You when
//! Slack can't be heard in deaf.rs, and GitHub's reader, against its own
//! stand-in, in github.rs, and the guard each case holds its reader by
//! in guard.rs.

mod deaf;
mod github;
mod guard;
mod kept;
mod keyring;
mod live;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// How long a case waits on the reader before it calls it stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// How often the reader searches in a case, in milliseconds.
const PACE: &str = "100";

/// Where in its home a reader's journal is kept.
const JOURNAL: &str = "journal";

/// A reader a case started in `home`, stopped once the case is done with
/// it, whether the case passes or fails part way, so a failed case leaves
/// no reader still asking its stand-in. A case failing while it holds the
/// guard is shown its home's journal too, whichever wait or check gave up;
/// one failing in a helper that has already dropped it is not.
struct Running {
    reader: Child,
    home: PathBuf,
}

impl Running {
    /// The guard over `reader`, started in `home`.
    fn over(home: &Path) -> impl FnOnce(Child) -> Self {
        let home = home.to_owned();
        move |reader| Self { reader, home }
    }
}

impl Deref for Running {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.reader
    }
}

impl DerefMut for Running {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.reader
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // std reaps a child once and never signals one it has reaped, so a
        // case that stopped its reader itself makes these do nothing; and
        // one failing part way has its own panic to say, so there is
        // nothing more to tell from a kill or a wait that goes wrong.
        let _ = self.reader.kill();
        let _ = self.reader.wait();
        if thread::panicking() {
            let said = std::fs::read_to_string(self.home.join(JOURNAL))
                .unwrap_or_else(|why| format!("(its journal couldn't be read: {why})"));
            eprintln!(
                "what the readers in {} said to their journal:\n{said}",
                self.home.display()
            );
        }
    }
}

/// The journal in a reader's `home`, opened to be added to, so a
/// reader started again in the same home keeps what the one before it said.
fn journal_of(home: &Path) -> std::fs::File {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join(JOURNAL))
        .expect("its journal is opened")
}

/// What the readers started in `home` have said to its journal so far,
/// which their start opened, so a journal that can't be read fails the
/// case rather than reading as one that said nothing.
fn journal(home: &Path) -> String {
    std::fs::read_to_string(home.join(JOURNAL)).expect("its journal is read")
}

/// The stand-in's Joel, and the colleagues who tag him.
const JOEL: &str = "U0JOEL0001";

/// A stand-in for Slack's API that keeps the first line of each request,
/// a post's form after it, and answers as Slack would to a search and to
/// a thread's first message.
fn stand_in() -> (String, Arc<Mutex<Vec<String>>>) {
    answering(answer)
}

/// The stand-in, answering each request as `answer` says.
fn answering(answer: fn(&str, u64) -> Value) -> (String, Arc<Mutex<Vec<String>>>) {
    answering_whole(move |line, now| Answered::ok(answer(line, now)))
}

/// What the stand-in answers one request with.
struct Answered {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: Value,
}

impl Answered {
    /// An answer as a service gives one that went well.
    fn ok(body: Value) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body,
        }
    }
}

/// The stand-in, answering each request whole, its status and headers
/// too, as `answer` says.
fn answering_whole(
    answer: impl Fn(&str, u64) -> Answered + Send + 'static,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on this machine");
    let origin = format!("http://{}", listener.local_addr().expect("its address"));
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&asked);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("the stream again"));
            let mut line = String::new();
            // A connection closed before it asks, as a killed reader's
            // is, asked nothing.
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                continue;
            }
            // Its headers end at an empty line; a GET has no body, and a
            // post's form is as long as its Content-Length says.
            let mut header = String::new();
            let mut length = 0;
            while reader.read_line(&mut header).unwrap_or(0) > 0 && !header.trim().is_empty() {
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
                header.clear();
            }
            let mut form = vec![0; length];
            if reader.read_exact(&mut form).is_err() {
                continue;
            }
            let answered = answer(&line, now);
            let said = answered.body.to_string();
            let mut one = line.trim_end().to_owned();
            if length > 0 {
                one = format!("{one} {}", String::from_utf8_lossy(&form));
            }
            kept.lock().expect("the stand-in's list").push(one);
            let headers = answered
                .headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}\r\n"))
                .collect::<Vec<_>>()
                .concat();
            let reply = format!(
                "HTTP/1.1 {} Answered\r\nContent-Type: application/json\r\n{headers}\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                answered.status,
                said.len()
            );
            // A reader killed mid-answer leaves nobody to write to.
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    (origin, asked)
}

/// A stop said `ago` seconds before `now` in the thread `thread` starts.
fn stop_in(now: u64, ago: u64, thread: &str) -> Value {
    let ts = format!("{}.000100", now - ago);
    json!({
        "channel": {"id": "C0ONE00001"}, "ts": ts, "user": "U0OTHER001",
        "text": format!("<@{JOEL}> pineapple override"),
        "permalink": format!(
            "https://example.slack.com/archives/C0ONE00001/p{}?thread_ts={thread}&cid=C0ONE00001",
            ts.replace('.', "")
        ),
    })
}

/// What Slack would answer to one request: an hour-old stop in one
/// thread; a stop, an ask without the phrase and the stop again in
/// another, in the last minute.
fn answer(line: &str, now: u64) -> Value {
    let old = stop_in(now, 3600, "1700000000.000002");
    let stop = stop_in(now, 60, "1700000000.000001");
    let again = stop_in(now, 20, "1700000000.000001");
    let ask_ts = format!("{}.000200", now - 30);
    let ask = json!({
        "channel": {"id": "C0ONE00001"}, "ts": ask_ts, "user": "U0OTHER002",
        "text": format!("<@{JOEL}> can you please STOP your robots?"),
        "permalink": format!(
            "https://example.slack.com/archives/C0ONE00001/p{}",
            ask_ts.replace('.', "")
        ),
    });
    // Slack names the IM for whoever is asked for, and makes none here.
    if line.contains("/api/conversations.open") {
        let (id, user) = if line.contains(&format!("users={JOEL}&")) {
            ("D0SELF0001", JOEL)
        } else {
            ("D0OTHER001", "U0OTHER001")
        };
        return json!({"ok": true, "channel": {"id": id, "is_im": true, "user": user}});
    }
    if line.contains("/api/conversations.info") && line.contains("D0SELF0001") {
        return json!({"ok": true, "channel": {"id": "D0SELF0001", "is_im": true, "user": JOEL}});
    }
    if line.contains("/api/conversations.replies") {
        return json!({"ok": true, "messages": [
            {"text": "a review, please: https://code.example.com/example/shop/pull/12"}
        ]});
    }
    if line.contains("query=pineapple") {
        json!({"ok": true, "messages": {"matches": [again, stop, old]}})
    } else {
        json!({"ok": true, "messages": {"matches": [again, ask, stop, old]}})
    }
}

/// What Slack would answer when Joel himself says "pineapple resume" in
/// the thread about example/shop#12, a minute ago.
fn joel_resumes(line: &str, now: u64) -> Value {
    let ts = format!("{}.000300", now - 60);
    let resume = json!({
        "channel": {"id": "C0ONE00001"}, "ts": ts, "user": JOEL,
        "text": "pineapple resume",
        "permalink": format!(
            "https://example.slack.com/archives/C0ONE00001/p{}\
             ?thread_ts=1700000000.000001&cid=C0ONE00001",
            ts.replace('.', "")
        ),
    });
    if line.contains("/api/conversations.replies") {
        return answer(line, now);
    }
    let matches = if line.contains("query=pineapple") {
        vec![resume]
    } else {
        Vec::new()
    };
    json!({"ok": true, "messages": {"matches": matches}})
}

/// What Slack would answer when the only match is the reader's own alert
/// in Joel's own IM, written as Joel, quoting words made so that its
/// quotation marks pair early and "pineapple resume" falls outside them.
fn joel_hears_his_own_alert(line: &str, now: u64) -> Value {
    let ts = format!("{}.000400", now - 60);
    let echo = json!({
        "channel": {"id": "D0SELF0001"}, "ts": ts, "user": JOEL,
        "text": format!(
            "pineapple: Someone asked you to resume your agents\n\
             Said: \"&lt;@{JOEL}&gt; \" pineapple resume\"\n\
             Where: https://example.slack.com/archives/C0ONE00001/p1"
        ),
        "permalink": format!(
            "https://example.slack.com/archives/D0SELF0001/p{}",
            ts.replace('.', "")
        ),
    });
    let matches = if line.contains("query=pineapple") {
        vec![echo]
    } else {
        Vec::new()
    };
    json!({"ok": true, "messages": {"matches": matches}})
}

/// A home for one case, its sign-in in place as pi keeps it, and the
/// reader last stopped two hours ago, so it catches up on what it missed.
fn home() -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "pineapple-reader-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_nanos()
    ));
    let signed_in = home.join("config/agentic-harness/slack");
    std::fs::create_dir_all(&signed_in).expect("a home for the case");
    let account = json!({"token": {
        "accessToken": "xoxc-stand-in", "cookie": "stand-in", "teamId": "T0TEAM0001",
        "userId": JOEL,
    }});
    std::fs::write(signed_in.join("credentials.json"), account.to_string())
        .expect("the sign-in written");
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let judged = json!({"since": format!("{}.000000", now - 2 * 3600), "keys": []});
    std::fs::write(state.join("judged.json"), judged.to_string()).expect("the mark written");
    home
}

/// One field of each decision, in the order they were written.
/// The one reply the reader let through, as replies.jsonl keeps it.
fn the_one_reply(home: &Path) -> Value {
    let replies = std::fs::read_to_string(home.join("state/pineapple/replies.jsonl"))
        .expect("the replies let through are kept");
    assert_eq!(replies.lines().count(), 1, "one reply would have gone out");
    serde_json::from_str(replies.lines().next().unwrap_or_default())
        .expect("a reply's line is JSON")
}

/// Holds that the stand-in was asked only Slack's methods that post
/// nothing: searching and reading threads, and looking for Joel's own IM,
/// which the reader does once as it starts, for him alone and never
/// making one. That lookup, conversations.open, is write-scoped all the
/// same, and marks his IM open again if he closed it, so the list isn't
/// strictly read-only.
fn only_reads(asked: &Mutex<Vec<String>>) {
    let opened: Vec<String> = asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter(|line| line.contains("/api/conversations.open"))
        .cloned()
        .collect();
    assert!(
        opened.iter().all(|line| {
            line.contains(&format!(
                "?users={JOEL}&prevent_creation=true&return_im=true "
            ))
        }),
        "Joel's own IM is looked for with him alone, and never made: {opened:?}"
    );
    let methods: Vec<String> = asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|path| path.strip_prefix("/api/"))
        .map(|path| path.split('?').next().unwrap_or_default().to_owned())
        .collect();
    let reading = [
        "search.messages",
        "conversations.replies",
        "conversations.open",
        "conversations.info",
    ];
    assert!(
        methods
            .iter()
            .all(|method| reading.contains(&method.as_str())),
        "only methods that post nothing are asked: {methods:?}"
    );
}

fn each<'a>(said: &'a [Value], field: &str) -> Vec<&'a str> {
    said.iter().filter_map(|one| one[field].as_str()).collect()
}

fn reader(home: &Path, origin: &str) -> Running {
    reader_with(home, origin, &[])
}

/// The reader, given `more` on its line after the stand-in and pace.
fn reader_with(home: &Path, origin: &str, more: &[&str]) -> Running {
    reading(home, origin, more, None)
}

/// The reader, telling For You in `waiting` when it names one, as its
/// unit does, and finding its own file otherwise, whatever the run's
/// own environment names. It runs in its home, so a relative name lands
/// there rather than in the crate, and says to its journal there.
fn reading(home: &Path, origin: &str, more: &[&str], waiting: Option<&Path>) -> Running {
    let mut reader = Command::new(env!("CARGO_BIN_EXE_pineapple-slack"));
    reader
        .current_dir(home)
        .env_remove("SAFE_WORD_WAITING")
        .env_remove("SIGN_IN_KEYRING");
    if let Some(waiting) = waiting {
        reader.env("SAFE_WORD_WAITING", waiting);
    }
    reader
        .args(["run", "--origin", origin, "--every", PACE])
        .args(more)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .stdout(Stdio::null())
        .stderr(journal_of(home))
        .spawn()
        .map(Running::over(home))
        .expect("the reader starts")
}

/// The decisions written down so far.
fn decisions(home: &Path) -> Vec<Value> {
    std::fs::read_to_string(home.join("state/pineapple/decisions.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// When the reader last marked what it judged in its `mark`, if it has.
fn marked(home: &Path, mark: &str) -> Option<u64> {
    let kept = std::fs::read_to_string(home.join("state/pineapple").join(mark)).ok()?;
    serde_json::from_str::<Value>(&kept).ok()?["read_at"].as_u64()
}

/// Until the reader marks its `mark` anew, after the mark it had then,
/// so the mark holds all it had decided by the time it was asked. A
/// round marks only once it ends, so a reader stopped before then
/// judges that round again when it restarts.
fn until_marked_anew(home: &Path, mark: &str) {
    let was = marked(home, mark);
    let began = Instant::now();
    while marked(home, mark).is_none_or(|at| Some(at) == was) {
        assert!(
            began.elapsed() < PATIENCE,
            "{mark} was never marked anew past {was:?}; it holds {:?}",
            std::fs::read_to_string(home.join("state/pineapple").join(mark))
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Waits until the reader has written `count` decisions, or says it's stuck.
fn until_decided(home: &Path, count: usize) {
    let began = Instant::now();
    while decisions(home).len() < count {
        assert!(
            began.elapsed() < PATIENCE,
            "the reader wrote {} decisions, not {count}",
            decisions(home).len()
        );
        // The reader searches every tenth of a second, so a short wait
        // sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_stop_in_a_thread_reaches_its_pull_request_in_shadow_and_is_judged_once() {
    let (origin, asked) = stand_in();
    let home = home();
    let mut first = reader(&home, &origin);
    until_decided(&home, 4);
    until_marked_anew(&home, "judged.json");
    first.kill().expect("the reader stops");
    first.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "stop", "stop-asked", "stop"],
        "oldest first, each judged"
    );
    assert_eq!(
        each(&said, "gate"),
        [
            "held:too-old",
            "through:first-in-thread",
            "unasked",
            "held:repeat-in-thread"
        ],
        "only the first fresh stop in its thread would be answered"
    );
    assert_eq!(
        each(&said, "tell"),
        ["loud", "loud", "loud", "loud"],
        "Joel hears of each, the ask without the phrase too"
    );
    assert_eq!(
        said[1]["reaches"],
        json!(["pr:example/shop#12"]),
        "the thread's pull request is what stops"
    );
    assert_eq!(said[1]["shadow"], true, "it is in shadow");
    assert_eq!(
        the_one_reply(&home)["post"],
        json!({
            "channel": "C0ONE00001",
            "thread_ts": "1700000000.000001",
            "text": "Automated reply, not Joel: his agents have stopped work on pull request \
                     example/shop#12. Only he can restart them, and he's been told."
        }),
        "the reply it would say is kept, in the thread that asked, in Joel's words"
    );

    let stops: Vec<PathBuf> = std::fs::read_dir(home.join("state/pineapple/stops"))
        .expect("the stops folder")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    assert_eq!(stops.len(), 3, "each stop is kept, answered or not");
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(&stops[0]).expect("a stop's record"))
            .expect("the record is JSON");
    assert_eq!(record["shadow"], true, "the record says it is in shadow");
    assert_eq!(record["said_by"], "U0OTHER001", "it names who said it");
    assert_eq!(record["resumed"], Value::Null, "nobody has resumed it");

    only_reads(&asked);

    let mut again = reader(&home, &origin);
    let rounds = asked.lock().expect("what was asked").len();
    let began = Instant::now();
    while asked.lock().expect("what was asked").len() < rounds + 6 {
        assert!(
            began.elapsed() < PATIENCE,
            "the restarted reader asked nothing"
        );
        // As above, a short wait sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    // A round with nothing new still says when Slack was read.
    until_marked_anew(&home, "judged.json");
    again.kill().expect("the reader stops");
    again.wait().expect("the reader is gone");
    assert_eq!(decisions(&home).len(), 4, "a restart judges nothing twice");
    let heard = Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
        .arg("heard")
        .env("XDG_STATE_HOME", home.join("state"))
        .output()
        .expect("heard answers");
    assert!(
        String::from_utf8_lossy(&heard.stdout).contains("stop"),
        "heard still lists what was heard: {heard:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// A stop's record in `home`'s stops folder, in shadow or not, said `ago`
/// seconds back.
fn a_stop(home: &Path, id: &str, reach: &str, shadow: bool, ago: u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let record = json!({"id": id, "shadow": shadow, "reaches": [reach], "resumed": null,
        "said_at": format!("{}.000100", now - ago)});
    std::fs::create_dir_all(home.join("state/pineapple/stops")).expect("a stops folder");
    std::fs::write(
        home.join(format!("state/pineapple/stops/{id}.json")),
        record.to_string(),
    )
    .expect("a stop written");
}

/// Who resumed the stop `id`, once the reader has had its time.
fn resumed(home: &Path, id: &str) -> Value {
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(home.join(format!("state/pineapple/stops/{id}.json")))
            .expect("the stop's record"),
    )
    .expect("the record is JSON");
    record["resumed"].clone()
}

#[test]
fn joels_own_resume_in_shadow_ends_only_the_shadow_stop_it_names() {
    let (origin, _) = answering(joel_resumes);
    let home = home();
    // Joel's resume was said a minute ago.
    a_stop(&home, "named", "pr:example/shop#12", true, 120);
    a_stop(&home, "elsewhere", "quest:QEST-A", true, 120);
    a_stop(&home, "live", "pr:example/shop#12", false, 120);
    a_stop(&home, "later", "pr:example/shop#12", true, 30);
    let mut reading = reader(&home, &origin);
    until_decided(&home, 1);
    let began = Instant::now();
    while resumed(&home, "named").is_null() {
        assert!(began.elapsed() < PATIENCE, "Joel's resume ended nothing");
        // The decision is written just before the resume; a short wait
        // sees it land without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(each(&said, "verdict"), ["resume"], "Joel's own resume");
    assert_eq!(
        resumed(&home, "named")["by"],
        "joel",
        "the stop it names is resumed by Joel"
    );
    assert_eq!(
        resumed(&home, "elsewhere"),
        Value::Null,
        "a stop it doesn't name still stands"
    );
    assert_eq!(
        resumed(&home, "live"),
        Value::Null,
        "a reader in shadow never ends a live stop"
    );
    assert_eq!(
        resumed(&home, "later"),
        Value::Null,
        "a stop said after the resume, though found beside it, still stands"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn the_readers_own_alert_in_joels_im_is_never_judged_as_his_saying() {
    let (origin, asked) = answering(joel_hears_his_own_alert);
    let home = home();
    a_stop(&home, "standing", "everything", true, 120);
    let mut reading = reader(&home, &origin);
    let began = Instant::now();
    // Each round asks twice; three rounds give the echo every chance.
    while asked.lock().expect("what was asked").len() < 6 {
        assert!(began.elapsed() < PATIENCE, "the reader asked nothing");
        // As above, a short wait sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        resumed(&home, "standing"),
        Value::Null,
        "the echo resumes nothing"
    );
    assert!(
        decisions(&home).is_empty(),
        "the reader's own alert is no saying: {:?}",
        decisions(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn heard_says_when_slack_was_last_read_while_nothing_is_heard() {
    let home = home();
    let heard = || {
        let said = Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
            .arg("heard")
            .env("XDG_STATE_HOME", home.join("state"))
            .output()
            .expect("heard answers");
        String::from_utf8_lossy(&said.stdout).into_owned()
    };
    assert_eq!(
        heard(),
        "nothing heard yet, and Slack hasn't been read yet\n",
        "a mark with no read_at, as before a first round"
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    let judged = json!({"since": "1.000000", "keys": [], "read_at": now - 5000});
    std::fs::write(home.join("state/pineapple/judged.json"), judged.to_string())
        .expect("the mark written");
    let said = heard();
    assert!(
        said.starts_with("nothing heard yet; Slack was last read ") && said.ends_with(" s ago\n"),
        "says how long ago it read: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn the_switch_holds_replies_back_and_stops_carry_on() {
    let (origin, _) = stand_in();
    let home = home();
    std::fs::write(home.join("state/pineapple/replies-switch"), "off\n")
        .expect("the switch written");
    let mut reading = reader(&home, &origin);
    until_decided(&home, 4);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        said[1]["gate"], "held:switched-off",
        "the reply it would have sent is held back"
    );
    assert_eq!(said[1]["verdict"], "stop", "and it still stops");
    assert!(
        !home.join("state/pineapple/replies.jsonl").exists(),
        "no reply is kept as sent"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_for_joels_phone_finds_his_own_im_and_in_shadow_sends_nothing() {
    let (origin, asked) = stand_in();
    let home = home();
    let at = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_millis(),
    )
    .expect("a time in range");
    let alert = json!({"at": at, "key": "pineapple:slack:C1:1.0", "saying": "stopped",
        "shadow": false, "title": "Someone stopped your agents on quest SAFEQA",
        "words": "pineapple override", "link": "https://example.slack.com/archives/C1/p1"});
    let stale = json!({"at": at - 3_600_000, "key": "pineapple:slack:C2:1.0",
        "saying": "stopped", "shadow": false, "title": "an hour ago", "words": "",
        "link": ""});
    std::fs::write(
        home.join("state/pineapple/would-tell.jsonl"),
        format!("{stale}\n{alert}\n"),
    )
    .expect("what the launcher's daemon would tell");
    let phoned = home.join("state/pineapple/phoned.jsonl");
    let mut reading = reader(&home, &origin);
    until_decided(&home, 4);
    let began = Instant::now();
    while !phoned.exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "Joel's phone was never written down"
        );
        // The reader goes round every tenth of a second.
        thread::sleep(Duration::from_millis(50));
    }
    // Another round, to be sure the alert is told only once.
    thread::sleep(Duration::from_millis(300));
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let kept = std::fs::read_to_string(&phoned).expect("what Joel's phone was told");
    let lines: Vec<Value> = kept
        .lines()
        .map(|line| serde_json::from_str(line).expect("a line is JSON"))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "the fresh alert once, not the hour-old one: {kept}"
    );
    assert_eq!(
        (&lines[0]["im"], &lines[0]["shadow"], &lines[0]["sent"]),
        (&json!("D0SELF0001"), &json!(true), &json!(false)),
        "Joel's own IM is found, and in shadow nothing is sent"
    );
    assert_eq!(
        lines[0]["text"],
        "pineapple: Someone stopped your agents on quest SAFEQA\n\
         Said: \"pineapple override\"\n\
         Where: https://example.slack.com/archives/C1/p1",
        "in the alert's one form"
    );
    let methods = asked.lock().expect("the stand-in's list").clone();
    assert!(
        !methods.iter().any(|line| line.contains("chat.postMessage")),
        "nothing is posted in shadow: {methods:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
