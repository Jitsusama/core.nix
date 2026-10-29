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
//! - `chrome-open-tabs list` prints each tab in Chrome's own windows, one a
//!   line: its id, its window, whether it's the window's tab in front, when
//!   it was last looked at (milliseconds since 1970), its address and its
//!   title, apart by tabs;
//! - `chrome-open-tabs go ID` brings that tab to the front of its window;
//! - `chrome-open-tabs close ID` closes it.
//!
//! With Chrome closed, or the extension not answering in time, it prints
//! nothing and succeeds, so the launcher lists what was open lately alone.

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
    // One left by a host that ended without tidying is stale.
    let _ = fs::remove_file(socket);
    let listener = UnixListener::bind(socket)?;
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

/// Asks the host on `socket` and prints its answer; with no host there, it
/// prints nothing.
fn ask(socket: &Path, ask: &Ask) -> io::Result<()> {
    let Ok(mut stream) = UnixStream::connect(socket) else {
        return Ok(());
    };
    stream.set_read_timeout(Some(WAITING + Duration::from_secs(1)))?;
    stream.write_all(format!("{}\n", ask.line()).as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut said = String::new();
    let _ = stream.read_to_string(&mut said);
    io::stdout().write_all(said.as_bytes())
}

fn socket() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join(SOCKET))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(socket) = socket() else {
        eprintln!("chrome-open-tabs: there's no runtime folder for the socket");
        return ExitCode::FAILURE;
    };
    // Chrome names the extension's origin first.
    let hosting = args
        .first()
        .is_some_and(|origin| origin.starts_with("chrome-extension://"));
    let done = if hosting {
        host(io::stdin().lock(), io::stdout(), &socket)
    } else if let Some(asked) = Ask::from_line(&args.join(" ")) {
        ask(&socket, &asked)
    } else {
        eprintln!("usage: chrome-open-tabs list | go ID | close ID");
        return ExitCode::from(2);
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chrome-open-tabs: {error}");
            ExitCode::FAILURE
        }
    }
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
        for line in ["", "go", "go twelve", "close -1", "list all", "open 3"] {
            assert_eq!(Ask::from_line(line), None, "{line}");
        }
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

    #[test]
    fn asking_with_no_host_says_nothing_and_succeeds() {
        let socket = env::temp_dir().join("chrome-open-tabs-nobody.sock");
        assert!(ask(&socket, &Ask::List).is_ok());
    }
}
