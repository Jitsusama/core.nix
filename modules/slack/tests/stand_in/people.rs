//! slack-people: who each address is, found once and kept across a
//! restart, how they are asked each time, their conversation with Joel
//! found ahead, and nobody asked once Slack refuses the sign-in.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::{Heard, StandIn, WORDS, cache, credentials, forget};

/// Runs the program over the stand-in, given these lines, and says what
/// it said back, a line each. What it keeps goes in a cache home beside
/// the sign-in's file, so a second run with the same file finds it.
fn told(stand_in: &StandIn, credentials: &PathBuf, lines: &str) -> Vec<String> {
    said_to(started(&stand_in.origin, credentials), lines)
}

/// What the program said back to these lines, a line each.
fn said_to(mut program: Child, lines: &str) -> Vec<String> {
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

/// The program, started over Slack at `origin` with this sign-in, and
/// without the launcher's words, so it says who they are alone, as each
/// case but one hears it.
fn started(origin: &str, credentials: &PathBuf) -> Child {
    program(origin, credentials).spawn().unwrap()
}

/// The program to run over Slack at `origin` with this sign-in, and only
/// that one: never one the keyring of whoever runs the tests would give,
/// which would reach the stand-in and be printed by any assert of what
/// it heard.
fn program(origin: &str, credentials: &PathBuf) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_slack-people"));
    command
        .arg(credentials)
        .arg(origin)
        .env("XDG_CACHE_HOME", cache(credentials))
        .env_remove("SIGN_IN_KEYRING")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
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
                || header == "Authorization: Bearer xoxc-mine"),
        "the sign-in's token should go as its bearer, in {:?}",
        heard[0].headers
    );
    assert!(
        heard[0]
            .headers
            .iter()
            .any(|header| header.ends_with("d=d-cookie")),
        "the sign-in's cookie should go with it, in {:?}",
        heard[0].headers
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
            .any(|heard| heard.line.contains("in%3A%40lin") || heard.line.contains("in:@lin")),
        "Lin's conversation should be searched for, in {:?}",
        first.heard()
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

/// How many asks `pausing` has answered.
static PAUSING_ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Slack as `slack` has it, but over its limit at the first ask, saying
/// to wait two seconds, as a grid does after a restart's asks ahead.
fn pausing(line: &str) -> String {
    if PAUSING_ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 2".into()
    } else {
        slack(line)
    }
}

#[test]
fn asked_to_wait_it_asks_nothing_until_then_and_drops_nobody_ahead() {
    let stand_in = StandIn::new(pausing);
    let path = credentials("pausing");
    let said = told(&stand_in, &path, "?ada@example.com\n?grace@example.com\n");
    // Ada, whose ask Slack refused, is asked again once the wait is over.
    assert!(said.contains(&ADA_UNSAID.to_owned()), "{said:?}");
    assert!(said.contains(&GRACE_WHO.to_owned()), "{said:?}");
    let heard = stand_in.heard();
    let waited = heard[1].at.duration_since(heard[0].at);
    assert!(
        waited >= Duration::from_millis(1900),
        "asked again after {waited:?}"
    );
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
fn handed_the_launchers_words_it_says_their_facts_and_how_to_write_to_them() {
    let stand_in = StandIn::new(slack);
    let path = credentials("words");
    // The launcher's words, as the people daemon's unit hands them. Ada
    // asked again says her status, which only Slack's profile of her
    // holds. Nobody, asked ahead once stepped into, as the daemon's watch
    // of a step-in asks, still says Slack knows no one there.
    let said = said_to(
        program(&stand_in.origin, &path)
            .env("LAUNCHER_WORDS", WORDS)
            .spawn()
            .unwrap(),
        "ada@example.com\nada@example.com\nnobody@example.com\n?nobody@example.com\n",
    );
    let said: Vec<serde_json::Value> = said
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let of = |address: &str| -> Vec<&serde_json::Value> {
        said.iter()
            .filter(|line| line["email"] == address)
            .collect()
    };
    // Stepped into before anything is known of her, Write in Slack's place
    // is held at once while Slack is asked, and so is nobody's.
    let looking = serde_json::json!([{"key": "verb:person:slack", "side": "looking in Slack",
        "gone": true, "main": true}]);
    assert_eq!(
        (
            &of("ada@example.com")[0]["acts"],
            &of("ada@example.com")[0]["known"]
        ),
        (&looking, &serde_json::json!(false))
    );
    assert_eq!(of("nobody@example.com")[0]["acts"], looking);
    let ada = [
        *of("ada@example.com").last().unwrap(),
        *of("nobody@example.com").last().unwrap(),
    ];
    assert_eq!(
        (&ada[0]["facts"], &ada[0]["acts"]),
        (
            &serde_json::json!([
                {"name": "Status", "said": "📝 Writing"},
                {"name": "Presence", "said": "Active"},
                {"name": "Time Zone", "zone": "Europe/London", "as": "zone"}
            ]),
            &serde_json::json!([{"key": "verb:person:slack",
                "open": "slack://user?team=T1&id=U1", "main": true}])
        )
    );
    // Who she is stays as the daemon reads how she stands.
    assert_eq!(ada[0]["presence"], "active");
    // Nobody Slack knows has nothing to write to, Write in Slack's place
    // saying so where Joel stepped in, and a fact says so too.
    assert_eq!(
        (&ada[1]["facts"], &ada[1]["acts"], &ada[1]["known"]),
        (
            &serde_json::json!([{"name": "Slack", "said": "no one at this address"}]),
            &serde_json::json!([{"key": "verb:person:slack",
                "side": "no one at this address", "gone": true, "main": true}]),
            &serde_json::json!(false)
        )
    );
    // Lin, asked ahead, has her conversation with Joel between them as a
    // row ready for her Messages, with its messages in its drawer, waiting
    // on Joel's answer, which Reply in Slack gives first of her ways.
    let lin = said_to(
        program(&stand_in.origin, &path)
            .env("LAUNCHER_WORDS", WORDS)
            .spawn()
            .unwrap(),
        "?lin@example.com\n",
    );
    let lin: serde_json::Value = serde_json::from_str(&lin[0]).unwrap();
    assert_eq!(
        lin["between"],
        serde_json::json!([{
            "at": 1_730_312_345_000_u64,
            "key": "slack:conversation:D4",
            "kind": "wrote",
            "mine": false,
            "what": "Slack",
            "waits": {},
            "row": {
                "name": "See you there & then",
                "glyph": "chat",
                "hue": "program",
                "side": [{"said": "Slack"}, {"at": 1_730_312_345_000_u64, "as": "ago"}],
                "acts": [
                    {"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D4"},
                    {"key": "verb:slack:link", "copy": "https://slack.example/archives/D4/p2"}
                ],
                "drawer": {
                    "title": "Lin Chen",
                    "facts": [],
                    "body": "You: Lunch, @Lin Chen?\n\nLin Chen: See you there & then"
                }
            }
        }])
    );
    assert_eq!(
        lin["acts"],
        serde_json::json!([{"key": "verb:person:slack:reply",
            "open": "slack://channel?team=T1&id=D4", "main": true}])
    );
    forget(&path);
}

#[test]
fn with_no_sign_in_it_says_so_and_slack_is_never_asked() {
    let stand_in = StandIn::new(slack);
    let missing = std::env::temp_dir().join("slack-people-no-such-file.json");
    let said = told(&stand_in, &missing, "ada@example.com\n?grace@example.com\n");
    assert_eq!(said, [r#"{"signedIn":false}"#; 2]);
    let heard = stand_in.heard();
    assert!(
        heard.is_empty(),
        "with no sign-in, Slack shouldn't be asked: {heard:?}"
    );
}
