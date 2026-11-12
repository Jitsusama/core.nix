//! What Chrome has open now, as the launcher is told of it: from the
//! extension's host, chrome-open-tabs, where it answers on its socket,
//! since only its ids can be gone to or closed, and from Chrome's session
//! file otherwise, with no process started for either.
//!
//! The two name a tab apart. The host gives the id Chrome's tabs have for
//! an extension, which `chrome-open-tabs go` and `close` take; the file
//! gives the id Chrome wrote it under, which may be from a Chrome before
//! the one running, so it's never said, and a tab read from the file has
//! no id. The launcher's Go To and Close It hand a tab's id to the host,
//! so only an id the host gave ever reaches it, and one it never gave
//! can't go to or close another tab (the host refuses an empty one). Both
//! are held to the same rule of which pages are rows: one with a program
//! of its own, or off the web, keeps its title alone.

use std::io::{Read as _, Write as _};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::{on_the_web, one_line, session};

/// How long the host is waited on: the two seconds it waits on the
/// extension (chrome-open-tabs's WAITING), and one for itself, as its own
/// command waits.
const WAITING: Duration = Duration::from_secs(3);

/// A tab open now, as a line the launcher reads.
#[derive(Debug, PartialEq, Eq)]
pub struct Told {
    /// The id the host gave it, or none where it was read from the file.
    pub id: Option<u64>,
    pub window: i64,
    pub front: bool,
    /// When it was last looked at, in milliseconds since 1970, or 0.
    pub last: i64,
    pub address: String,
    pub title: String,
}

impl Told {
    /// It as a line, as the host says a tab: its id, or nothing, its
    /// window, whether it's in front, when it was last looked at, its
    /// address and its title, tab-separated.
    #[must_use]
    pub fn line(&self) -> String {
        let id = self.id.map(|id| id.to_string()).unwrap_or_default();
        format!(
            "{id}\t{}\t{}\t{}\t{}\t{}",
            self.window, self.front, self.last, self.address, self.title
        )
    }
}

/// The tabs in what the host said, a tab a line: its id, its window,
/// whether it's in front, when it was looked at, its address and its
/// title. A line it can't read is left out.
fn from_host(said: &str) -> Vec<Told> {
    said.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(6, '\t').collect();
            let [id, window, front, last, address, title] = fields.as_slice() else {
                return None;
            };
            Some(Told {
                id: Some(id.parse().ok()?),
                window: window.parse().ok()?,
                front: front.parse().ok()?,
                last: last.parse().ok()?,
                address: if on_the_web(address) {
                    (*address).to_string()
                } else {
                    String::new()
                },
                title: one_line(title),
            })
        })
        .collect()
}

fn from_session(open: Vec<session::Open>) -> Vec<Told> {
    open.into_iter()
        .map(|tab| Told {
            id: None,
            window: i64::from(tab.window),
            front: tab.front,
            last: tab.last,
            address: tab.address,
            title: tab.title,
        })
        .collect()
}

/// What the host on `socket` lists, as its own `list` asks it, or nothing
/// when it isn't there, as it isn't with Chrome closed or refusing the
/// extension, or doesn't answer in time.
fn listed(socket: &Path) -> String {
    let Ok(mut stream) = UnixStream::connect(socket) else {
        return String::new();
    };
    let mut said = String::new();
    let asked = stream
        .set_read_timeout(Some(WAITING))
        .and_then(|()| stream.write_all(b"list\n"))
        .and_then(|()| stream.shutdown(Shutdown::Write))
        .and_then(|()| stream.read_to_string(&mut said));
    if asked.is_err() {
        said.clear();
    }
    said
}

/// What a profile has open now: the host's tabs, where its socket is
/// given and it lists any, and otherwise the session file's.
///
/// # Errors
///
/// When the profile's newest session file can't be read, saying why.
pub fn open(profile: &Path, host: Option<&Path>) -> Result<Vec<Told>, String> {
    let live = host
        .map(|socket| from_host(&listed(socket)))
        .unwrap_or_default();
    if !live.is_empty() {
        return Ok(live);
    }
    session::read(profile).map(from_session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::written::{session, tab};
    use std::fs;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::thread;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "chrome-session-test-{}-open-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A stand-in host on a socket in `folder`, answering one ask with
    /// `said` when it's `list`, as chrome-open-tabs does, and with nothing
    /// otherwise.
    fn host(folder: &Path, said: &'static str) -> PathBuf {
        let socket = folder.join("chrome-open-tabs.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut asked = String::new();
            stream.read_to_string(&mut asked).unwrap();
            if asked == "list\n" {
                stream.write_all(said.as_bytes()).unwrap();
            }
        });
        socket
    }

    /// A profile whose session file holds one tab, Chrome's id for it 10.
    fn sessioned(name: &str) -> PathBuf {
        let profile = folder(name);
        fs::create_dir_all(profile.join("Sessions")).unwrap();
        fs::write(
            profile.join("Sessions/Session_13437850409745633"),
            session(&tab(1, 10, 0, "https://saved.example.org/", 1)),
        )
        .unwrap();
        profile
    }

    fn lines(told: &[Told]) -> Vec<String> {
        told.iter().map(Told::line).collect()
    }

    #[test]
    fn the_hosts_tabs_are_told_with_the_ids_it_gave() {
        let profile = sessioned("hosted");
        let host = host(
            &profile,
            "412\t7\ttrue\t1761753600123\thttps://live.example.org/\tLive\n",
        );
        assert_eq!(
            lines(&open(&profile, Some(&host)).unwrap()),
            ["412\t7\ttrue\t1761753600123\thttps://live.example.org/\tLive"],
            "the host's tab, by its id"
        );
    }

    #[test]
    fn with_the_host_listing_nothing_the_session_files_tabs_have_no_id() {
        let profile = sessioned("unhosted");
        let quiet = host(&profile, "");
        let missing = profile.join("nowhere.sock");
        for asked in [Some(quiet.as_path()), Some(missing.as_path()), None] {
            assert_eq!(
                lines(&open(&profile, asked).unwrap()),
                ["\t1\tfalse\t1\thttps://saved.example.org/\tTitle 10"],
                "the file's tab, with no id, asking {asked:?}"
            );
        }
    }

    #[test]
    fn the_hosts_pages_keep_to_the_files_rule_of_which_are_rows() {
        let said = from_host(concat!(
            "1\t1\tfalse\t5\thttps://mail.google.com/mail/u/0/\tInbox\n",
            "2\t1\tfalse\t4\tchrome://newtab/\tNew Tab\n",
            "3\t1\ttrue\t3\thttps://example.org/\tTwo  words\n",
            "not a tab\n",
            "x\t1\tfalse\t2\thttps://example.org/\tNo id\n",
        ));
        assert_eq!(
            lines(&said),
            [
                "1\t1\tfalse\t5\t\tInbox",
                "2\t1\tfalse\t4\t\tNew Tab",
                "3\t1\ttrue\t3\thttps://example.org/\tTwo words",
            ],
            "Google's own and Chrome's own pages by their titles alone, a title on one line"
        );
    }
}
