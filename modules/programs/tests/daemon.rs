//! The daemon over its socket, with data folders and sources of its own:
//! hello says how it stands on the socket systemd hands it; an entry is
//! said whole, what it opens with each kind's globs and description; an
//! entry changed while watched is said again and one removed said gone;
//! a program's list is read as its watch begins and again as its file
//! changes, and not while nobody watches; what a program has open is
//! read as it's asked; and an ask or kind it doesn't know is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const VIEWER: &str = "[Desktop Entry]
Type=Application
Name=Viewer
Name[fr]=Visionneuse
GenericName=Document Viewer
Comment=Reads documents
Exec=viewer --fork %U
Categories=Office;Viewer;
Keywords=pdf;book;
Path=/srv
StartupWMClass=viewer-window
MimeType=application/pdf;x-scheme-handler/doc;
Actions=new;

[Desktop Action new]
Name=New Window
Exec=viewer --new
";

/// Data folders of a viewer's entry and the kinds it opens, and a
/// program's sources: a folder list read from a file it watches, each
/// read counted, and what an mpv has open.
fn fixture(folder: &Path) -> PathBuf {
    let write = |path: &str, text: &str| {
        let file = folder.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    write("near/applications/viewer.desktop", VIEWER);
    write(
        "far/applications/viewer.desktop",
        "[Desktop Entry]\nName=Hidden By The Near One\n",
    );
    write(
        "far/mime/globs2",
        "# made by hand\n50:application/pdf:*.pdf\n60:application/pdf:*.PDF\n",
    );
    write(
        "far/mime/application/pdf.xml",
        concat!(
            "<mime-type><comment xml:lang=\"fr\">Document</comment>",
            "<comment>PDF document</comment></mime-type>"
        ),
    );
    write("lists/folders", "/home/joel/src\n/home/joel\n");
    write("lists/playing", "412\t/films/a.mkv\n");
    let at = |path: &str| folder.join(path).to_string_lossy().into_owned();
    // Each list says it was read, so a test counts its reads.
    let folders = format!(
        "echo read >> {} && cat {}",
        at("lists/reads"),
        at("lists/folders")
    );
    let sites = format!(
        "echo read >> {} && printf 'https://a.example\\tA\\t1793000000\\n'",
        at("lists/site-reads")
    );
    let sources = json!([
        {
            "app": "kitty",
            "kind": "folder",
            "watch": at("lists/folders"),
            "list": ["sh", "-c", folders],
        },
        {"app": "mpv", "kind": "file", "opened": ["cat", at("lists/playing")]},
        {
            "app": "browser",
            "kind": "site",
            "asked": true,
            "dated": true,
            "list": ["sh", "-c", sites],
        },
    ]);
    let file = folder.join("sources.json");
    std::fs::write(&file, sources.to_string()).unwrap();
    file
}

struct Running {
    daemon: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Running {
    fn new(test: &str) -> Running {
        Running::through(test, &[])
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, launcher: &[&str]) -> Running {
        let folder = std::env::temp_dir().join(format!("programs-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let sources = fixture(&folder);
        let socket = folder.join("daemons").join("programs.sock");
        std::fs::create_dir_all(folder.join("daemons")).unwrap();
        let program = env!("CARGO_BIN_EXE_programs");
        let mut command = match launcher.split_first() {
            Some((first, arguments)) => {
                let mut command = Command::new(first);
                for argument in arguments {
                    command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
                }
                command.arg(program);
                command
            }
            None => Command::new(program),
        };
        let daemon = command
            .arg(&socket)
            .arg(&sources)
            .env("XDG_DATA_HOME", folder.join("near"))
            .env("XDG_DATA_DIRS", folder.join("far"))
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        until("the socket never came", || socket.exists());
        Running {
            daemon,
            socket,
            folder,
        }
    }

    fn path(&self, path: &str) -> PathBuf {
        self.folder.join(path)
    }

    fn reads(&self) -> usize {
        std::fs::read_to_string(self.path("lists/reads"))
            .map(|text| text.lines().count())
            .unwrap_or(0)
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Client {
            lines: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
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
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// The answer to an ask, passing over what a watch pushed meanwhile.
    fn ask(&mut self, asked: &Value) -> Value {
        writeln!(self.stream, "{asked}").unwrap();
        loop {
            let said = self.next();
            if said["id"] == asked["id"] && said.get("changed").is_none() {
                return said;
            }
        }
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.lines.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    /// The next change a watch is told of the record with this key.
    fn changed(&mut self, key: &str) -> Value {
        loop {
            let said = self.next();
            let found = said["changed"]
                .as_array()
                .and_then(|records| records.iter().find(|record| record["key"] == key));
            if let Some(record) = found {
                return record.clone();
            }
        }
    }

    /// The next change saying keys went.
    fn gone(&mut self) -> Vec<Value> {
        loop {
            let said = self.next();
            if let Some(gone) = said["gone"].as_array() {
                return gone.clone();
            }
        }
    }
}

fn until(what: &str, held: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !held() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn hello_says_it_answers_on_the_socket_systemd_hands_it() {
    let running = Running::through("hello", &["systemd-socket-activate", "--listen={socket}"]);
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "hello", "version": 1}));
    assert_eq!(said["domain"], "programs");
    assert_eq!(said["version"], 1);
    assert_eq!(said["state"], "answering");
}

#[test]
fn an_entry_is_said_whole_from_the_nearest_folder() {
    let running = Running::new("entry");
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "get", "keys": ["program:viewer"]}));
    let viewer = &said["records"][0];
    assert_eq!(viewer["name"], "Viewer");
    assert_eq!(viewer["generic_name"], "Document Viewer");
    assert_eq!(viewer["comment"], "Reads documents");
    assert_eq!(viewer["exec"], json!(["viewer", "--fork", "%U"]));
    assert_eq!(viewer["categories"], json!(["Office", "Viewer"]));
    assert_eq!(viewer["keywords"], json!(["pdf", "book"]));
    assert_eq!(viewer["starts_in"], "/srv");
    assert_eq!(viewer["window_class"], "viewer-window");
    assert_eq!(viewer["terminal"], false);
    assert_eq!(
        viewer["actions"],
        json!([{"id": "new", "name": "New Window", "exec": ["viewer", "--new"]}])
    );
    assert_eq!(
        viewer["opens"],
        json!([{"type": "application/pdf", "globs": ["*.PDF", "*.pdf"], "comment": "PDF document"}])
    );
    assert_eq!(viewer["handles"], json!(["doc"]));
    assert_eq!(viewer["package"], Value::Null);
    assert!(
        viewer["entry_path"]
            .as_str()
            .unwrap()
            .ends_with("near/applications/viewer.desktop")
    );
}

#[test]
fn an_entry_changed_is_said_again_and_one_removed_said_gone() {
    let running = Running::new("changed");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "watch", "keys": ["program:viewer"]}));
    assert_eq!(said["records"][0]["name"], "Viewer");
    let entry = running.path("near/applications/viewer.desktop");
    std::fs::write(&entry, VIEWER.replace("Name=Viewer", "Name=Reader")).unwrap();
    assert_eq!(client.changed("program:viewer")["name"], "Reader");
    std::fs::remove_file(&entry).unwrap();
    // The far folder's entry of that id stands in once the near one goes.
    assert_eq!(
        client.changed("program:viewer")["name"],
        "Hidden By The Near One"
    );
    std::fs::remove_file(running.path("far/applications/viewer.desktop")).unwrap();
    assert_eq!(client.gone(), [json!("program:viewer")]);
}

#[test]
fn a_list_is_read_as_its_watch_begins_and_as_its_file_changes() {
    let running = Running::new("list");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["recent:kitty"]}));
    let recent = client.changed("recent:kitty");
    assert_eq!(recent["of"], "folder");
    assert_eq!(
        recent["paths"],
        json!([
            {"path": "/home/joel/src", "used_at_s": null},
            {"path": "/home/joel", "used_at_s": null}
        ])
    );
    std::fs::write(running.path("lists/folders"), "/tmp\n").unwrap();
    assert_eq!(
        client.changed("recent:kitty")["paths"],
        json!([{"path": "/tmp", "used_at_s": null}])
    );
}

#[test]
fn a_list_isnt_read_while_nobody_watches_it() {
    let running = Running::new("unwatched");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["recent:kitty"]}));
    client.changed("recent:kitty");
    client.ask(&json!({"id": 1, "ask": "unwatch"}));
    let before = running.reads();
    std::fs::write(running.path("lists/folders"), "/a\n").unwrap();
    std::fs::write(running.path("lists/folders"), "/b\n").unwrap();
    // A get afterwards reads it once, its file having changed meanwhile.
    let said = client.ask(&json!({"id": 2, "ask": "get", "keys": ["recent:kitty"]}));
    assert_eq!(
        said["records"][0]["paths"],
        json!([{"path": "/b", "used_at_s": null}])
    );
    assert_eq!(running.reads(), before + 1);
}

#[test]
fn an_asked_list_is_read_only_as_its_program_is_watched() {
    let running = Running::new("asked");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "recent"}));
    client.changed("recent:kitty");
    assert!(
        !running.path("lists/site-reads").exists(),
        "a browser's history read with every opening"
    );
    client.ask(&json!({"id": 2, "ask": "watch", "keys": ["recent:browser"]}));
    assert_eq!(
        client.changed("recent:browser")["sites"],
        json!([{"address": "https://a.example", "title": "A", "used_at_s": 1_793_000_000}])
    );
}

#[test]
fn what_a_program_has_open_is_read_as_its_asked() {
    let running = Running::new("opened");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "get", "keys": ["opened:mpv"]}));
    assert_eq!(
        said["records"][0]["files"],
        json!([{"pid": 412, "window": null, "path": "/films/a.mkv", "go": null}])
    );
    std::fs::write(running.path("lists/playing"), "").unwrap();
    let said = client.ask(&json!({"id": 2, "ask": "get", "keys": ["opened:mpv"]}));
    assert_eq!(said["records"][0]["files"], json!([]));
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused() {
    let running = Running::new("refused");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "fly"}));
    assert_eq!(said["error"]["code"], "unknown-ask");
    let said = client.ask(&json!({"id": 2, "ask": "list", "kind": "disk"}));
    assert_eq!(said["error"]["code"], "unknown-kind");
}
