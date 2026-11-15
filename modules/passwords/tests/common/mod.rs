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

/// The fence's list as the harness publishes it, naming a file, a folder,
/// a link to a file outside home and a file that isn't there, keeping
/// SSH's settings and public keys, and naming npm's settings.
pub const FENCE_LIST: &str = r#"{
  "version": 1,
  "home": [".secret-file", ".secret-folder/", ".linked-secret", ".absent-file"],
  "sshKept": ["^(config|known_hosts(\\.old)?|.*\\.pub)$"],
  "npmrc": ".npmrc"
}"#;

/// npm's settings with a registry and its made-up token.
pub const NPMRC: &str = "registry=https://registry.example/\n\
                         //registry.example/:_authToken=npm-made-up-token\n";

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

/// The store's folder holding the program the tests' PATH names `name`,
/// as its links come to.
pub fn stored_folder(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .filter_map(|folder| std::fs::canonicalize(folder.join(name)).ok())
        .find(|program| program.starts_with("/nix/store"))
        .and_then(|program| program.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| panic!("no {name} in the store on PATH"))
}

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
    /// and a runtime folder and home of its own, whose `secret-asks`
    /// holds the asks (`asks`) and whose home is the account's (`home`).
    /// Its profiles are the store's folder holding the tests' bash, as a
    /// machine's are root's folders of the programs it chose, and then
    /// `planted_profile`, which a test may plant as the account's own.
    /// The fence's list it hides by is `fence_list`, as `FENCE_LIST` gives
    /// it, and its home holds one of each thing the list names, beside a
    /// cache folder. Its bubblewrap lays throwaway layers over folders.
    pub fn releasing(test: &str, releases: &[&str]) -> Running {
        Running::releasing_as(test, releases, true)
    }

    /// A daemon releasing as `releasing` does, whose bubblewrap lays
    /// throwaway layers over folders only where `overlays` says.
    pub fn releasing_as(test: &str, releases: &[&str], overlays: bool) -> Running {
        let words = std::env::var("LAUNCHER_WORDS")
            .expect("LAUNCHER_WORDS names no vocabulary; passwords-holds-no-secret hands it one");
        let folder = Running::folder_of(test);
        let bwrap = folder.join("bwrap");
        let runtime = folder.join("runtime");
        let home = folder.join("home");
        let also = json!({
            "releases": releases,
            "git": "git",
            "bwrap": bwrap,
            "references": {"1password": "op://"},
            "profiles": [stored_folder("bash"), folder.join("home/.nix-profile/bin")],
            "hidden": folder.join("fence-hidden.json"),
            "overlays": overlays,
        });
        let running = Running::started_with(
            test,
            &[("1password", RELEASING)],
            &[
                ("XDG_RUNTIME_DIR", runtime.to_str().unwrap()),
                ("HOME", home.to_str().unwrap()),
                ("RELEASED_VALUE", RELEASED_VALUE),
            ],
            &words,
            &also,
        );
        std::fs::write(&bwrap, format!("#!/bin/sh\n{BWRAP}")).unwrap();
        std::fs::set_permissions(&bwrap, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        std::fs::write(running.fence_list(), FENCE_LIST).unwrap();
        for folder in [".secret-folder", ".ssh", ".cache"] {
            std::fs::create_dir_all(home.join(folder)).unwrap();
        }
        for (file, held) in [
            (".secret-file", "a token"),
            (".secret-folder/inside", "a key"),
            (".ssh/id_ed25519", "a private key"),
            (".ssh/id_ed25519.pub", "a public key"),
            (".ssh/config", "Host *"),
            (".npmrc", NPMRC),
        ] {
            std::fs::write(home.join(file), held).unwrap();
        }
        std::fs::write(folder.join("outside-secret"), "a credential").unwrap();
        std::os::unix::fs::symlink(folder.join("outside-secret"), home.join(".linked-secret"))
            .unwrap();
        running
    }

    /// Where a releasing daemon reads the fence's list from.
    pub fn fence_list(&self) -> PathBuf {
        self.folder.join("fence-hidden.json")
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
        let socket = folder.join("daemons/passwords.sock");
        let daemon = Command::new(env!("CARGO_BIN_EXE_passwords"))
            .arg("daemon")
            .arg(&socket)
            .arg(&managers_file)
            .arg(words)
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

    /// The folder a releasing daemon's asks are put up in.
    pub fn asks(&self) -> PathBuf {
        self.folder.join("runtime").join("secret-asks")
    }

    /// The profile a releasing daemon lists last, in its home as
    /// `~/.nix-profile` is: not there unless a test plants it.
    pub fn planted_profile(&self) -> PathBuf {
        self.folder.join("home/.nix-profile/bin")
    }

    /// The arguments the stand-in bubblewrap was last given, a line each.
    pub fn bwrapped(&self) -> String {
        std::fs::read_to_string(self.folder.join("bwrap.args")).unwrap_or_default()
    }

    /// What the daemon and its managers logged so far.
    pub fn logged(&self) -> String {
        std::fs::read_to_string(self.folder.join("logged")).unwrap_or_default()
    }

    /// The daemon's pid.
    pub fn pid(&self) -> u32 {
        self.daemon.id()
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
