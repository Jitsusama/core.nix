//! What the daemon's tests and the command's share: the daemon running
//! with stand-in password managers of its own, a client on its socket, and
//! a wait on something coming to hold.

use std::io::{BufRead, BufReader, Write};
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
