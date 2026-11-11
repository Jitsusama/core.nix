//! The daemon over its socket, with stand-in sources of its own: a get is
//! a step-in, answered within its patience; a watch asks ahead, says what's
//! held, then each change, a burst gathered into one, and how a source
//! stands once that changes; a source signed out is never waited on, and
//! once back, as one that put someone off is, is asked again about
//! everyone watched; one that stops is started again and asked again
//! about everyone watched; a roster is searched and watched by who it
//! lists; and an ask or kind it doesn't know is refused.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use daemon_client::{Client, Daemon, until};
use serde_json::{Value, json};

/// A source on the line protocol, in the shell the tests run in: it
/// writes each line it's asked to its log, and knows Ada, Grace and Eve
/// by their addresses, everyone else as nobody, and says a step-in apart
/// from an ask ahead.
const KNOWING: &str = r#"
log=$1
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  case $line in
    '?'*) address=${line#?} way=ahead ;;
    *) address=$line way=step ;;
  esac
  case $address in
    ada@example.com|grace@example.com|eve@example.com)
      printf '{"email":"%s","name":"%s","way":"%s"}\n' "$address" "${address%@*}" "$way" ;;
    *) printf '{"email":"%s","id":null}\n' "$address" ;;
  esac
done
"#;

/// A source whose sign-in is refused, as slack-people says it.
const SIGNED_OUT: &str = r#"
while read -r line; do echo '{"signedOut":true}'; done
"#;

/// A source that can't answer until `$log.in` is there, saying so as
/// `UNABLE` says, later for each address or else signed out, then knows
/// everyone by their address, as vault-people does once pi signs in or
/// Vault is reached again. Grace it knows all along, as vault-people
/// says someone it kept. Each line is logged once it's answered, so a
/// test that sees it logged knows how it was answered.
const RECOVERING: &str = r#"
log=$1
while read -r line; do
  case $line in
    '?'*) address=${line#?} ;;
    *) address=$line ;;
  esac
  if [ -e "$log.in" ] || [ "$address" = grace@example.com ]; then
    printf '{"email":"%s","name":"%s"}\n' "$address" "${address%@*}"
  elif [ "$UNABLE" = later ]; then
    printf '{"email":"%s","later":true}\n' "$address"
  else
    echo '{"signedIn":false}'
  fi
  printf '%s\n' "$line" >> "$log"
done
"#;

/// A source that answers one line, then stops, as a crash would.
const STOPPING: &str = r#"
log=$1
read -r line
printf '%s\n' "$line" >> "$log"
printf '{"email":"%s","name":"once"}\n' "${line#?}"
"#;

/// A source that says everyone it knows unasked, as Tuple's contacts are
/// said: Ada a favourite, Eli, and Gus, whom it knows by its id alone.
/// Stepping into Eli makes him a favourite, into Ada makes her none, and
/// into Gus deletes him, as Tuple saying so meanwhile would.
const ROSTER: &str = r#"
say() { printf '{"email":"%s","id":%s,"name":"%s","status":"unknown","favourite":%s,"recent":false}\n' "$@"; }
echo '{"roster":true}'
say ada@example.com 1 'Ada Brightwater' true
say eli@example.com 2 'Eli Fenwick' false
say '' 7 'Gus Ada-Moor' false
while read -r line; do
  case $line in
    eli@example.com) say eli@example.com 2 'Eli Fenwick' true ;;
    ada@example.com) say ada@example.com 1 'Ada Brightwater' false ;;
    tuple:7) echo '{"email":"","id":7,"gone":true}' ;;
  esac
done
"#;

/// The daemon, running with these sources, and where its test keeps
/// what each source logged.
struct Running {
    daemon: Daemon,
}

impl Running {
    fn with(test: &str, sources: &[(&str, &str)]) -> Running {
        Running::started(test, sources, &[], &[])
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, sources: &[(&str, &str)], launcher: &[&str]) -> Running {
        Running::started(test, sources, &[], launcher)
    }

    /// Started with how some of its sources say someone stands, by name.
    fn started(
        test: &str,
        sources: &[(&str, &str)],
        stands: &[(&str, Value)],
        launcher: &[&str],
    ) -> Running {
        let folder = std::env::temp_dir().join(format!("people-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let shell = std::env::var("BASH").unwrap_or_else(|_| "sh".to_owned());
        let listed: Vec<Value> = sources
            .iter()
            .map(|(name, script)| {
                let log = folder.join(format!("{name}.log"));
                let stands = stands
                    .iter()
                    .find(|(source, _)| source == name)
                    .map(|(_, stands)| stands);
                json!({"name": name, "command": [shell, "-c", script, name, log], "stands": stands})
            })
            .collect();
        let config = folder.join("sources.json");
        std::fs::write(&config, json!({"sources": listed}).to_string()).unwrap();
        // The launcher's vocabulary, as far as People's words go, worded
        // apart from any source's own word so a test sees whose it says.
        let words = folder.join("words.json");
        let vocabulary = json!({"things": [{"id": "people", "said": {"available": "around"}}]});
        std::fs::write(&words, vocabulary.to_string()).unwrap();
        let socket = folder.join("daemons").join("people.sock");
        // Made here, since a launcher listens there before the daemon runs.
        std::fs::create_dir_all(folder.join("daemons")).unwrap();
        let people = env!("CARGO_BIN_EXE_people");
        let mut command = match launcher.split_first() {
            Some((program, arguments)) => {
                let mut command = Command::new(program);
                for argument in arguments {
                    command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
                }
                command.arg(people);
                command
            }
            None => Command::new(people),
        };
        command
            .arg(&socket)
            .arg(&config)
            .arg(&words)
            .stderr(Stdio::null());
        Running {
            daemon: Daemon::start(&mut command, socket, folder),
        }
    }

    fn client(&self) -> Client {
        self.daemon.client()
    }

    /// What a source was asked, a line each.
    fn asked(&self, source: &str) -> Vec<String> {
        std::fs::read_to_string(self.daemon.folder().join(format!("{source}.log")))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// The next change a watch is told, passing over how its sources stand,
/// which a source still starting may say first.
fn next_change(client: &mut Client) -> Value {
    loop {
        let said = client.heard();
        if said.get("changed").is_some() {
            return said;
        }
    }
}

#[test]
fn a_get_is_a_step_in_answered_by_every_source_within_its_patience() {
    let running = Running::with("get", &[("slack", KNOWING)]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello", "client": "test", "version": 1}));
    assert_eq!(
        (&said["id"], &said["version"], &said["domain"]),
        (&json!(1), &json!(1), &json!("people"))
    );
    let said = client
        .ask(&json!({"id": 2, "ask": "get", "keys": ["person:Ada@Example.com"], "patience": 5000}));
    assert_eq!(
        said["records"],
        json!([{"key": "person:ada@example.com", "kind": "person", "heard": said["records"][0]["heard"],
                "sources": {"slack": {"name": "ada", "way": "step"}}}])
    );
    assert_eq!(said["waiting"], json!([]));
    assert_eq!(running.asked("slack"), ["ada@example.com"]);
    let said = client.ask(&json!({"id": 3, "ask": "list"}));
    assert_eq!(said["records"].as_array().map(Vec::len), Some(1));
}

#[test]
fn a_watch_asks_ahead_says_what_is_held_and_gathers_a_burst_into_one_change() {
    let running = Running::with("watch", &[("slack", KNOWING)]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "person", "keys": [
        "person:ada@example.com", "person:grace@example.com", "person:eve@example.com"]}));
    assert_eq!(said["records"], json!([]));
    let mut changed = Vec::new();
    let mut messages = 0;
    while changed.len() < 3 {
        let said = next_change(&mut client);
        assert_eq!(said["id"], 1);
        changed.extend(said["changed"].as_array().unwrap().iter().cloned());
        messages += 1;
    }
    assert!(messages <= 2, "three answers came as {messages} changes");
    assert!(
        changed
            .iter()
            .all(|record| record["sources"]["slack"]["way"] == "ahead")
    );
    assert_eq!(
        running.asked("slack"),
        ["?ada@example.com", "?grace@example.com", "?eve@example.com"]
    );
    // Watched again, everyone is held already, and nobody is asked again.
    let said = client.ask(&json!({"id": 2, "ask": "watch", "keys": ["person:ada@example.com"]}));
    assert_eq!(said["records"][0]["key"], "person:ada@example.com");
    assert_eq!(running.asked("slack").len(), 3);
    // The same watch asked again by its id replaces the one before, and
    // one unwatched is gone.
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["person:grace@example.com"]}));
    client.ask(&json!({"id": 2, "ask": "unwatch"}));
    assert_eq!(
        client.ask(&json!({"id": 3, "ask": "health"}))["watching"],
        1
    );
}

#[test]
fn a_source_signed_out_is_never_waited_on() {
    let running = Running::with("signed-out", &[("slack", KNOWING), ("vault", SIGNED_OUT)]);
    let mut client = running.client();
    // The first ask has the refused source say so.
    client
        .ask(&json!({"id": 1, "ask": "get", "keys": ["person:ada@example.com"], "patience": 2000}));
    let began = Instant::now();
    let said = client.ask(
        &json!({"id": 2, "ask": "get", "keys": ["person:grace@example.com"], "patience": 5000}),
    );
    assert!(
        began.elapsed() < Duration::from_secs(4),
        "it waited on a source signed out"
    );
    assert_eq!(said["records"][0]["sources"]["slack"]["name"], "grace");
    let said = client.ask(&json!({"id": 3, "ask": "hello"}));
    assert_eq!(
        said["sources"],
        json!({"slack": "answering", "vault": "signed-out"})
    );
}

#[test]
fn a_watch_hears_a_source_sign_out_as_it_says_so() {
    let running = Running::with("standing", &[("slack", SIGNED_OUT)]);
    let mut client = running.client();
    let deadline = Instant::now() + Duration::from_secs(10);
    while client.ask(&json!({"id": 1, "ask": "hello"}))["sources"]["slack"] != "answering" {
        assert!(Instant::now() < deadline, "slack never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    client.send(&json!({"id": 2, "ask": "watch", "keys": ["person:ada@example.com"]}));
    // The ask ahead has it say it's signed out, which the watch hears
    // without asking, as a client learns a sign-in went. The source may
    // say so before the watch is answered, so either comes first.
    let said = [client.heard(), client.heard()];
    let pushed = json!({"id": 2, "sources": {"slack": "signed-out"}});
    assert!(said.contains(&pushed), "{said:?}");
    assert!(
        said.iter().any(|line| line.get("records").is_some()),
        "{said:?}"
    );
}

/// A source that put Ada off as she was watched, signed out or saying
/// later, is asked about her again, by nobody asking, once it says who
/// someone else is; and only about her, never about Grace, whom it
/// answered, so someone it says while still unable to answer the rest
/// asks it again once, not over and over.
fn a_source_back_is_asked_again_about_everyone_watched(test: &str, unable: &str) {
    let script = format!("UNABLE={unable}\n{RECOVERING}");
    let running = Running::with(test, &[("vault", &script)]);
    let mut watching = running.client();
    watching.ask(&json!({"id": 1, "ask": "watch", "keys": [
        "person:ada@example.com", "person:grace@example.com"]}));
    until("Ada was never asked again after Grace", || {
        running.asked("vault") == ["?ada@example.com", "?grace@example.com", "?ada@example.com"]
    });
    std::fs::write(running.daemon.folder().join("vault.log.in"), "").unwrap();
    // Joel steps into Eve, and the source says who she is: it's back.
    // One signed out isn't waited on, so the step-in may answer first.
    running
        .client()
        .ask(&json!({"id": 2, "ask": "get", "keys": ["person:eve@example.com"], "patience": 5000}));
    loop {
        let changed = next_change(&mut watching);
        let ada = changed["changed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["key"] == "person:ada@example.com")
            .cloned();
        if let Some(ada) = ada {
            assert_eq!(ada["sources"]["vault"]["name"], "ada", "{ada}");
            break;
        }
    }
    until("Ada was never asked ahead again, or Grace was", || {
        running.asked("vault")
            == [
                "?ada@example.com",
                "?grace@example.com",
                "?ada@example.com",
                "eve@example.com",
                "?ada@example.com",
            ]
    });
}

#[test]
fn a_source_signed_out_and_back_is_asked_again_about_everyone_watched() {
    a_source_back_is_asked_again_about_everyone_watched("back-in", "signed-out");
}

#[test]
fn a_source_that_said_later_and_answers_again_is_asked_again_about_everyone_watched() {
    a_source_back_is_asked_again_about_everyone_watched("back-later", "later");
}

#[test]
fn a_socket_systemd_hands_it_is_the_one_it_answers_on() {
    // As systemd does for people.socket: the socket is listened on before
    // the daemon runs, and handed to it as it starts, so the asking that
    // started it is answered. A daemon that made a socket of its own
    // would leave that ask waiting on one nobody accepts from.
    let running = Running::through(
        "handed",
        &[("slack", KNOWING)],
        &["systemd-socket-activate", "--listen={socket}"],
    );
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(said["domain"], "people");
}

#[test]
fn a_source_that_stops_is_started_again_and_asked_again_about_everyone_watched() {
    let running = Running::with("stopping", &[("slack", STOPPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["person:ada@example.com"]}));
    until("the source wasn't asked again once started again", || {
        running.asked("slack").len() >= 2
    });
    assert!(
        running
            .asked("slack")
            .iter()
            .all(|line| line == "?ada@example.com")
    );
    let said = client.ask(&json!({"id": 2, "ask": "health"}));
    assert!(said["sources"]["slack"]["state"].is_string());
}

#[test]
fn a_source_running_is_answering_before_anything_asks_it() {
    let running = Running::with("answering", &[("slack", KNOWING)]);
    let mut client = running.client();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let said = client.ask(&json!({"id": 1, "ask": "hello"}));
        if said["sources"]["slack"] == "answering" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "it never said slack was answering: {said}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(running.asked("slack"), [] as [std::string::String; 0]);
}

#[test]
fn a_get_nothing_has_answered_in_time_is_not_ready() {
    let silent = "while read -r line; do :; done";
    let running = Running::with("silent", &[("slack", silent)]);
    let mut client = running.client();
    let said = client
        .ask(&json!({"id": 1, "ask": "get", "keys": ["person:ada@example.com"], "patience": 100}));
    assert_eq!(said["error"]["code"], "not-ready");
}

#[test]
fn a_roster_is_searched_and_watched_by_who_it_lists() {
    let running = Running::with("roster", &[("tuple", ROSTER)]);
    let mut client = running.client();
    let deadline = Instant::now() + Duration::from_secs(10);
    while client.ask(&json!({"id": 0, "ask": "health"}))["people"] != 3 {
        assert!(Instant::now() < deadline, "the roster was never held");
        std::thread::sleep(Duration::from_millis(20));
    }
    let keys = |said: &Value| -> Vec<String> {
        said["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["key"].as_str().unwrap().to_owned())
            .collect()
    };
    // Ada's listed, so first; Gus is found by his name, by Tuple's id.
    let said = client.ask(&json!({"id": 1, "ask": "search", "words": "ada"}));
    assert_eq!(keys(&said), ["person:ada@example.com", "person:tuple:7"]);
    let said = client.ask(&json!({"id": 2, "ask": "search", "words": "ada", "limit": 1}));
    assert_eq!(keys(&said), ["person:ada@example.com"]);
    assert_eq!(
        keys(&client.ask(&json!({"id": 3, "ask": "search", "words": "fenw"}))),
        ["person:eli@example.com"]
    );
    let said = client.ask(&json!({"id": 4, "ask": "list", "listed": true}));
    assert_eq!(keys(&said), ["person:ada@example.com"]);
    let said = client.ask(&json!({"id": 5, "ask": "watch", "listed": true}));
    assert_eq!(keys(&said), ["person:ada@example.com"]);
    // A roster isn't waited on: a step-in answers with what's held.
    let began = Instant::now();
    let said = client
        .ask(&json!({"id": 6, "ask": "get", "keys": ["person:eli@example.com"], "patience": 5000}));
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "it waited on a roster"
    );
    assert_eq!(said["waiting"], json!([]));
    // Eli made a favourite is told to the watch of the listed.
    let said = next_change(&mut client);
    assert_eq!(said["id"], 5);
    assert_eq!(said["changed"][0]["key"], "person:eli@example.com");
    assert_eq!(said["changed"][0]["sources"]["tuple"]["favourite"], true);
    // Ada no longer one is told too, so it's dropped.
    client.ask(&json!({"id": 7, "ask": "get", "keys": ["person:ada@example.com"]}));
    let said = next_change(&mut client);
    assert_eq!(said["changed"][0]["key"], "person:ada@example.com");
    assert_eq!(said["changed"][0]["sources"]["tuple"]["favourite"], false);
    // Gus, never listed, goes without a word to it, and from the book.
    let mut everyone = running.client();
    everyone.ask(&json!({"id": 1, "ask": "watch"}));
    client.ask(&json!({"id": 8, "ask": "get", "keys": ["person:tuple:7"]}));
    let said = next_change(&mut everyone);
    assert_eq!(
        said["changed"],
        json!([{"key": "person:tuple:7", "kind": "person", "gone": true}])
    );
    assert_eq!(client.ask(&json!({"id": 9, "ask": "health"}))["people"], 2);
    let said = client.ask(&json!({"id": 10, "ask": "list", "listed": true}));
    assert_eq!(keys(&said), ["person:eli@example.com"]);
}

#[test]
fn those_listed_are_told_in_order_and_after_only_whoever_changed() {
    let stands =
        json!({"field": "status", "words": {"unknown": {"word": "available", "order": 0}}});
    let running = Running::started("listing", &[("tuple", ROSTER)], &[("tuple", stands)], &[]);
    let mut client = running.client();
    let deadline = Instant::now() + Duration::from_secs(10);
    while client.ask(&json!({"id": 0, "ask": "health"}))["people"] != 3 {
        assert!(Instant::now() < deadline, "the roster was never held");
        // The roster's three lines land as the source starts, asked about
        // again each 20 ms until the deadline.
        std::thread::sleep(Duration::from_millis(20));
    }
    let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "listing"}));
    let records = said["records"].as_array().expect("a listing is records");
    assert_eq!(
        records.len(),
        2,
        "Ada, the one favourite, and the order: {said}"
    );
    assert_eq!(records[0]["person"]["name"], "Ada Brightwater", "{said}");
    assert_eq!(records[0]["person"]["stood"]["favourite"], true, "{said}");
    assert_eq!(
        records[0]["person"]["stood"]["said"], "around",
        "how they stand, in the launcher's words: {said}"
    );
    assert_eq!(
        records[0]["sources"]["tuple"]["id"], 1,
        "with what each source said: {said}"
    );
    assert_eq!(
        records[1]["order"],
        json!(["person:ada@example.com"]),
        "{said}"
    );
    // Eli made a favourite is told alone, with the order he joins.
    client.ask(&json!({"id": 2, "ask": "get", "keys": ["person:eli@example.com"]}));
    let said = next_change(&mut client);
    assert_eq!(said["id"], 1, "{said}");
    assert_eq!(
        said["changed"],
        json!([
            {"key": "person:eli@example.com", "kind": "listing",
             "person": said["changed"][0]["person"], "sources": said["changed"][0]["sources"]},
            {"key": "people:order", "kind": "listing",
             "order": ["person:ada@example.com", "person:eli@example.com"]}
        ]),
        "Ada, unchanged, isn't told again"
    );
    // Ada no longer a favourite is gone from the listing.
    client.ask(&json!({"id": 3, "ask": "get", "keys": ["person:ada@example.com"]}));
    let said = next_change(&mut client);
    assert_eq!(
        said["changed"],
        json!([
            {"key": "person:ada@example.com", "kind": "listing", "gone": true},
            {"key": "people:order", "kind": "listing", "order": ["person:eli@example.com"]}
        ])
    );
    let said = client.ask(&json!({"id": 4, "ask": "list", "kind": "listing"}));
    assert_eq!(
        said["records"].as_array().map(Vec::len),
        Some(2),
        "a list is the whole: {said}"
    );
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused_by_its_code() {
    let running = Running::with("refused", &[("slack", KNOWING)]);
    let mut client = running.client();
    assert_eq!(
        client.ask(&json!({"id": 1, "ask": "shout"}))["error"]["code"],
        "unknown-ask"
    );
    assert_eq!(
        client.ask(&json!({"id": 2, "ask": "list", "kind": "file"}))["error"]["code"],
        "unknown-kind"
    );
}
