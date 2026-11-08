//! The daemon over its socket, with stand-in password managers of its own:
//! a list holds every entry without a secret, even one a manager gives
//! with its secrets left on; a get reads an entry in full, still without
//! them; a secret goes only to the client that asked to see it, or to the
//! clipboard, cleared when its time is up; a one-time code says its
//! period; a manager signed out is said so and refuses a secret; a watch
//! hears entries arrive; and an ask or kind it doesn't know is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A manager on the line protocol, in the shell the tests run in: it
/// answers at once, knows GitHub, a bank card and a note, gives GitHub in
/// full with its password and one-time seed left on, as op does, and
/// logs each ask.
const KEEPING: &str = r#"
log=$1
echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s,%s,%s]}\n' "$id" \
      '{"id":"gh","title":"GitHub","category":"login","version":1,"username":"ada","websites":[{"address":"https://github.com"}]}' \
      '{"id":"card","title":"Bank Card","category":"card","version":1}' \
      '{"id":"note","title":"Door Codes","category":"note","version":1,"tags":["home"]}' ;;
    get) printf '{"id":%s,"entry":%s}\n' "$id" \
      '{"id":"gh","title":"GitHub","category":"login","version":1,"details":[{"id":"username","label":"username","type":"text","purpose":"username","value":"ada"},{"id":"password","label":"password","type":"hidden","purpose":"password","value":"hunter2"},{"id":"otp","label":"one-time password","type":"code","value":"otpauth://totp/x?secret=SEED"}]}' ;;
    reveal) printf '{"id":%s,"value":"hunter2"}\n' "$id" ;;
    code) printf '{"id":%s,"code":"123456","period":30,"left":12}\n' "$id" ;;
    let-in) printf '{"id":%s}\n' "$id" ;;
  esac
done
"#;

/// A manager that hasn't been let in, as 1Password's is until Joel
/// approves it: signed out, refusing everything.
const SIGNED_OUT: &str = r#"
echo '{"state":"signed-out","reason":"Let the launcher in"}'
while read -r line; do
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  printf '{"id":%s,"error":{"code":"signed-out","message":"Let the launcher in"}}\n' "$id"
done
"#;

/// A stand-in for wl-copy in the foreground: what it's given goes to its
/// file, and it holds on until it's ended, as wl-copy holds the clipboard.
const COPIER: &str = r#"
cat > "$COPIED"
printf '%s\n' "$*" > "$COPIED.flags"
while :; do sleep 1; done
"#;

/// The daemon, running with these managers, and where its test keeps
/// what each manager logged.
struct Running {
    daemon: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Running {
    fn with(test: &str, managers: &[(&str, &str)]) -> Running {
        let folder = std::env::temp_dir().join(format!("passwords-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let shell = std::env::var("BASH").unwrap_or_else(|_| "bash".to_owned());
        let listed: Vec<Value> = managers
            .iter()
            .map(|(name, script)| {
                let log = folder.join(format!("{name}.log"));
                json!({"name": name, "command": [shell, "-c", script, name, log]})
            })
            .collect();
        let managers_file = folder.join("managers.json");
        std::fs::write(&managers_file, json!({"managers": listed}).to_string()).unwrap();
        let copier = folder.join("copier");
        // Run as wl-copy is, by its path alone, so it names its own shell;
        // it needs nothing past POSIX sh, which Nix's build sandbox has too.
        std::fs::write(&copier, format!("#!/bin/sh\n{COPIER}")).unwrap();
        std::fs::set_permissions(&copier, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let socket = folder.join("daemons/passwords.sock");
        let daemon = Command::new(env!("CARGO_BIN_EXE_passwords"))
            .arg("daemon")
            .arg(&socket)
            .arg(&managers_file)
            .env("PASSWORDS_COPIER", &copier)
            .env("PASSWORDS_CLEARS_MS", "700")
            .env("COPIED", folder.join("copied"))
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let running = Running {
            daemon,
            socket,
            folder,
        };
        until("the socket is there", || running.socket.exists());
        running
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        Client {
            reading: BufReader::new(stream.try_clone().unwrap()),
            writing: stream,
        }
    }

    fn copied(&self) -> PathBuf {
        self.folder.join("copied")
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

struct Client {
    reading: BufReader<UnixStream>,
    writing: UnixStream,
}

impl Client {
    fn say(&mut self, asked: &Value) {
        writeln!(self.writing, "{asked}").unwrap();
    }

    fn hear(&mut self) -> Value {
        let mut line = String::new();
        self.reading.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("not a line of JSON: {line:?}"))
    }

    /// Asks one thing and hears its answer, passing over any other.
    fn ask(&mut self, asked: &Value) -> Value {
        let id = asked["id"].clone();
        self.say(asked);
        loop {
            let heard = self.hear();
            if heard["id"] == id {
                return heard;
            }
        }
    }
}

fn until(what: &str, holds: impl Fn() -> bool) {
    let started = Instant::now();
    while !holds() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "{what} never held"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

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
    let log = std::fs::read_to_string(running.folder.join("keeping.log")).unwrap();
    assert_eq!(log.matches("\"list\"").count(), 1, "{log}");
    assert!(
        log.contains("\"asked\":true"),
        "Joel's key wasn't passed on: {log}"
    );
}

#[test]
fn a_secret_goes_only_where_it_was_asked_for() {
    let running = Running::with("secret", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let shown = client.ask(&json!({"id": 2, "ask": "reveal", "key": "entry:keeping:gh"}));
    assert_eq!(shown["value"], "hunter2");
    assert_eq!(shown["detail"], "password");
    let mut other = running.client();
    holds_no_secret(&other.ask(&json!({"id": 1, "ask": "get", "keys": ["entry:keeping:gh"]})));
    holds_no_secret(&other.ask(&json!({"id": 2, "ask": "search", "words": "git"})));
    let health = other.ask(&json!({"id": 3, "ask": "health"}));
    holds_no_secret(&health);
    assert_eq!(health["entries"], 3);
}

#[test]
fn a_copy_is_marked_sensitive_and_cleared_when_its_time_is_up() {
    let running = Running::with("copy", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    holds_no_secret(&copied);
    until("the password is copied", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "hunter2")
    });
    let flags = std::fs::read_to_string(running.copied().with_extension("flags")).unwrap();
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
fn an_ask_or_a_kind_it_doesnt_know_is_refused() {
    let running = Running::with("unknown", &[("keeping", KEEPING)]);
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "dance"}));
    assert_eq!(refused["error"]["code"], "unknown-ask");
    let refused = client.ask(&json!({"id": 2, "ask": "list", "kind": "person"}));
    assert_eq!(refused["error"]["code"], "unknown-kind");
}
