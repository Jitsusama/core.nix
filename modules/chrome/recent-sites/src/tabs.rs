//! What Chrome has open now, as the launcher is told of it: from the
//! extension's host, chrome-open-tabs, where it answers, since only its
//! ids can be gone to or closed, and from Chrome's session file otherwise.
//!
//! The two name a tab apart. The host gives the id Chrome's tabs have for
//! an extension, which `chrome-open-tabs go` and `close` take; the file
//! gives the id Chrome wrote it under, which may be from a Chrome before
//! the one running, so it's never printed, and a tab read from the file
//! has no id. The launcher's Go To and Close It hand a tab's id to the
//! host, so only an id the host gave ever reaches it, and one it never
//! gave can't go to or close another tab (the host refuses an empty one).
//! Both are held to the same rule of which pages are rows: one with a
//! program of its own, or off the web, keeps its title alone.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::{on_the_web, one_line, session};

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
    pub fn line(&self) -> String {
        let id = self.id.map(|id| id.to_string()).unwrap_or_default();
        format!(
            "{id}\t{}\t{}\t{}\t{}\t{}",
            self.window, self.front, self.last, self.address, self.title
        )
    }
}

/// The tabs in what the host printed, a tab a line: its id, its window,
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

/// What the host lists, or nothing when it can't be run or fails; it
/// waits on the extension as long as the launcher would.
fn listed(host: &Path) -> String {
    Command::new(host)
        .arg("list")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|ran| ran.status.success())
        .map(|ran| String::from_utf8_lossy(&ran.stdout).into_owned())
        .unwrap_or_default()
}

/// What a profile has open now: the host's tabs, where `host` is given and
/// lists any, and otherwise the session file's.
pub fn open(profile: &Path, host: Option<&Path>) -> Result<Vec<Told>, String> {
    let live = host
        .map(listed)
        .map(|said| from_host(&said))
        .unwrap_or_default();
    if !live.is_empty() {
        return Ok(live);
    }
    session::read(profile).map(from_session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        let folder = env::temp_dir().join(format!(
            "chrome-recent-sites-test-{}-tabs-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A stand-in host in `folder`, printing `said` when asked to list, as
    /// chrome-open-tabs does, and failing when asked anything else.
    fn host(folder: &Path, said: &str) -> PathBuf {
        let program = folder.join("chrome-open-tabs");
        fs::write(
            &program,
            format!("#!/bin/sh\n[ \"$*\" = list ] || exit 2\nprintf '{said}'\n"),
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    /// A profile whose session file holds one tab, Chrome's id for it 10.
    fn sessioned(name: &str) -> PathBuf {
        let profile = folder(name);
        fs::create_dir_all(profile.join("Sessions")).unwrap();
        fs::write(
            profile.join("Sessions/Session_13437850409745633"),
            session::tests::one_tab(10, "https://saved.example.org/"),
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
            "412\\t7\\ttrue\\t1761753600123\\thttps://live.example.org/\\tLive\\n",
        );
        assert_eq!(
            lines(&open(&profile, Some(&host)).unwrap()),
            ["412\t7\ttrue\t1761753600123\thttps://live.example.org/\tLive"]
        );
    }

    #[test]
    fn with_the_host_listing_nothing_the_session_files_tabs_have_no_id() {
        let profile = sessioned("unhosted");
        let quiet = host(&profile, "");
        let missing = profile.join("nowhere");
        for asked in [Some(quiet.as_path()), Some(missing.as_path()), None] {
            assert_eq!(
                lines(&open(&profile, asked).unwrap()),
                ["\t1\tfalse\t1\thttps://saved.example.org/\tTitle 10"],
                "{asked:?}"
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
            ]
        );
    }
}
