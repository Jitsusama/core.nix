//! What the daemon's tests share: the daemon running with stand-in
//! password managers of its own, asked over its standard input and output
//! as the shell asks it, a stand-in for Have I Been Pwned on a local port,
//! and a wait on something coming to hold.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A manager that answers at once (keeping-manager.sh says what it knows).
pub const KEEPING: &str = include_str!("../keeping-manager.sh");

/// A manager that hasn't been let in, as 1Password's is until Joel
/// approves it: signed out, refusing everything.
pub const SIGNED_OUT: &str = r#"
echo '{"state":"signed-out","reason":"Let the launcher in"}'
while read -r line; do
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  printf '{"id":%s,"error":{"code":"signed-out","message":"Let the launcher in"}}\n' "$id"
done
"#;

/// A manager whose logins share passwords, for an audit: GitHub, Gist and
/// Mail share one, which Have I Been Pwned's stand-in has seen in breaches,
/// and Bank keeps its own, which it hasn't. Its secrets are made up and
/// look it; it answers an audit only as Joel's own ask, as 1Password's
/// manager does.
pub const AUDITING: &str = r#"
log=$1
entry() { printf '{"id":"%s","title":"%s","category":"login","version":1}' "$1" "$2"; }
echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s,%s,%s,%s]}\n' "$id" "$(entry gh GitHub)" \
      "$(entry gist Gist)" "$(entry mail Mail)" "$(entry bank Bank)" ;;
    audit)
      if [[ $line != *'"asked":true'* ]]; then
        printf '{"id":%s,"error":{"code":"refused","message":"not asked"}}\n' "$id"
        continue
      fi
      printf '{"id":%s,"passwords":[' "$id"
      printf '{"entry":"gh","value":"password"},{"entry":"gist","value":"password"},'
      printf '{"entry":"mail","value":"password"},{"entry":"bank","value":"abc"}]}\n' ;;
  esac
done
"#;

/// A manager that releases a listed reference's value to Joel's yes,
/// as 1Password's does once it's started with `--release`: a made-up
/// value that looks it, said only to a release `asked` as his.
pub const RELEASING: &str = r#"
log=$1
echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[]}\n' "$id" ;;
    release)
      if [[ $line != *'"asked":true'* ]]; then
        printf '{"id":%s,"error":{"code":"refused","message":"not asked"}}\n' "$id"
        continue
      fi
      printf '{"id":%s,"value":"%s"}\n' "$id" "$RELEASED_VALUE" ;;
    *) printf '{"id":%s,"error":{"code":"refused","message":"no"}}\n' "$id" ;;
  esac
done
"#;

/// The made-up value RELEASING releases, which no answer, log or
/// command line may hold.
pub const RELEASED_VALUE: &str = "tr0ub4dor-made-up-for-the-test";

/// A stand-in for bubblewrap, which a build's sandbox can't nest: it
/// keeps the arguments it was given, then runs what follows `--` with
/// the environment it was handed, as bwrap does once it's set its
/// sandbox up.
const BWRAP: &str = r#"
printf '%s\n' "$@" > "$0.args"
while [ "$1" != -- ]; do shift; done
shift
exec "$@"
"#;

/// A stand-in for wl-copy in the foreground: what it's given goes to its
/// file, and it holds on until it's ended, as wl-copy holds the clipboard.
const COPIER: &str = r#"
cat > "$COPIED"
printf '%s\n' "$*" > "$COPIED.flags"
while :; do sleep 1; done
"#;

/// The daemon, running with these managers, its one channel, and where
/// its test keeps what each manager logged.
pub struct Running {
    daemon: Child,
    channel: Rc<RefCell<Channel>>,
    pub folder: PathBuf,
}

/// The daemon's standard input, each line it said, and those said that
/// no ask has taken yet, as a watch's while an ask waits on its answer.
struct Channel {
    writing: Option<ChildStdin>,
    lines: Receiver<String>,
    unheard: VecDeque<Value>,
}

impl Running {
    pub fn with(test: &str, managers: &[(&str, &str)]) -> Running {
        Running::with_env(test, managers, &[])
    }

    /// The daemon with these managers, and these variables set beside the
    /// tests' own, as `PASSWORDS_BREACHES` points it at a stand-in. What
    /// the checks find is kept in the test's own folder, never the
    /// account's, unless `XDG_STATE_HOME` is set to another that outlives
    /// it, as a test of a daemon starting again sets it.
    pub fn with_env(test: &str, managers: &[(&str, &str)], set: &[(&str, &str)]) -> Running {
        // Rows are said in the account's own vocabulary, as the shell
        // says them, which passwords-holds-no-secret hands the tests.
        let words = std::env::var("LAUNCHER_WORDS")
            .expect("LAUNCHER_WORDS names no vocabulary; passwords-holds-no-secret hands it one");
        Running::started(test, managers, set, &words)
    }

    /// The daemon with these managers and variables, its rows said in the
    /// vocabulary at `words`, which a test names that isn't there to start
    /// it as an account whose words didn't read.
    pub fn started(
        test: &str,
        managers: &[(&str, &str)],
        set: &[(&str, &str)],
        words: &str,
    ) -> Running {
        Running::started_with(test, managers, set, words, &json!({}))
    }

    /// The daemon with RELEASING as 1Password, reading `op://`
    /// references, these listed as `releases`, a stand-in bubblewrap,
    /// and a runtime folder of its own, whose `secret-asks` holds the
    /// asks (`asks`).
    pub fn releasing(test: &str, releases: &[&str]) -> Running {
        let words = std::env::var("LAUNCHER_WORDS")
            .expect("LAUNCHER_WORDS names no vocabulary; passwords-holds-no-secret hands it one");
        let folder = Running::folder_of(test);
        let bwrap = folder.join("bwrap");
        let runtime = folder.join("runtime");
        let also = json!({
            "releases": releases,
            "git": "git",
            "bwrap": bwrap,
            "references": {"1password": "op://"},
        });
        let running = Running::started_with(
            test,
            &[("1password", RELEASING)],
            &[
                ("XDG_RUNTIME_DIR", runtime.to_str().unwrap()),
                ("RELEASED_VALUE", RELEASED_VALUE),
            ],
            &words,
            &also,
        );
        std::fs::write(&bwrap, format!("#!/bin/sh\n{BWRAP}")).unwrap();
        std::fs::set_permissions(&bwrap, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        running
    }

    /// The folder a test keeps everything in.
    fn folder_of(test: &str) -> PathBuf {
        std::env::temp_dir().join(format!("passwords-{test}-{}", std::process::id()))
    }

    /// The daemon as `started` makes it, with `also`'s fields beside
    /// the managers in its file, its `references` given each manager
    /// by name.
    fn started_with(
        test: &str,
        managers: &[(&str, &str)],
        set: &[(&str, &str)],
        words: &str,
        also: &Value,
    ) -> Running {
        let folder = Running::folder_of(test);
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let shell = std::env::var("BASH").unwrap_or_else(|_| "bash".to_owned());
        let listed: Vec<Value> = managers
            .iter()
            .map(|(name, script)| {
                let log = folder.join(format!("{name}.log"));
                let mut manager =
                    json!({"name": name, "command": [shell, "-c", script, name, log]});
                if let Some(prefix) = also["references"].get(name) {
                    manager["references"] = prefix.clone();
                }
                manager
            })
            .collect();
        let mut file = json!({"managers": listed});
        for (field, value) in also.as_object().into_iter().flatten() {
            if field != "references" {
                file[field] = value.clone();
            }
        }
        let managers_file = folder.join("managers.json");
        std::fs::write(&managers_file, file.to_string()).unwrap();
        let copier = folder.join("copier");
        // Run as wl-copy is, by its path alone, so it names its own shell;
        // it needs nothing past POSIX sh, which Nix's build sandbox has too.
        std::fs::write(&copier, format!("#!/bin/sh\n{COPIER}")).unwrap();
        std::fs::set_permissions(&copier, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let mut daemon = Command::new(env!("CARGO_BIN_EXE_passwords"))
            .arg("daemon")
            .arg(&managers_file)
            .arg(words)
            .env("PASSWORDS_COPIER", &copier)
            .env("PASSWORDS_CLEARS_MS", "700")
            .env("COPIED", folder.join("copied"))
            .env("XDG_STATE_HOME", folder.join("state"))
            .envs(set.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // What it and its managers log, kept so a test can say no
            // secret was logged.
            .stderr(std::fs::File::create(folder.join("logged")).unwrap())
            .spawn()
            .unwrap();
        let writing = daemon.stdin.take();
        let reading = BufReader::new(daemon.stdout.take().unwrap());
        let (told, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in reading.lines() {
                let Ok(line) = line else { break };
                if told.send(line).is_err() {
                    break;
                }
            }
        });
        let channel = Rc::new(RefCell::new(Channel {
            writing,
            lines,
            unheard: VecDeque::new(),
        }));
        Running {
            daemon,
            channel,
            folder,
        }
    }

    /// A client of the one channel: each shares what's said on it, and an
    /// ask leaves what isn't its answer for the others to hear.
    pub fn client(&self) -> Client {
        Client {
            channel: Rc::clone(&self.channel),
        }
    }

    /// Closes the daemon's standard input, as the shell going does.
    pub fn close(&self) {
        self.channel.borrow_mut().writing = None;
    }

    /// The daemon's process id.
    pub fn pid(&self) -> u32 {
        self.daemon.id()
    }

    /// Whether the daemon has ended.
    pub fn ended(&mut self) -> bool {
        self.daemon.try_wait().unwrap().is_some()
    }

    pub fn copied(&self) -> PathBuf {
        self.folder.join("copied")
    }

    /// What a manager was asked so far, a line an ask.
    pub fn asked_of(&self, manager: &str) -> String {
        std::fs::read_to_string(self.folder.join(format!("{manager}.log"))).unwrap_or_default()
    }

    /// The folder a releasing daemon's asks are put up in.
    pub fn asks(&self) -> PathBuf {
        self.folder.join("runtime").join("secret-asks")
    }

    /// The arguments the stand-in bubblewrap was last given, a line each.
    pub fn bwrapped(&self) -> String {
        std::fs::read_to_string(self.folder.join("bwrap.args")).unwrap_or_default()
    }

    /// What the daemon and its managers logged so far.
    pub fn logged(&self) -> String {
        std::fs::read_to_string(self.folder.join("logged")).unwrap_or_default()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

pub struct Client {
    channel: Rc<RefCell<Channel>>,
}

impl Client {
    pub fn say(&mut self, asked: &Value) {
        let mut channel = self.channel.borrow_mut();
        let writing = channel.writing.as_mut().expect("the channel was closed");
        writeln!(writing, "{asked}").unwrap();
        writing.flush().unwrap();
    }

    /// The next line said, one no ask took first.
    pub fn hear(&mut self) -> Value {
        let mut channel = self.channel.borrow_mut();
        if let Some(heard) = channel.unheard.pop_front() {
            return heard;
        }
        next(&channel)
    }

    /// Asks one thing and hears its answer, leaving every other line said
    /// meanwhile for the next to hear.
    pub fn ask(&mut self, asked: &Value) -> Value {
        let id = asked["id"].clone();
        self.say(asked);
        let mut channel = self.channel.borrow_mut();
        if let Some(at) = channel.unheard.iter().position(|heard| heard["id"] == id) {
            return channel.unheard.remove(at).unwrap();
        }
        loop {
            let heard = next(&channel);
            if heard["id"] == id {
                return heard;
            }
            channel.unheard.push_back(heard);
        }
    }

    /// Every line said that nothing has heard yet.
    pub fn unheard(&mut self) -> Vec<Value> {
        let mut channel = self.channel.borrow_mut();
        while let Ok(line) = channel.lines.try_recv() {
            channel.unheard.push_back(json_of(&line));
        }
        channel.unheard.drain(..).collect()
    }
}

/// The next line the daemon says, within 20 seconds.
fn next(channel: &Channel) -> Value {
    let line = channel
        .lines
        .recv_timeout(Duration::from_secs(20))
        .expect("the daemon said nothing for 20 s");
    json_of(&line)
}

fn json_of(line: &str) -> Value {
    serde_json::from_str(line).unwrap_or_else(|_| panic!("not a line of JSON: {line:?}"))
}

/// A stand-in for Have I Been Pwned's range API on a local port, answering
/// each range it's asked with `ranges`' answer for its five characters, or
/// with padding alone, and keeping every request it's sent, whole, so a
/// test can say what left the machine.
pub struct Breaches {
    pub address: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl Breaches {
    pub fn answering(ranges: Vec<(String, String)>) -> Breaches {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/range/", listener.local_addr().unwrap());
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let kept = std::sync::Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n")
                    && stream.read(&mut byte).is_ok_and(|n| n == 1)
                {
                    request.push(byte[0]);
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
                kept.lock().unwrap().push(request);
                let prefix = path.rsplit('/').next().unwrap_or("");
                let mut body = ranges
                    .iter()
                    .find(|(asked, _)| asked == prefix)
                    .map(|(_, range)| range.clone())
                    .unwrap_or_default();
                body.push_str("0000000000000000000000000000000000F:0\r\n");
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\
                     Connection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Breaches { address, requests }
    }

    /// Every request it was sent, whole.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

pub fn until(what: &str, holds: impl Fn() -> bool) {
    let started = Instant::now();
    while !holds() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "{what} never held"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
