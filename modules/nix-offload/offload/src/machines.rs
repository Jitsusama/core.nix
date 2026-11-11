//! What Nix and the machine queue read: the one builder lent, if any, for
//! every build; and beside it the builders standing by that answer now, for
//! a build that asks for them by naming their file.
//!
//! Standing builders are never offered to every build, since they may be
//! machines whose owner wants only some work on them: a build of private
//! code must never reach one by default. Builders beside are the other
//! kind, machines that may take any of this one's work, and are named to
//! every build while they answer, for their big work, as the one lent is.
//! A builder beside that fails two proofs in a row (src/proof.rs) is left
//! out of every list until one passes.
//!
//! While the builder lent for every build fails its proofs, or none is
//! lent, the file for any work names stand-ins that passed theirs: the
//! second lent builder, else the warm VM builders, in their own lines for
//! any work. The one lent here is named alone again once it passes.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::cloud::{Cloud, Place};
use crate::freeing::{self, Freeing};
use crate::proof::{self, Health, Keeper, State};
use crate::there;

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
/// and offering nothing a VM test needs. What the line already requires
/// stays required beside that kind, since a builder taking only some
/// builds, such as one whose x86 runs through Rosetta, would otherwise be
/// sent every build of that kind. Nix counts a builder's slots by its URI,
/// so the lines share one set. A line supporting none of them is kept as
/// it is.
pub fn for_big_work(line: &str) -> String {
    let (fields, offered) = offering(line);
    let supported: Vec<&str> = fields[5].split(',').collect();
    let required: Vec<&str> = fields[6]
        .split(',')
        .filter(|feature| !feature.is_empty() && *feature != "-")
        .collect();
    let lines: Vec<String> = BIG
        .iter()
        .filter(|kind| supported.contains(kind))
        .map(|kind| {
            let mut mandatory = required.clone();
            if !mandatory.contains(kind) {
                mandatory.push(kind);
            }
            let mandatory = mandatory.join(",");
            let mut fields = fields.clone();
            fields[5] = &offered;
            fields[6] = &mandatory;
            fields.join(" ")
        })
        .collect();
    if lines.is_empty() {
        line.to_owned()
    } else {
        lines.join("\n")
    }
}

/// A lent builder's machines line with nothing mandatory, so Nix sends it
/// any piece that asks for nothing it lacks, still offering nothing a VM
/// test needs.
pub fn for_any_work(line: &str) -> String {
    let (mut fields, offered) = offering(line);
    fields[5] = &offered;
    fields[6] = "-";
    fields.join(" ")
}

/// A line's eight fields, and the features it supports less a VM test's.
fn offering(line: &str) -> (Vec<&str>, String) {
    let mut fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 {
        fields.resize(8, "-");
    }
    let offered = fields[5]
        .split(',')
        .filter(|feature| !VM.contains(feature))
        .collect::<Vec<_>>()
        .join(",");
    (fields, offered)
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

/// Who the file for any work names, as [`for_anything`] decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    /// The one lent here, which answers or is yet to be proven.
    Lent,
    /// The second lent builder, standing in.
    Second,
    /// This many warm VM builders, standing in.
    Vm(usize),
    /// One lent here that doesn't answer, or none since one was left out,
    /// with nothing to stand in.
    Unanswered,
    /// None is lent, and nothing stands in.
    Nothing,
}

/// The file for any work's text, and who it names: the one lent here, as
/// `lent` names it for any work, while it answers or is yet to be proven,
/// as its proofs' `state` says; else the `second` lent builder's lines
/// beside it while that answers, else the warm `vm` builders' that do.
/// The one lent is named first while it's lent, since one failing once
/// may yet answer. One whose store is full is named only while nothing
/// stands in: it collects at its own min-free as it builds, which beats
/// no builder, but any with room comes first.
pub fn for_anything(
    lent: Option<&str>,
    state: State,
    second: &[String],
    vm: &[String],
) -> (String, Named) {
    let answers = lent.is_some() && matches!(state, State::None | State::Unproven | State::Passing);
    let full = lent.is_some() && state == State::Full;
    let (stand_ins, named): (&[String], Named) = if answers {
        (&[], Named::Lent)
    } else if !second.is_empty() {
        (second, Named::Second)
    } else if !vm.is_empty() {
        (vm, Named::Vm(vm.len()))
    } else if full {
        (&[], Named::Lent)
    } else if lent.is_some() || state == State::Dropped {
        (&[], Named::Unanswered)
    } else {
        (&[], Named::Nothing)
    };
    let lent = lent.filter(|_| !full || stand_ins.is_empty());
    let lines = lent.into_iter().chain(stand_ins.iter().map(String::as_str));
    (text(lines, None), named)
}

/// Say who the file for any work names, once it changes from `was`.
fn say_named(was: Named, named: Named) {
    let stood_in = matches!(was, Named::Second | Named::Vm(_) | Named::Unanswered);
    let said = match named {
        _ if named == was => return,
        Named::Lent if stood_in => {
            "the lent builder answers again, so it alone is named for any work".to_owned()
        }
        Named::Second => {
            "no lent builder answers here; the second lent builder stands in".to_owned()
        }
        Named::Vm(1) => "no lent builder answers; 1 VM builder stands in".to_owned(),
        Named::Vm(count) => format!("no lent builder answers; {count} VM builders stand in"),
        Named::Unanswered => "no builder answers at all".to_owned(),
        Named::Nothing if stood_in => "nothing stands in any more, and none is lent".to_owned(),
        Named::Lent | Named::Nothing => return,
    };
    eprintln!("nix-offload: {said}");
}

/// What a program told of the file for any work hears as it changes from
/// `was` to `named`: `none` and why, once no builder answers any work, and
/// `back` and who, once one does or none is wanted; nothing otherwise.
fn told(was: Named, named: Named) -> Option<(&'static str, String)> {
    let back = |who: &str| Some(("back", format!("a builder answers again: {who}")));
    match (was, named) {
        (Named::Unanswered, Named::Unanswered) => None,
        (_, Named::Unanswered) => Some((
            "none",
            format!(
                "no builder answers: the lent one failed {} proofs in a row and none stands \
                 in, so another is asked for",
                proof::FAILS
            ),
        )),
        (Named::Unanswered, Named::Lent) => back("a newly lent one takes any work"),
        (Named::Unanswered, Named::Second) => back("the second lent builder stands in"),
        (Named::Unanswered, Named::Vm(1)) => back("1 VM builder stands in"),
        (Named::Unanswered, Named::Vm(count)) => back(&format!("{count} VM builders stand in")),
        (Named::Unanswered, Named::Nothing) => {
            Some(("back", "none is lent, and no build waits on one".to_owned()))
        }
        _ => None,
    }
}

/// Run `program` with `word` and `sentence`, waiting for it, and say in the
/// journal when it couldn't run or failed.
fn tell(program: &Path, word: &str, sentence: &str) {
    let ran = std::process::Command::new(program)
        .args([word, sentence])
        .stdin(std::process::Stdio::null())
        .status();
    match ran {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("nix-offload: {} ended {status}", program.display()),
        Err(error) => eprintln!("nix-offload: running {}: {error}", program.display()),
    }
}

/// The lines for any work of the builders that may stand in, the `second`
/// lent builder's while it passed its last proof, and each warm VM
/// builder's of `vm` that did, as of `at`.
fn stand_ins(second: &Path, vm: &[PathBuf], at: u64) -> (Vec<String>, Vec<String>) {
    let lines = |dir: &Path| -> Vec<String> {
        fs::read_to_string(dir.join("machines-for-anything"))
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let second = if proof::passes_in(second, at) {
        lines(second)
    } else {
        Vec::new()
    };
    let vm = vm
        .iter()
        .filter(|dir| proof::passes_in(dir, at) && there::warm(dir))
        .flat_map(|dir| lines(dir))
        .collect();
    (second, vm)
}

/// How the builder beside `line` names stands, keeping it in `kept`:
/// whether it's left out, having failed its proofs or with its store
/// full, saying so as that changes. Where root's prover writes `root`,
/// that says, since only an account the daemon trusts can prove a builder
/// beside; elsewhere it's proven here when due, with `nix`.
fn kept_beside(
    kept: &mut Vec<(String, Keeper)>,
    line: &str,
    (nix, root): (&str, &Path),
    now: Instant,
) -> bool {
    let index = kept
        .iter()
        .position(|(kept, _)| kept == line)
        .unwrap_or_else(|| {
            kept.push((line.to_owned(), Keeper::of(&proof::named(line))));
            kept.len() - 1
        });
    let keeper = &mut kept[index].1;
    let was = keeper.health.state();
    let uri = line.split_whitespace().next().unwrap_or_default();
    if let Some(health) = proof::standing_health(root, uri, &keeper.health.builder, seconds()) {
        keeper.health = health;
    } else {
        let (proving, nix) = (line.to_owned(), nix.to_owned());
        keeper.keep(now, seconds(), move || {
            proof::through_daemon(&proving, &nix, proof::WITHIN)
        });
    }
    let state = keeper.health.state();
    let name = &keeper.health.builder;
    match (was, state) {
        (was, is) if was == is => {}
        (_, State::Dropped) => eprintln!(
            "nix-offload: {name} failed {} proofs in a row, so no build goes there until one \
             passes",
            proof::FAILS
        ),
        (_, State::Full) => eprintln!(
            "nix-offload: {name} has {} GB free, under the {} GB floor, so no build goes \
             there until it has room",
            keeper.health.free.unwrap_or_default() / 1_000_000_000,
            proof::FLOOR / 1_000_000_000
        ),
        (State::Dropped, _) => {
            eprintln!("nix-offload: {name} passed its proof again, so builds go there again");
        }
        (State::Full, State::Passing) => {
            eprintln!("nix-offload: {name} has room again, so builds go there again");
        }
        _ => {}
    }
    keeper.health.left_out()
}

fn seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
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
    /// The files `standing` and `beside` are read again from, once
    /// `read_from` gives them.
    lists: [Option<PathBuf>; 2],
    /// Where the builders beside that answer are named alone, once
    /// `beside` gives it.
    alone: Option<File>,
    /// Where the one lent is named for any work, once `for_anything`
    /// gives it.
    anything: Option<File>,
    /// The line of the one lent as `proven` last said it, and how its
    /// proofs stood then.
    proven: (Option<String>, State),
    /// The second lent builder's folder and the VM builders', whose
    /// builders stand in for the one lent here, once `stand_ins_from`
    /// gives them.
    stand_ins: Option<(PathBuf, Vec<PathBuf>)>,
    /// Who the file for any work named last.
    named: Named,
    /// The program told when no builder answers any work, and again once
    /// one does, once `telling` gives it.
    telling: Option<PathBuf>,
    /// Each builder beside's proofs, by its line.
    kept: Vec<(String, Keeper)>,
    /// The nix the builders beside are proven through.
    nix: String,
    /// Where root's prover says how the builders beside stand.
    root: PathBuf,
    /// The file naming what frees each builder beside, read again with
    /// `lists`, once `freeing_from` gives it.
    freeing_from: Option<PathBuf>,
    /// What frees each builder beside that names one, by its address.
    freeing: Vec<(String, Freeing)>,
    /// Where the one lent is said to be in the cloud, for the service
    /// that writes root's lines, once `in_cloud` gives the files.
    cloud: Option<Cloud>,
    /// Where the one lent is, as `placing` last said.
    place: Option<Place>,
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
            lists: [None, None],
            alone: None,
            anything: None,
            proven: (None, State::None),
            stand_ins: None,
            named: Named::Nothing,
            telling: None,
            kept: Vec::new(),
            nix: "nix".to_owned(),
            root: PathBuf::from(proof::STANDING),
            freeing_from: None,
            freeing: Vec::new(),
            cloud: None,
            place: None,
        }
    }

    /// With `cloud`, where the one lent is said to be, written before
    /// any machines file names it and emptied once none is lent, as
    /// [`crate::cloud`] says.
    #[must_use]
    pub fn in_cloud(self, cloud: Cloud) -> Self {
        Self {
            cloud: Some(cloud),
            ..self
        }
    }

    /// Say where the one lent is, for the next [`Machines::show`].
    pub fn placing(&mut self, place: Option<Place>) {
        self.place = place;
    }

    /// With the file naming what frees each builder beside, read again
    /// each time it names who answers, as [`freeing::programs`] reads it:
    /// a builder beside whose proof finds it full is freed by its program.
    #[must_use]
    pub fn freeing_from(self, path: Option<PathBuf>) -> Self {
        Self {
            freeing_from: path,
            ..self
        }
    }

    /// With the folders whose builders stand in for the one lent here in
    /// the file for any work while it fails its proofs or none is lent: the
    /// `second` lent builder's, then the `vm` builders'.
    #[must_use]
    pub fn stand_ins_from(self, second: PathBuf, vm: Vec<PathBuf>) -> Self {
        Self {
            stand_ins: Some((second, vm)),
            ..self
        }
    }

    /// With `program` told, as [`told`] says, when no builder answers any
    /// work, and again once one does.
    #[must_use]
    pub fn telling(self, program: Option<PathBuf>) -> Self {
        Self {
            telling: program,
            ..self
        }
    }

    /// With `nix` to prove the builders beside through.
    #[cfg(test)]
    #[must_use]
    pub fn proving_with(self, nix: &str) -> Self {
        Self {
            nix: nix.to_owned(),
            ..self
        }
    }

    /// With `root` where root's prover says how the builders beside stand.
    #[cfg(test)]
    #[must_use]
    pub fn proven_by_root_in(self, root: PathBuf) -> Self {
        Self { root, ..self }
    }

    /// Say how the proofs of the one lent, whose line is `line`, stand, or
    /// of the last, once it's gone.
    pub fn proven(&mut self, line: Option<&str>, state: State) {
        self.proven = (line.map(str::to_owned), state);
    }

    /// How the builders beside stand, where there are builders beside.
    pub fn beside_health(&self) -> Option<Vec<Health>> {
        self.alone.as_ref()?;
        Some(
            self.kept
                .iter()
                .map(|(_, keeper)| keeper.health.clone())
                .collect(),
        )
    }

    /// With `anything`, the one lent named alone with nothing mandatory,
    /// for a build that may send any public piece away rather than build
    /// it here, such as one with no local jobs; empty while none is lent.
    /// Never a builder beside or standing: only some work may reach them.
    #[must_use]
    pub fn for_anything(self, anything: PathBuf) -> Self {
        Self {
            anything: Some(File {
                path: anything,
                written: None,
            }),
            ..self
        }
    }

    /// With `beside`, builders standing by whose owner lets them take any
    /// of this machine's work, so they're named to every build while they
    /// answer, and never to a build that asks for the standing ones; and
    /// named alone, as themselves, in `alone`, for a build that may go to
    /// them and nowhere else, empty while none answers.
    #[must_use]
    pub fn beside(self, alone: PathBuf, beside: Vec<Standing>) -> Self {
        Self {
            beside,
            alone: Some(File {
                path: alone,
                written: None,
            }),
            ..self
        }
    }

    /// With the files the standing builders and those beside are read
    /// from, read again each time it names who answers, so a switch that
    /// changes who they are is taken up at once. A lender superseded waits
    /// for its builder to go idle before it starts again, which under load
    /// on 4 November kept a switch's new builders unnamed 2 h 41 min. One
    /// that can't be read leaves its builders as they were.
    #[must_use]
    pub fn read_from(self, standing: Option<PathBuf>, beside: Option<PathBuf>) -> Self {
        Self {
            lists: [standing, beside],
            ..self
        }
    }

    fn reread(&mut self) {
        let [standing, beside] = self.lists.each_ref().map(|path| {
            fs::read_to_string(path.as_ref()?)
                .ok()
                .map(|text| self::standing(&text))
        });
        if let Some(standing) = standing {
            self.standing = standing;
        }
        if let Some(beside) = beside {
            self.beside = beside;
        }
        let Some(programs) = self
            .freeing_from
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|text| freeing::programs(&text))
        else {
            return;
        };
        // A run under way is kept while its builder still names the same
        // program, so a reread never strands one.
        let mut was = std::mem::take(&mut self.freeing);
        for (address, program) in programs {
            let kept = was
                .iter()
                .position(|(was, freeing)| *was == address && freeing.program() == program);
            let freeing = kept.map_or_else(|| Freeing::of(&program), |at| was.swap_remove(at).1);
            self.freeing.push((address, freeing));
        }
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
        self.reread();
        // First, so the service never meets a line before its instance.
        if let Some(cloud) = &mut self.cloud {
            cloud.say(lending.and(self.place.as_ref()));
        }
        self.store.say(
            lending
                .and_then(|lending| lending.store)
                .map_or_else(String::new, |store| format!("{store}\n")),
        );
        if let Some(anything) = &mut self.anything {
            let any = lending.map(|lending| for_any_work(lending.line));
            let (line, state) = &self.proven;
            let state = match lending {
                Some(lending) if line.as_deref() == Some(lending.line) => *state,
                Some(_) => State::Unproven,
                None if line.is_none() => *state,
                None => State::None,
            };
            let (second, vm) = self
                .stand_ins
                .as_ref()
                .map(|(second, vm)| stand_ins(second, vm, seconds()))
                .unwrap_or_default();
            let (said, named) = for_anything(any.as_deref(), state, &second, &vm);
            if self.stand_ins.is_some() {
                say_named(self.named, named);
            }
            if let Some((word, sentence)) = told(self.named, named)
                && let Some(program) = &self.telling
            {
                tell(program, word, &sentence);
            }
            self.named = named;
            anything.say(said);
        }
        let lent = lending.map(|lending| for_big_work(lending.line));
        let lent = lent.as_deref();
        let answering = self
            .standing
            .iter()
            .filter(|builder| answers(&builder.address))
            .map(|builder| builder.line.as_str());
        self.with_standing.say(text(answering, lent));
        // Proofs are kept only of those still listed.
        self.kept
            .retain(|(line, _)| self.beside.iter().any(|builder| &builder.line == line));
        let now = Instant::now();
        let mut beside: Vec<&str> = Vec::new();
        for builder in &self.beside {
            if answers(&builder.address)
                && !kept_beside(&mut self.kept, &builder.line, (&self.nix, &self.root), now)
            {
                beside.push(builder.line.as_str());
            }
        }
        for (address, freeing) in &mut self.freeing {
            let kept = self
                .beside
                .iter()
                .find(|builder| builder.address == *address);
            let full = kept.is_some_and(|builder| {
                self.kept.iter().any(|(line, keeper)| {
                    *line == builder.line && keeper.health.state() == State::Full
                })
            });
            let name = kept.map_or_else(|| address.clone(), |builder| proof::named(&builder.line));
            freeing.tend(&name, full, now);
        }
        if let Some(alone) = &mut self.alone {
            alone.say(text(beside.iter().copied(), None));
        }
        let big: Vec<String> = beside.iter().map(|line| for_big_work(line)).collect();
        self.every.say(text(big.iter().map(String::as_str), lent));
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
    use crate::testing::executable;
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
    fn a_builder_named_for_its_big_work_keeps_what_it_already_requires() {
        assert_eq!(
            for_big_work("ssh-ng://vm x86_64-linux /key 4 0.5 big-parallel,own own KEY"),
            "ssh-ng://vm x86_64-linux /key 4 0.5 big-parallel,own own,big-parallel KEY"
        );
        assert_eq!(
            for_big_work("ssh-ng://vm x86_64-linux /key 4 1 big-parallel big-parallel KEY"),
            "ssh-ng://vm x86_64-linux /key 4 1 big-parallel big-parallel KEY"
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
        assert_eq!(
            found("gone.local:22", &dir.path().join("resolve")),
            [] as [std::net::SocketAddr; 0]
        );
        asked.join().expect("the stand-in");
    }

    #[test]
    fn an_address_is_never_asked_and_a_name_falls_back_without_resolved() {
        let nowhere = Path::new("/nonexistent/resolve");
        assert_eq!(
            found("127.0.0.1:9", nowhere),
            ["127.0.0.1:9".parse::<std::net::SocketAddr>().expect("v4")]
        );
        assert_eq!(
            found("not an address", nowhere),
            [] as [std::net::SocketAddr; 0]
        );
    }

    #[test]
    fn a_builder_answers_only_while_something_listens() {
        // On a loopback address of its own, since a port it frees on the
        // one every other test binds may be taken by one of them at once.
        let listening = TcpListener::bind("127.0.0.3:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        assert!(answers(&address));
        drop(listening);
        // A child another test forks holds the listener open until it
        // execs, which beside the proofs' tests took long enough to be
        // answered, so it's given a moment to go.
        let ending = std::time::Instant::now() + Duration::from_secs(5);
        while answers(&address) && std::time::Instant::now() < ending {
            std::thread::sleep(Duration::from_millis(20));
        }
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
        let alone = dir.path().join("machines-beside");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let mut machines = Machines::new(
            every.clone(),
            asked.clone(),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(
            alone.clone(),
            vec![Standing {
                address,
                line: "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark".to_owned(),
            }],
        );
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
        // Named alone, as itself, for a build that may go there and
        // nowhere else.
        assert_eq!(
            read(&alone),
            "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark\n"
        );
        drop(listening);
        machines.beside[0].address = "127.0.0.1:0".to_owned();
        machines.show(None);
        assert_eq!(read(&every), "");
        assert_eq!(read(&alone), "");
    }

    #[test]
    fn builders_standing_and_beside_are_read_again_each_time_they_are_named() {
        let dir = tempfile::tempdir().expect("a folder");
        let asked = dir.path().join("machines-with-standing");
        let alone = dir.path().join("machines-beside");
        let (standing, beside) = (dir.path().join("standing"), dir.path().join("beside"));
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        fs::write(&standing, format!("{address} ssh-ng://home\n")).expect("write");
        fs::write(&beside, "").expect("write");
        let mut machines = Machines::new(
            dir.path().join("machines"),
            asked.clone(),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(alone.clone(), vec![])
        .read_from(Some(standing.clone()), Some(beside.clone()));
        let read = |path| fs::read_to_string(path).expect("read");
        machines.show(None);
        assert_eq!(read(&asked), "ssh-ng://home\n");
        assert_eq!(read(&alone), "");
        // A switch puts new lists in place, with the lender still running.
        fs::write(&standing, "").expect("write");
        fs::write(&beside, format!("{address} ssh-ng://work\n")).expect("write");
        machines.show(None);
        assert_eq!(read(&asked), "");
        assert_eq!(read(&alone), "ssh-ng://work\n");
        fs::remove_file(&beside).expect("remove");
        machines.show(None);
        assert_eq!(
            read(&alone),
            "ssh-ng://work\n",
            "one unread stays as it was"
        );
    }

    // A build that may go anywhere public, such as a machine's update
    // built with no local jobs, reads the one lent with nothing
    // mandatory, so it takes any piece; never a builder beside or
    // standing, which only some work may reach.
    #[test]
    fn a_lent_builder_is_named_for_any_work_to_builds_that_ask() {
        let dir = tempfile::tempdir().expect("a folder");
        let anything = dir.path().join("machines-for-anything");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let mut machines = Machines::new(
            dir.path().join("machines"),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![Standing {
                address: address.clone(),
                line: "ssh-ng://home".to_owned(),
            }],
        )
        .beside(
            dir.path().join("machines-beside"),
            vec![Standing {
                address,
                line: "ssh-ng://work x86_64-linux - 8 1 big-parallel".to_owned(),
            }],
        )
        .for_anything(anything.clone());
        let read = |path| fs::read_to_string(path).expect("read");
        machines.show(Some(Lending {
            line: "ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark - KEY",
            store: None,
        }));
        assert_eq!(
            read(&anything),
            "ssh-ng://lent x86_64-linux /key 8 1 big-parallel,benchmark - KEY\n"
        );
        machines.show(None);
        assert_eq!(read(&anything), "", "emptied while none is lent");
        drop(listening);
    }

    #[test]
    fn the_lent_builder_is_named_for_any_work_while_it_answers_else_a_stand_in() {
        let lines = |lines: &[&str]| lines.iter().map(|line| (*line).to_owned()).collect();
        let (second, vm): (Vec<String>, Vec<String>) = (lines(&["second"]), lines(&["vm", "vm-3"]));
        for state in [State::None, State::Unproven, State::Passing] {
            assert_eq!(
                for_anything(Some("lent"), state, &second, &vm),
                ("lent\n".to_owned(), Named::Lent),
                "{state:?}"
            );
        }
        assert_eq!(
            for_anything(Some("lent"), State::Failing, &second, &vm),
            ("lent\nsecond\n".to_owned(), Named::Second),
            "one failing once is still named, the second first beside it"
        );
        assert_eq!(
            for_anything(Some("lent"), State::Failing, &[], &vm),
            ("lent\nvm\nvm-3\n".to_owned(), Named::Vm(2))
        );
        assert_eq!(
            for_anything(None, State::Dropped, &[], &vm),
            ("vm\nvm-3\n".to_owned(), Named::Vm(2))
        );
        assert_eq!(
            for_anything(None, State::None, &[], &vm),
            ("vm\nvm-3\n".to_owned(), Named::Vm(2)),
            "none lent"
        );
        assert_eq!(
            for_anything(Some("lent"), State::Failing, &[], &[]),
            ("lent\n".to_owned(), Named::Unanswered)
        );
        assert_eq!(
            for_anything(None, State::Dropped, &[], &[]),
            (String::new(), Named::Unanswered)
        );
        assert_eq!(
            for_anything(None, State::None, &[], &[]),
            (String::new(), Named::Nothing)
        );
        assert_eq!(
            for_anything(Some("lent"), State::Full, &second, &vm),
            ("second\n".to_owned(), Named::Second),
            "one with room stands in for a full one"
        );
        assert_eq!(
            for_anything(Some("lent"), State::Full, &[], &vm),
            ("vm\nvm-3\n".to_owned(), Named::Vm(2))
        );
        assert_eq!(
            for_anything(Some("lent"), State::Full, &[], &[]),
            ("lent\n".to_owned(), Named::Lent),
            "a full one, collecting as it builds, beats none"
        );
    }

    /// A lender's folder whose builder passed its last proof just now,
    /// naming `line` for any work.
    fn passing(dir: &Path, line: &str) {
        fs::create_dir_all(dir).expect("a folder");
        let mut health = Health::of("builder");
        health.heard(true, seconds());
        proof::write(dir, Some(&health), None, seconds());
        fs::write(dir.join("machines-for-anything"), format!("{line}\n")).expect("write");
        fs::write(dir.join("store"), "ssh-ng://far\n").expect("write");
    }

    #[test]
    fn builders_that_passed_stand_in_for_one_lent_that_fails_until_it_passes_again() {
        let dir = tempfile::tempdir().expect("a folder");
        let folder = |name: &str| dir.path().join(name);
        let anything = folder("machines-for-anything");
        let mut machines = Machines::new(
            folder("machines"),
            folder("machines-with-standing"),
            folder("store"),
            folder("lending"),
            vec![],
        )
        .for_anything(anything.clone())
        .stand_ins_from(
            folder("more"),
            vec![folder("vm"), folder("vm-2"), folder("vm-3")],
        );
        passing(
            &folder("vm"),
            "ssh-ng://vm x86_64-linux /k 4 1 big-parallel - K",
        );
        passing(
            &folder("vm-2"),
            "ssh-ng://vm-2 x86_64-linux /k 4 1 big-parallel - K",
        );
        // Warming, so its tests would build the desktop first.
        fs::write(
            folder("vm-2").join(there::WARMING),
            format!("{} vm-2\n", std::process::id()),
        )
        .expect("write");
        // Failed its last proof.
        passing(
            &folder("vm-3"),
            "ssh-ng://vm-3 x86_64-linux /k 4 1 big-parallel - K",
        );
        let mut failed = Health::of("vm-3");
        failed.heard(false, seconds());
        proof::write(&folder("vm-3"), Some(&failed), None, seconds());
        let read = |path| fs::read_to_string(path).expect("read");
        let line = "ssh-ng://lent x86_64-linux /k 8 1 big-parallel - K";
        let lent = Some(Lending { line, store: None });
        machines.proven(Some(line), State::Passing);
        machines.show(lent);
        assert_eq!(read(&anything), format!("{line}\n"));
        machines.proven(Some(line), State::Failing);
        machines.show(lent);
        assert_eq!(
            read(&anything),
            format!("{line}\nssh-ng://vm x86_64-linux /k 4 1 big-parallel - K\n"),
            "the warm VM builder that passed stands in"
        );
        machines.proven(None, State::Dropped);
        machines.show(None);
        assert_eq!(
            read(&anything),
            "ssh-ng://vm x86_64-linux /k 4 1 big-parallel - K\n"
        );
        passing(
            &folder("more"),
            "ssh-ng://more x86_64-linux /k 8 1 big-parallel - K",
        );
        machines.show(None);
        assert_eq!(
            read(&anything),
            "ssh-ng://more x86_64-linux /k 8 1 big-parallel - K\n",
            "the second lent builder stands in first"
        );
        // A new builder stands, its line not yet proven.
        let next = "ssh-ng://next x86_64-linux /k 8 1 big-parallel - K";
        machines.show(Some(Lending {
            line: next,
            store: None,
        }));
        assert_eq!(read(&anything), format!("{next}\n"), "named alone again");
        fs::remove_file(folder("more").join(proof::HEALTH)).expect("remove");
        fs::remove_file(folder("vm").join(proof::HEALTH)).expect("remove");
        machines.proven(None, State::Dropped);
        machines.show(None);
        assert_eq!(read(&anything), "", "no builder answers at all");
    }

    // Whoever keeps the machine's builds moving hears the moment no
    // builder answers, rather than finding a switch stalled, and hears
    // once more when one does; once each, however many ticks pass.
    #[test]
    fn no_builder_answering_is_told_once_and_one_answering_again_once() {
        let dir = tempfile::tempdir().expect("a folder");
        let folder = |name: &str| dir.path().join(name);
        let (program, told) = (folder("tell"), folder("told"));
        executable(
            &program,
            &format!("#!/bin/sh\necho \"$1: $2\" >> '{}'\n", told.display()),
        );
        let mut machines = Machines::new(
            folder("machines"),
            folder("machines-with-standing"),
            folder("store"),
            folder("lending"),
            vec![],
        )
        .for_anything(folder("machines-for-anything"))
        .stand_ins_from(folder("more"), vec![folder("vm")])
        .telling(Some(program));
        let heard = || fs::read_to_string(&told).unwrap_or_default();
        let line = "ssh-ng://lent x86_64-linux /k 8 1 big-parallel - K";
        machines.proven(Some(line), State::Passing);
        machines.show(Some(Lending { line, store: None }));
        assert_eq!(heard(), "", "nothing to tell while the lent one answers");
        machines.proven(None, State::Dropped);
        machines.show(None);
        machines.show(None);
        assert_eq!(
            heard(),
            "none: no builder answers: the lent one failed 2 proofs in a row and none \
             stands in, so another is asked for\n",
            "told once"
        );
        let next = "ssh-ng://next x86_64-linux /k 8 1 big-parallel - K";
        machines.show(Some(Lending {
            line: next,
            store: None,
        }));
        machines.show(Some(Lending {
            line: next,
            store: None,
        }));
        assert_eq!(
            heard().lines().skip(1).collect::<Vec<_>>(),
            ["back: a builder answers again: a newly lent one takes any work"],
            "told once more"
        );
    }

    #[test]
    fn each_way_back_from_no_builder_answering_says_which() {
        assert_eq!(
            told(Named::Lent, Named::Unanswered).map(|(word, _)| word),
            Some("none")
        );
        assert_eq!(told(Named::Unanswered, Named::Unanswered), None);
        assert_eq!(told(Named::Lent, Named::Second), None, "a stand-in answers");
        assert_eq!(
            told(Named::Unanswered, Named::Second),
            Some((
                "back",
                "a builder answers again: the second lent builder stands in".into()
            ))
        );
        assert_eq!(
            told(Named::Unanswered, Named::Vm(2)),
            Some((
                "back",
                "a builder answers again: 2 VM builders stand in".into()
            ))
        );
        assert_eq!(
            told(Named::Unanswered, Named::Nothing),
            Some(("back", "none is lent, and no build waits on one".into()))
        );
    }

    /// A stand-in for nix that exits as the `status` file beside it says,
    /// its proof's log saying the store has what the `free` file does,
    /// where there is one.
    fn nix_in(dir: &Path) -> String {
        let nix = dir.join("nix");
        let (status, free) = (dir.join("status"), dir.join("free"));
        executable(
            &nix,
            &format!(
                "#!/bin/sh\n\
                 if [ -e '{free}' ]; then\n\
                 echo \"nix-offload-proof> nix-offload-free $(cat '{free}')\" >&2\n\
                 fi\n\
                 exit \"$(cat '{status}')\"\n",
                free = free.display(),
                status = status.display()
            ),
        );
        nix.display().to_string()
    }

    /// Whether the builder beside `line` names is left out, once the proof
    /// due at `now` has ended, waiting at most 5 s.
    fn left_out(kept: &mut Vec<(String, Keeper)>, line: &str, nix: &str, now: Instant) -> bool {
        let heard = |kept: &Vec<(String, Keeper)>| {
            kept.first().map(|(_, keeper)| {
                (
                    keeper.health.last_success,
                    keeper.health.last_failure,
                    keeper.health.failures,
                    keeper.health.free,
                )
            })
        };
        let before = heard(kept);
        let ending = Instant::now() + Duration::from_secs(5);
        // No prover of root's writes here, so the builder is proven through `nix`.
        let proving = (nix, Path::new("/nonexistent/nix-offload-standing.json"));
        let mut dropped = kept_beside(kept, line, proving, now);
        // One just kept has heard no proof yet.
        let unheard = |kept: &Vec<(String, Keeper)>| {
            heard(kept) == before || heard(kept) == Some((None, None, 0, None))
        };
        while unheard(kept) && Instant::now() < ending {
            std::thread::sleep(Duration::from_millis(10));
            dropped = kept_beside(kept, line, proving, now);
        }
        assert_ne!(heard(kept), before, "no proof ended");
        dropped
    }

    #[test]
    fn a_builder_beside_stands_as_roots_prover_says_where_one_writes() {
        let dir = tempfile::tempdir().expect("a folder");
        let alone = dir.path().join("machines-beside");
        let root = dir.path().join("standing.json");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
        // A nix that fails every proof, so only root's word can list it.
        let failing = dir.path().join("nix");
        executable(&failing, "#!/bin/sh\nexit 1\n");
        let mut machines = Machines::new(
            dir.path().join("machines"),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(
            alone.clone(),
            vec![Standing {
                address,
                line: line.to_owned(),
            }],
        )
        .proving_with(&failing.display().to_string())
        .proven_by_root_in(root.clone());
        let says = |state: &str, free: u64, at: u64| {
            let said = serde_json::json!({"at": at, "builders": [{"uri": "ssh-ng://work",
                "builder": "work", "state": state, "lastSuccess": at, "lastFailure": null,
                "consecutiveFailures": 0, "freeBytes": free}]});
            fs::write(&root, said.to_string()).expect("write");
        };
        let read = |path| fs::read_to_string(path).expect("read");
        says("passing", 80_000_000_000, seconds());
        machines.show(None);
        assert_eq!(
            read(&alone),
            format!("{line}\n"),
            "listed while root says it passes"
        );
        says("full", 6_000_000_000, seconds());
        machines.show(None);
        assert_eq!(
            read(&alone),
            "",
            "left out while root says its store is under the floor"
        );
        says("passing", 80_000_000_000, seconds() - proof::FRESH - 1);
        machines.show(None);
        assert_eq!(
            read(&alone),
            "",
            "left out once root's prover has gone quiet"
        );
        let health = machines.beside_health().expect("builders beside");
        assert_eq!(
            health[0].state(),
            State::Dropped,
            "a quiet prover vouches for nothing"
        );
    }

    #[test]
    fn a_builder_beside_failing_two_proofs_in_a_row_is_left_out_until_one_passes() {
        let dir = tempfile::tempdir().expect("a folder");
        let nix = nix_in(dir.path());
        let status = dir.path().join("status");
        let every = dir.path().join("machines");
        let alone = dir.path().join("machines-beside");
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
        let mut machines = Machines::new(
            every.clone(),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(
            alone.clone(),
            vec![Standing {
                address,
                line: line.to_owned(),
            }],
        )
        .proving_with(&nix);
        let read = |path| fs::read_to_string(path).expect("read");
        let began = Instant::now();
        fs::write(&status, "1").expect("write");
        assert!(
            !left_out(&mut machines.kept, line, &nix, began),
            "one failure is a blip"
        );
        machines.show(None);
        assert_eq!(read(&alone), format!("{line}\n"));
        assert!(left_out(
            &mut machines.kept,
            line,
            &nix,
            began + proof::EVERY
        ));
        machines.show(None);
        assert_eq!(read(&alone), "");
        assert_eq!(read(&every), "");
        let health = machines.beside_health().expect("builders beside");
        assert_eq!(health[0].builder, "work");
        assert_eq!(health[0].state(), State::Dropped);
        fs::write(&status, "0").expect("write");
        assert!(!left_out(
            &mut machines.kept,
            line,
            &nix,
            began + proof::EVERY * 2
        ));
        machines.show(None);
        assert_eq!(read(&alone), format!("{line}\n"), "back once one passes");
        drop(listening);
    }

    // On 9 November the work Mac's builder had 6 GB free, and every check
    // sent there failed for room while its proofs passed.
    #[test]
    fn a_builder_beside_under_the_floor_is_left_out_until_it_has_room() {
        let dir = tempfile::tempdir().expect("a folder");
        let nix = nix_in(dir.path());
        let (status, free) = (dir.path().join("status"), dir.path().join("free"));
        let (every, alone) = (
            dir.path().join("machines"),
            dir.path().join("machines-beside"),
        );
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
        let mut machines = Machines::new(
            every.clone(),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(
            alone.clone(),
            vec![Standing {
                address,
                line: line.to_owned(),
            }],
        )
        .proving_with(&nix);
        let read = |path| fs::read_to_string(path).expect("read");
        let began = Instant::now();
        fs::write(&status, "0").expect("write");
        fs::write(&free, (proof::FLOOR - 1).to_string()).expect("write");
        assert!(
            left_out(&mut machines.kept, line, &nix, began),
            "full at once"
        );
        machines.show(None);
        assert_eq!(read(&alone), "");
        assert_eq!(read(&every), "");
        let health = machines.beside_health().expect("builders beside");
        assert_eq!(health[0].state(), State::Full);
        fs::write(&free, (proof::FLOOR * 2).to_string()).expect("write");
        assert!(
            !left_out(&mut machines.kept, line, &nix, began + proof::EVERY),
            "a builder with room again is taken back"
        );
        machines.show(None);
        assert_eq!(read(&alone), format!("{line}\n"), "back once it has room");
        drop(listening);
    }

    // The same builder sat full until a person freed it, since nothing
    // here could.
    #[test]
    fn a_full_builder_beside_is_freed_by_the_program_it_names() {
        let dir = tempfile::tempdir().expect("a folder");
        let nix = nix_in(dir.path());
        let (status, free) = (dir.path().join("status"), dir.path().join("free"));
        let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = listening.local_addr().expect("its address").to_string();
        let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
        let (program, ran) = (dir.path().join("free-work"), dir.path().join("ran"));
        executable(
            &program,
            &format!("#!/bin/sh\necho freed >> '{}'\n", ran.display()),
        );
        let freeing = dir.path().join("freeing");
        fs::write(&freeing, format!("{address} {}\n", program.display())).expect("write");
        let mut machines = Machines::new(
            dir.path().join("machines"),
            dir.path().join("machines-with-standing"),
            dir.path().join("store"),
            dir.path().join("lending"),
            vec![],
        )
        .beside(
            dir.path().join("machines-beside"),
            vec![Standing {
                address,
                line: line.to_owned(),
            }],
        )
        .freeing_from(Some(freeing))
        .proving_with(&nix);
        fs::write(&status, "0").expect("write");
        fs::write(&free, (proof::FLOOR * 2).to_string()).expect("write");
        assert!(
            !left_out(&mut machines.kept, line, &nix, Instant::now()),
            "one with room takes builds"
        );
        machines.show(None);
        assert!(!ran.exists(), "one with room is never freed");
        fs::write(&free, (proof::FLOOR - 1).to_string()).expect("write");
        assert!(
            left_out(
                &mut machines.kept,
                line,
                &nix,
                Instant::now() + proof::EVERY
            ),
            "full"
        );
        let ending = Instant::now() + Duration::from_secs(5);
        while !ran.exists() && Instant::now() < ending {
            machines.show(None);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            fs::read_to_string(&ran).expect("freed"),
            "freed\n",
            "a full one is freed once"
        );
        drop(listening);
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
