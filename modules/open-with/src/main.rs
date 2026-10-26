//! The portal's chooser of an app to open something with, for niri. When a
//! program asks the portal to open a file or a link, and the portal can't
//! tell which app should (more than one can, none is the default for its
//! kind, and none has been picked often enough), it asks its `AppChooser`,
//! which GNOME's portal answers with a dialog of its own. This answers it in
//! the launcher instead.
//!
//! Each choice is asked of the shell through Quickshell's IPC (OpenWith.qml)
//! with a socket of its own, in a folder of the runtime folder that only
//! Joel can open. The shell holds an "open with" collection of the apps the
//! portal offers and says the one picked on that socket, as a line naming its
//! id, or an empty line for none. The portal opens what was picked and
//! remembers it, and stops asking once the same app has been picked enough.
//!
//! A choice ends when the shell says, when the program asking gives up (the
//! portal closes the request), or after ten minutes, since a shell that
//! restarted has forgotten what it was asked; each but the first is
//! withdrawn from the shell. The only argument is the qs command.
//!
//! It answers the portal's `FileChooser` the same way (files.rs), so a file
//! is picked in the launcher too.

mod files;

use std::collections::HashMap;
use std::env;
use std::io::ErrorKind;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::net::UnixListener;
use tokio::sync::oneshot;
use zbus::object_server::ObjectServer;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str};
use zbus::{connection, interface};

/// The name the portal's description (open-with.portal) gives this.
const NAME: &str = "org.freedesktop.impl.portal.desktop.launcher";
const PATH: &str = "/org/freedesktop/portal/desktop";

/// How long a choice may wait for the shell: long enough to come back to,
/// and short enough that one a restarted shell forgot doesn't hold its
/// program for good.
const WAITING: Duration = Duration::from_mins(10);

/// What the portal is told: an app picked, none, or the choice ended some
/// other way.
const PICKED: u32 = 0;
const REFUSED: u32 = 1;
const NEITHER: u32 = 2;

/// How a choice ended.
#[derive(Debug, PartialEq)]
enum Ended {
    /// The shell said this line.
    Said(String),
    /// The program asking gave up.
    Closed,
    /// Nothing was said in time.
    Late,
    /// The shell couldn't be asked, or its answer couldn't be read.
    Unasked,
}

/// What the portal is told for how a choice ended, and the app picked,
/// which must be one it offered, since the shell's answer is only a line.
fn response(ended: &Ended, offered: &[String]) -> (u32, Option<String>) {
    match ended {
        Ended::Said(id) if id.is_empty() => (REFUSED, None),
        Ended::Said(id) if offered.contains(id) => (PICKED, Some(id.clone())),
        _ => (NEITHER, None),
    }
}

/// An option's text, if it has one.
fn text(options: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    options
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// What's being opened, as the shell names it: the file, else the link,
/// else its kind.
fn what(options: &HashMap<String, OwnedValue>) -> String {
    ["filename", "uri", "content_type"]
        .into_iter()
        .find_map(|key| text(options, key))
        .unwrap_or_default()
}

/// The first line said on the socket, once someone connects to it and
/// either ends the line or hangs up, if no longer than `longest`; None if
/// the socket fails.
async fn answer(listener: &UnixListener, longest: usize) -> Option<String> {
    let (stream, _) = listener.accept().await.ok()?;
    let mut said = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        stream.readable().await.ok()?;
        match stream.try_read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                said.extend_from_slice(&buffer[..read]);
                // A line longer than any answer is no answer.
                if said.contains(&b'\n') || said.len() > longest {
                    break;
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => return None,
        }
    }
    let said = String::from_utf8_lossy(&said);
    Some(said.lines().next().unwrap_or("").trim().to_owned())
}

fn held<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A choice waiting on the shell.
struct Pending {
    /// Where the shell answers, which names the choice to the shell.
    place: String,
    /// The apps it may be answered with, which the portal may change.
    offered: Mutex<Vec<String>>,
    /// Told once the program asking gives up.
    closing: Mutex<Option<oneshot::Sender<()>>>,
}

/// The portal's request for a choice, at the path it gave, which it closes
/// when the program asking gives up.
struct Request(Arc<Pending>);

#[interface(name = "org.freedesktop.impl.portal.Request")]
impl Request {
    fn close(&self) {
        if let Some(closing) = held(&self.0.closing).take() {
            let _ = closing.send(());
        }
    }
}

/// The shell, asked through qs, and the folder of sockets it answers on.
struct Shell {
    qs: PathBuf,
    sockets: PathBuf,
    asked: AtomicU64,
}

impl Shell {
    /// Calls one of the shell's functions with these words, saying whether
    /// it took them. A shell that doesn't have the function refuses them.
    async fn tell(&self, target: &'static str, function: &'static str, words: Vec<String>) -> bool {
        let qs = self.qs.clone();
        tokio::task::spawn_blocking(move || {
            Command::new(qs)
                .args(["ipc", "call", target, function])
                .args(words)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
        .await
        .unwrap_or(false)
    }

    /// Asks the shell's `target` on a socket of its own, with the socket's
    /// place before these words, and waits until it says, the program asking
    /// gives up or the time runs out; the last two are withdrawn from the
    /// shell. The request at `handle` is served for its close meanwhile, and
    /// the ask is kept in `waiting` while it waits, when given.
    async fn ask(
        &self,
        target: &'static str,
        handle: &OwnedObjectPath,
        server: &ObjectServer,
        offered: Vec<String>,
        words: Vec<String>,
        waiting: Option<&Mutex<HashMap<OwnedObjectPath, Arc<Pending>>>>,
        longest: usize,
    ) -> (Ended, Arc<Pending>) {
        let asked = self.asked.fetch_add(1, Ordering::Relaxed);
        let socket = self.sockets.join(format!("{asked}.sock"));
        let place = socket.to_string_lossy().into_owned();
        let (closing, closed) = oneshot::channel();
        let pending = Arc::new(Pending {
            place: place.clone(),
            offered: Mutex::new(offered),
            closing: Mutex::new(Some(closing)),
        });
        let _ = std::fs::remove_file(&socket);
        let listener = match UnixListener::bind(&socket) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!(
                    "<3>open-with: can't listen on {}: {error}",
                    socket.display()
                );
                return (Ended::Unasked, pending);
            }
        };
        if let Err(error) = server
            .at(handle.clone().into_inner(), Request(pending.clone()))
            .await
        {
            eprintln!("<4>open-with: can't take the request's close: {error}");
        }
        if let Some(waiting) = waiting {
            held(waiting).insert(handle.clone(), pending.clone());
        }

        let told = std::iter::once(place.clone()).chain(words).collect();
        let ended = if self.tell(target, "ask", told).await {
            tokio::select! {
                said = answer(&listener, longest) => said.map_or(Ended::Unasked, Ended::Said),
                _ = closed => Ended::Closed,
                () = tokio::time::sleep(WAITING) => Ended::Late,
            }
        } else {
            Ended::Unasked
        };
        if matches!(ended, Ended::Closed | Ended::Late) {
            self.tell(target, "withdraw", vec![place]).await;
        }

        drop(listener);
        let _ = std::fs::remove_file(&socket);
        if let Some(waiting) = waiting {
            held(waiting).remove(handle);
        }
        let _ = server.remove::<Request, _>(handle.clone().into_inner()).await;
        (ended, pending)
    }
}

struct Chooser {
    shell: Arc<Shell>,
    /// The choices waiting, by the portal's request path.
    pending: Mutex<HashMap<OwnedObjectPath, Arc<Pending>>>,
}

#[interface(name = "org.freedesktop.impl.portal.AppChooser")]
impl Chooser {
    #[zbus(out_args("response", "results"))]
    async fn choose_application(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        choices: Vec<String>,
        options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> (u32, HashMap<String, OwnedValue>) {
        // The shell asks over its own window, whichever program asked.
        let _ = (app_id, parent_window);
        let words = vec![
            what(&options),
            text(&options, "content_type").unwrap_or_default(),
            text(&options, "last_choice").unwrap_or_default(),
            choices.join(";"),
        ];
        // An app's id is short; a line this long is no answer.
        let (ended, pending) = self
            .shell
            .ask(
                "open-with",
                &handle,
                server,
                choices,
                words,
                Some(&self.pending),
                4096,
            )
            .await;

        let offered = held(&pending.offered).clone();
        let (code, choice) = response(&ended, &offered);
        let mut results = HashMap::new();
        if let Some(choice) = choice {
            results.insert("choice".to_owned(), OwnedValue::from(Str::from(choice)));
            // The token the program gave, for the app to come to the front.
            if let Some(token) = text(&options, "activation_token") {
                results.insert(
                    "activation_token".to_owned(),
                    OwnedValue::from(Str::from(token)),
                );
            }
        }
        (code, results)
    }

    /// The apps that open it changed while it waits, as one was installed.
    async fn update_choices(&self, handle: OwnedObjectPath, choices: Vec<String>) {
        let Some(pending) = held(&self.pending).get(&handle).cloned() else {
            return;
        };
        held(&pending.offered).clone_from(&choices);
        self.shell
            .tell(
                "open-with",
                "update",
                vec![pending.place.clone(), choices.join(";")],
            )
            .await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [qs] = args.as_slice() else {
        eprintln!("usage: open-with <the qs command>");
        return ExitCode::from(2);
    };
    let Some(runtime) = env::var_os("XDG_RUNTIME_DIR") else {
        eprintln!("<3>open-with: there's no runtime folder to listen in");
        return ExitCode::FAILURE;
    };
    let sockets = PathBuf::from(runtime).join("open-with");
    if let Err(error) = std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&sockets)
    {
        eprintln!("<3>open-with: can't make {}: {error}", sockets.display());
        return ExitCode::FAILURE;
    }
    let shell = Arc::new(Shell {
        qs: PathBuf::from(qs),
        sockets,
        asked: AtomicU64::new(0),
    });
    let chooser = Chooser {
        shell: shell.clone(),
        pending: Mutex::new(HashMap::new()),
    };
    let served = match connection::Builder::session()
        .and_then(|builder| builder.name(NAME))
        .and_then(|builder| builder.serve_at(PATH, chooser))
        .and_then(|builder| builder.serve_at(PATH, files::Files::new(shell)))
    {
        Ok(builder) => builder.build().await,
        Err(error) => Err(error),
    };
    let _connection = match served {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("<3>open-with: can't serve the portal: {error}");
            return ExitCode::FAILURE;
        }
    };
    std::future::pending::<()>().await;
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    fn offered() -> Vec<String> {
        vec!["org.gnome.TextEditor".to_owned(), "nvim".to_owned()]
    }

    #[test]
    fn an_app_offered_and_picked_is_opened() {
        assert_eq!(
            response(&Ended::Said("nvim".to_owned()), &offered()),
            (PICKED, Some("nvim".to_owned()))
        );
    }

    #[test]
    fn an_app_not_offered_opens_nothing() {
        assert_eq!(
            response(&Ended::Said("rm".to_owned()), &offered()),
            (NEITHER, None)
        );
    }

    #[test]
    fn none_picked_is_refused() {
        assert_eq!(
            response(&Ended::Said(String::new()), &offered()),
            (REFUSED, None)
        );
    }

    #[test]
    fn a_choice_given_up_late_or_unasked_is_neither() {
        for ended in [Ended::Closed, Ended::Late, Ended::Unasked] {
            assert_eq!(response(&ended, &offered()), (NEITHER, None));
        }
    }

    fn options(pairs: &[(&str, &str)]) -> HashMap<String, OwnedValue> {
        pairs
            .iter()
            .map(|(key, text)| ((*key).to_owned(), OwnedValue::from(Str::from(*text))))
            .collect()
    }

    #[test]
    fn what_is_named_by_its_file_then_its_link_then_its_kind() {
        let all = [
            ("filename", "notes.txt"),
            ("uri", "file:///home/someone/notes.txt"),
            ("content_type", "text/plain"),
        ];
        assert_eq!(what(&options(&all)), "notes.txt");
        assert_eq!(what(&options(&all[1..])), "file:///home/someone/notes.txt");
        assert_eq!(what(&options(&all[2..])), "text/plain");
        assert_eq!(
            what(&options(&[
                ("filename", ""),
                ("content_type", "text/plain")
            ])),
            "text/plain"
        );
        assert_eq!(what(&HashMap::new()), "");
    }

    /// A socket's place of its own for a test, empty to start with.
    fn place(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("open-with-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    async fn said(name: &str, words: &'static [u8], hang_up: bool) -> Option<String> {
        let path = place(name);
        let listener = UnixListener::bind(&path).unwrap();
        let (done, ending) = std::sync::mpsc::channel::<()>();
        let client = {
            let path = path.clone();
            std::thread::spawn(move || {
                let mut stream = UnixStream::connect(&path).unwrap();
                stream.write_all(words).unwrap();
                if !hang_up {
                    // Still connected once the line is said, until heard.
                    let _ = ending.recv_timeout(Duration::from_secs(30));
                }
            })
        };
        let heard = tokio::time::timeout(Duration::from_secs(5), answer(&listener, 4096))
            .await
            .unwrap();
        let _ = done.send(());
        client.join().unwrap();
        std::fs::remove_file(&path).unwrap();
        heard
    }

    #[tokio::test]
    async fn a_line_said_is_heard_without_waiting_for_a_hang_up() {
        assert_eq!(
            said("line", b"nvim\nmore", false).await,
            Some("nvim".to_owned())
        );
    }

    #[tokio::test]
    async fn words_said_then_a_hang_up_are_heard() {
        assert_eq!(
            said("hang-up", b" nvim ", true).await,
            Some("nvim".to_owned())
        );
    }

    #[tokio::test]
    async fn a_hang_up_with_nothing_said_picks_none() {
        assert_eq!(said("nothing", b"", true).await, Some(String::new()));
    }
}
