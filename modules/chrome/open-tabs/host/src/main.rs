//! chrome-open-tabs: Chrome's open tabs, for its Recent in the launcher.
//!
//! Chrome tells only an extension which tabs are open, so the extension of
//! ours beside this (`../extension`) answers this program, and this program
//! answers the launcher. Chrome starts it as the extension's native-messaging
//! host, naming the extension's origin first: it then listens on
//! `$XDG_RUNTIME_DIR/chrome-open-tabs.sock`, readable by its account alone,
//! and asks the extension each thing it's asked there, over Chrome's own
//! framing (a length in four bytes, then JSON), until Chrome closes its
//! input, when the socket goes.
//!
//! Run by hand, it asks that socket:
//!
//! - `chrome-open-tabs tabs` prints each tab in Chrome's own windows, one a
//!   line: its id, its window, whether it's the window's tab in front, when
//!   it was last looked at (milliseconds since 1970), its address and its
//!   title, apart by tabs;
//! - `chrome-open-tabs tabs ID` says that one tab's facts;
//! - `chrome-open-tabs tabs ID go` brings it to the front of its window;
//! - `chrome-open-tabs tabs ID close` closes it.
//!
//! `list`, `go ID` and `close ID`, as it was asked before the tabs took a
//! word of their own, still answer, unlisted; the socket's own lines are
//! those words, since the programs daemon asks it so.
//!
//! An ID is one `tabs` gave, the id Chrome's tabs have for an extension;
//! one missing or not a number is refused, so a tab told of by Chrome's
//! session file, which `chrome-recent-sites open` gives no id, is never
//! gone to or closed by another's.
//!
//! With Chrome closed, or the extension not answering in time, it prints
//! nothing and succeeds, so the launcher lists what was open lately alone.
//!
//! Run by hand, it takes the shape every command of ours does
//! (docs/commands.md), through the terminal kit. `list`'s lines are read by
//! the launcher, so they stay as they are; with `--json`, it answers as
//! JSON instead, with RFC 3339 times. Chrome's own words, as
//! `--parent-window=0`, pass through the kit to the host untouched.

use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value as Json, json};
use terminal_kit::{Context, Exit, Failure, Help, layout, time};

/// The socket's name in the account's runtime folder.
const SOCKET: &str = "chrome-open-tabs.sock";
/// How long an ask waits on the extension, as the launcher waits on it.
const WAITING: Duration = Duration::from_secs(2);
/// The largest message Chrome sends a host.
const LONGEST: u32 = 64 * 1024 * 1024;

/// What the launcher asks.
#[derive(Debug, PartialEq)]
enum Ask {
    List,
    Go(u64),
    Close(u64),
}

impl Ask {
    /// The ask a line says: `list`, `go ID` or `close ID`.
    fn from_line(line: &str) -> Option<Ask> {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["list"] => Some(Ask::List),
            ["go", tab] => tab.parse().ok().map(Ask::Go),
            ["close", tab] => tab.parse().ok().map(Ask::Close),
            _ => None,
        }
    }

    fn line(&self) -> String {
        match self {
            Ask::List => "list".to_string(),
            Ask::Go(tab) => format!("go {tab}"),
            Ask::Close(tab) => format!("close {tab}"),
        }
    }

    /// The message the extension is sent for it, carrying `id` back.
    fn message(&self, id: u64) -> Json {
        match self {
            Ask::List => json!({ "id": id, "ask": "list" }),
            Ask::Go(tab) => json!({ "id": id, "ask": "go", "tab": tab }),
            Ask::Close(tab) => json!({ "id": id, "ask": "close", "tab": tab }),
        }
    }
}

/// One message in Chrome's framing: its length in four bytes in the
/// machine's own order, then its JSON. None once the input ends.
fn read_message(input: &mut impl Read) -> io::Result<Option<Json>> {
    let mut length = [0u8; 4];
    match input.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = u32::from_ne_bytes(length);
    if length > LONGEST {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a message of {length} bytes is longer than Chrome sends"),
        ));
    }
    let length = usize::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a message too long to hold"))?;
    let mut body = vec![0u8; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_message(output: &mut impl Write, message: &Json) -> io::Result<()> {
    let body = message.to_string();
    let length = u32::try_from(body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a message too long to frame"))?;
    output.write_all(&length.to_ne_bytes())?;
    output.write_all(body.as_bytes())?;
    output.flush()
}

/// A field said whole on one line: its tabs and line breaks as spaces.
fn flat(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

/// The lines an answer gives the launcher: a tab a line for a list, none
/// for anything else.
fn lines(answer: &Json) -> String {
    let Some(tabs) = answer.get("tabs").and_then(Json::as_array) else {
        return String::new();
    };
    let mut said = String::new();
    for tab in tabs {
        let (Some(id), Some(window)) = (
            tab.get("id").and_then(Json::as_u64),
            tab.get("window").and_then(Json::as_u64),
        ) else {
            continue;
        };
        let active = tab.get("active").and_then(Json::as_bool).unwrap_or(false);
        let looked = tab
            .get("lastAccessed")
            .and_then(Json::as_f64)
            .unwrap_or(0.0);
        let text = |name: &str| flat(tab.get(name).and_then(Json::as_str).unwrap_or(""));
        let _ = writeln!(
            said,
            "{id}\t{window}\t{active}\t{looked:.0}\t{}\t{}",
            text("address"),
            text("title")
        );
    }
    said
}

/// Who waits on which answer, by the id their ask carried.
type Waiting = Arc<Mutex<HashMap<u64, Sender<Json>>>>;

fn waiting(asks: &Waiting) -> std::sync::MutexGuard<'_, HashMap<u64, Sender<Json>>> {
    asks.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Serves the launcher on `socket` while the extension speaks on `input`
/// and `output`, and takes the socket away once the input ends.
fn host(
    mut input: impl Read,
    output: impl Write + Send + 'static,
    socket: &Path,
) -> io::Result<()> {
    let listener = claim(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;

    let asks: Waiting = Arc::default();
    let output = Arc::new(Mutex::new(output));
    let next = Arc::new(AtomicU64::new(1));
    {
        let asks = Arc::clone(&asks);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (asks, output, next) =
                    (Arc::clone(&asks), Arc::clone(&output), Arc::clone(&next));
                thread::spawn(move || answer(&stream, &asks, &output, &next));
            }
        });
    }

    let ended = loop {
        match read_message(&mut input) {
            Ok(Some(said)) => {
                let Some(id) = said.get("id").and_then(Json::as_u64) else {
                    continue;
                };
                if let Some(waiter) = waiting(&asks).remove(&id) {
                    let _ = waiter.send(said);
                }
            }
            Ok(None) => break Ok(()),
            Err(error) => break Err(error),
        }
    };
    let _ = fs::remove_file(socket);
    ended
}

/// Binds `socket`, first clearing one a host left as it ended without
/// tidying, which nothing answers on. One another host still answers on
/// is that host's, so this one refuses rather than take the launcher from
/// it, as a test's Chromium once took Joel's Chrome's name and left it
/// dead (E2ZIAQ, 2026-11-11).
fn claim(socket: &Path) -> io::Result<UnixListener> {
    match UnixStream::connect(socket) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("another host answers on {}", socket.display()),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            fs::remove_file(socket)?;
        }
        Err(error) => return Err(error),
    }
    UnixListener::bind(socket)
}

/// Answers one of the launcher's asks: reads its line, asks the extension,
/// and says what it answered, or nothing once it's late.
fn answer(stream: &UnixStream, asks: &Waiting, output: &Mutex<impl Write>, next: &AtomicU64) {
    let mut line = String::new();
    if BufReader::new(stream).read_line(&mut line).is_err() {
        return;
    }
    let Some(ask) = Ask::from_line(&line) else {
        return;
    };
    let id = next.fetch_add(1, Ordering::Relaxed);
    let (told, heard) = mpsc::channel();
    waiting(asks).insert(id, told);
    let sent = {
        let mut output = output
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        write_message(&mut *output, &ask.message(id))
    };
    let answered = if sent.is_ok() {
        heard.recv_timeout(WAITING).ok()
    } else {
        None
    };
    waiting(asks).remove(&id);
    if let Some(answered) = answered {
        let mut stream = stream;
        let _ = stream.write_all(lines(&answered).as_bytes());
    }
}

/// Asks the host on `socket` and gives back its answer; with no host
/// there, nothing.
fn ask(socket: &Path, ask: &Ask) -> io::Result<String> {
    let Ok(mut stream) = UnixStream::connect(socket) else {
        return Ok(String::new());
    };
    stream.set_read_timeout(Some(WAITING + Duration::from_secs(1)))?;
    stream.write_all(format!("{}\n", ask.line()).as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut said = String::new();
    let _ = stream.read_to_string(&mut said);
    Ok(said)
}

/// `list`'s lines as JSON: each tab with its fields named, and when it was
/// looked at as a time, null where Chrome didn't say.
fn tabs(said: &str) -> Json {
    let tabs: Vec<Json> = said
        .lines()
        .filter_map(|line| {
            let [id, window, front, looked, address, title] =
                line.splitn(6, '\t').collect::<Vec<_>>()[..]
            else {
                return None;
            };
            let looked_at = looked
                .parse::<i64>()
                .ok()
                .filter(|millis| *millis > 0)
                .map(|millis| time::rfc3339(millis.div_euclid(1000)));
            Some(json!({
                "id": id.parse::<u64>().ok()?,
                "window": window.parse::<u64>().ok()?,
                "front": front == "true",
                "looked_at": looked_at,
                "address": address,
                "title": title,
            }))
        })
        .collect();
    json!({ "schema": "chrome-open-tabs.list/1", "tabs": tabs })
}

fn socket() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join(SOCKET))
}

/// The help as Chrome starts it, with no subcommands for the kit to walk,
/// since Chrome's first word is the extension's address.
fn hosting_help() -> Help {
    Help::new("chrome-open-tabs", "Chrome's open tabs, for the launcher")
        .example("chrome-open-tabs tabs", "a tab a line, apart by tabs")
        .exit(
            Exit::Failed,
            "the host couldn't serve, or the answer be said",
        )
        .exit(Exit::Unavailable, "the host's socket broke off")
        .exit(Exit::Config, "there's no runtime folder for the socket")
}

fn help() -> Help {
    hosting_help()
        .example("chrome-open-tabs tabs --json", "the same, as JSON")
        .example("chrome-open-tabs tabs 7 go", "tab 7, in front")
        .subcommand(
            asks(
                "tabs",
                "each tab: id, window, front, looked at, address, title",
            )
            .alias("tab")
            .example("chrome-open-tabs tabs", "a tab a line, apart by tabs")
            .example("chrome-open-tabs tabs 7", "tab 7's facts")
            .example("chrome-open-tabs tabs 7 close", "tab 7, closed")
            .argument("<id>", "a tab's id, as tabs gives it")
            .exit(Exit::Usage, "the id isn't a number")
            .exit(Exit::NotFound, "no tab open has that id")
            .subcommand(
                asks("go", "brings the tab to the front of its window")
                    .example("chrome-open-tabs tabs 7 go", "tab 7, in front"),
            )
            .subcommand(
                asks("close", "closes the tab")
                    .example("chrome-open-tabs tabs 7 close", "tab 7, closed"),
            ),
        )
        // How the launcher asked before the tabs took a word of their own.
        .subcommand(asks("list", "each tab").unlisted())
        .subcommand(
            asks("go", "a tab, in front")
                .argument("<id>", "")
                .unlisted(),
        )
        .subcommand(
            asks("close", "a tab, closed")
                .argument("<id>", "")
                .unlisted(),
        )
}

/// A subcommand that asks the host over its socket.
fn asks(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .exit(Exit::Unavailable, "the host's socket broke off")
        .exit(Exit::Config, "there's no runtime folder for the socket")
}

fn asked_wrongly(what: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this("chrome-open-tabs --help")
}

/// Whether Chrome started it, which names the extension's origin first.
fn hosting(first: Option<&str>) -> bool {
    first.is_some_and(|origin| origin.starts_with("chrome-extension://"))
}

fn main() -> ExitCode {
    let hosted = hosting(env::args().nth(1).as_deref());
    let help = if hosted { hosting_help() } else { help() };
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let socket = socket().ok_or_else(|| {
            Failure::new(Exit::Config, "there's no runtime folder for the socket")
                .because("XDG_RUNTIME_DIR isn't set")
        })?;
        if hosting(context.args.first().map(String::as_str)) {
            return host(io::stdin().lock(), io::stdout(), &socket).map_err(|error| {
                Failure::new(Exit::Failed, "it couldn't host the extension")
                    .because(error.to_string())
            });
        }
        asked(context, &socket)
    })
}

/// What the command line asks of the host, and the one tab it names when
/// it asks for that tab alone: none where it's asked wrongly.
fn from_words(words: &[&str]) -> Option<(Ask, Option<u64>)> {
    match words {
        ["tabs" | "tab"] => Some((Ask::List, None)),
        ["tabs" | "tab", tab] => tab.parse().ok().map(|tab| (Ask::List, Some(tab))),
        ["tabs" | "tab", tab, "go"] => tab.parse().ok().map(|tab| (Ask::Go(tab), None)),
        ["tabs" | "tab", tab, "close"] => tab.parse().ok().map(|tab| (Ask::Close(tab), None)),
        // The words it was asked in before, the socket's own.
        _ => Ask::from_line(&words.join(" ")).map(|ask| (ask, None)),
    }
}

/// Asks the host what the command line says, and says its answer.
fn asked(context: &Context, socket: &Path) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    let Some((asked, one)) = from_words(&words) else {
        return Err(match words.first() {
            Some(&"tabs" | &"tab") => asked_wrongly("a tab's id is a number, as tabs gives it"),
            Some(&"list") => asked_wrongly("list takes nothing more"),
            Some(&"go" | &"close") => asked_wrongly("go and close take a tab's ID, a number"),
            Some(word) => terminal_kit::flags::unknown("command", word, &["tabs"]),
            None => asked_wrongly("chrome-open-tabs needs a command")
                .because("tabs lists the tabs open")
                .try_this("chrome-open-tabs tabs"),
        });
    };
    let said = ask(socket, &asked).map_err(|error| {
        Failure::new(Exit::Unavailable, "the host's socket broke off").because(error.to_string())
    })?;
    let unsaid = |error: io::Error| {
        Failure::new(Exit::Failed, "its answer couldn't be said").because(error.to_string())
    };
    if let Some(tab) = one {
        return one_tab(context, &said, tab);
    }
    if !context.format().is_json() {
        return io::stdout().write_all(said.as_bytes()).map_err(unsaid);
    }
    let answer = match asked {
        Ask::List => tabs(&said),
        Ask::Go(tab) => json!({ "schema": "chrome-open-tabs.asked/1", "ask": "go", "tab": tab }),
        Ask::Close(tab) => {
            json!({ "schema": "chrome-open-tabs.asked/1", "ask": "close", "tab": tab })
        }
    };
    context.answer(&answer).map_err(unsaid)
}

/// `tabs ID`: one open tab's facts, from the host's list.
fn one_tab(context: &Context, said: &str, tab: u64) -> Result<(), Failure> {
    let listed = tabs(said);
    let Some(found) = listed["tabs"]
        .as_array()
        .and_then(|tabs| tabs.iter().find(|one| one["id"] == tab))
    else {
        return Err(
            Failure::new(Exit::NotFound, format!("no tab {tab} is open"))
                .because("Chrome is closed, or the tab is")
                .try_this("chrome-open-tabs tabs"),
        );
    };
    let unsaid = |error: io::Error| {
        Failure::new(Exit::Failed, "its answer couldn't be said").because(error.to_string())
    };
    if context.format().is_json() {
        let mut answer = found.clone();
        answer["schema"] = json!("chrome-open-tabs.tab/1");
        return context.answer(&answer).map_err(unsaid);
    }
    let text = |field: &str| found[field].as_str().map(str::to_owned);
    let window = found["window"].to_string();
    let front = if found["front"] == true { "yes" } else { "no" };
    let facts = [
        ("title", text("title")),
        ("address", text("address")),
        ("window", Some(window)),
        ("in front", Some(front.to_owned())),
        ("looked at", text("looked_at")),
    ];
    let facts: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|(label, value)| (*label, value.as_deref()))
        .collect();
    let lines = layout::facts(&facts, "not said", &context.out, context.room);
    context.say(&(lines.join("\n") + "\n")).map_err(unsaid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream as Pipe;

    fn framed(message: &Json) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_message(&mut bytes, message).unwrap();
        bytes
    }

    #[test]
    fn a_message_reads_back_as_it_was_framed() {
        let message = json!({ "id": 3, "tabs": [] });
        let bytes = framed(&message);
        let length = u32::try_from(message.to_string().len()).unwrap();
        assert_eq!(&bytes[..4], &length.to_ne_bytes());
        let mut input = bytes.as_slice();
        assert_eq!(read_message(&mut input).unwrap(), Some(message));
        assert_eq!(read_message(&mut input).unwrap(), None);
    }

    #[test]
    fn a_length_past_what_chrome_sends_is_refused() {
        let mut input: &[u8] = &(LONGEST + 1).to_ne_bytes();
        assert!(read_message(&mut input).is_err());
    }

    #[test]
    fn each_ask_reads_from_its_line_and_nothing_else_does() {
        assert_eq!(Ask::from_line("list\n"), Some(Ask::List));
        assert_eq!(Ask::from_line("go 12"), Some(Ask::Go(12)));
        assert_eq!(Ask::from_line("close 7\n"), Some(Ask::Close(7)));
        // An empty id is how chrome-recent-sites says a tab it read from
        // Chrome's session file, whose ids aren't the extension's.
        for line in [
            "",
            "go",
            "go ",
            "close ",
            "go twelve",
            "close -1",
            "list all",
            "open 3",
        ] {
            assert_eq!(Ask::from_line(line), None, "{line}");
        }
    }

    // The words the launcher asked in before the tabs took their own ask
    // the host the same thing as the new ones.
    #[test]
    fn the_old_words_ask_as_the_new_ones() {
        for (old, new) in [
            (vec!["list"], vec!["tabs"]),
            (vec!["go", "12"], vec!["tabs", "12", "go"]),
            (vec!["close", "7"], vec!["tabs", "7", "close"]),
        ] {
            assert_eq!(from_words(&old), from_words(&new), "{old:?} beside {new:?}");
            assert!(from_words(&new).is_some(), "{new:?} should ask the host");
        }
        assert_eq!(from_words(&["tabs", "12"]), Some((Ask::List, Some(12))));
        assert_eq!(
            from_words(&["tabs", "twelve", "go"]),
            None,
            "an id is a number"
        );
        assert!(
            hosting(Some("chrome-extension://abc/")),
            "Chrome's start is hosting"
        );
        assert!(!hosting(Some("tabs")), "a person's words aren't");
    }

    #[test]
    fn a_tab_is_one_line_its_title_flattened() {
        let answer = json!({ "id": 1, "tabs": [
            { "id": 4, "window": 2, "active": true, "lastAccessed": 1_761_753_600_123.4,
              "address": "https://example.org/", "title": "Two\tlines\nhere" },
            { "window": 2, "address": "https://no-id.example.org/" }
        ] });
        assert_eq!(
            lines(&answer),
            "4\t2\ttrue\t1761753600123\thttps://example.org/\tTwo lines here\n"
        );
        assert_eq!(lines(&json!({ "id": 1, "done": true })), "");
    }

    /// A host over pipes, the extension stood in for by the test: what the
    /// host sends it is read back, and it answers as the extension would.
    #[test]
    fn the_host_asks_the_extension_and_says_its_answer_then_tidies_up() {
        let folder = env::temp_dir().join(format!("chrome-open-tabs-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let socket = folder.join(SOCKET);
        let (chrome_in, host_in) = Pipe::pair().unwrap();
        let (host_out, chrome_out) = Pipe::pair().unwrap();
        let hosting = {
            let socket = socket.clone();
            thread::spawn(move || host(host_in, host_out, &socket))
        };
        while !socket.exists() {
            thread::sleep(Duration::from_millis(10));
        }
        let mode = fs::metadata(&socket).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let asking = {
            let socket = socket.clone();
            thread::spawn(move || {
                let mut stream = UnixStream::connect(&socket).unwrap();
                stream.write_all(b"list\n").unwrap();
                stream.shutdown(std::net::Shutdown::Write).unwrap();
                let mut said = String::new();
                stream.read_to_string(&mut said).unwrap();
                said
            })
        };
        let mut from_host = &chrome_out;
        let asked = read_message(&mut from_host).unwrap().unwrap();
        assert_eq!(asked["ask"], "list");
        let tabs = json!({ "id": asked["id"], "tabs": [
            { "id": 9, "window": 1, "active": false, "lastAccessed": 5,
              "address": "https://one.example.org/", "title": "One" }
        ] });
        let mut to_host = &chrome_in;
        to_host.write_all(&framed(&tabs)).unwrap();
        assert_eq!(
            asking.join().unwrap(),
            "9\t1\tfalse\t5\thttps://one.example.org/\tOne\n"
        );

        drop(chrome_in);
        hosting.join().unwrap().unwrap();
        assert!(!socket.exists(), "the socket outlived Chrome's closing");
        fs::remove_dir_all(&folder).unwrap();
    }

    fn folder(name: &str) -> PathBuf {
        let folder =
            env::temp_dir().join(format!("chrome-open-tabs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_socket_another_host_answers_on_is_left_to_it() {
        let folder = folder("answering");
        let socket = folder.join(SOCKET);
        let other = UnixListener::bind(&socket).unwrap();
        let refused = claim(&socket).expect_err("a host took a socket another answers on");
        assert_eq!(refused.kind(), io::ErrorKind::AddrInUse);
        assert!(
            UnixStream::connect(&socket).is_ok(),
            "the other host's socket no longer answers"
        );
        drop(other);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_socket_nothing_answers_on_is_cleared_and_taken() {
        let folder = folder("dead");
        let socket = folder.join(SOCKET);
        drop(UnixListener::bind(&socket).unwrap());
        assert!(socket.exists(), "the dead socket was never left to clear");
        let listener = claim(&socket).expect("a dead socket wasn't cleared");
        assert!(
            UnixStream::connect(&socket).is_ok(),
            "the socket taken doesn't answer"
        );
        drop(listener);
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn asking_with_no_host_says_nothing_and_succeeds() {
        let socket = env::temp_dir().join("chrome-open-tabs-nobody.sock");
        assert_eq!(ask(&socket, &Ask::List).unwrap(), "");
    }

    #[test]
    fn a_list_answers_as_json_with_its_fields_named() {
        let said = "7\t2\ttrue\t1794556320000\thttps://a.example/\tA title\n\
                    8\t2\tfalse\t0\tabout:blank\t\n";
        let answer = tabs(said);
        assert_eq!(answer["schema"], "chrome-open-tabs.list/1");
        assert_eq!(
            answer["tabs"][0],
            json!({
                "id": 7, "window": 2, "front": true,
                "looked_at": "2026-11-13T07:52:00Z",
                "address": "https://a.example/", "title": "A title",
            })
        );
        assert_eq!(answer["tabs"][1]["looked_at"], Json::Null);
        assert_eq!(answer["tabs"][1]["front"], false);
    }
}
