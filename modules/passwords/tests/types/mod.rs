//! What the daemon types, as the shell asks for it: nothing while no
//! typist is named; the password into the window the launcher opened
//! over, on the typist's standard input alone, once the launcher has let
//! go of the keys; nothing, and no secret read, where another window has
//! them, where an agent is behind the window, among its forebears or
//! below it, a session reached by ssh or a multiplexer below it, where
//! it's X11's, where it's kitty and kitty says an agent or a multiplexer
//! runs in it, names what isn't its own or can't say what does, where
//! something named kitty isn't the kitty kitten asks, and where the keys
//! move once the secret is read;
//! and a journal line for each, naming the entry and the window's app id,
//! never the secret (ISXEG8's terms for R-type-password, 2026-11-24).
//! niri, the typist and kitty are stand-ins, and each window's process is
//! one of the test's own, its forebears made as each case needs. They run
//! in the build's sandbox: run beneath an agent, every window is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixDatagram, UnixListener};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::common::{KEEPING, Running, until};
use crate::holds_no_secret;
use serde_json::{Value, json};

/// The window the stand-in niri says has the keys, by its id.
const WINDOW: u64 = 7;

/// A stand-in typist: what it's given goes to its file, then its
/// arguments, as wtype reads `-` from its standard input.
const TYPIST: &str = r#"#!/bin/sh
cat > "$TYPED"
printf '%s\n' "$*" > "$TYPED.args"
"#;

/// A stand-in kitty's remote control: what it was asked goes to its file,
/// and it says what its `ls` file holds, failing where there's none, as
/// kitty does when it can't be reached.
const KITTEN: &str = r#"#!/bin/sh
printf '%s\n' "$*" > "$KITTY_ASKED"
cat "$KITTY_SAYS"
"#;

/// A stand-in pi, which starts one process beneath it and says its id,
/// as a fenced agent's command is a Wayland client beneath pi.
const PI: &str = r#"#!/bin/sh
sleep 1000 &
echo $! > "$1"
wait
"#;

/// Where the stand-in kitten is, in a package's bin as Nix's is, and the
/// package, where only the stand-in kitty is believed.
const KITTEN_AT: &str = "kitty-package/bin/kitten";
const KITTY_PACKAGE: &str = "kitty-package/bin";

/// What a program the desk names after another runs: it says its own id,
/// then holds on, so Linux names it as the program it stands in for.
const HOLDS: &str = "echo $$ > \"$0\"; while :; do sleep 1; done";

/// The stand-ins for one test: niri, answering from what the test sets,
/// the typist and kitty, and each process started as a window's, ended
/// as the test ends.
struct Desk {
    folder: PathBuf,
    focused: Arc<Mutex<Value>>,
    holding: Arc<Mutex<bool>>,
    // Once niri has said which window has the keys this many times, it
    // says another has, as when the keys move while the secret is read.
    moves_after: Arc<Mutex<Option<usize>>>,
    started: Vec<u32>,
}

impl Desk {
    fn new(test: &str) -> Desk {
        let folder = std::env::temp_dir().join(format!("typing-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join(KITTY_PACKAGE)).unwrap();
        for (name, script) in [
            ("typist", TYPIST),
            (KITTEN_AT, KITTEN),
            ("pi", PI),
            ("pi-spawner", PI),
        ] {
            let path = folder.join(name);
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(&path, PermissionsExt::from_mode(0o755)).unwrap();
        }
        let focused = Arc::new(Mutex::new(Value::Null));
        let holding = Arc::new(Mutex::new(false));
        let moves_after = Arc::new(Mutex::new(None));
        let listener = UnixListener::bind(folder.join("niri.sock")).expect("niri's stand-in binds");
        let (window, held, moves) = (
            Arc::clone(&focused),
            Arc::clone(&holding),
            Arc::clone(&moves_after),
        );
        std::thread::spawn(move || {
            let mut said = 0;
            for stream in listener.incoming().flatten() {
                let mut asked = String::new();
                if BufReader::new(&stream).read_line(&mut asked).is_err() {
                    continue;
                }
                let layers = if *held.lock().unwrap() {
                    json!([{"namespace": "launcher", "keyboard_interactivity": "Exclusive"}])
                } else {
                    json!([])
                };
                let answer = match asked.trim() {
                    "\"Layers\"" => json!({"Ok": {"Layers": layers}}),
                    "\"FocusedWindow\"" => {
                        said += 1;
                        let mut now = window.lock().unwrap().clone();
                        if moves.lock().unwrap().is_some_and(|after| said > after) {
                            now["id"] = json!(WINDOW + 1);
                        }
                        json!({"Ok": {"FocusedWindow": now}})
                    }
                    _ => json!({"Err": "unknown"}),
                };
                let _ = (&stream).write_all(format!("{answer}\n").as_bytes());
            }
        });
        Desk {
            folder,
            focused,
            holding,
            moves_after,
            started: Vec::new(),
        }
    }

    /// What the daemon's environment names: niri, the typist and kitty,
    /// and where the stand-ins write.
    fn env(&self) -> Vec<(String, String)> {
        let at = |name: &str| self.folder.join(name).to_string_lossy().into_owned();
        vec![
            ("NIRI_SOCKET".to_owned(), at("niri.sock")),
            ("PASSWORDS_TYPIST".to_owned(), at("typist")),
            ("PASSWORDS_KITTY".to_owned(), at(KITTEN_AT)),
            ("TYPED".to_owned(), at("typed")),
            ("KITTY_ASKED".to_owned(), at("kitty-asked")),
            ("KITTY_SAYS".to_owned(), at("kitty-says")),
        ]
    }

    /// Starts `program` with `args` away from the test, its parent gone
    /// so it's taken in above, as a window's program is, and answers its
    /// id.
    fn start(&mut self, program: &str, args: &str) -> u32 {
        let said = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("{program} {args} >/dev/null 2>&1 & echo $!"))
            .output()
            .unwrap();
        let pid: u32 = String::from_utf8_lossy(&said.stdout)
            .trim()
            .parse()
            .unwrap();
        self.started.push(pid);
        pid
    }

    /// A plain window's process, with nothing of pi above it.
    fn plain(&mut self) -> u32 {
        self.start("sleep", "1000")
    }

    /// A process beneath a stand-in pi, answering its id once it's said.
    fn beneath_pi(&mut self) -> u32 {
        self.beneath("pi")
    }

    /// A process beneath a stand-in agent's program, `agent`, answering
    /// its id once it's said.
    fn beneath(&mut self, agent: &str) -> u32 {
        let said = self.folder.join("beneath");
        let pi = self.folder.join(agent).to_string_lossy().into_owned();
        self.start(&pi, &said.to_string_lossy());
        until("pi starts its child", || {
            std::fs::read_to_string(&said).is_ok_and(|id| id.ends_with('\n'))
        });
        let pid = std::fs::read_to_string(&said)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        self.started.push(pid);
        pid
    }

    /// A window's process with a stand-in pi below it, as a terminal
    /// other than kitty running a session has, answering the window's id.
    fn above_pi(&mut self) -> u32 {
        let said = self.folder.join("beneath");
        let pi = self.folder.join("pi");
        // The `:` keeps the shell from becoming pi, so pi stays below it.
        let window = self.start(
            "/bin/sh",
            &format!("-c '\"{}\" \"{}\"; :'", pi.display(), said.display()),
        );
        until("pi starts below the window", || {
            std::fs::read_to_string(&said).is_ok_and(|id| id.ends_with('\n'))
        });
        window
    }

    /// A program named `name` with `args` after its own, a copy of the
    /// shell so Linux names it so, holding on; answers its id once it's
    /// said.
    fn named(&mut self, name: &str, args: &str) -> u32 {
        let program = self.copied(name);
        let said = self.folder.join(format!("{name}.pid"));
        self.start(
            &program,
            &format!("-c '{HOLDS}' \"{}\" {args}", said.display()),
        );
        self.said(&said, name)
    }

    /// bash, copied into the desk's folder as `name`, answering its path.
    /// bash rather than /bin/sh, which in Nix's build sandbox is busybox,
    /// which takes the name it's run by for the program to be.
    fn copied(&self, name: &str) -> String {
        let program = self.folder.join(name);
        if !program.exists() {
            let bash = std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths)
                .map(|folder| folder.join("bash"))
                .find(|bash| bash.is_file())
                .expect("bash is on the path");
            std::fs::copy(bash, &program).expect("bash is copied");
        }
        program.to_string_lossy().into_owned()
    }

    /// The id a stand-in wrote to `said`, once it has, kept to end.
    fn said(&mut self, said: &std::path::Path, name: &str) -> u32 {
        until(&format!("{name} says its id"), || {
            std::fs::read_to_string(said).is_ok_and(|id| id.ends_with('\n'))
        });
        let pid = std::fs::read_to_string(said)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        self.started.push(pid);
        pid
    }

    /// A window's process with a program named `name` below it, given
    /// `args`, as a terminal other than kitty running it has, answering the
    /// window's id.
    fn above(&mut self, name: &str, args: &str) -> u32 {
        let program = self.copied(name);
        let said = self.folder.join(format!("{name}.pid"));
        // The `:` keeps the shell from becoming the program, so it stays
        // below it.
        let window = self.start(
            "/bin/sh",
            &format!(
                "-c '\"{program}\" -c \"{}\" \"{}\" {args}; :'",
                HOLDS.replace('"', "\\\"").replace('$', "\\$"),
                said.display()
            ),
        );
        self.said(&said, name);
        window
    }

    /// A stand-in kitty, a copy of the shell in the folder kitten is in,
    /// named as Linux names kitty, running `child` as its window's process;
    /// answers kitty's id and the child's.
    fn kitty(&mut self, child: &str) -> (u32, u32) {
        let kitty = self.copied(&format!("{KITTY_PACKAGE}/kitty"));
        let said = self.folder.join("kitty-child");
        let pid = self.start(
            &kitty,
            &format!("-c '{child} & echo $! > \"{}\"; wait'", said.display()),
        );
        until("kitty is named kitty", || {
            std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|comm| comm == "kitty\n")
        });
        (pid, self.said(&said, "kitty's child"))
    }

    /// niri saying window `id`, of `pid` and `app`, has the keys.
    fn focus(&self, id: u64, pid: u32, app: &str) {
        *self.focused.lock().unwrap() = json!({"id": id, "pid": pid, "app_id": app});
    }

    /// kitty's `ls`, its focused window running these processes.
    fn kitty_runs(&self, processes: &Value) {
        let listed = json!([{"is_focused": true, "tabs": [{"is_focused": true, "windows": [
            {"is_focused": true, "foreground_processes": processes}]}]}]);
        std::fs::write(self.folder.join("kitty-says"), listed.to_string()).unwrap();
    }

    fn typed(&self) -> Option<String> {
        std::fs::read_to_string(self.folder.join("typed")).ok()
    }

    fn typed_args(&self) -> String {
        std::fs::read_to_string(self.folder.join("typed.args")).unwrap_or_default()
    }

    fn kitty_asked(&self) -> String {
        std::fs::read_to_string(self.folder.join("kitty-asked")).unwrap_or_default()
    }
}

impl Drop for Desk {
    fn drop(&mut self) {
        for pid in &self.started {
            let _ = Command::new("/bin/sh")
                .arg("-c")
                .arg(format!("kill {pid}"))
                .status();
        }
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// The daemon with the keeping manager and the desk's stand-ins, GitHub
/// listed, and its journal's stand-in bound.
fn running(test: &str, desk: &Desk, also: &[(&str, &str)]) -> (Running, UnixDatagram) {
    let env = desk.env();
    let mut set: Vec<(&str, &str)> = env.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    set.extend_from_slice(also);
    let running = Running::with_env(test, &[("keeping", KEEPING)], &set);
    let journal = running.journal();
    running
        .client()
        .ask(&json!({"id": 1, "ask": "list", "asked": true}));
    (running, journal)
}

fn type_github(running: &Running) -> Value {
    running
        .client()
        .ask(&json!({"id": 2, "ask": "type", "key": "entry:keeping:gh",
        "window": WINDOW}))
}

/// The journal's one line for the type, as its fields.
fn told(journal: &UnixDatagram) -> String {
    let mut line = vec![0; 4096];
    let length = journal
        .recv(&mut line)
        .expect("the type reached the journal");
    String::from_utf8_lossy(&line[..length]).into_owned()
}

fn refused_reading_nothing(running: &Running, refused: &Value, why: &str) {
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains(why)),
        "a refused type didn't say {why:?}: {refused}"
    );
    holds_no_secret(refused);
    let asked = running.asked_of("keeping");
    assert!(
        !asked.contains("reveal"),
        "a secret was read to type: {asked}"
    );
}

#[test]
fn nothing_is_typed_while_no_typist_is_named() {
    let mut desk = Desk::new("untyped");
    let window = desk.plain();
    desk.focus(WINDOW, window, "firefox");
    let (running, journal) = running("untyped", &desk, &[("PASSWORDS_TYPIST", "")]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "typing is off");
    assert!(desk.typed().is_none(), "something was typed");
    let line = told(&journal);
    assert!(line.contains("PASSWORDS_TYPED=didn't type"), "{line}");
}

#[test]
fn the_password_is_typed_into_the_window_on_the_typists_input_alone() {
    let mut desk = Desk::new("typed");
    let window = desk.plain();
    desk.focus(WINDOW, window, "firefox");
    let (running, journal) = running("typed", &desk, &[]);
    let typed = type_github(&running);
    assert_eq!(typed["typed"], true, "{typed}");
    holds_no_secret(&typed);
    assert_eq!(desk.typed().as_deref(), Some("hunter2"));
    assert_eq!(
        desk.typed_args(),
        "-\n",
        "the secret went somewhere but stdin"
    );
    let line = told(&journal);
    for field in [
        "PASSWORDS_TYPED=typed",
        "PASSWORDS_TYPED_TITLE=GitHub",
        "PASSWORDS_TYPED_APP_ID=firefox",
    ] {
        assert!(
            line.contains(field),
            "the journal's line lacks {field}: {line}"
        );
    }
    assert!(
        !line.contains("hunter2"),
        "the journal was told the secret: {line}"
    );
    assert!(
        !running.logged().contains("hunter2"),
        "the secret was logged"
    );
}

#[test]
fn nothing_is_typed_while_the_launcher_keeps_the_keys() {
    let mut desk = Desk::new("kept");
    let window = desk.plain();
    desk.focus(WINDOW, window, "firefox");
    *desk.holding.lock().unwrap() = true;
    let (running, _journal) = running("kept", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "the launcher kept the keys");
    assert!(desk.typed().is_none(), "it was typed into the launcher");
}

#[test]
fn nothing_is_typed_where_another_window_has_the_keys() {
    let mut desk = Desk::new("moved");
    let window = desk.plain();
    desk.focus(WINDOW + 1, window, "firefox");
    let (running, journal) = running("moved", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "another window");
    assert!(desk.typed().is_none(), "it was typed into another window");
    let line = told(&journal);
    assert!(line.contains("PASSWORDS_TYPED_APP_ID=firefox"), "{line}");
}

#[test]
fn nothing_is_typed_into_a_window_beneath_pi() {
    let mut desk = Desk::new("beneath-pi");
    let window = desk.beneath_pi();
    desk.focus(WINDOW, window, "chromium");
    let (running, journal) = running("beneath-pi", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "an agent reads that window");
    assert!(desk.typed().is_none(), "it was typed into pi's window");
    let line = told(&journal);
    assert!(line.contains("PASSWORDS_TYPED=didn't type"), "{line}");
    assert!(line.contains("PASSWORDS_TYPED_APP_ID=chromium"), "{line}");
}

#[test]
fn nothing_is_typed_into_a_window_beneath_an_agents_command() {
    // pi-spawner runs a session's commands, so a window one of them opens
    // has it above, and no pi.
    let mut desk = Desk::new("beneath-spawner");
    let window = desk.beneath("pi-spawner");
    desk.focus(WINDOW, window, "foot");
    let (running, _journal) = running("beneath-spawner", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "an agent reads that window");
    assert!(
        desk.typed().is_none(),
        "it was typed into a window an agent's command opened"
    );
}

#[test]
fn nothing_is_typed_into_a_window_with_a_session_below_it() {
    // A terminal other than kitty can't be asked what it runs, so what
    // runs below its process is read instead.
    let mut desk = Desk::new("above-pi");
    let window = desk.above_pi();
    desk.focus(WINDOW, window, "foot");
    let (running, _journal) = running("above-pi", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "an agent runs in that window");
    assert!(
        desk.typed().is_none(),
        "it was typed into a terminal running pi"
    );
}

#[test]
fn a_terminal_running_a_shell_is_typed_into_once_kitty_says_so() {
    let mut desk = Desk::new("kitty-shell");
    let (kitty, shell) = desk.kitty("sleep 1000");
    desk.focus(WINDOW, kitty, "kitty");
    desk.kitty_runs(&json!([{"pid": shell, "cmdline": ["-zsh"]}]));
    let (running, _journal) = running("kitty-shell", &desk, &[]);
    let typed = type_github(&running);
    assert_eq!(typed["typed"], true, "{typed}");
    assert_eq!(desk.typed().as_deref(), Some("hunter2"));
    assert!(
        desk.kitty_asked()
            .contains(&format!("@ --to unix:@kitty-{kitty} ls")),
        "kitty wasn't asked where it listens: {}",
        desk.kitty_asked()
    );
}

#[test]
fn nothing_is_typed_into_a_terminal_running_a_session() {
    for (test, runs) in [
        ("kitty-pi", json!(["pi"])),
        (
            "kitty-attached",
            json!(["mycelium", "--realm", "work", "attach", "2289f47b"]),
        ),
        (
            "kitty-ssh",
            json!(["ssh", "-t", "far-machine", "mycelium", "attach", "d8604594"]),
        ),
    ] {
        let mut desk = Desk::new(test);
        let (kitty, process) = desk.kitty("sleep 1000");
        desk.focus(WINDOW, kitty, "kitty");
        desk.kitty_runs(&json!([{"pid": process, "cmdline": runs}]));
        let (running, _journal) = running(test, &desk, &[]);
        let refused = type_github(&running);
        refused_reading_nothing(&running, &refused, "an agent runs in that terminal");
        assert!(
            desk.typed().is_none(),
            "{test}: it was typed into a session"
        );
    }
}

#[test]
fn nothing_is_typed_into_a_terminal_whose_multiplexer_kitty_cant_see_past() {
    for (test, runs) in [
        ("kitty-tmux", json!(["tmux", "attach"])),
        (
            "kitty-zellij",
            json!(["/nix/store/x-zellij/bin/zellij", "attach"]),
        ),
        ("kitty-screen", json!(["screen", "-r"])),
    ] {
        let mut desk = Desk::new(test);
        let (kitty, process) = desk.kitty("sleep 1000");
        desk.focus(WINDOW, kitty, "kitty");
        desk.kitty_runs(&json!([{"pid": process, "cmdline": runs}]));
        let (running, _journal) = running(test, &desk, &[]);
        let refused = type_github(&running);
        refused_reading_nothing(&running, &refused, "kitty can't see past the multiplexer");
        assert!(
            desk.typed().is_none(),
            "{test}: it was typed past a multiplexer"
        );
    }
}

#[test]
fn nothing_is_typed_into_a_terminal_that_cant_say_what_it_runs() {
    let mut desk = Desk::new("kitty-silent");
    let (kitty, _) = desk.kitty("sleep 1000");
    desk.focus(WINDOW, kitty, "kitty");
    let (running, _journal) = running("kitty-silent", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "kitty didn't say");
    assert!(
        desk.typed().is_none(),
        "it was typed into a terminal unasked"
    );
}

#[test]
fn nothing_is_typed_where_kitty_names_what_isnt_its_own() {
    // Whatever answers as kitty could name a harmless process elsewhere,
    // so what it names must be its own, and something.
    for (test, others) in [("kitty-others", true), ("kitty-nothing", false)] {
        let mut desk = Desk::new(test);
        let (kitty, _) = desk.kitty("sleep 1000");
        let elsewhere = desk.plain();
        desk.focus(WINDOW, kitty, "kitty");
        let runs = if others {
            json!([{"pid": elsewhere, "cmdline": ["-zsh"]}])
        } else {
            json!([])
        };
        desk.kitty_runs(&runs);
        let (running, _journal) = running(test, &desk, &[]);
        let refused = type_github(&running);
        refused_reading_nothing(&running, &refused, "kitty named nothing of its own");
        assert!(desk.typed().is_none(), "{test}: it was typed in");
    }
}

#[test]
fn nothing_is_typed_into_a_kitty_kitten_doesnt_come_with() {
    // A program renamed kitty, outside the package kitten comes from, is
    // neither asked nor read past as another terminal is.
    let mut desk = Desk::new("kitty-renamed");
    let window = desk.named("kitty", "");
    desk.focus(WINDOW, window, "kitty");
    let (running, _journal) = running("kitty-renamed", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "that kitty isn't the one kitten asks");
    assert_eq!(desk.kitty_asked(), "", "a renamed kitty was asked");
    assert!(desk.typed().is_none(), "it was typed into a renamed kitty");
}

#[test]
fn nothing_is_typed_into_an_x11_window() {
    let mut desk = Desk::new("x11");
    let window = desk.named("xwayland-satellite", "");
    desk.focus(WINDOW, window, "steam");
    let (running, _journal) = running("x11", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "X11's");
    assert!(desk.typed().is_none(), "it was typed into an X11 window");
}

#[test]
fn nothing_is_typed_into_a_window_with_a_multiplexer_or_a_far_session_below() {
    // A terminal other than kitty can't be asked what it runs, so what
    // runs below it is read past a session reached by ssh or a multiplexer.
    for (test, name, args, why) in [
        (
            "above-tmux",
            "tmux",
            "",
            "a multiplexer runs in that window",
        ),
        (
            "above-ssh",
            "ssh",
            "-t far-machine mycelium attach d8604594",
            "an agent runs in that window",
        ),
    ] {
        let mut desk = Desk::new(test);
        let window = desk.above(name, args);
        desk.focus(WINDOW, window, "foot");
        let (running, _journal) = running(test, &desk, &[]);
        let refused = type_github(&running);
        refused_reading_nothing(&running, &refused, why);
        assert!(desk.typed().is_none(), "{test}: it was typed in");
    }
}

#[test]
fn nothing_is_typed_into_a_terminal_with_a_session_below_what_kitty_names() {
    // kitty names only what runs in front, so pi run from a shell in
    // front, as from nvim's terminal, is read below it.
    let mut desk = Desk::new("kitty-below");
    let shell = desk.folder.join("zsh");
    let at = desk.folder.display().to_string();
    std::fs::write(
        &shell,
        format!("#!/bin/sh\n\"{at}/pi\" \"{at}/beneath\"\n:\n"),
    )
    .unwrap();
    std::fs::set_permissions(&shell, PermissionsExt::from_mode(0o755)).unwrap();
    let (kitty, front) = desk.kitty(&shell.display().to_string());
    until("pi starts below the shell", || {
        std::fs::read_to_string(desk.folder.join("beneath")).is_ok_and(|id| id.ends_with('\n'))
    });
    desk.focus(WINDOW, kitty, "kitty");
    desk.kitty_runs(&json!([{"pid": front, "cmdline": ["-zsh"]}]));
    let (running, _journal) = running("kitty-below", &desk, &[]);
    let refused = type_github(&running);
    refused_reading_nothing(&running, &refused, "an agent runs in that terminal");
    assert!(desk.typed().is_none(), "it was typed above pi");
}

#[test]
fn nothing_is_typed_where_the_keys_move_once_the_secret_is_read() {
    // niri says the window has the keys once, then that another has, as
    // when Joel's focus moves while the manager answers.
    let mut desk = Desk::new("moves-after");
    let window = desk.plain();
    desk.focus(WINDOW, window, "firefox");
    *desk.moves_after.lock().unwrap() = Some(1);
    let (running, _journal) = running("moves-after", &desk, &[]);
    let refused = type_github(&running);
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("another window")),
        "the move wasn't said: {refused}"
    );
    holds_no_secret(&refused);
    let asked = running.asked_of("keeping");
    assert!(
        asked.contains("\"reveal\""),
        "the secret wasn't read, so the second look went untried: {asked}"
    );
    assert!(desk.typed().is_none(), "it was typed once the keys moved");
}
