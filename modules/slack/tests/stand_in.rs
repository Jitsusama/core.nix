//! The program against a stand-in for Slack on this machine: what it asks,
//! with which sign-in, what it tells the shell, that it finds who each
//! address is once, keeping it across a restart, and asks how they are
//! each time, that asking ahead never holds up a step-in, and that it
//! asks nobody once Slack refuses the sign-in.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// One request the stand-in heard: its first line and its headers.
#[derive(Clone, Debug)]
struct Heard {
    line: String,
    headers: Vec<String>,
}

/// A stand-in for Slack's API, answering each request's first line with
/// what `answer` gives.
struct StandIn {
    origin: String,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl StandIn {
    fn new(answer: fn(&str) -> String) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut headers = Vec::new();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end().to_owned();
                    if header.is_empty() {
                        break;
                    }
                    if let Some(size) = header.to_lowercase().strip_prefix("content-length: ") {
                        length = size.parse().unwrap_or(0);
                    }
                    headers.push(header);
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let line = line.trim_end().to_owned();
                let said = answer(&line);
                kept.lock().unwrap().push(Heard { line, headers });
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                    said.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        StandIn { origin, heard }
    }

    fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }
}

/// A sign-in as the harness keeps it, in a file of the test's own.
fn credentials(test: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("slack-people-{test}-{}.json", std::process::id()));
    std::fs::write(
        &path,
        r#"{"token": {"accessToken": "xoxc-mine", "cookie": "d-cookie", "userId": "U0",
            "teamId": "T1", "teamName": "Team", "scopes": []}}"#,
    )
    .unwrap();
    path
}

/// Runs the program over the stand-in, given these lines, and says what
/// it said back, a line each. What it keeps goes in a cache home beside
/// the sign-in's file, so a second run with the same file finds it.
fn told(stand_in: &StandIn, credentials: &PathBuf, lines: &str) -> Vec<String> {
    let mut program = started(&stand_in.origin, credentials);
    program
        .stdin
        .take()
        .unwrap()
        .write_all(lines.as_bytes())
        .unwrap();
    let out = program.wait_with_output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The program, started over Slack at `origin` with this sign-in.
fn started(origin: &str, credentials: &PathBuf) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_slack-people"))
        .arg(credentials)
        .arg(origin)
        .env("XDG_CACHE_HOME", cache(credentials))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

/// The cache home a run with this sign-in's file keeps in.
fn cache(credentials: &std::path::Path) -> PathBuf {
    credentials.with_extension("cache")
}

/// Forgets a test's sign-in and what its runs kept.
fn forget(credentials: &std::path::Path) {
    let _ = std::fs::remove_file(credentials);
    let _ = std::fs::remove_dir_all(cache(credentials));
}

/// Ada as the program says her, her zone with her, active.
const ADA: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"active","status":"","emoji":"","handle":""}"#
);

/// Ada asked about again, once she's set a status.
const ADA_WRITING: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"active","status":"Writing","emoji":"📝","handle":""}"#
);

/// Ada while Slack can't say whether she's active or away.
const ADA_UNSAID: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"","status":"","emoji":"","handle":""}"#
);

/// Grace as the program says her, found by a search, as an enterprise
/// grid has it, and away.
const GRACE: &str = concat!(
    r#"{"email":"grace@example.com","id":"U2","team":"T1","name":"Grace Hopper","#,
    r#""zone":"America/New_York","presence":"away","status":"","emoji":"","handle":""}"#
);

/// Grace found ahead, before anyone asked how she is.
const GRACE_WHO: &str = concat!(
    r#"{"email":"grace@example.com","id":"U2","team":"T1","name":"Grace Hopper","#,
    r#""zone":"America/New_York","presence":"","status":"","emoji":"","handle":""}"#
);

/// Someone Slack knows nobody by, as the program says them.
fn nobody(email: &str) -> String {
    format!(r#"{{"email":"{email}","id":null,"team":"T1","name":"","zone":"","presence":"","#)
        + r#""status":"","emoji":"","handle":""}"#
}

/// Every method the program may ask, each only reading.
const READS: [&str; 4] = [
    "users.lookupByEmail?",
    "search.messages?",
    "users.info?",
    "users.getPresence?",
];

/// Whether everything the stand-in heard was a GET of one of the reads.
fn only_read(heard: &[Heard]) -> bool {
    heard.iter().all(|heard| {
        let method = heard.line.trim_start_matches("GET /api/");
        heard.line.starts_with("GET /api/") && READS.iter().any(|read| method.starts_with(read))
    })
}

/// Slack as a workspace that looks Ada up by her address, and as an
/// enterprise grid for Grace and Eve, refusing that to the sign-in: a
/// search finds each by their handle, and only Grace's profile keeps the
/// address asked about.
fn slack(line: &str) -> String {
    if line.contains("email=lin%40example.com") || line.contains("users.info?user=U4") {
        r#"{"ok": true, "user": {"id": "U4", "team_id": "T1", "name": "lin",
            "profile": {"real_name": "Lin Chen"}}}"#
            .into()
    } else if line.contains("search.messages") && line.contains("lin") {
        // Lin's direct messages with Joel, U0, newest first, and one of
        // hers elsewhere that isn't theirs.
        concat!(
            r#"{"ok": true, "messages": {"matches": ["#,
            r#"{"channel": {"id": "D4", "is_im": true, "name": "U4"}, "user": "U4","#,
            r#""text": "See you there &amp; then", "ts": "1730312345.000100","#,
            r#""permalink": "https://slack.example/archives/D4/p2"},"#,
            r#"{"channel": {"id": "C1", "is_im": false, "name": "general"}, "user": "U4","#,
            r#""text": "Hello all", "ts": "1730312320.000000"},"#,
            r#"{"channel": {"id": "D4", "is_im": true, "name": "U4"}, "user": "U0","#,
            r#""text": "Lunch, <@U4>?", "ts": "1730312300.000000"}]}}"#
        )
        .into()
    } else if line.contains("email=ada%40example.com") {
        r#"{"ok": true, "user": {"id": "U1", "team_id": "T1", "tz": "Europe/London",
            "profile": {"real_name": "Ada Lovelace"}}}"#
            .into()
    } else if line.contains("email=grace%40example.com") || line.contains("email=eve%40example.com")
    {
        r#"{"ok": false, "error": "not_allowed_token_type"}"#.into()
    } else if line.contains("search.messages") && line.contains("grace") {
        r#"{"ok": true, "messages": {"matches": [{"user": "U2", "username": "grace"}]}}"#.into()
    } else if line.contains("search.messages") && line.contains("eve") {
        r#"{"ok": true, "messages": {"matches": [{"user": "U3", "username": "eve"}]}}"#.into()
    } else if line.contains("users.info") && line.contains("user=U2") {
        r#"{"ok": true, "user": {"id": "U2", "team_id": "T1", "tz": "America/New_York",
            "profile": {"real_name": "Grace Hopper", "email": "Grace@Example.com"}}}"#
            .into()
    } else if line.contains("users.info") && line.contains("user=U1") {
        r#"{"ok": true, "user": {"id": "U1", "team_id": "T1", "tz": "Europe/London",
            "profile": {"real_name": "Ada Lovelace", "status_text": "Writing",
                "status_emoji_display_info": [{"emoji_name": "memo", "unicode": "1f4dd"}]}}}"#
            .into()
    } else if line.contains("users.getPresence") && line.contains("user=U1") {
        r#"{"ok": true, "presence": "active"}"#.into()
    } else if line.contains("users.getPresence") && line.contains("user=U2") {
        r#"{"ok": true, "presence": "away"}"#.into()
    } else if line.contains("users.info") && line.contains("user=U3") {
        r#"{"ok": true, "user": {"id": "U3", "team_id": "T1",
            "profile": {"real_name": "Another Eve", "email": "eve@elsewhere.example"}}}"#
            .into()
    } else {
        r#"{"ok": false, "error": "users_not_found"}"#.into()
    }
}

/// Slack as `slack` has it, but too busy to say who anyone is again or
/// whether they're active.
fn busy(line: &str) -> String {
    if line.contains("users.info") || line.contains("users.getPresence") {
        r#"{"ok": false, "error": "ratelimited"}"#.into()
    } else {
        slack(line)
    }
}

#[test]
fn who_an_address_is_is_found_once_and_how_they_are_asked_each_time() {
    let stand_in = StandIn::new(slack);
    let path = credentials("found");
    let said = told(
        &stand_in,
        &path,
        "Ada@Example.com\nnobody@example.com\nnot an address\nada@example.com\n\
         nobody@example.com\n",
    );
    let nobody = nobody("nobody@example.com");
    // Ada asked again is said at once as she was, then as she is now.
    assert_eq!(said, [ADA, &nobody, ADA, ADA_WRITING, &nobody]);
    let heard = stand_in.heard();
    let asked: Vec<&str> = heard
        .iter()
        .map(|heard| heard.line.split('?').next().unwrap_or_default())
        .collect();
    assert_eq!(
        asked,
        [
            "GET /api/users.lookupByEmail",
            "GET /api/users.getPresence",
            "GET /api/users.lookupByEmail",
            "GET /api/users.info",
            "GET /api/users.getPresence",
        ]
    );
    assert!(
        heard[0].line.starts_with("GET /api/users.lookupByEmail?"),
        "{}",
        heard[0].line
    );
    assert!(
        heard[0]
            .headers
            .iter()
            .any(|header| header == "authorization: Bearer xoxc-mine"
                || header == "Authorization: Bearer xoxc-mine")
    );
    assert!(
        heard[0]
            .headers
            .iter()
            .any(|header| header.ends_with("d=d-cookie"))
    );
    // Only ever reading: nothing is posted, nothing sent.
    assert!(only_read(&heard), "{heard:?}");
    forget(&path);
}

#[test]
fn who_was_found_is_said_at_once_after_a_restart_then_how_they_are() {
    let first = StandIn::new(slack);
    let path = credentials("kept");
    told(&first, &path, "ada@example.com\nnobody@example.com\n");
    let again = StandIn::new(slack);
    let said = told(&again, &path, "ada@example.com\nnobody@example.com\n");
    assert_eq!(
        said,
        [ADA_UNSAID, ADA_WRITING, &nobody("nobody@example.com")]
    );
    let asked: Vec<String> = again
        .heard()
        .iter()
        .map(|heard| heard.line.split('?').next().unwrap_or_default().to_owned())
        .collect();
    // Nobody is looked up by address again; Ada is only asked how she is.
    assert_eq!(asked, ["GET /api/users.info", "GET /api/users.getPresence"]);
    forget(&path);
}

#[test]
fn their_conversation_with_joel_is_found_ahead_once_and_again_at_each_step_in() {
    let first = StandIn::new(slack);
    let path = credentials("talk");
    let ahead = told(&first, &path, "?lin@example.com\n?lin@example.com\n");
    let asked = |stand_in: &StandIn| -> Vec<String> {
        stand_in
            .heard()
            .iter()
            .map(|heard| heard.line.split('?').next().unwrap_or_default().to_owned())
            .collect()
    };
    // Asked ahead twice, found once with what they said, never how they are.
    assert_eq!(
        asked(&first),
        ["GET /api/users.lookupByEmail", "GET /api/search.messages"]
    );
    let said: Vec<serde_json::Value> = ahead
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(said.len(), 1, "{ahead:?}");
    assert_eq!(said[0]["handle"], "lin");
    assert_eq!(
        said[0]["conversation"],
        serde_json::json!({
            "channel": "D4",
            "link": "https://slack.example/archives/D4/p2",
            "last": "See you there & then",
            "at": 1_730_312_345_000_u64,
            "messages": [
                { "mine": true, "text": "Lunch, @Lin Chen?", "at": 1_730_312_300_000_u64 },
                { "mine": false, "text": "See you there & then", "at": 1_730_312_345_000_u64 }
            ]
        })
    );
    assert!(
        first
            .heard()
            .iter()
            .any(|heard| heard.line.contains("in%3A%40lin") || heard.line.contains("in:@lin"))
    );
    // After a restart, who she is is said at once, kept without what she
    // said, then with how she is and what she said, asked again.
    let again = StandIn::new(slack);
    let stepped = told(&again, &path, "lin@example.com\n");
    assert_eq!(
        asked(&again),
        [
            "GET /api/users.info",
            "GET /api/users.getPresence",
            "GET /api/search.messages"
        ]
    );
    assert_eq!(stepped.len(), 2, "{stepped:?}");
    assert!(!stepped[0].contains("conversation"), "{}", stepped[0]);
    assert!(stepped[1].contains(r#""channel":"D4""#), "{}", stepped[1]);
    // Asked ahead after another restart, who she is is said at once too,
    // then what she said, which is all Slack is asked.
    let third = StandIn::new(slack);
    let ahead = told(&third, &path, "?lin@example.com\n");
    assert_eq!(asked(&third), ["GET /api/search.messages"]);
    assert_eq!(ahead.len(), 2, "{ahead:?}");
    assert!(!ahead[0].contains("conversation"), "{}", ahead[0]);
    assert!(ahead[1].contains(r#""channel":"D4""#), "{}", ahead[1]);
    forget(&path);
}

#[test]
fn who_was_kept_is_said_before_slack_answers() {
    let first = StandIn::new(slack);
    let path = credentials("before");
    told(&first, &path, "ada@example.com\n");
    // A Slack that takes each request and never answers it.
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut program = started(&format!("http://{}", silent.local_addr().unwrap()), &path);
    let mut asking = program.stdin.take().unwrap();
    writeln!(asking, "ada@example.com").unwrap();
    let mut said = String::new();
    BufReader::new(program.stdout.take().unwrap())
        .read_line(&mut said)
        .unwrap();
    program.kill().unwrap();
    program.wait().unwrap();
    assert_eq!(said.trim_end(), ADA_UNSAID);
    forget(&path);
}

#[test]
fn asking_ahead_finds_who_they_are_after_any_step_in_and_not_how_they_are() {
    let stand_in = StandIn::new(slack);
    let path = credentials("ahead");
    let said = told(
        &stand_in,
        &path,
        "?grace@example.com\n?grace@example.com\n?not an address\nada@example.com\n",
    );
    assert_eq!(said, [ADA, GRACE_WHO]);
    let heard = stand_in.heard();
    assert!(
        heard[0].line.contains("email=ada%40example.com"),
        "{heard:?}"
    );
    // Grace, asked ahead twice, is found once and never asked how she is.
    assert_eq!(
        heard
            .iter()
            .filter(|heard| heard.line.contains("email=grace%40example.com"))
            .count(),
        1
    );
    assert!(
        !heard
            .iter()
            .any(|heard| heard.line.contains("getPresence") && heard.line.contains("U2")),
        "{heard:?}"
    );
    assert!(only_read(&heard), "{heard:?}");
    forget(&path);
}

#[test]
fn slack_too_busy_to_say_how_they_are_leaves_who_they_are_said() {
    let stand_in = StandIn::new(busy);
    let path = credentials("busy");
    let said = told(&stand_in, &path, "ada@example.com\nada@example.com\n");
    assert_eq!(said, [ADA_UNSAID, ADA_UNSAID]);
    forget(&path);
}

#[test]
fn a_grid_refusing_the_look_up_finds_them_by_a_search_their_profile_confirms() {
    let stand_in = StandIn::new(slack);
    let path = credentials("grid");
    let said = told(&stand_in, &path, "grace@example.com\neve@example.com\n");
    assert_eq!(said, [GRACE.to_owned(), nobody("eve@example.com")]);
    let heard = stand_in.heard();
    assert!(
        heard
            .iter()
            .any(|heard| heard.line.contains("query=from%3A%40grace")),
        "{heard:?}"
    );
    // Still only reading: a look-up, a search, a profile and presence.
    assert!(only_read(&heard), "{heard:?}");
    forget(&path);
}

#[test]
fn a_refused_sign_in_is_said_once_and_nobody_more_is_asked_about() {
    let stand_in = StandIn::new(|_| r#"{"ok": false, "error": "invalid_auth"}"#.into());
    let path = credentials("refused");
    let said = told(&stand_in, &path, "ada@example.com\ngrace@example.com\n");
    assert_eq!(said, [r#"{"signedOut":true}"#]);
    assert_eq!(stand_in.heard().len(), 1);
    forget(&path);
}

#[test]
fn with_no_sign_in_it_says_so_and_slack_is_never_asked() {
    let stand_in = StandIn::new(slack);
    let missing = std::env::temp_dir().join("slack-people-no-such-file.json");
    let said = told(&stand_in, &missing, "ada@example.com\n?grace@example.com\n");
    assert_eq!(said, [r#"{"signedIn":false}"#; 2]);
    assert!(stand_in.heard().is_empty());
}

/// Slack as it answers what wants Joel: Ada's direct message with two
/// unread, a channel with one unread and a mention of him, another unread
/// with none, and three mentions in threads.
fn wanting(line: &str) -> String {
    let said = if line.contains("client.counts") {
        concat!(
            r#"{"ok": true, "ims": [{"id": "D1", "has_unreads": true, "mention_count": 2}],"#,
            r#""mpims": [], "channels": [{"id": "C2", "has_unreads": true, "mention_count": 1},"#,
            r#"{"id": "C3", "has_unreads": true, "mention_count": 0}],"#,
            r#""threads": {"has_unreads": true, "mention_count": 3}}"#
        )
    } else if line.contains("conversations.info?channel=D1") {
        r#"{"ok": true, "channel": {"id": "D1", "is_im": true, "user": "U7"}}"#
    } else if line.contains("conversations.info?channel=C2") {
        r#"{"ok": true, "channel": {"id": "C2", "is_channel": true, "name": "launcher"}}"#
    } else if line.contains("users.info?user=U7") {
        r#"{"ok": true, "user": {"id": "U7", "profile": {"real_name": "Ada Lovelace"}}}"#
    } else {
        r#"{"ok": false, "error": "unknown_method"}"#
    };
    said.into()
}

/// slack-unreads, started over the stand-in with this sign-in, asking
/// every 50 ms and keeping what it reads in `file`.
fn unreads(stand_in: &StandIn, credentials: &std::path::Path, file: &std::path::Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_slack-unreads"))
        .arg(credentials)
        .arg(file)
        .arg(&stand_in.origin)
        .arg("50")
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

/// Waits until `held` holds, failing with `what` after ten seconds.
fn until(what: &str, held: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !held() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(20));
    }
}

/// What the file says, once it says anything whole.
fn kept(file: &std::path::Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

#[test]
fn what_wants_joel_is_kept_whole_in_a_file_each_conversation_named_once() {
    let stand_in = StandIn::new(wanting);
    let path = credentials("unreads");
    let file = path.with_extension("unreads.json");
    let mut program = unreads(&stand_in, &path, &file);
    until("the file never said anything", || kept(&file).is_some());
    let said = kept(&file).unwrap();
    assert_eq!(said["state"], "ok");
    assert_eq!(
        said["conversations"],
        serde_json::json!([
            {"id": "D1", "name": "Ada Lovelace", "kind": "dm", "count": 2,
             "link": "slack://channel?team=T1&id=D1"},
            {"id": "C2", "name": "#launcher", "kind": "mention", "count": 1,
             "link": "slack://channel?team=T1&id=C2"},
            {"id": "threads", "name": "Threads", "kind": "thread", "count": 3,
             "link": "slack://open?team=T1"}
        ])
    );
    // Asked again and again, each conversation is named only once a run.
    let asked = |method: &str| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains(method))
            .count()
    };
    until("Slack wasn't asked again", || asked("client.counts") >= 4);
    program.kill().unwrap();
    program.wait().unwrap();
    assert_eq!((asked("conversations.info"), asked("users.info")), (2, 1));
    let heard = stand_in.heard();
    assert!(
        heard.iter().all(|heard| heard.line.starts_with("GET /api/")
            && heard
                .headers
                .iter()
                .any(|header| header == "authorization: Bearer xoxc-mine"
                    || header == "Authorization: Bearer xoxc-mine")),
        "{heard:?}"
    );
    let _ = std::fs::remove_file(&file);
    forget(&path);
}

#[test]
fn a_refused_sign_in_is_kept_as_signed_out_with_nothing_counted() {
    let stand_in = StandIn::new(|_| r#"{"ok": false, "error": "invalid_auth"}"#.into());
    let path = credentials("unreads-refused");
    let file = path.with_extension("unreads.json");
    let mut program = unreads(&stand_in, &path, &file);
    until("the file never said anything", || kept(&file).is_some());
    program.kill().unwrap();
    program.wait().unwrap();
    let said = kept(&file).unwrap();
    assert_eq!(
        (&said["state"], &said["conversations"]),
        (&serde_json::json!("signedOut"), &serde_json::json!([]))
    );
    let _ = std::fs::remove_file(&file);
    forget(&path);
}

#[test]
fn with_no_sign_in_nothing_is_kept_and_slack_is_never_asked() {
    let stand_in = StandIn::new(wanting);
    let missing = std::env::temp_dir().join("slack-unreads-no-such-file.json");
    let file = std::env::temp_dir().join(format!("slack-unreads-none-{}.json", std::process::id()));
    let mut program = unreads(&stand_in, &missing, &file);
    // Long enough for several rounds of asking.
    thread::sleep(Duration::from_millis(300));
    program.kill().unwrap();
    program.wait().unwrap();
    assert!(!file.exists());
    assert!(stand_in.heard().is_empty());
}
