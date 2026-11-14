//! The daemon over its standard input and output, as the shell runs it,
//! with stand-in password managers of its own: a list holds every entry
//! without a secret, even one a manager gives with its secrets left on; a
//! get reads an entry in full, still without them; a held list or get asks
//! no manager anything; a secret goes only to the ask for it, never to a
//! watch, or to the clipboard, cleared when its time is up, and no copy is
//! made while no copier is named; the daemon can't be read through /proc
//! by the account's other processes, and ends once whatever ran it closes
//! its end; a one-time code says its period, and its detail gives
//! the code, never its seed; a manager signed out is said so and refuses
//! a secret; a watch hears entries arrive; a watch of rows hears each
//! entry's ready row, worded and without a secret, apart from the entries
//! a watch of them hears, and nothing again while no row changed; an ask
//! or kind it doesn't
//! know is refused; and an audit, asked by Joel alone, finds the entries
//! sharing a password and those Have I Been Pwned's stand-in has seen in a
//! breach, sending it each hash's five characters alone, and saying no
//! secret, nor any hash, in an answer or a log. What an audit found is
//! kept in the state folder, readable by Joel alone and holding no secret
//! nor any hash, and dresses the entries again once the daemon starts
//! again; and since when trouble's been found holds while it does, and
//! clears once a check finds none, each told to a watch. Secret asks the
//! daemon answers are its own module's (asks/mod.rs).

mod asks;
mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::{AUDITING, Breaches, KEEPING, Running, SIGNED_OUT, until};
use passwords::audit::hash;
use serde_json::{Value, json};

fn keys(answer: &Value) -> Vec<String> {
    answer["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["key"].as_str().unwrap().to_owned())
        .collect()
}

fn holds_no_secret(said: &Value) {
    let said = said.to_string();
    for secret in ["hunter2", "SEED"] {
        assert!(!said.contains(secret), "{secret} was held: {said}");
    }
}

#[test]
fn a_list_holds_every_entry_and_no_secret() {
    let running = Running::with("list", &[("keeping", KEEPING)]);
    let mut client = running.client();
    until("the manager answers", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "answering"
    });
    let hello = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(hello["domain"], "passwords");
    assert_eq!(hello["version"], 1);
    let listed = client.ask(&json!({"id": 2, "ask": "list", "asked": true}));
    assert_eq!(
        keys(&listed),
        [
            "entry:keeping:gh",
            "entry:keeping:card",
            "entry:keeping:note"
        ]
    );
    assert_eq!(listed["records"][0]["username"], "ada");
    let got =
        client.ask(&json!({"id": 3, "ask": "get", "keys": ["entry:keeping:gh"], "asked": true}));
    let details = got["records"][0]["details"].as_array().unwrap();
    assert_eq!(details.len(), 3, "{got}");
    assert_eq!(details[0]["value"], "ada");
    assert_eq!(details[1]["hidden"], true);
    holds_no_secret(&got);
    holds_no_secret(&client.ask(&json!({"id": 4, "ask": "list"})));
    // Once read, the list was asked of the manager just once.
    let log = running.asked_of("keeping");
    assert_eq!(log.matches("\"list\"").count(), 1, "{log}");
    assert!(
        log.contains("\"asked\":true"),
        "Joel's key wasn't passed on: {log}"
    );
}

#[test]
fn a_held_list_or_get_asks_no_manager_anything() {
    let running = Running::with("held", &[("keeping", KEEPING)]);
    let mut client = running.client();
    until("the manager answers", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "answering"
    });
    // Nothing's been read, so nothing's held, and the manager isn't asked.
    let held = client.ask(&json!({"id": 1, "ask": "list", "held": true}));
    assert_eq!(held["records"], json!([]), "{held}");
    let got = client.ask(&json!({"id": 2, "ask": "get", "keys": ["entry:keeping:gh"],
                                 "held": true}));
    assert_eq!(got["records"], json!([]), "{got}");
    assert_eq!(running.asked_of("keeping"), "");
    client.ask(&json!({"id": 3, "ask": "list", "asked": true}));
    // Held in summary, it's answered so, and still nothing more is asked.
    let got = client.ask(&json!({"id": 4, "ask": "get", "keys": ["entry:keeping:gh"],
                                 "held": true}));
    assert_eq!(got["records"][0]["title"], "GitHub");
    assert!(got["records"][0].get("details").is_none(), "{got}");
    let held = client.ask(&json!({"id": 5, "ask": "list", "held": true, "fresh": true}));
    assert_eq!(keys(&held).len(), 3);
    let log = running.asked_of("keeping");
    assert_eq!(log.lines().count(), 1, "{log}");
}

#[test]
fn a_secret_goes_only_to_the_ask_for_it() {
    let running = Running::with("secret", &[("keeping", KEEPING)]);
    let mut watching = running.client();
    watching.say(&json!({"id": "w", "ask": "watch"}));
    watching.hear();
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let shown = client.ask(&json!({"id": 2, "ask": "reveal", "key": "entry:keeping:gh"}));
    assert_eq!(shown["value"], "hunter2");
    assert_eq!(shown["detail"], "password");
    holds_no_secret(&client.ask(&json!({"id": 3, "ask": "get", "keys": ["entry:keeping:gh"]})));
    holds_no_secret(&client.ask(&json!({"id": 4, "ask": "search", "words": "git"})));
    let health = client.ask(&json!({"id": 5, "ask": "health"}));
    holds_no_secret(&health);
    assert_eq!(health["entries"], 3);
    // Whatever else was said, the watch's lines among it, holds none.
    for heard in watching.unheard() {
        holds_no_secret(&heard);
    }
}

#[test]
fn no_copy_is_made_while_no_copier_is_named() {
    let running = Running::with_env(
        "uncopied",
        &[("keeping", KEEPING)],
        &[("PASSWORDS_COPIER", "")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("browser extension")),
        "a refused copy didn't say what to use instead: {refused}"
    );
    holds_no_secret(&refused);
    // Refused before any secret was read: the manager was asked no reveal.
    let asked = running.asked_of("keeping");
    assert!(
        !asked.contains("reveal"),
        "a secret was read to copy: {asked}"
    );
    assert!(!running.copied().exists(), "something was copied");
}

#[test]
fn the_account_cant_read_the_daemon_through_proc() {
    let running = Running::with("undumpable", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    let proc = PathBuf::from(format!("/proc/{}", running.pid()));
    // Its dumpable flag off, its own files are root's, which the account
    // running it isn't, so it can read neither its environment nor its
    // memory, and can't open its descriptors.
    for each in ["environ", "mem", "fd/0"] {
        let read = std::fs::File::open(proc.join(each));
        assert!(
            read.is_err(),
            "/proc's {each} of the daemon could be opened"
        );
    }
    let limits = std::fs::read_to_string(proc.join("limits")).unwrap_or_default();
    assert!(
        limits
            .lines()
            .any(|line| line.starts_with("Max core file size")
                && line.split_whitespace().nth(4) == Some("0")),
        "the daemon could write a core dump: {limits}"
    );
}

#[test]
fn the_daemon_ends_once_whatever_ran_it_closes_its_end() {
    let mut running = Running::with("ends", &[("keeping", KEEPING)]);
    running.client().ask(&json!({"id": 1, "ask": "hello"}));
    running.close();
    let started = std::time::Instant::now();
    while !running.ended() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(20),
            "the daemon outlived its channel"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn a_copy_is_marked_sensitive_and_cleared_when_its_time_is_up() {
    let running = Running::with("copy", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    holds_no_secret(&copied);
    // The copier writes what it's given, then its flags, so both are
    // waited for: reading the flags once the copy is there races it.
    let flagged = running.copied().with_extension("flags");
    until("the password is copied with its flags", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "hunter2")
            && std::fs::read_to_string(&flagged).is_ok_and(|flags| flags.ends_with('\n'))
    });
    let flags = std::fs::read_to_string(&flagged).unwrap();
    assert!(
        flags.contains("--sensitive") && flags.contains("--foreground"),
        "{flags}"
    );
    // The copier is ended once its time is up, which clears the clipboard.
    until("the copy is cleared", || !copier_runs(&running.copied()));
}

/// Whether the stand-in copier still holds the clipboard.
fn copier_runs(copied: &Path) -> bool {
    let wanted = copied.to_string_lossy().into_owned();
    std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .any(|process| {
            std::fs::read(process.path().join("environ"))
                .is_ok_and(|environ| String::from_utf8_lossy(&environ).contains(&wanted))
                && std::fs::read_to_string(process.path().join("cmdline"))
                    .is_ok_and(|line| line.contains("copier"))
        })
}

#[test]
fn a_one_time_code_says_how_long_it_lasts() {
    let running = Running::with("code", &[("keeping", KEEPING)]);
    let mut client = running.client();
    let code = client.ask(&json!({"id": 1, "ask": "code", "key": "entry:keeping:gh"}));
    assert_eq!(code["code"], "123456");
    assert_eq!(code["period"], 30);
    assert_eq!(code["left"], 12);
}

#[test]
fn a_one_time_codes_detail_gives_its_code_never_its_seed() {
    let running = Running::with("code-detail", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh",
                                    "detail": "otp"}));
    assert_eq!(copied["copied"], true, "{copied}");
    assert_eq!(copied["detail"], "one-time password");
    // How long the code lasts goes with it, so the shell can say so.
    assert_eq!(copied["period"], 30, "{copied}");
    assert_eq!(copied["left"], 12, "{copied}");
    until("the code is copied", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "123456")
    });
    let shown = client.ask(&json!({"id": 3, "ask": "reveal", "key": "entry:keeping:gh",
                                   "detail": "otp"}));
    assert_eq!(shown["value"], "123456", "{shown}");
    // The seed was never asked of the manager, so it went nowhere.
    let log = std::fs::read_to_string(running.folder.join("keeping.log")).unwrap();
    assert!(!log.contains("\"reveal\""), "the seed was asked for: {log}");
}

#[test]
fn a_manager_signed_out_is_said_so_and_gives_no_secret() {
    let running = Running::with(
        "signed-out",
        &[("locked", SIGNED_OUT), ("keeping", KEEPING)],
    );
    let mut client = running.client();
    until("both managers have said how they stand", || {
        let hello = running.client().ask(&json!({"id": 0, "ask": "hello"}));
        hello["sources"]["locked"]["state"] == "signed-out"
            && hello["sources"]["keeping"]["state"] == "answering"
    });
    let listed = client.ask(&json!({"id": 1, "ask": "list"}));
    assert_eq!(listed["sources"]["locked"]["reason"], "Let the launcher in");
    assert_eq!(
        keys(&listed).len(),
        3,
        "the manager answering wasn't listed: {listed}"
    );
    let refused = client.ask(&json!({"id": 2, "ask": "reveal", "key": "entry:locked:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    let refused = client.ask(&json!({"id": 3, "ask": "code", "key": "entry:locked:gh"}));
    assert_eq!(refused["error"]["code"], "signed-out", "{refused}");
    let let_in = client.ask(&json!({"id": 4, "ask": "let-in", "manager": "locked"}));
    assert_eq!(let_in["error"]["code"], "signed-out", "{let_in}");
    let nobody = client.ask(&json!({"id": 5, "ask": "let-in", "manager": "nobody"}));
    assert_eq!(nobody["error"]["code"], "refused", "{nobody}");
}

#[test]
fn a_watch_hears_entries_arrive() {
    let running = Running::with("watch", &[("keeping", KEEPING)]);
    let mut watching = running.client();
    watching.say(&json!({"id": "w", "ask": "watch"}));
    let first = watching.hear();
    assert_eq!(first["id"], "w");
    assert!(
        first["records"].is_array(),
        "a watch's first answer isn't whole: {first}"
    );
    running
        .client()
        .ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let changed = loop {
        let heard = watching.hear();
        if heard["changed"]
            .as_array()
            .is_some_and(|changed| !changed.is_empty())
        {
            break heard;
        }
    };
    assert_eq!(changed["changed"].as_array().unwrap().len(), 3);
    holds_no_secret(&changed);
}

#[test]
fn a_watch_of_rows_hears_each_entrys_row_without_a_secret() {
    let running = Running::with("rows", &[("keeping", KEEPING)]);
    let mut watching = running.client();
    watching.say(&json!({"id": "r", "ask": "watch", "kind": "row"}));
    assert_eq!(
        watching.hear()["records"],
        json!([]),
        "rows came before any entry"
    );
    let mut asking = running.client();
    asking.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let heard = loop {
        let heard = watching.hear();
        if heard["changed"]
            .as_array()
            .is_some_and(|changed| !changed.is_empty())
        {
            break heard;
        }
    };
    let rows = heard["changed"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "not every entry was a row: {heard}");
    let github = rows
        .iter()
        .find(|row| row["key"] == "row:passwords:entry:keeping:gh")
        .unwrap_or_else(|| panic!("GitHub's entry had no row: {heard}"));
    assert_eq!(github["kind"], "row");
    assert_eq!(github["row"]["thing"], "entry:keeping:gh");
    let mut orders: Vec<i64> = rows
        .iter()
        .filter_map(|row| row["row"]["order"].as_i64())
        .collect();
    orders.sort_unstable();
    assert_eq!(
        orders,
        [1024, 2048, 3072],
        "the rows weren't spaced 1024 apart: {heard}"
    );
    assert_eq!(github["row"]["side"], json!([{"said": "ada"}]));
    assert!(
        rows.iter().all(|row| row["row"]["kind"] != "entry"),
        "a row was said as an entry: {heard}"
    );
    assert_eq!(
        github["row"]["drawer"], "drawer:passwords:entry:keeping:gh",
        "a row carried its drawer rather than naming it"
    );
    holds_no_secret(&heard);
    // Reading the same list again changes no row, so the watch hears none.
    asking.ask(&json!({"id": 2, "ask": "list", "fresh": true, "asked": true}));
    let listed = asking.ask(&json!({"id": 3, "ask": "list", "kind": "row", "held": true}));
    assert_eq!(
        keys(&listed).len(),
        3,
        "a list of rows didn't hold them: {listed}"
    );
    watching.say(&json!({"id": "h", "ask": "hello"}));
    let next = watching.hear();
    assert_eq!(
        next["id"], "h",
        "a watch of rows heard rows that hadn't changed: {next}"
    );
}

#[test]
fn a_drawer_is_heard_by_its_key_alone_and_only_as_it_changes() {
    let running = Running::with("drawers", &[("keeping", KEEPING)]);
    let mut asking = running.client();
    asking.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let mut watching = running.client();
    watching.say(&json!({"id": "d", "ask": "watch", "kind": "drawer",
                         "keys": ["drawer:passwords:entry:keeping:gh"]}));
    let first = watching.hear();
    assert_eq!(
        keys(&first),
        ["drawer:passwords:entry:keeping:gh"],
        "a drawer watch heard drawers it didn't name: {first}"
    );
    assert_eq!(first["records"][0]["kind"], "drawer");
    assert_eq!(first["records"][0]["drawer"]["title"], "GitHub");
    holds_no_secret(&first);
    let listed = asking.ask(
        &json!({"id": 2, "ask": "list", "kind": "drawer", "held": true,
                                    "keys": ["drawer:passwords:entry:keeping:gh"]}),
    );
    assert_eq!(keys(&listed), ["drawer:passwords:entry:keeping:gh"]);
    let none = asking.ask(&json!({"id": 3, "ask": "list", "kind": "drawer", "held": true}));
    assert_eq!(
        keys(&none),
        Vec::<String>::new(),
        "drawers asked by no key: {none}"
    );
    // Reading the same list again changes no drawer, so the watch hears none.
    asking.ask(&json!({"id": 4, "ask": "list", "fresh": true, "asked": true}));
    watching.say(&json!({"id": "h", "ask": "hello"}));
    let next = watching.hear();
    assert_eq!(
        next["id"], "h",
        "a drawer watch heard a drawer that hadn't changed: {next}"
    );
}

#[test]
fn rows_asked_of_a_daemon_whose_words_didnt_read_are_refused_saying_so() {
    let running = Running::started(
        "wordless",
        &[("keeping", KEEPING)],
        &[],
        "/nowhere/words.json",
    );
    let mut asking = running.client();
    asking.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    for (id, asked) in [(2, "list"), (3, "watch")] {
        let answer = asking.ask(&json!({"id": id, "ask": asked, "kind": "row"}));
        assert_eq!(
            answer["error"]["code"], "no-words",
            "a {asked} of rows without words wasn't refused: {answer}"
        );
    }
    let entries = asking.ask(&json!({"id": 4, "ask": "list", "held": true}));
    assert_eq!(
        keys(&entries).len(),
        3,
        "entries were refused with the rows: {entries}"
    );
}

/// Have I Been Pwned's stand-in, having seen "password" 1,204 times and
/// listing "abc"'s hash only as padding, which counts 0.
fn breaches() -> Breaches {
    let password = hash("password");
    let abc = hash("abc");
    Breaches::answering(vec![
        (
            password[..5].to_owned(),
            format!(
                "0018A45C4D1DEF81644B54AB7F969B88D65:1\r\n{}:1204\r\n",
                &password[5..]
            ),
        ),
        (abc[..5].to_owned(), format!("{}:0\r\n", &abc[5..])),
    ])
}

/// Fails if anything said holds a password the auditing manager keeps,
/// or any part of a hash past the five characters sent.
fn says_no_secret(said: &str) {
    for secret in [
        "\"value\"".to_owned(),
        hash("password")[5..].to_owned(),
        hash("abc")[5..].to_owned(),
    ] {
        assert!(!said.contains(&secret), "{secret} was said: {said}");
    }
}

#[test]
fn an_audit_finds_passwords_reused_and_breached() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    let audited = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(audited["audited"], 4, "{audited}");
    assert_eq!(audited["reused"], 3, "{audited}");
    assert_eq!(audited["breached"], 3, "{audited}");
    assert!(audited.get("unreached").is_none(), "{audited}");
    assert_eq!(
        keys(&audited),
        [
            "entry:auditing:gh",
            "entry:auditing:gist",
            "entry:auditing:mail"
        ]
    );
    // Each of three sharing a password counts the two others.
    assert_eq!(
        audited["records"][0]["health"],
        json!({"reused": 2, "breached": 1204})
    );
    // Bank's hash is in its range only as padding, so it's no breach.
    let listed = client.ask(&json!({"id": 2, "ask": "list", "held": true}));
    assert_eq!(
        listed["records"][3]["health"],
        json!({"reused": 0, "breached": 0})
    );
    // Each range was asked by its five characters alone, padded.
    let requests = breaches.requests();
    let mut paths: Vec<&str> = requests
        .iter()
        .filter_map(|request| request.split_whitespace().nth(1))
        .collect();
    paths.sort_unstable();
    let mut wanted = [
        format!("/range/{}", &hash("password")[..5]),
        format!("/range/{}", &hash("abc")[..5]),
    ];
    wanted.sort();
    assert_eq!(paths, wanted);
    for request in &requests {
        assert!(
            request.to_lowercase().contains("add-padding: true"),
            "{request}"
        );
        says_no_secret(request);
        assert!(!request.contains("abc"), "{request}");
    }
    says_no_secret(&audited.to_string());
    says_no_secret(&listed.to_string());
    says_no_secret(&running.logged());
}

#[test]
fn an_audit_unasked_by_joel_is_refused() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit-unasked",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "audit"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    let refused = client.ask(&json!({"id": 3, "ask": "audit", "asked": false}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    // Nothing was read and nothing sent, and no list says a health.
    assert!(!running.asked_of("auditing").contains("audit"));
    assert_eq!(breaches.requests(), Vec::<String>::new());
    let listed = client.ask(&json!({"id": 4, "ask": "list", "held": true}));
    assert!(
        listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record.get("health").is_none()),
        "{listed}"
    );
}

#[test]
fn an_audit_of_a_manager_signed_out_is_refused() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit-signed-out",
        &[("locked", SIGNED_OUT)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(refused["error"]["code"], "signed-out", "{refused}");
    assert_eq!(breaches.requests(), Vec::<String>::new());
}

#[test]
fn an_audit_unable_to_ask_about_breaches_says_reuse_alone() {
    // A port nothing listens on any more.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/range/", closed.local_addr().unwrap());
    drop(closed);
    let running = Running::with_env(
        "audit-unreached",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &address)],
    );
    let mut client = running.client();
    let audited = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(audited["breached"], Value::Null, "{audited}");
    assert!(audited["unreached"].is_string(), "{audited}");
    assert_eq!(audited["records"][0]["health"], json!({"reused": 2}));
    says_no_secret(&audited.to_string());
    says_no_secret(&running.logged());
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused() {
    let running = Running::with("unknown", &[("keeping", KEEPING)]);
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "dance"}));
    assert_eq!(refused["error"]["code"], "unknown-ask");
    let refused = client.ask(&json!({"id": 2, "ask": "list", "kind": "person"}));
    assert_eq!(refused["error"]["code"], "unknown-kind");
}

/// A folder of the test's own that outlives one run of the daemon, as the
/// account's state folder does, gone once the test ends.
struct Outliving(PathBuf);

impl Outliving {
    fn named(test: &str) -> Outliving {
        let folder = std::env::temp_dir().join(format!("passwords-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        Outliving(folder)
    }

    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }

    fn kept(&self) -> PathBuf {
        self.0.join("passwords/checked.json")
    }
}

impl Drop for Outliving {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn what_an_audit_found_is_kept_without_a_secret_and_outlives_the_daemon() {
    let breaches = breaches();
    let state = Outliving::named("kept-state");
    let set = [
        ("PASSWORDS_BREACHES", breaches.address.as_str()),
        ("XDG_STATE_HOME", state.path()),
    ];
    let checked = {
        let running = Running::with_env("kept", &[("auditing", AUDITING)], &set);
        let audited = running
            .client()
            .ask(&json!({"id": 1, "ask": "audit", "asked": true}));
        assert_eq!(audited["reused"], 3, "{audited}");
        let text = std::fs::read_to_string(state.kept()).expect("the audit was kept");
        let mode = std::fs::metadata(state.kept())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "it can be read by others: {mode:o}");
        let folder = std::fs::metadata(state.0.join("passwords")).unwrap();
        assert_eq!(folder.permissions().mode() & 0o777, 0o700);
        // Written whole through a file beside it, which is gone.
        assert!(!state.0.join("passwords/checked.json.writing").exists());
        says_no_secret(&text);
        let password = hash("password");
        assert!(!text.contains(&password[..5]), "a hash was kept: {text}");
        let kept: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(kept["version"], 1);
        assert!(kept["at"].is_u64(), "{kept}");
        assert_eq!(kept["since"], kept["at"]);
        assert_eq!(kept["cleared"], Value::Null);
        // Only the entries in trouble, each by its two counts.
        let mut troubled: Vec<&String> = kept["health"].as_object().unwrap().keys().collect();
        troubled.sort();
        assert_eq!(
            troubled,
            [
                "entry:auditing:gh",
                "entry:auditing:gist",
                "entry:auditing:mail"
            ]
        );
        assert_eq!(
            kept["health"]["entry:auditing:gh"],
            json!({"reused": 2, "breached": 1204})
        );
        let health = running.client().ask(&json!({"id": 2, "ask": "health"}));
        assert_eq!(health["checked"]["at"], kept["at"], "{health}");
        assert_eq!(health["checked"]["breached"], 3, "{health}");
        assert_eq!(health["checked"]["reused"], 3, "{health}");
        assert_eq!(health["checked"]["troubled"], 3, "{health}");
        health["checked"].clone()
    };
    // Started again, it says what the last check found before any is run
    // or anything is read, and once its manager lists the entries, they
    // wear their health again.
    let running = Running::with_env("kept-again", &[("auditing", AUDITING)], &set);
    let mut client = running.client();
    let health = client.ask(&json!({"id": 1, "ask": "health"}));
    assert_eq!(health["checked"], checked, "{health}");
    let listed = client.ask(&json!({"id": 2, "ask": "list", "asked": true}));
    assert_eq!(
        listed["records"][0]["health"],
        json!({"reused": 2, "breached": 1204}),
        "{listed}"
    );
    assert!(listed["records"][3].get("health").is_none(), "{listed}");
    assert!(!running.asked_of("auditing").contains("audit"));
    says_no_secret(&running.logged());
}

/// A manager whose two logins' passwords are what `$AUDITED`'s file says
/// as each audit asks, so a test changes them between checks, as Joel
/// changes a password between one check and the next.
const CHANGING: &str = r#"
log=$1
entry() { printf '{"id":"%s","title":"%s","category":"login","version":1}' "$1" "$2"; }
echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s,%s]}\n' "$id" "$(entry gh GitHub)" "$(entry bank Bank)" ;;
    audit) printf '{"id":%s,"passwords":%s}\n' "$id" "$(cat "$AUDITED")" ;;
  esac
done
"#;

#[test]
fn since_holds_while_a_check_finds_trouble_and_clears_once_one_finds_none() {
    let breaches = breaches();
    let state = Outliving::named("since-state");
    let audited = state.0.join("audited.json");
    let set = [
        ("PASSWORDS_BREACHES", breaches.address.as_str()),
        ("XDG_STATE_HOME", state.path()),
        ("AUDITED", audited.to_str().unwrap()),
    ];
    let running = Running::with_env("since", &[("changing", CHANGING)], &set);
    let mut watching = running.client();
    watching.say(&json!({"id": "w", "ask": "watch"}));
    watching.hear();
    let mut client = running.client();
    let mut check = |gh: &str, id: u64| {
        std::fs::write(
            &audited,
            json!([{"entry": "gh", "value": gh}, {"entry": "bank", "value": "abc"}]).to_string(),
        )
        .unwrap();
        // Each check ends at a later millisecond than the one before.
        std::thread::sleep(std::time::Duration::from_millis(5));
        let answer = client.ask(&json!({"id": id, "ask": "audit", "asked": true}));
        assert!(answer.get("error").is_none(), "{answer}");
        let told = loop {
            let heard = watching.hear();
            if heard.get("checked").is_some() {
                break heard["checked"].clone();
            }
        };
        let kept: Value =
            serde_json::from_str(&std::fs::read_to_string(state.kept()).unwrap()).unwrap();
        assert_eq!(
            told["at"], kept["at"],
            "the watch wasn't told what was kept"
        );
        told
    };
    // GitHub's password is breached, so trouble's found from this check on.
    let first = check("password", 1);
    assert_eq!(first["since"], first["at"], "{first}");
    assert_eq!(first["breached"], 1, "{first}");
    assert_eq!(first["cleared"], Value::Null, "{first}");
    // Still breached at the next, it's been found since the first.
    let second = check("password", 2);
    assert!(second["at"].as_u64() > first["at"].as_u64(), "{second}");
    assert_eq!(second["since"], first["since"], "{second}");
    // Changed, none is breached or reused: it clears, and since is gone.
    let third = check("a changed one", 3);
    assert_eq!(third["since"], Value::Null, "{third}");
    assert_eq!(third["cleared"], third["at"], "{third}");
    assert_eq!(
        (&third["breached"], &third["reused"], &third["troubled"]),
        (&json!(0), &json!(0), &json!(0)),
        "{third}"
    );
    let kept: Value =
        serde_json::from_str(&std::fs::read_to_string(state.kept()).unwrap()).unwrap();
    assert_eq!(kept["health"], json!({}), "{kept}");
    let health = client.ask(&json!({"id": 4, "ask": "health"}));
    assert_eq!(health["checked"], third, "{health}");
}
