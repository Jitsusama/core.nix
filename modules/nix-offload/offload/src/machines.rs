//! What Nix and the machine queue read: the one builder lent, if any, for
//! every build; and beside it the builders standing by that answer now, for
//! a build that asks for them by naming their file.
//!
//! Standing builders are never offered to every build, since they may be
//! machines whose owner wants only some work on them: a build of private
//! code must never reach one by default.

use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a standing builder has to answer before it's left out.
const ANSWER: Duration = Duration::from_secs(2);

/// The work a lent builder is for: a derivation needing one of these is
/// sent to it first. Anything else stays here, since copying its inputs
/// out and its output back takes longer than building it: on 29 October a
/// system build that takes 34 s to 2.5 min here ran past ten minutes once
/// its small derivations went out.
const BIG: [&str; 2] = ["big-parallel", "nixos-test"];

/// A lent builder's machines line written once for each kind of big work
/// it supports, with that kind mandatory, so Nix sends it only that work.
/// Nix counts a builder's slots by its URI, so the lines share one set. A
/// line supporting none of them is kept as it is.
pub fn for_big_work(line: &str) -> String {
    let mut fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 {
        fields.resize(8, "-");
    }
    let supported: Vec<&str> = fields[5].split(',').collect();
    let lines: Vec<String> = BIG
        .iter()
        .filter(|kind| supported.contains(kind))
        .map(|kind| {
            let mut fields = fields.clone();
            fields[6] = kind;
            fields.join(" ")
        })
        .collect();
    if lines.is_empty() {
        line.to_owned()
    } else {
        lines.join("\n")
    }
}

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

/// A file of machines lines, written only when what it should say changes.
struct File {
    path: PathBuf,
    written: Option<String>,
}

impl File {
    fn say(&mut self, text: String) {
        if self.written.as_ref() != Some(&text) {
            write(&self.path, &text);
            self.written = Some(text);
        }
    }
}

/// The builder lent, as Nix is told of it: its machines line, and the
/// store a whole build may be sent to, if the helper gave one.
#[derive(Clone, Copy)]
pub struct Lending<'a> {
    pub line: &'a str,
    pub store: Option<&'a str>,
}

/// The two machines files: `every` for every build, holding only the one
/// lent, and `with_standing` for a build that names it, holding the
/// standing builders that answer as well; and `store`, the lent builder's
/// store, for a build sent there whole (`--store`), so only its
/// derivations cross a slow connection rather than what's built here; and
/// `lending`, the builder being made, for the machine queue to hold builds
/// for. Nix's build hook reads its builders once, when a build starts, and
/// an empty list keeps that whole build here, even once a builder stands.
pub struct Machines {
    every: File,
    with_standing: File,
    store: File,
    lending: File,
    standing: Vec<Standing>,
}

impl Machines {
    pub fn new(
        every: PathBuf,
        with_standing: PathBuf,
        store: PathBuf,
        lending: PathBuf,
        standing: Vec<Standing>,
    ) -> Self {
        let file = |path| File {
            path,
            written: None,
        };
        Self {
            every: file(every),
            with_standing: file(with_standing),
            store: file(store),
            lending: file(lending),
            standing,
        }
    }

    /// Say the builder being made, a line of its own, or nothing once it
    /// stands or won't.
    pub fn asking(&mut self, name: Option<&str>) {
        self.lending
            .say(name.map_or_else(String::new, |name| format!("{name}\n")));
    }

    /// Say the one lent to every build, for its big work, and it with the
    /// standing builders that answer to a build that asks for them; and its
    /// store, a line of its own, or nothing.
    pub fn show(&mut self, lending: Option<Lending>) {
        self.store.say(
            lending
                .and_then(|lending| lending.store)
                .map_or_else(String::new, |store| format!("{store}\n")),
        );
        let lent = lending.map(|lending| for_big_work(lending.line));
        let lent = lent.as_deref();
        let answering = self
            .standing
            .iter()
            .filter(|builder| answers(&builder.address))
            .map(|builder| builder.line.as_str());
        self.with_standing.say(text(answering, lent));
        self.every.say(text(std::iter::empty(), lent));
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
    fn a_lent_builder_is_named_for_its_big_work_alone() {
        let line =
            "ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark - KEY";
        assert_eq!(
            for_big_work(line),
            "ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark \
             big-parallel KEY\n\
             ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark \
             nixos-test KEY"
        );
        assert_eq!(
            for_big_work("ssh-ng://lent x86_64-linux - 4 1 big-parallel"),
            "ssh-ng://lent x86_64-linux - 4 1 big-parallel big-parallel -"
        );
        assert_eq!(
            for_big_work("ssh-ng://small x86_64-linux"),
            "ssh-ng://small x86_64-linux"
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
    fn a_standing_builder_is_named_only_to_builds_that_ask_while_it_answers() {
        let dir = tempfile::tempdir().expect("a folder");
        let every = dir.path().join("machines");
        let asked = dir.path().join("machines-with-standing");
        let store = dir.path().join("store");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let mut machines = Machines::new(
            every.clone(),
            asked.clone(),
            store.clone(),
            dir.path().join("lending"),
            vec![Standing {
                address,
                line: "ssh-ng://home".to_owned(),
            }],
        );
        let read = |path| fs::read_to_string(path).expect("read");
        machines.show(Some(Lending {
            line: "ssh-ng://lent",
            store: Some("ssh-ng://lent?ssh-key=/k"),
        }));
        assert_eq!(read(&every), "ssh-ng://lent\n");
        assert_eq!(read(&asked), "ssh-ng://home\nssh-ng://lent\n");
        assert_eq!(read(&store), "ssh-ng://lent?ssh-key=/k\n");
        machines.show(None);
        assert_eq!(read(&every), "");
        assert_eq!(read(&asked), "ssh-ng://home\n");
        assert_eq!(read(&store), "");
        drop(listening);
        machines.show(None);
        assert_eq!(read(&every), "");
        assert_eq!(read(&asked), "");
    }

    #[test]
    fn the_builder_being_made_is_named_until_it_stands_or_wont() {
        let dir = tempfile::tempdir().expect("a folder");
        let lending = dir.path().join("lending");
        let mut machines = Machines::new(
            dir.path().join("machines"),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            lending.clone(),
            Vec::new(),
        );
        let read = |path| fs::read_to_string(path).expect("read");
        machines.asking(Some("lend-1"));
        assert_eq!(read(&lending), "lend-1\n");
        machines.asking(None);
        assert_eq!(read(&lending), "", "emptied, not removed, once it stands");
    }
}
