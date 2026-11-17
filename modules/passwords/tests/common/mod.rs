//! What the daemon's tests share: the daemon running with stand-in
//! password managers of its own, asked over its standard input and output
//! as the shell asks it, a stand-in for Have I Been Pwned on a local port,
//! and a wait on something coming to hold.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixDatagram;
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
/// value that looks it, said only to a release `asked` as his. It lists
/// one entry, so a watch is seen to hear entries apart from their rows.
pub const RELEASING: &str = r#"
log=$1
entry() { printf '{"id":"%s","title":"%s","category":"login","version":1}' "$1" "$2"; }
echo '{"state":"answering"}'
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s]}\n' "$id" "$(entry gh GitHub)" ;;
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
/// SSH's settings and public keys, naming npm's settings, and emptying
/// Docker's settings and another's only where they hold a credential.
pub const FENCE_LIST: &str = r#"{
  "version": 1,
  "home": [".secret-file", ".secret-folder/", ".linked-secret", ".absent-file"],
  "sshKept": ["^(config|known_hosts(\\.old)?|.*\\.pub)$"],
  "npmrc": ".npmrc",
  "emptied": [
    {"path": ".docker/config.json",
     "when": ["auths", "credsStore", "credHelpers", "HttpHeaders", "proxies"]},
    {"path": ".kept-settings.json", "when": ["auths"]}
  ]
}"#;

/// Docker's settings with a registry's made-up login.
pub const DOCKER_CONFIG: &str =
    r#"{"auths": {"registry.example": {"auth": "ZG9ja2VyLW1hZGUtdXA="}}}"#;

/// Settings naming a registry with no login, which nothing empties.
pub const KEPT_SETTINGS: &str = r#"{"auths": {"registry.example": {}}}"#;

/// npm's settings with a registry and its made-up token.
pub const NPMRC: &str = "registry=https://registry.example/\n\
                         //registry.example/:_authToken=npm-made-up-token\n";

/// The made-up value RELEASING releases, which no answer, log or
/// command line may hold.
pub const RELEASED_VALUE: &str = "tr0ub4dor-made-up-for-the-test";

/// The stand-in for journald's socket, in the folder the daemon works in.
const JOURNAL: &str = "journal.sock";

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

/// Runs the git the daemon is handed in `tree`, as the test's own, with
/// no settings of Joel's, so its commits are made unsigned and what it
/// makes is what the daemon's git reads.
pub fn git_in(tree: &std::path::Path, args: &[&str]) {
    let ran = Command::new(stored_folder("git").join("git"))
        .arg("-C")
        .arg(tree)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?} failed: {ran:?}");
}

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
            // The daemon runs git with no PATH of its own, as a machine
            // hands it git's whole path, so a bare name would read no tree.
            "git": stored_folder("git").join("git"),
            "bwrap": bwrap,
            "references": {"1password": "op://"},
            "profiles": [stored_folder("bash"), folder.join("home/.nix-profile/bin")],
            "hidden": folder.join("fence-hidden.json"),
            "overlays": overlays,
            // The stand-in forge a yes stands for, the folder of bare
            // repositories a test's tree takes as origin, as a machine
            // names GitHub.
            "forges": [format!("file://{}/", folder.join("forge").display())],
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
        for folder in [".secret-folder", ".ssh", ".cache", ".docker"] {
            std::fs::create_dir_all(home.join(folder)).unwrap();
        }
        for (file, held) in [
            (".secret-file", "a token"),
            (".secret-folder/inside", "a key"),
            (".ssh/id_ed25519", "a private key"),
            (".ssh/id_ed25519.pub", "a public key"),
            (".ssh/config", "Host *"),
            (".npmrc", NPMRC),
            (".docker/config.json", DOCKER_CONFIG),
            (".kept-settings.json", KEPT_SETTINGS),
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
        // The readings go to a stand-in for journald in the test's own
        // folder, which the daemon works in, never to the account's
        // journal, whose health counts them. It's named from the folder,
        // since a socket's whole path may be no longer than 107 bytes,
        // and nothing hears it unless a test binds it with `journal`.
        let mut file = json!({"managers": listed, "journal": JOURNAL});
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
            .current_dir(&folder)
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

    /// The stand-in for journald the daemon tells its readings to, bound
    /// through the folder's descriptor, since the folder's own path may
    /// be too long for a socket's. Bound only where a test reads it: a
    /// stand-in no one reads would fill and hold the daemon's next line.
    pub fn journal(&self) -> UnixDatagram {
        let folder = std::fs::File::open(&self.folder).unwrap();
        let path = format!("/proc/self/fd/{}/{JOURNAL}", folder.as_raw_fd());
        let journal = UnixDatagram::bind(path).expect("the journal stand-in binds");
        // A reading comes as the ask ends, so a few seconds is long.
        journal
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        journal
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
