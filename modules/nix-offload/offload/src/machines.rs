//! What Nix and the machine queue read: the one builder lent, if any, for
//! every build; and beside it the builders standing by that answer now, for
//! a build that asks for them by naming their file.
//!
//! Standing builders are never offered to every build, since they may be
//! machines whose owner wants only some work on them: a build of private
//! code must never reach one by default. Builders beside are the other
//! kind, machines that may take any of this one's work, and are named to
//! every build while they answer, for their big work, as the one lent is.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a standing builder has to answer before it's left out.
const ANSWER: Duration = Duration::from_secs(2);

/// The work a lent builder is for: a derivation needing one of these is
/// sent to it first. Anything else stays here, since copying its inputs
/// out and its output back takes longer than building it: on 29 October a
/// system build that takes 34 s to 2.5 min here ran past ten minutes once
/// its small derivations went out. VM tests stay too: their machines are
/// built here, and copying those out is most of their time over a slow
/// upload. On 29 October one took 17 minutes there, its test 55 s of it,
/// against 60 s here. Sent to the builder as a command naming a commit
/// instead, they're fetched, evaluated and built there, and only the
/// command crosses.
const BIG: [&str; 1] = ["big-parallel"];

/// What a VM test needs, kept out of what the lent line offers, so neither
/// Nix nor the machine queue, which counts a builder's VM room by them,
/// takes it for a place to run one.
const VM: [&str; 2] = ["kvm", "nixos-test"];

/// A lent builder's machines line written once for each kind of big work
/// it supports, with that kind mandatory, so Nix sends it only that work,
/// and offering nothing a VM test needs. Nix counts a builder's slots by
/// its URI, so the lines share one set. A line supporting none of them is
/// kept as it is.
pub fn for_big_work(line: &str) -> String {
    let mut fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 {
        fields.resize(8, "-");
    }
    let supported: Vec<&str> = fields[5].split(',').collect();
    let offered = supported
        .iter()
        .filter(|feature| !VM.contains(feature))
        .copied()
        .collect::<Vec<_>>()
        .join(",");
    let lines: Vec<String> = BIG
        .iter()
        .filter(|kind| supported.contains(kind))
        .map(|kind| {
            let mut fields = fields.clone();
            fields[5] = &offered;
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
    found(address, Path::new(RESOLVED))
        .iter()
        .any(|address| TcpStream::connect_timeout(address, ANSWER).is_ok())
}

/// systemd-resolved's own socket, asked directly rather than through the C
/// library. With nscd running, as on NixOS, the library reaches resolved only
/// through it, and once a lookup there times out, as one for a `.local` name
/// nobody answers does after 5 s, it leaves nscd out of the process's next
/// lookups and falls back to plain DNS, which finds no `.local` name at all.
/// On 30 October an absent builder kept one that answered out of every tick.
const RESOLVED: &str = "/run/systemd/resolve/io.systemd.Resolve";

/// How long resolved has to say where a name is.
const RESOLVING: Duration = Duration::from_secs(6);

/// The socket addresses `host:port` names: itself when it's an address,
/// else what resolved says the host is, else, where resolved can't be
/// reached, what the C library says.
fn found(address: &str, resolved: &Path) -> Vec<SocketAddr> {
    if let Ok(address) = address.parse::<SocketAddr>() {
        return vec![address];
    }
    address
        .rsplit_once(':')
        .and_then(|(host, port)| Some((host, port.parse::<u16>().ok()?)))
        .and_then(|(host, port)| asked(resolved, host, port))
        .unwrap_or_else(|| {
            address
                .to_socket_addrs()
                .map(Iterator::collect)
                .unwrap_or_default()
        })
}

/// Ask resolved where `host` is, over varlink: a JSON call ended by a NUL
/// byte, answered the same way. `None` when its socket can't be reached;
/// a name it can't find, or a reply that doesn't come, finds nothing rather
/// than falling back to the library that would lose every later name.
fn asked(resolved: &Path, host: &str, port: u16) -> Option<Vec<SocketAddr>> {
    let mut stream = UnixStream::connect(resolved).ok()?;
    let call = json!({
        "method": "io.systemd.Resolve.ResolveHostname",
        "parameters": { "name": host },
    });
    let mut reply = Vec::new();
    let answered = stream
        .set_read_timeout(Some(RESOLVING))
        .and_then(|()| stream.write_all(call.to_string().as_bytes()))
        .and_then(|()| stream.write_all(b"\0"))
        .and_then(|()| BufReader::new(&stream).read_until(0, &mut reply));
    if answered.is_err() || reply.pop() != Some(0) {
        return Some(Vec::new());
    }
    let reply: Value = serde_json::from_slice(&reply).unwrap_or_default();
    Some(
        reply["parameters"]["addresses"]
            .as_array()
            .map(|addresses| {
                addresses
                    .iter()
                    .filter_map(|found| address_of(found, port))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// One of resolved's addresses, its bytes and family, at `port`. A
/// link-local one keeps the interface it was found on, without which it
/// can't be reached.
fn address_of(found: &Value, port: u16) -> Option<SocketAddr> {
    let bytes: Vec<u8> = found["address"]
        .as_array()?
        .iter()
        .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
        .collect::<Option<_>>()?;
    if let Ok(v4) = <[u8; 4]>::try_from(bytes.as_slice()) {
        return Some(SocketAddr::from((v4, port)));
    }
    let v6 = Ipv6Addr::from(<[u8; 16]>::try_from(bytes.as_slice()).ok()?);
    let interface = found["ifindex"]
        .as_u64()
        .and_then(|index| u32::try_from(index).ok())
        .filter(|_| v6.is_unicast_link_local())
        .unwrap_or(0);
    Some(SocketAddr::V6(SocketAddrV6::new(v6, port, 0, interface)))
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
    beside: Vec<Standing>,
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
            beside: Vec::new(),
        }
    }

    /// With `beside`, builders standing by whose owner lets them take any
    /// of this machine's work, so they're named to every build while they
    /// answer, and never to a build that asks for the standing ones.
    #[must_use]
    pub fn beside(self, beside: Vec<Standing>) -> Self {
        Self { beside, ..self }
    }

    /// Say the builder being made, a line of its own, or nothing once it
    /// stands or won't.
    pub fn asking(&mut self, name: Option<&str>) {
        self.lending
            .say(name.map_or_else(String::new, |name| format!("{name}\n")));
    }

    /// Say the one lent, and those beside that answer, to every build, each
    /// for its big work; the one lent with the standing builders that
    /// answer to a build that asks for them; and its store, a line of its
    /// own, or nothing.
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
        let beside: Vec<String> = self
            .beside
            .iter()
            .filter(|builder| answers(&builder.address))
            .map(|builder| for_big_work(&builder.line))
            .collect();
        self.every
            .say(text(beside.iter().map(String::as_str), lent));
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
            "ssh-ng://lent x86_64-linux /key 8 1 big-parallel,benchmark big-parallel KEY"
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

    /// A stand-in for resolved's socket: answers one request with `reply`,
    /// and hands back what it was asked.
    fn resolved(reply: &'static str) -> (tempfile::TempDir, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let dir = tempfile::tempdir().expect("a folder");
        let socket =
            std::os::unix::net::UnixListener::bind(dir.path().join("resolve")).expect("a socket");
        let asked = std::thread::spawn(move || {
            let (mut stream, _) = socket.accept().expect("a request");
            let mut request = Vec::new();
            let mut byte = [0];
            while stream.read_exact(&mut byte).is_ok() && byte[0] != 0 {
                request.push(byte[0]);
            }
            stream.write_all(reply.as_bytes()).expect("the reply");
            stream.write_all(b"\0").expect("its end");
            String::from_utf8(request).expect("text")
        });
        (dir, asked)
    }

    #[test]
    fn a_name_is_asked_of_resolved_itself_with_its_port_kept() {
        let (dir, asked) = resolved(
            r#"{"parameters":{"addresses":[
                {"ifindex":3,"family":2,"address":[192,0,2,45]},
                {"ifindex":3,"family":10,"address":[254,128,0,0,0,0,0,0,0,0,0,0,0,0,0,45]}
            ],"name":"home.local","flags":0}}"#,
        );
        let found = found("home.local:22", &dir.path().join("resolve"));
        let request: serde_json::Value =
            serde_json::from_str(&asked.join().expect("the stand-in")).expect("json");
        assert_eq!(request["method"], "io.systemd.Resolve.ResolveHostname");
        assert_eq!(request["parameters"]["name"], "home.local");
        assert_eq!(
            found,
            [
                "192.0.2.45:22".parse::<SocketAddr>().expect("v4"),
                SocketAddr::V6(SocketAddrV6::new("fe80::2d".parse().expect("v6"), 22, 0, 3)),
            ]
        );
    }

    #[test]
    fn a_name_resolved_cannot_find_answers_nothing() {
        let (dir, asked) =
            resolved(r#"{"error":"io.systemd.Resolve.NoNameServers","parameters":{}}"#);
        assert!(found("gone.local:22", &dir.path().join("resolve")).is_empty());
        asked.join().expect("the stand-in");
    }

    #[test]
    fn an_address_is_never_asked_and_a_name_falls_back_without_resolved() {
        let nowhere = Path::new("/nonexistent/resolve");
        assert_eq!(
            found("127.0.0.1:9", nowhere),
            ["127.0.0.1:9".parse::<std::net::SocketAddr>().expect("v4")]
        );
        assert!(found("not an address", nowhere).is_empty());
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
        // Gone: nothing listens on port 0, while a port a dropped listener
        // freed can be taken at once by another test binding one.
        drop(listening);
        machines.standing[0].address = "127.0.0.1:0".to_owned();
        machines.show(None);
        assert_eq!(read(&every), "");
        assert_eq!(read(&asked), "");
    }

    // A machine whose owner lets it take any of this one's work, such as a
    // second machine for the same work, is named to every build while it
    // answers, for its big work alone as a lent one is, and never beside
    // the standing builders, which take only what asks for them.
    #[test]
    fn a_builder_beside_is_named_to_every_build_for_its_big_work_while_it_answers() {
        let dir = tempfile::tempdir().expect("a folder");
        let every = dir.path().join("machines");
        let asked = dir.path().join("machines-with-standing");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let mut machines = Machines::new(
            every.clone(),
            asked.clone(),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(vec![Standing {
            address,
            line: "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark".to_owned(),
        }]);
        let read = |path| fs::read_to_string(path).expect("read");
        machines.show(Some(Lending {
            line: "ssh-ng://lent",
            store: None,
        }));
        assert_eq!(
            read(&every),
            "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark big-parallel -\n\
             ssh-ng://lent\n"
        );
        assert_eq!(read(&asked), "ssh-ng://lent\n");
        drop(listening);
        machines.beside[0].address = "127.0.0.1:0".to_owned();
        machines.show(None);
        assert_eq!(read(&every), "");
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
