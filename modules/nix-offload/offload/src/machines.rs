//! What Nix and the machine queue read: the builders standing by that
//! answer now, and the one lent, if any.

use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a standing builder has to answer before it's left out.
const ANSWER: Duration = Duration::from_secs(2);

/// A builder always there to lend, such as another machine at home, named
/// by the address that says whether it answers and its Nix machines line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub address: String,
    pub line: String,
}

/// The standing builders in a file of them, one a line, its address and
/// then its machines line: `192.168.4.46:22 ssh-ng://builder@home ...`.
/// Blank lines and those starting `#` say nothing.
pub fn standing(text: &str) -> Vec<Standing> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (address, line) = line.split_once(char::is_whitespace)?;
            Some(Standing {
                address: address.to_owned(),
                line: line.trim().to_owned(),
            })
        })
        .collect()
}

/// The file's text, as it should read now.
pub fn text<'a>(answering: impl Iterator<Item = &'a str>, lent: Option<&'a str>) -> String {
    answering.chain(lent).fold(String::new(), |mut text, line| {
        text.push_str(line);
        text.push('\n');
        text
    })
}

/// Whether something listens at an address, `host:port`.
pub fn answers(address: &str) -> bool {
    address.to_socket_addrs().is_ok_and(|mut found| {
        found.any(|address| TcpStream::connect_timeout(&address, ANSWER).is_ok())
    })
}

/// The machines file, written only when what it should say changes.
pub struct Machines {
    path: PathBuf,
    standing: Vec<Standing>,
    written: Option<String>,
}

impl Machines {
    pub fn new(path: PathBuf, standing: Vec<Standing>) -> Self {
        Self {
            path,
            standing,
            written: None,
        }
    }

    /// Say the standing builders that answer, and the one lent.
    pub fn show(&mut self, lent: Option<&str>) {
        let answering = self
            .standing
            .iter()
            .filter(|builder| answers(&builder.address))
            .map(|builder| builder.line.as_str());
        let text = text(answering, lent);
        if self.written.as_ref() != Some(&text) {
            write(&self.path, &text);
            self.written = Some(text);
        }
    }
}

/// Write the file whole or not at all.
fn write(path: &Path, text: &str) {
    let writing = path.with_extension("writing");
    let written = fs::write(&writing, text).and_then(|()| fs::rename(&writing, path));
    if let Err(error) = written {
        eprintln!("nix-offload: writing {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn standing_builders_are_read_a_line_each() {
        let file = "# at home\n\n192.168.4.46:22 ssh-ng://builder@home x86_64-linux - 2\n";
        assert_eq!(
            standing(file),
            [Standing {
                address: "192.168.4.46:22".to_owned(),
                line: "ssh-ng://builder@home x86_64-linux - 2".to_owned(),
            }]
        );
    }

    #[test]
    fn the_file_says_those_answering_then_the_one_lent() {
        assert_eq!(text(["a", "b"].into_iter(), Some("lent")), "a\nb\nlent\n");
        assert_eq!(text(std::iter::empty(), None), "");
    }

    #[test]
    fn a_builder_answers_only_while_something_listens() {
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        assert!(answers(&address));
        drop(listening);
        assert!(!answers(&address));
        assert!(!answers("not an address"));
    }

    #[test]
    fn a_builder_that_stops_answering_leaves_the_file() {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("machines");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let mut machines = Machines::new(
            path.clone(),
            vec![Standing {
                address,
                line: "ssh-ng://home".to_owned(),
            }],
        );
        machines.show(Some("ssh-ng://lent"));
        assert_eq!(
            fs::read_to_string(&path).expect("read"),
            "ssh-ng://home\nssh-ng://lent\n"
        );
        drop(listening);
        machines.show(None);
        assert_eq!(fs::read_to_string(&path).expect("read"), "");
    }
}
