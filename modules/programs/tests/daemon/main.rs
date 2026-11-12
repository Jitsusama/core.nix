//! The daemon over its socket, with data folders and sources of its own:
//! hello says how it stands on the socket systemd hands it; an entry is
//! said whole, what it opens with each kind's globs and description; an
//! entry changed while watched is said again and one removed said gone;
//! a program's list is read as its watch begins and again as its file
//! changes, and not while nobody watches, and one whose command fails
//! is heard empty, saying why; a page's own list is read only as its
//! key is watched, its lines as printed, and a told one only once its
//! program says it changed; what a program has open is read as it's
//! asked; what a file shows is asked of one file-preview kept; and an ask
//! or kind it doesn't know is refused. Each module beside this one tests
//! one more of what it serves, over the same daemon: the repositories
//! (repositories.rs), what kitty's page reads (kitty.rs), Programs' rows
//! (rows.rs), their search and what programs tell (search.rs), Capture's
//! (taken.rs), and what Chrome has open (chrome.rs).

mod chrome;
mod kitty;
mod repositories;
mod rows;
mod search;
mod taken;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use daemon_client::{Client, Daemon};
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
    write("shots/2026-11-10 09-00-00.png", "shot");
    write("shots/.hidden.png", "");
    write("lists/playing", "412\t/films/a.mkv\n");
    write("lists/open", "one\n");
    // Chrome's profile, its session folder there before it's watched.
    std::fs::create_dir_all(folder.join("chrome/Sessions")).unwrap();
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
    let downloads = format!(
        "echo read >> {} && printf '/d/a.pdf\t12\t1793000000\thttps://a.example\n'",
        at("lists/download-reads")
    );
    let open = format!(
        "echo read >> {} && cat {}",
        at("lists/open-reads"),
        at("lists/open")
    );
    let sources = json!([
        // What a terminal has open, read again only once it says so.
        {
            "app": "terminal",
            "name": "open",
            "asked": true,
            "told": true,
            "list": ["sh", "-c", open],
        },
        // What was taken lately, of a folder that's there and one that
        // isn't yet, read with no command.
        {"name": "taken", "folders": [at("shots"), at("recordings")]},
        {
            "app": "browser",
            "name": "downloads",
            "asked": true,
            "list": ["sh", "-c", downloads],
        },
        {
            "app": "kitty",
            "kind": "folder",
            "watch": at("lists/folders"),
            "list": ["sh", "-c", folders],
        },
        {"app": "mpv", "kind": "file", "opened": ["cat", at("lists/playing")]},
        {"app": "chrome", "kind": "site", "session": {"profile": at("chrome"), "host": true}},
        // A list whose command fails, as one whose program isn't set up.
        {"app": "broken", "kind": "file", "list": ["false"]},
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
    previewer(&folder.join("file-preview"));
    std::fs::write(folder.join("words.json"), WORDS).unwrap();
    file
}

/// The launcher's words Capture's rows are said in, as words.json holds them.
const WORDS: &str = r#"{"things": [
  {"id": "capture:recent", "said": {"in": "in {folder}", "picture": "picture",
    "video": "video", "silent": "not answering"}},
  {"id": "capture:taken", "name": "Taken"},
  {"id": "capture:dimensions", "name": "Dimensions"},
  {"id": "capture:length", "name": "Length"},
  {"id": "file-chooser:size", "name": "Size"},
  {"id": "path:file", "glyph": "file"}
]}"#;

/// A stand-in for file-preview, kept (serve), answering each file asked
/// with its path and the process that answered, so a case sees whether
/// one kept was asked, and never answering a file named stuck, as one
/// whose read never ends, waiting on a child of its own as file-preview
/// waits on pdftoppm, its process kept beside it (`.waiting`). It runs
/// on the shell the path finds, as the build has no /bin/sh.
fn previewer(path: &Path) {
    let shell = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|folder| Path::new(folder).join("sh"))
        .find(|shell| shell.exists())
        .expect("no sh on the path");
    let script = r#"#!SHELL
while IFS= read -r line; do
  case $line in *stuck*) sleep infinity & echo $! > "$0.waiting"; wait ;; esac
  printf '{"picture": "%s", "by": %s, "across": 4, "down": 3}\n' "${line#show }" $$
done
"#
    .replace("SHELL", &shell.display().to_string());
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

struct Running {
    daemon: Daemon,
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
        command
            .arg(&socket)
            .arg(&sources)
            .arg(folder.join("words.json"))
            .env("HOME", &folder)
            .env("XDG_RUNTIME_DIR", &folder)
            .env("XDG_DATA_HOME", folder.join("near"))
            .env("XDG_DATA_DIRS", folder.join("far"))
            .env("PROGRAMS_PREVIEW", folder.join("file-preview"))
            .stderr(Stdio::null());
        Running {
            daemon: Daemon::start(&mut command, socket, folder),
        }
    }

    fn path(&self, path: &str) -> PathBuf {
        self.daemon.folder().join(path)
    }

    fn reads(&self) -> usize {
        std::fs::read_to_string(self.path("lists/reads")).map_or(0, |text| text.lines().count())
    }

    fn client(&self) -> Client {
        self.daemon.client()
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
    assert_eq!(viewer["application"], true);
    assert_eq!(viewer["no_display"], false);
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
            .ends_with("near/applications/viewer.desktop"),
        "the viewer's entry is the nearer one: {}",
        viewer["entry_path"]
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
fn a_list_whose_command_fails_is_heard_empty_saying_why() {
    let running = Running::new("broken");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["recent:broken"]}));
    let recent = client.changed("recent:broken");
    assert_eq!(recent["paths"], json!([]));
    assert!(recent["refused"].is_string(), "{recent}");
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
fn a_told_list_is_read_again_only_once_its_program_says_it_changed() {
    let running = Running::new("told");
    let reads = || {
        std::fs::read_to_string(running.path("lists/open-reads"))
            .map_or(0, |text| text.lines().count())
    };
    let key = "listed:terminal:open";
    let mut client = running.client();
    let get = |client: &mut Client, id: u64| {
        client.ask(&json!({"id": id, "ask": "get", "keys": [key]}))["records"][0]["lines"].clone()
    };
    assert_eq!(get(&mut client, 1), json!(["one"]));
    assert_eq!(get(&mut client, 2), json!(["one"]));
    assert_eq!(reads(), 1, "a told list was read again with nothing said");
    std::fs::write(running.path("lists/open"), "one\ntwo\n").unwrap();
    let heard = client.ask(&json!({"id": 3, "ask": "changed", "keys": [key]}));
    assert_eq!(heard["heard"], json!([key]));
    assert_eq!(get(&mut client, 4), json!(["one", "two"]));
    assert_eq!(
        reads(),
        2,
        "a told list wasn't read once as it was asked after a change"
    );
    // Watched by its key, as a page open on it, a change is read at once
    // and told; the watch beginning reads nothing nobody said changed.
    client.ask(&json!({"id": 5, "ask": "watch", "keys": [key]}));
    std::fs::write(running.path("lists/open"), "three\n").unwrap();
    running
        .client()
        .ask(&json!({"id": 6, "ask": "changed", "keys": [key]}));
    assert_eq!(client.changed(key)["lines"], json!(["three"]));
    assert_eq!(
        reads(),
        3,
        "a watch beginning read a told list nobody said changed"
    );
}

#[test]
fn a_told_list_naming_a_process_that_never_told_is_read_as_each_ask_comes() {
    let running = Running::new("untold");
    let reads = || {
        std::fs::read_to_string(running.path("lists/open-reads"))
            .map_or(0, |text| text.lines().count())
    };
    let key = "listed:terminal:open";
    // Two terminals, as two kitties: one started before its watcher, which
    // never tells, and one that does.
    std::fs::write(running.path("lists/open"), "7\tone\n8\ttwo\n").unwrap();
    let mut client = running.client();
    let get = |client: &mut Client, id: u64| {
        client.ask(&json!({"id": id, "ask": "get", "keys": [key]}));
    };
    get(&mut client, 1);
    get(&mut client, 2);
    assert_eq!(
        reads(),
        2,
        "a list of processes that never told was held as told"
    );
    client.ask(&json!({"id": 3, "ask": "changed", "keys": [key], "from": 8}));
    get(&mut client, 4);
    get(&mut client, 5);
    assert_eq!(
        reads(),
        4,
        "a list naming a process that never told wasn't read as each ask came"
    );
    client.ask(&json!({"id": 6, "ask": "changed", "keys": [key], "from": 7}));
    get(&mut client, 7);
    get(&mut client, 8);
    assert_eq!(
        reads(),
        5,
        "once every process it names tells, it's read only once told"
    );
}

#[test]
fn a_pages_own_list_is_read_only_as_its_key_is_watched() {
    let running = Running::new("listed");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": ["recent", "listed"]}));
    client.changed("recent:kitty");
    client.ask(&json!({"id": 2, "ask": "watch", "keys": ["recent:browser"]}));
    client.changed("recent:browser");
    assert!(
        !running.path("lists/download-reads").exists(),
        "a page's downloads read with every opening or with its program's Recent"
    );
    client.ask(&json!({"id": 3, "ask": "watch", "keys": ["listed:browser:downloads"]}));
    let listed = client.changed("listed:browser:downloads");
    assert_eq!(listed["kind"], "listed");
    assert_eq!(
        listed["lines"],
        json!(["/d/a.pdf\t12\t1793000000\thttps://a.example"])
    );
}

#[test]
fn a_list_of_folders_is_read_with_no_command_and_again_as_a_file_comes() {
    let running = Running::new("folders");
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["listed:shared:taken"]}));
    let shots = running.path("shots").to_string_lossy().into_owned();
    let paths = |listed: &Value| -> Vec<String> {
        listed["lines"]
            .as_array()
            .expect("a list of folders said no lines")
            .iter()
            .map(|line| {
                let fields: Vec<&str> = line.as_str().unwrap_or_default().split('\t').collect();
                assert_eq!(
                    fields.len(),
                    3,
                    "a file's line isn't its time, size and path: {line}"
                );
                assert!(
                    fields[0].parse::<f64>().is_ok(),
                    "a file's time isn't seconds: {line}"
                );
                fields[2].to_owned()
            })
            .collect()
    };
    let listed = client.changed("listed:shared:taken");
    assert!(
        listed.get("refused").is_none(),
        "a folder not there refused the list: {listed}"
    );
    assert_eq!(paths(&listed), [format!("{shots}/2026-11-10 09-00-00.png")]);
    std::fs::write(running.path("shots/2026-11-11 01-00-00.png"), "shot").unwrap();
    let again = client.changed("listed:shared:taken");
    assert_eq!(
        paths(&again).len(),
        2,
        "a file made in a watched folder wasn't heard: {again}"
    );
}

#[test]
fn a_folder_named_by_its_key_is_read_as_its_watch_begins_and_as_a_file_comes() {
    let running = Running::new("folder");
    let pictures = running.path("pictures");
    std::fs::create_dir_all(&pictures).unwrap();
    std::fs::write(pictures.join("tidepool.jpg"), "pool").unwrap();
    let key = format!("folder:{}", pictures.display());
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "folder", "keys": [key]}));
    let count = |said: &Value| said["lines"].as_array().map_or(0, Vec::len);
    let read = client.changed(&key);
    assert_eq!(
        read["path"],
        json!(pictures.display().to_string()),
        "a folder's record didn't say its path: {read}"
    );
    assert_eq!(
        count(&read),
        1,
        "the folder's one picture wasn't said: {read}"
    );
    std::fs::write(pictures.join("cove.png"), "cove").unwrap();
    let again = client.changed(&key);
    assert_eq!(
        count(&again),
        2,
        "a file made in the folder wasn't heard: {again}"
    );
    let missing = format!("folder:{}", running.path("gone").display());
    let said = client.ask(&json!({"id": 2, "ask": "get", "keys": [missing]}));
    assert_eq!(
        said["records"][0]["lines"],
        json!([]),
        "a folder that isn't there didn't hold nothing: {said}"
    );
}

/// Runs git in a repository, failing the case where it fails.
#[test]
fn what_a_file_shows_is_asked_of_one_file_preview_kept() {
    let running = Running::new("preview");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "get", "keys": ["preview:/c.png"]}));
    let first = &said["records"][0];
    assert_eq!(
        first["answering"],
        json!(true),
        "file-preview didn't answer: {said}"
    );
    assert_eq!(
        first["shown"]["picture"],
        json!("/c.png"),
        "what it said of the file wasn't held: {said}"
    );
    let again = client.ask(&json!({"id": 2, "ask": "get", "keys": ["preview:/d.png"]}));
    let second = &again["records"][0];
    assert_eq!(
        second["shown"]["picture"],
        json!("/d.png"),
        "the second file wasn't said: {again}"
    );
    assert_eq!(
        first["shown"]["by"], second["shown"]["by"],
        "both weren't asked of one file-preview, kept"
    );
}

#[test]
fn a_file_that_file_preview_never_answers_is_said_unanswered_and_holds_up_nothing() {
    const GLANCE: Duration = Duration::from_millis(10);
    let running = Running::new("stuck");
    let mut client = running.client();
    let before = client.ask(&json!({"id": 1, "ask": "get", "keys": ["preview:/c.png"]}));
    let asked = Instant::now();
    let stuck = client.ask(&json!({"id": 2, "ask": "get", "keys": ["preview:/stuck.png"]}));
    assert_eq!(
        stuck["records"][0]["answering"],
        json!(false),
        "a file file-preview never answers isn't said as answered: {stuck}"
    );
    assert!(
        asked.elapsed() < Duration::from_secs(6),
        "it waited {:?} on a file that never answers",
        asked.elapsed()
    );
    let waiting = std::fs::read_to_string(running.path("file-preview.waiting"))
        .expect("the stuck stand-in says the child it waits on");
    let left = Path::new("/proc").join(waiting.trim()).join("stat");
    // Stopped, it's gone or a zombie waiting on its reaper, polled for as
    // the kill lands, a GLANCE apart.
    let stopped = Instant::now();
    while std::fs::read_to_string(&left).is_ok_and(|stat| !stat.contains(") Z ")) {
        assert!(
            stopped.elapsed() < Duration::from_secs(2),
            "the stuck file-preview's own child ran on: {}",
            waiting.trim()
        );
        std::thread::sleep(GLANCE);
    }
    let after = client.ask(&json!({"id": 3, "ask": "get", "keys": ["preview:/d.png"]}));
    assert_eq!(
        after["records"][0]["shown"]["picture"],
        json!("/d.png"),
        "the file after a stuck one wasn't said: {after}"
    );
    assert_ne!(
        before["records"][0]["shown"]["by"], after["records"][0]["shown"]["by"],
        "the stuck file-preview was kept rather than stopped"
    );
    let asked = Instant::now();
    let again = client.ask(&json!({"id": 4, "ask": "get", "keys": ["preview:/stuck.png"]}));
    assert_eq!(
        again["records"][0]["answering"],
        json!(false),
        "the stuck file is said as answered: {again}"
    );
    assert!(
        asked.elapsed() < Duration::from_secs(1),
        "an unchanged stuck file was asked again, waiting {:?}",
        asked.elapsed()
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
