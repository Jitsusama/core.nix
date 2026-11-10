//! What the daemon's tests and the command's share: the daemon running
//! with stand-in password managers of its own, a client on its socket, a
//! stand-in for Have I Been Pwned on a local port, and a wait on
//! something coming to hold.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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

/// A stand-in for wl-copy in the foreground: what it's given goes to its
/// file, and it holds on until it's ended, as wl-copy holds the clipboard.
const COPIER: &str = r#"
cat > "$COPIED"
printf '%s\n' "$*" > "$COPIED.flags"
while :; do sleep 1; done
"#;

/// The daemon, running with these managers, and where its test keeps
/// what each manager logged.
pub struct Running {
    daemon: Child,
    pub socket: PathBuf,
    pub folder: PathBuf,
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
            // Rows are said in the tests' own vocabulary (rows_tests.rs's).
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json"))
            .env("PASSWORDS_COPIER", &copier)
            .env("PASSWORDS_CLEARS_MS", "700")
            .env("COPIED", folder.join("copied"))
            .env("XDG_STATE_HOME", folder.join("state"))
            .envs(set.iter().copied())
            .stdin(Stdio::null())
            // What it and its managers log, kept so a test can say no
            // secret was logged.
            .stderr(std::fs::File::create(folder.join("logged")).unwrap())
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

    pub fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        Client {
            reading: BufReader::new(stream.try_clone().unwrap()),
            writing: stream,
        }
    }

    pub fn copied(&self) -> PathBuf {
        self.folder.join("copied")
    }

    /// What a manager was asked so far, a line an ask.
    pub fn asked_of(&self, manager: &str) -> String {
        std::fs::read_to_string(self.folder.join(format!("{manager}.log"))).unwrap_or_default()
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
    reading: BufReader<UnixStream>,
    writing: UnixStream,
}

impl Client {
    pub fn say(&mut self, asked: &Value) {
        writeln!(self.writing, "{asked}").unwrap();
    }

    pub fn hear(&mut self) -> Value {
        let mut line = String::new();
        self.reading.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("not a line of JSON: {line:?}"))
    }

    /// Asks one thing and hears its answer, passing over any other.
    pub fn ask(&mut self, asked: &Value) -> Value {
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
