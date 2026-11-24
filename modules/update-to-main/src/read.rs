//! A hosted main's folder read from its host's REST API, tree by tree and
//! blob by blob, rather than fetched into the account's checkout. A fetch
//! that moves main checks afterwards that the new head's whole tree is
//! there, every object of it however little moved: in a repository of 97
//! million objects that took six minutes of a seven-minute fetch on 10
//! November, while the folder's few hundred files read in five seconds.
//!
//! Only two answers are the host's word: which tree the commit holds, for a
//! commit whose id is the one `ls-remote` named, and whether one commit
//! descends from another. Both come over TLS with the token main's head is
//! asked with, so they're as good as that head. Every tree on the way down
//! to the folder, and every tree and blob in it, is checked against the id
//! the tree above names before it's used, so the host can give nothing the
//! commit doesn't hold. What root's copy already reads, through the
//! checkout's objects or an earlier read, isn't asked for again.
//!
//! What a read found is kept beside root's copy, so the rest of an update
//! finds the folder, and whether main descends, for a commit the copy
//! hasn't got.
//!
//! Long because the read, its checks against each id and the record it
//! keeps change together, for one reason: what a hosted folder holds.

use crate::config::{Config, Hosted};
use crate::git::{self, Repo};
use crate::run::{self, Failure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How many of the host's answers are asked for at once.
const AT_ONCE: &str = "16";

/// How long a whole read may take before main is fetched instead. A host
/// that answers reads a folder of a few hundred files in seconds, so this
/// only ever ends one that hangs, and the fetch comes in minutes.
const READ_FOR: Duration = Duration::from_secs(120);

/// The longest one answer may take, in seconds.
const ONE_ANSWER: u64 = 60;

/// What a read of main's head found: the folder's tree, and whether the
/// head descends from `from`, the commit the machine last switched to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Found {
    pub folder: String,
    pub from: Option<String>,
    pub descends: bool,
}

/// What an earlier read found for `commit`, if one did.
pub fn found(config: &Config, commit: &str) -> Option<Found> {
    let text = fs::read_to_string(config.reads().join(commit)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Reads the folder at `head` from the host's API at `api`, asked with
/// `token`, into root's copy, and keeps what it found, with whether `head`
/// descends from `from`. Earlier reads are let go, but for `from`'s, which
/// the next update compares its folder with. A folder where it was at
/// `from` is given back unkept and unasked how main moved, as not
/// descending: the machine stays at `from` then, so nothing reads it.
pub fn read(
    config: &Config,
    hosted: &Hosted,
    api: &str,
    token: &str,
    head: &str,
    from: Option<&str>,
) -> Result<Found, Failure> {
    let host = Host::new(config, api, token)?;
    let folder = folder(&host, hosted, head)?;
    // On 12 November the host refused how main moved twice, with 429 and
    // 422, and main was fetched for minutes though its folder hadn't.
    if let Some(from) = from
        && crate::hosted::folder(config, hosted, from).as_ref() == Some(&folder)
    {
        return Ok(Found {
            folder,
            from: Some(from.to_owned()),
            descends: false,
        });
    }
    let descends = match from {
        Some(from) => host.descends(from, head)?,
        None => true,
    };
    let found = Found {
        folder,
        from: from.map(str::to_owned),
        descends,
    };
    keep(config, head, &found, from)?;
    Ok(found)
}

/// Whether `head` descends from `older`, by the host's word, for a head
/// root's copy hasn't got.
pub fn descends(
    config: &Config,
    api: &str,
    token: &str,
    older: &str,
    head: &str,
) -> Result<bool, Failure> {
    Host::new(config, api, token)?.descends(older, head)
}

fn keep(config: &Config, head: &str, found: &Found, from: Option<&str>) -> Result<(), Failure> {
    let reads = config.reads();
    fs::create_dir_all(&reads).map_err(|e| Failure::new(format!("{}: {e}", reads.display())))?;
    if let Ok(entries) = fs::read_dir(&reads) {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            if Some(name.to_string_lossy().as_ref()) != from {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    let text = serde_json::to_string(found).map_err(|e| Failure::new(e.to_string()))?;
    let file = reads.join(head);
    fs::write(&file, text).map_err(|e| Failure::new(format!("{}: {e}", file.display())))
}

/// The folder's tree at `head`, with everything in it written to root's
/// copy.
fn folder(host: &Host, hosted: &Hosted, head: &str) -> Result<String, Failure> {
    let commit = host.get_one(&format!("git/commits/{head}"))?;
    if commit["sha"].as_str() != Some(head) {
        return Err(Failure::new(format!(
            "the host's API answered for another commit than {}",
            git::short(head)
        )));
    }
    let mut tree = commit["tree"]["sha"]
        .as_str()
        .filter(|sha| is_id(sha))
        .ok_or_else(|| Failure::new("the host's API named no tree for main's head"))?
        .to_owned();
    for part in hosted.directory.split('/').filter(|p| !p.is_empty()) {
        let entries = host.tree(&tree)?;
        if host.ids(&[tree_bytes(&entries)?], "tree", false)? != [tree.clone()] {
            return Err(Failure::new(format!(
                "the host's API listed tree {} with other contents",
                git::short(&tree)
            )));
        }
        tree = entries
            .into_iter()
            .find(|e| e.name == part && e.kind == "tree")
            .map(|e| e.id)
            .ok_or_else(|| Failure::new(format!("main has no folder {}", hosted.directory)))?;
    }
    fill(host, &tree)?;
    Ok(tree)
}

/// Writes the tree `top` and everything in it that root's copy can't read
/// yet, each checked against the id its tree names. A tree root's copy
/// already holds is read from it and gone down too, rather than taken as
/// whole: a pack can bring a tree without what it holds, as one carrying a
/// commit no ref reaches did on 23 November, a speculative merge's, whose
/// files were in a branch the checkout never had.
fn fill(host: &Host, top: &str) -> Result<(), Failure> {
    let mut listed: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    // The trees the host listed, which root's copy is written.
    let mut asked: BTreeSet<String> = BTreeSet::new();
    let mut wanted = vec![top.to_owned()];
    while !wanted.is_empty() {
        let absent: BTreeSet<String> = host.absent(&wanted)?.into_iter().collect();
        let (from_host, held): (Vec<String>, Vec<String>) =
            wanted.into_iter().partition(|id| absent.contains(id));
        let mut found = from_host
            .iter()
            .cloned()
            .zip(host.trees(&from_host)?)
            .collect::<Vec<_>>();
        found.extend(held.iter().cloned().zip(host.held_trees(&held)?));
        asked.extend(from_host);
        let mut next = BTreeSet::new();
        for (id, entries) in found {
            next.extend(
                entries
                    .iter()
                    .filter(|e| e.kind == "tree" && !listed.contains_key(&e.id))
                    .map(|e| e.id.clone()),
            );
            listed.insert(id, entries);
        }
        next.retain(|id| !listed.contains_key(id));
        wanted = next.into_iter().collect();
    }
    let blobs: BTreeSet<String> = listed
        .values()
        .flatten()
        .filter(|e| e.kind == "blob")
        .map(|e| e.id.clone())
        .collect();
    let blobs = host.absent(&blobs.into_iter().collect::<Vec<_>>())?;
    let contents = host.blobs(&blobs)?;
    if host.ids(&contents, "blob", true)? != blobs {
        return Err(Failure::new(
            "the host's API gave a file whose contents aren't the ones its tree names",
        ));
    }
    // Each tree after the trees in it, so none is in root's copy before
    // what it holds.
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    after_its_trees(top, &listed, &mut seen, &mut order);
    order.retain(|id| asked.contains(id));
    let bytes = order
        .iter()
        .map(|id| tree_bytes(&listed[id]))
        .collect::<Result<Vec<_>, _>>()?;
    if host.ids(&bytes, "tree", true)? != order {
        return Err(Failure::new(
            "the host's API listed a tree whose contents aren't the ones its id names",
        ));
    }
    Ok(())
}

fn after_its_trees(
    id: &str,
    listed: &BTreeMap<String, Vec<Entry>>,
    seen: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    let Some(entries) = listed.get(id) else {
        return;
    };
    if !seen.insert(id.to_owned()) {
        return;
    }
    for entry in entries.iter().filter(|e| e.kind == "tree") {
        after_its_trees(&entry.id, listed, seen, order);
    }
    order.push(id.to_owned());
}

/// One entry of a tree as the host lists it.
#[derive(Clone, Debug)]
struct Entry {
    name: String,
    mode: String,
    kind: String,
    id: String,
}

impl Entry {
    /// Whether it's one a tree may hold: a name of one part, a kind git
    /// knows, a whole id and a mode of digits.
    fn sound(&self) -> bool {
        let named = !self.name.is_empty()
            && !self.name.contains(['/', '\0'])
            && self.name != "."
            && self.name != "..";
        let known = matches!(self.kind.as_str(), "blob" | "tree" | "commit");
        named
            && known
            && is_id(&self.id)
            && !self.mode.is_empty()
            && self.mode.bytes().all(|b| b.is_ascii_digit())
    }
}

fn entries(listing: &Value, id: &str) -> Result<Vec<Entry>, Failure> {
    let bad = || {
        Failure::new(format!(
            "the host's API listed tree {} in a form it doesn't read",
            git::short(id)
        ))
    };
    if listing["sha"].as_str() != Some(id) || listing["truncated"].as_bool() == Some(true) {
        return Err(bad());
    }
    listing["tree"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|e| {
            let text = |field: &str| e[field].as_str().map(str::to_owned).ok_or_else(bad);
            let entry = Entry {
                name: text("path")?,
                mode: text("mode")?,
                kind: text("type")?,
                id: text("sha")?,
            };
            entry.sound().then_some(entry).ok_or_else(bad)
        })
        .collect()
}

/// The bytes of a raw object id as git writes it, two hex digits each.
fn hex_of(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// What an entry of mode `mode` is, read as git reads it: as an octal
/// number whose type bits say, so a zero-padded `040000` an old tool wrote
/// is a tree still, not a file.
fn kind_of(mode: &str) -> Option<&'static str> {
    const TYPE: u32 = 0o170_000;
    let mode = u32::from_str_radix(mode, 8).ok()?;
    Some(match mode & TYPE {
        0o040_000 => "tree",
        0o160_000 => "commit",
        _ => "blob",
    })
}

/// The entries of the tree `id`, from its object as git stores it, each
/// `<mode> <name>\0<id's bytes>`, checked as the host's are. A tree's
/// mode says what each entry is, as git's own listing reads it.
fn held_entries(object: &[u8], id: &str) -> Result<Vec<Entry>, Failure> {
    let bad = || {
        Failure::new(format!(
            "root's copy holds tree {} in a form it doesn't read",
            git::short(id)
        ))
    };
    let width = id.len() / 2;
    let mut rest = object;
    let mut entries = Vec::new();
    while !rest.is_empty() {
        let space = rest.iter().position(|b| *b == b' ').ok_or_else(bad)?;
        let end = rest.iter().position(|b| *b == 0).ok_or_else(bad)?;
        if end < space || rest.len() < end + 1 + width {
            return Err(bad());
        }
        let mode = String::from_utf8_lossy(&rest[..space]).into_owned();
        let kind = kind_of(&mode).ok_or_else(bad)?;
        let entry = Entry {
            name: String::from_utf8_lossy(&rest[space + 1..end]).into_owned(),
            kind: kind.to_owned(),
            id: hex_of(&rest[end + 1..end + 1 + width]),
            mode,
        };
        if !entry.sound() {
            return Err(bad());
        }
        entries.push(entry);
        rest = &rest[end + 1 + width..];
    }
    Ok(entries)
}

/// The trees `ids` as `git cat-file --batch` gives them, each
/// `<id> <type> <size>\n<object>\n`, in their order. Anything but a
/// tree, or missing, is refused: an id a tree names as a tree is one.
fn held_objects<'s>(said: &'s [u8], ids: &[String]) -> Result<Vec<&'s [u8]>, Failure> {
    let mut rest = said;
    ids.iter()
        .map(|id| {
            let bad = || {
                Failure::new(format!(
                    "root's copy didn't give tree {} as a tree",
                    git::short(id)
                ))
            };
            let line = rest.iter().position(|b| *b == b'\n').ok_or_else(bad)?;
            let header = String::from_utf8_lossy(&rest[..line]).into_owned();
            let size: usize = match header.split(' ').collect::<Vec<_>>()[..] {
                [named, "tree", size] if named == id => size.parse().map_err(|_| bad())?,
                _ => return Err(bad()),
            };
            let body = rest.get(line + 1..line + 1 + size).ok_or_else(bad)?;
            rest = rest.get(line + 2 + size..).ok_or_else(bad)?;
            Ok(body)
        })
        .collect()
}

/// A tree as git stores it, from its entries in the order the host gives:
/// git's own, or the tree's id won't come out as the one named.
fn tree_bytes(entries: &[Entry]) -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    for entry in entries {
        let mode = entry.mode.trim_start_matches('0');
        bytes.extend_from_slice(mode.as_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(entry.name.as_bytes());
        bytes.push(0);
        bytes.extend(hex(&entry.id).ok_or_else(|| Failure::new("a tree entry's id isn't hex"))?);
    }
    Ok(bytes)
}

/// Whether `id` is a whole object id, SHA-1's or SHA-256's.
fn is_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn hex(id: &str) -> Option<Vec<u8>> {
    (0..id.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(id.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Standard base64, as the host gives a blob's contents, with the line
/// breaks it puts in them.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut bits = 0u32;
    let mut held = 0;
    for c in text.bytes().filter(|c| !c.is_ascii_whitespace()) {
        if c == b'=' {
            break;
        }
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        held += 6;
        if held >= 8 {
            held -= 8;
            bytes.push(u8::try_from(bits >> held).ok()?);
            bits &= (1 << held) - 1;
        }
    }
    Some(bytes)
}

/// The host's API for the repository, asked with a token, its answers
/// written to a folder of root's own while they're read.
struct Host<'a> {
    config: &'a Config,
    api: String,
    token: &'a str,
    work: PathBuf,
    until: Instant,
}

impl<'a> Host<'a> {
    fn new(config: &'a Config, api: &str, token: &'a str) -> Result<Host<'a>, Failure> {
        let work = config.reads().join(".work");
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).map_err(|e| Failure::new(format!("{}: {e}", work.display())))?;
        Ok(Host {
            config,
            api: api.trim_end_matches('/').to_owned(),
            token,
            work,
            until: Instant::now() + READ_FOR,
        })
    }

    fn repo(&self) -> Repo<'a> {
        Repo::main(self.config)
    }

    /// The host's answers to `paths` under its API, each as JSON. The token
    /// goes to curl in a file only root reads, rather than its arguments.
    fn get(&self, paths: &[String]) -> Result<Vec<Value>, Failure> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let Some((each, retrying)) = allowed(self.until.saturating_duration_since(Instant::now()))
        else {
            return Err(Failure::new(format!(
                "reading main's folder from the host took over {} s",
                READ_FOR.as_secs()
            )));
        };
        let asks = self.work.join("asks");
        let mut said = format!("header = \"Authorization: Bearer {}\"\n", self.token);
        let answers: Vec<PathBuf> = (0..paths.len())
            .map(|n| self.work.join(format!("answer-{n}")))
            .collect();
        for (path, answer) in paths.iter().zip(&answers) {
            let _ = write!(
                said,
                "url = \"{}/{path}\"\noutput = \"{}\"\n",
                self.api,
                answer.display()
            );
        }
        write_private(&asks, said.as_bytes())?;
        let curl = run::command(&self.config.tools.curl, &self.config.state);
        let mut args = vec![OsString::from("--config"), asks.clone().into_os_string()];
        args.extend(
            [
                "--silent",
                "--show-error",
                "--fail",
                "--fail-early",
                "--parallel",
                "--parallel-max",
                AT_ONCE,
                "--connect-timeout",
                "10",
                "--retry",
                "2",
                "--header",
                "Accept: application/json",
                // Which answer failed, and how, for the journal: curl's
                // own error doesn't say which of its parallel asks it was.
                "--write-out",
                "%{onerror}%{url} answered %{http_code}\n",
            ]
            .map(OsString::from),
        );
        args.extend([
            OsString::from("--max-time"),
            OsString::from(each.to_string()),
            OsString::from("--retry-max-time"),
            OsString::from(retrying.to_string()),
        ]);
        let asked = run::output(curl, "reading main's folder from the host", args);
        let _ = fs::remove_file(&asks);
        asked?;
        answers
            .iter()
            .map(|answer| {
                let text = fs::read_to_string(answer)
                    .map_err(|e| Failure::new(format!("{}: {e}", answer.display())))?;
                let _ = fs::remove_file(answer);
                serde_json::from_str(&text)
                    .map_err(|e| Failure::new(format!("reading the host's answer: {e}")))
            })
            .collect()
    }

    fn get_one(&self, path: &str) -> Result<Value, Failure> {
        self.get(&[path.to_owned()])?
            .pop()
            .ok_or_else(|| Failure::new("the host gave no answer"))
    }

    fn tree(&self, id: &str) -> Result<Vec<Entry>, Failure> {
        entries(&self.get_one(&format!("git/trees/{id}"))?, id)
    }

    fn trees(&self, ids: &[String]) -> Result<Vec<Vec<Entry>>, Failure> {
        let paths: Vec<String> = ids.iter().map(|id| format!("git/trees/{id}")).collect();
        self.get(&paths)?
            .iter()
            .zip(ids)
            .map(|(listing, id)| entries(listing, id))
            .collect()
    }

    /// The entries of each of the trees `ids` root's copy holds, in their
    /// order, read by one git for them all, each checked against its id.
    /// A process each cost about 9 ms on a busy workstation, so a folder of a hundred
    /// trees took most of a second on every read.
    fn held_trees(&self, ids: &[String]) -> Result<Vec<Vec<Entry>>, Failure> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let list = self.work.join("held");
        fs::write(&list, ids.join("\n") + "\n")
            .map_err(|e| Failure::new(format!("{}: {e}", list.display())))?;
        let mut command = git::git(&self.repo());
        command.stdin(File::open(&list).map_err(|e| Failure::new(e.to_string()))?);
        let said = run::bytes(
            command,
            "reading main's folder in root's copy",
            ["cat-file", "--batch"],
        )?;
        let objects = held_objects(&said, ids)?;
        let owned: Vec<Vec<u8>> = objects.iter().map(|o| o.to_vec()).collect();
        let hashed = self.ids(&owned, "tree", false)?;
        if hashed.len() != ids.len() {
            return Err(Failure::new("git didn't hash every tree of root's copy"));
        }
        if let Some(id) = ids
            .iter()
            .zip(&hashed)
            .find(|(id, h)| id != h)
            .map(|(id, _)| id)
        {
            return Err(Failure::new(format!(
                "root's copy holds tree {} with contents its id doesn't name",
                git::short(id)
            )));
        }
        objects
            .iter()
            .zip(ids)
            .map(|(object, id)| held_entries(object, id))
            .collect()
    }

    /// Each blob's contents, in the order of `ids`.
    fn blobs(&self, ids: &[String]) -> Result<Vec<Vec<u8>>, Failure> {
        let paths: Vec<String> = ids.iter().map(|id| format!("git/blobs/{id}")).collect();
        self.get(&paths)?
            .iter()
            .zip(ids)
            .map(|(blob, id)| {
                let bad = || {
                    Failure::new(format!(
                        "the host's API gave blob {} in a form it doesn't read",
                        git::short(id)
                    ))
                };
                if blob["sha"].as_str() != Some(id) || blob["encoding"].as_str() != Some("base64") {
                    return Err(bad());
                }
                blob["content"].as_str().and_then(base64).ok_or_else(bad)
            })
            .collect()
    }

    /// Of `ids`, those root's copy can't read, in their order.
    fn absent(&self, ids: &[String]) -> Result<Vec<String>, Failure> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let list = self.work.join("ids");
        fs::write(&list, ids.join("\n") + "\n")
            .map_err(|e| Failure::new(format!("{}: {e}", list.display())))?;
        let mut command = git::git(&self.repo());
        command.stdin(File::open(&list).map_err(|e| Failure::new(e.to_string()))?);
        let said = run::output(
            command,
            "looking for main's folder in root's copy",
            ["cat-file", "--batch-check=%(objectname)"],
        )?;
        Ok(said
            .lines()
            .filter_map(|line| line.strip_suffix(" missing"))
            .map(str::to_owned)
            .collect())
    }

    /// The ids git gives each of `objects` as a `kind`, writing them to
    /// root's copy when `write` is set.
    fn ids(&self, objects: &[Vec<u8>], kind: &str, write: bool) -> Result<Vec<String>, Failure> {
        if objects.is_empty() {
            return Ok(Vec::new());
        }
        let files: Vec<PathBuf> = (0..objects.len())
            .map(|n| self.work.join(format!("{kind}-{n}")))
            .collect();
        for (file, object) in files.iter().zip(objects) {
            fs::write(file, object)
                .map_err(|e| Failure::new(format!("{}: {e}", file.display())))?;
        }
        let mut args = vec![OsString::from("hash-object"), "-t".into(), kind.into()];
        if write {
            args.push("-w".into());
        }
        args.push("--".into());
        args.extend(files.iter().map(|f| f.clone().into_os_string()));
        let said = run::output(git::git(&self.repo()), "writing main's folder", args);
        for file in &files {
            let _ = fs::remove_file(file);
        }
        Ok(said?.lines().map(str::to_owned).collect())
    }

    /// Whether `newer` descends from `older`, by the host's word.
    fn descends(&self, older: &str, newer: &str) -> Result<bool, Failure> {
        let compared = self.get_one(&compare(older, newer))?;
        match compared["status"].as_str() {
            Some("identical") => Ok(true),
            Some("ahead") => Ok(compared["merge_base_commit"]["sha"].as_str() == Some(older)),
            Some("behind" | "diverged") => Ok(false),
            _ => Err(Failure::new(format!(
                "the host's API didn't say how {} and {} stand",
                git::short(older),
                git::short(newer)
            ))),
        }
    }
}

/// The host's comparison of `older` with `newer`, asked for what says how
/// they stand alone. The whole answer lists every commit and file between
/// them, which grows with each merge, while the second page of one commit
/// carries the status and merge base with no files at all.
fn compare(older: &str, newer: &str) -> String {
    format!("compare/{older}...{newer}?per_page=1&page=2")
}

/// Writes `bytes` to a file only its owner reads or writes.
fn write_private(file: &Path, bytes: &[u8]) -> Result<(), Failure> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(file)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|e| Failure::new(format!("{}: {e}", file.display())))
}

/// How long one ask of curl may take, with `left` of the read's time to
/// go: each answer at most a minute and the retries no longer than what's
/// left, in whole seconds. None once less than a second is left. curl stops
/// at the first answer that fails, so a host that hangs costs one
/// answer's time, not one per answer.
fn allowed(left: Duration) -> Option<(u64, u64)> {
    let left = left.as_secs();
    (left > 0).then(|| (left.min(ONE_ANSWER), left))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comparison_asks_for_no_commit_or_file_it_doesnt_read() {
        // 13 November: World's comparison since the last switch was 56 MB
        // of commits and files by 16:07, and took past a minute twice, so
        // the switch went without World. Its second page of one commit
        // says how the two stand, with no files, in 15 KB.
        assert_eq!(
            compare("ba8d", "8dbb"),
            "compare/ba8d...8dbb?per_page=1&page=2",
            "a comparison asked for every commit and file between"
        );
    }

    #[test]
    fn an_ask_takes_no_longer_than_the_read_has_left() {
        assert_eq!(allowed(Duration::from_secs(120)), Some((60, 120)));
        assert_eq!(allowed(Duration::from_secs(30)), Some((30, 30)));
        assert_eq!(allowed(Duration::from_millis(500)), None);
        assert_eq!(allowed(Duration::ZERO), None);
    }

    #[test]
    fn base64_reads_what_the_host_writes_with_its_line_breaks() {
        assert_eq!(base64("aGVs\nbG8=\n").as_deref(), Some(&b"hello"[..]));
        assert_eq!(base64("").as_deref(), Some(&b""[..]));
        assert_eq!(base64("AP8=").as_deref(), Some(&[0u8, 255][..]));
        assert_eq!(base64("a*b="), None);
    }

    #[test]
    fn a_trees_mode_is_the_one_git_stores() {
        let entries = [Entry {
            name: "a".to_owned(),
            mode: "040000".to_owned(),
            kind: "tree".to_owned(),
            id: "00".repeat(20),
        }];
        let bytes = tree_bytes(&entries).unwrap();
        assert_eq!(&bytes[..8], b"40000 a\0");
        assert_eq!(bytes.len(), 8 + 20);
    }

    #[test]
    fn a_held_trees_entries_are_read_as_git_lists_them() {
        let blob = "22".repeat(20);
        let tree = "33".repeat(20);
        let held = |entries: &[(&str, &str, &str)]| {
            let mut object = Vec::new();
            for (mode, name, id) in entries {
                object.extend_from_slice(format!("{mode} {name}\0").as_bytes());
                object.extend(hex(id).unwrap());
            }
            object
        };
        // Bytes below 0x10 too, which are written with their leading zero.
        let module = "0a".repeat(20);
        let padded = "00".repeat(20);
        let object = held(&[
            ("100644", "README.md", &blob),
            ("40000", "src", &tree),
            ("160000", "vendor", &module),
            ("040000", "zz", &padded),
        ]);
        let entries = held_entries(&object, &"11".repeat(20)).unwrap();
        let read: Vec<_> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.kind.as_str(), e.id.as_str()))
            .collect();
        assert_eq!(
            read,
            [
                ("README.md", "blob", blob.as_str()),
                ("src", "tree", tree.as_str()),
                ("vendor", "commit", module.as_str()),
                ("zz", "tree", padded.as_str()),
            ],
            "a held tree's entries were misread, so what it holds goes unchecked"
        );
        for (mode, name) in [
            ("100644", ".."),
            ("100644", "."),
            ("", "file"),
            ("1x0", "file"),
            ("100694", "file"),
        ] {
            assert!(
                held_entries(&held(&[(mode, name, &blob)]), &tree).is_err(),
                "an entry with mode {mode:?} and name {name:?} was read"
            );
        }
        let mut short = held(&[("100644", "file", &blob)]);
        short.pop();
        assert!(
            held_entries(&short, &tree).is_err(),
            "an entry whose id was cut short was read"
        );
    }

    #[test]
    fn git_s_batch_gives_each_tree_asked_for_in_turn() {
        let one = "11".repeat(20);
        let two = "22".repeat(20);
        let ids = [one.clone(), two.clone()];
        let said = format!("{one} tree 3\nabc\n{two} tree 0\n\n");
        let objects = held_objects(said.as_bytes(), &ids).unwrap();
        assert_eq!(
            objects,
            [b"abc".as_slice(), b"".as_slice()],
            "git's batch was misread, so a tree's entries came from another"
        );
        for said in [
            format!("{one} commit 3\nabc\n{two} tree 0\n\n"),
            format!("{two} tree 3\nabc\n{one} tree 0\n\n"),
            format!("{one} missing\n{two} tree 0\n\n"),
            format!("{one} tree 9\nabc\n"),
        ] {
            assert!(
                held_objects(said.as_bytes(), &ids).is_err(),
                "git's batch {said:?} was taken as the trees asked for"
            );
        }
    }

    #[test]
    fn a_listing_naming_a_path_outside_its_tree_is_refused() {
        let id = "11".repeat(20);
        let listing = |name: &str| {
            serde_json::json!({"sha": id, "truncated": false, "tree": [
                {"path": name, "mode": "100644", "type": "blob", "sha": "22".repeat(20)}
            ]})
        };
        assert!(
            entries(&listing("file"), &id).is_ok(),
            "a plain file's name was refused"
        );
        for name in ["../x", "a/b", "..", ""] {
            assert!(entries(&listing(name), &id).is_err(), "{name}");
        }
        assert!(
            entries(&listing("file"), &"33".repeat(20)).is_err(),
            "a listing for another tree's id was read"
        );
    }
}
