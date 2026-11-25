//! git's objects: loose files, packs found through their indexes or one
//! multi-pack-index, and the stores an `alternates` file names. Nothing is
//! read whole: an index is searched where it lies on disk, a few bytes at a
//! time, so a repository of hundreds of millions of objects costs what one
//! of a hundred does. Finding an object in a pack's index is the work of
//! `pack_index`; reading it, loose or from a pack through deltas, is here.

use crate::hash::Oid;
use crate::pack_index::{Index, Midx};
use crate::stamp::{Stamp, Stamps};
use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl Kind {
    fn from_pack(code: u8) -> Option<Kind> {
        match code {
            1 => Some(Kind::Commit),
            2 => Some(Kind::Tree),
            3 => Some(Kind::Blob),
            4 => Some(Kind::Tag),
            _ => None,
        }
    }

    fn from_name(name: &[u8]) -> Option<Kind> {
        match name {
            b"commit" => Some(Kind::Commit),
            b"tree" => Some(Kind::Tree),
            b"blob" => Some(Kind::Blob),
            b"tag" => Some(Kind::Tag),
            _ => None,
        }
    }
}

pub struct Object {
    pub kind: Kind,
    pub data: Vec<u8>,
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

struct Pack {
    data: File,
    /// Its own index, for a pack the multi-pack-index doesn't cover.
    index: Option<Index>,
}

/// One object store: its packs and the folder of loose objects.
struct Store {
    dir: PathBuf,
    midx: Option<Midx>,
    /// Packs by their `.idx` name; the multi-pack-index's come first, in its
    /// order.
    packs: Vec<Pack>,
    midx_packs: usize,
    /// The own indexes of the packs the multi-pack-index covers, in its
    /// order, read only once it can't answer a lookup.
    covered: OnceLock<Vec<Option<Index>>>,
}

impl Store {
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    fn open(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let midx = Midx::open(&pack_dir.join("multi-pack-index")).ok();
        let mut packs = Vec::new();
        let mut covered = std::collections::HashSet::new();
        let mut midx_packs = 0;
        if let Some(midx) = &midx {
            for name in &midx.packs {
                let data = pack_dir.join(name.replace(".idx", ".pack"));
                // A pack the index names but that's gone ends the index's use.
                match File::open(&data) {
                    Ok(data) => packs.push(Pack { data, index: None }),
                    Err(_) => {
                        return Store::without_midx(dir);
                    }
                }
                covered.insert(name.clone());
            }
            midx_packs = packs.len();
        }
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx") && !covered.contains(name))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        for name in names {
            let index = Index::open(&pack_dir.join(&name));
            let data = File::open(pack_dir.join(name.replace(".idx", ".pack")));
            if let (Ok(index), Ok(data)) = (index, data) {
                packs.push(Pack {
                    data,
                    index: Some(index),
                });
            }
        }
        Store {
            dir: dir.to_path_buf(),
            midx,
            packs,
            midx_packs,
            covered: OnceLock::new(),
        }
    }

    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "git names a pack's index .idx, in lowercase"
    )]
    fn without_midx(dir: &Path) -> Store {
        let pack_dir = dir.join("pack");
        let mut names: Vec<String> = std::fs::read_dir(&pack_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".idx"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        let packs = names
            .into_iter()
            .filter_map(|name| {
                let index = Index::open(&pack_dir.join(&name)).ok()?;
                let data = File::open(pack_dir.join(name.replace(".idx", ".pack"))).ok()?;
                Some(Pack {
                    data,
                    index: Some(index),
                })
            })
            .collect();
        Store {
            dir: dir.to_path_buf(),
            midx: None,
            packs,
            midx_packs: 0,
            covered: OnceLock::new(),
        }
    }

    fn locate(&self, oid: &Oid) -> io::Result<Option<(usize, u64)>> {
        // An entry the multi-pack-index refuses, or one naming a pack past
        // those it covers, can't say where the object is. Its packs' own
        // indexes are read instead, and its error stands only when they
        // and the other packs hold the object nowhere, so a broken entry
        // neither hides an object nor passes for a missing one.
        let mut refused = None;
        if let Some(midx) = &self.midx {
            match midx.locate(oid) {
                Ok(Some((pack, offset))) if pack < self.midx_packs => {
                    return Ok(Some((pack, offset)));
                }
                Ok(None) => {}
                Ok(Some(_)) => {
                    refused = Some(invalid("multi-pack-index pack past its count"));
                }
                Err(error) => refused = Some(error),
            }
        }
        if refused.is_some()
            && let Some(found) = self.in_covered(oid)?
        {
            return Ok(Some(found));
        }
        for (at, pack) in self.packs.iter().enumerate().skip(self.midx_packs) {
            if let Some(index) = &pack.index
                && let Some(offset) = index.offset(oid)?
            {
                return Ok(Some((at, offset)));
            }
        }
        refused.map_or(Ok(None), Err)
    }

    /// Where `oid` lies by the own indexes of the packs the
    /// multi-pack-index covers, a pack whose index won't open passed over.
    fn in_covered(&self, oid: &Oid) -> io::Result<Option<(usize, u64)>> {
        let indexes = self.covered.get_or_init(|| {
            let pack_dir = self.dir.join("pack");
            self.midx
                .iter()
                .flat_map(|midx| &midx.packs)
                .map(|name| Index::open(&pack_dir.join(name)).ok())
                .collect()
        });
        for (at, index) in indexes.iter().enumerate() {
            if let Some(index) = index
                && let Some(offset) = index.offset(oid)?
            {
                return Ok(Some((at, offset)));
            }
        }
        Ok(None)
    }

    fn loose(&self, oid: &Oid) -> io::Result<Option<Object>> {
        let hex = oid.hex();
        let path = self.dir.join(&hex[..2]).join(&hex[2..]);
        let raw = match std::fs::read(path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let data = miniz_oxide::inflate::decompress_to_vec_zlib(&raw)
            .map_err(|_| invalid("loose object"))?;
        let nul = data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("loose header"))?;
        let space = data[..nul]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| invalid("loose header"))?;
        let kind = Kind::from_name(&data[..space]).ok_or_else(|| invalid("loose kind"))?;
        Ok(Some(Object {
            kind,
            data: data[nul + 1..].to_vec(),
        }))
    }
}

/// Inflates the zlib stream at `offset`, which inflates to `size` bytes.
fn inflate_at(file: &File, mut offset: u64, size: usize) -> io::Result<Vec<u8>> {
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let mut out = vec![0; size];
    let mut written = 0;
    let mut chunk = vec![0; (size + 64).clamp(256, 1 << 16)];
    loop {
        let got = file.read_at(&mut chunk, offset)?;
        if got == 0 {
            return Err(invalid("pack ends inside an object"));
        }
        let mut input = &chunk[..got];
        while !input.is_empty() {
            let result = inflate(&mut state, input, &mut out[written..], MZFlush::None);
            written += result.bytes_written;
            input = &input[result.bytes_consumed..];
            offset += result.bytes_consumed as u64;
            match result.status {
                Ok(MZStatus::StreamEnd) if written == size => return Ok(out),
                Ok(MZStatus::StreamEnd) => return Err(invalid("pack object's size")),
                _ if written == size => return Ok(out),
                Ok(_) | Err(MZError::Buf) => {}
                Err(_) => return Err(invalid("pack object's stream")),
            }
            if result.bytes_consumed == 0 && result.bytes_written == 0 {
                break;
            }
        }
    }
}

/// Applies a delta to its base.
pub fn apply_delta(base: &[u8], delta: &[u8]) -> io::Result<Vec<u8>> {
    let mut at = 0;
    let mut size = || -> io::Result<usize> {
        let mut value = 0usize;
        let mut shift = 0;
        loop {
            let byte = *delta.get(at).ok_or_else(|| invalid("delta size"))?;
            at += 1;
            value |= usize::from(byte & 0x7f) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
    };
    let base_size = size()?;
    let out_size = size()?;
    if base_size != base.len() {
        return Err(invalid("delta's base size"));
    }
    let mut out = Vec::with_capacity(out_size);
    while at < delta.len() {
        let cmd = delta[at];
        at += 1;
        if cmd & 0x80 != 0 {
            let mut offset = 0usize;
            let mut len = 0usize;
            for bit in 0..4 {
                if cmd & (1 << bit) != 0 {
                    offset |= usize::from(*delta.get(at).ok_or_else(|| invalid("delta copy"))?)
                        << (8 * bit);
                    at += 1;
                }
            }
            for bit in 0..3 {
                if cmd & (0x10 << bit) != 0 {
                    len |= usize::from(*delta.get(at).ok_or_else(|| invalid("delta copy"))?)
                        << (8 * bit);
                    at += 1;
                }
            }
            if len == 0 {
                len = 0x10000;
            }
            let piece = base
                .get(offset..offset + len)
                .ok_or_else(|| invalid("delta copy range"))?;
            out.extend_from_slice(piece);
        } else if cmd != 0 {
            let len = usize::from(cmd);
            let piece = delta
                .get(at..at + len)
                .ok_or_else(|| invalid("delta insert"))?;
            out.extend_from_slice(piece);
            at += len;
        } else {
            return Err(invalid("delta opcode"));
        }
    }
    if out.len() != out_size {
        return Err(invalid("delta's result size"));
    }
    Ok(out)
}

/// Every store a repository reads, its own first.
pub struct Odb {
    stores: Vec<Store>,
    pub stamps: Stamps,
    bases: HashMap<Place, Arc<(Kind, Vec<u8>)>>,
    /// Trees already read, which the staged comparison asks for again on
    /// every question while a folder differs from HEAD, and how many bytes
    /// they hold.
    trees: HashMap<Oid, Vec<u8>>,
    tree_bytes: usize,
}

/// How many bytes of trees are kept before starting again.
const TREE_BUDGET: usize = 32 << 20;

/// Where an object sits in a pack: the store, the pack and its offset.
type Place = (usize, usize, u64);

impl Odb {
    pub fn open(objects: &Path) -> Odb {
        let mut dirs = vec![objects.to_path_buf()];
        let mut at = 0;
        while at < dirs.len() && dirs.len() < 16 {
            let alternates = dirs[at].join("info/alternates");
            if let Ok(text) = std::fs::read_to_string(&alternates) {
                for line in text
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                {
                    let dir = dirs[at].join(line);
                    if !dirs.contains(&dir) {
                        dirs.push(dir);
                    }
                }
            }
            at += 1;
        }
        let mut stamps = Vec::new();
        for dir in &dirs {
            for watched in [
                dir.join("pack"),
                dir.join("pack/multi-pack-index"),
                dir.join("info/alternates"),
            ] {
                stamps.push((watched.clone(), Stamp::of(&watched)));
            }
        }
        Odb {
            stores: dirs.iter().map(|dir| Store::open(dir)).collect(),
            stamps,
            bases: HashMap::new(),
            trees: HashMap::new(),
            tree_bytes: 0,
        }
    }

    /// The object, or None when no store here has it (a partial clone's
    /// promised object, say).
    pub fn read(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        if let Some(data) = self.trees.get(oid) {
            return Ok(Some(Object {
                kind: Kind::Tree,
                data: data.clone(),
            }));
        }
        let object = self.find(oid)?;
        if let Some(object) = &object
            && object.kind == Kind::Tree
        {
            if self.tree_bytes + object.data.len() > TREE_BUDGET {
                self.trees.clear();
                self.tree_bytes = 0;
            }
            self.tree_bytes += object.data.len();
            self.trees.insert(*oid, object.data.clone());
        }
        Ok(object)
    }

    fn find(&mut self, oid: &Oid) -> io::Result<Option<Object>> {
        for store in 0..self.stores.len() {
            if let Some((pack, offset)) = self.stores[store].locate(oid)? {
                let (kind, data) = self.unpack(store, pack, offset, 0)?;
                return Ok(Some(Object { kind, data }));
            }
        }
        for store in &self.stores {
            if let Some(object) = store.loose(oid)? {
                return Ok(Some(object));
            }
        }
        Ok(None)
    }

    fn unpack(
        &mut self,
        store: usize,
        pack: usize,
        offset: u64,
        depth: usize,
    ) -> io::Result<(Kind, Vec<u8>)> {
        if depth > 10_000 {
            return Err(invalid("delta chain too long"));
        }
        if let Some(base) = self.bases.get(&(store, pack, offset)) {
            return Ok((base.0, base.1.clone()));
        }
        let file = &self.stores[store].packs[pack].data;
        let head = {
            let mut buf = [0; 32];
            let got = file.read_at(&mut buf, offset)?;
            buf[..got].to_vec()
        };
        let mut at = 0;
        let byte = *head.first().ok_or_else(|| invalid("pack entry"))?;
        let code = (byte >> 4) & 7;
        let mut size = usize::from(byte & 15);
        let mut shift = 4;
        let mut more = byte & 0x80 != 0;
        at += 1;
        while more {
            let byte = *head.get(at).ok_or_else(|| invalid("pack entry size"))?;
            at += 1;
            size |= usize::from(byte & 0x7f) << shift;
            shift += 7;
            more = byte & 0x80 != 0;
        }
        let result = match code {
            6 => {
                let mut byte = *head.get(at).ok_or_else(|| invalid("delta offset"))?;
                at += 1;
                let mut back = u64::from(byte & 0x7f);
                while byte & 0x80 != 0 {
                    byte = *head.get(at).ok_or_else(|| invalid("delta offset"))?;
                    at += 1;
                    back = ((back + 1) << 7) | u64::from(byte & 0x7f);
                }
                let base_at = offset
                    .checked_sub(back)
                    .ok_or_else(|| invalid("delta base offset"))?;
                let delta = inflate_at(file, offset + at as u64, size)?;
                let (kind, base) = self.unpack(store, pack, base_at, depth + 1)?;
                (kind, apply_delta(&base, &delta)?)
            }
            7 => {
                let base_oid =
                    Oid::from_slice(head.get(at..at + 20).ok_or_else(|| invalid("delta base"))?)
                        .ok_or_else(|| invalid("delta base"))?;
                let delta = inflate_at(file, offset + at as u64 + 20, size)?;
                let base = self
                    .read(&base_oid)?
                    .ok_or_else(|| invalid("delta base missing"))?;
                (base.kind, apply_delta(&base.data, &delta)?)
            }
            code => {
                let kind = Kind::from_pack(code).ok_or_else(|| invalid("pack entry kind"))?;
                (kind, inflate_at(file, offset + at as u64, size)?)
            }
        };
        // Trees and commits are what deltas build on most; keep a few.
        if depth > 0 && result.0 != Kind::Blob {
            if self.bases.len() > 4096 {
                self.bases.clear();
            }
            self.bases.insert(
                (store, pack, offset),
                Arc::new((result.0, result.1.clone())),
            );
        }
        Ok(result)
    }
}

/// A tree's entries, in its own order.
pub struct TreeEntry<'a> {
    pub mode: u32,
    pub name: &'a [u8],
    pub oid: Oid,
}

pub fn tree_entries(data: &[u8]) -> io::Result<Vec<TreeEntry<'_>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let space = data[at..]
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| invalid("tree entry"))?;
        let mode = std::str::from_utf8(&data[at..at + space])
            .ok()
            .and_then(|m| u32::from_str_radix(m, 8).ok())
            .ok_or_else(|| invalid("tree mode"))?;
        at += space + 1;
        let nul = data[at..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| invalid("tree name"))?;
        let name = &data[at..at + nul];
        at += nul + 1;
        let oid = Oid::from_slice(data.get(at..at + 20).ok_or_else(|| invalid("tree oid"))?)
            .ok_or_else(|| invalid("tree oid"))?;
        at += 20;
        out.push(TreeEntry { mode, name, oid });
    }
    Ok(out)
}

/// What the engine reads from a commit.
pub struct Commit {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub time: i64,
    pub encoding: String,
    pub summary: String,
}

pub fn parse_commit(data: &[u8]) -> io::Result<Commit> {
    let mut tree = None;
    let mut parents = Vec::new();
    let mut time = 0;
    let mut encoding = String::new();
    let mut lines = data.split(|&b| b == b'\n');
    let mut at = 0;
    for line in lines.by_ref() {
        at += line.len() + 1;
        if line.is_empty() {
            break;
        }
        if let Some(hex) = line.strip_prefix(b"tree ") {
            tree = Oid::from_hex(hex);
        } else if let Some(hex) = line.strip_prefix(b"parent ") {
            parents.push(Oid::from_hex(hex).ok_or_else(|| invalid("commit parent"))?);
        } else if let Some(who) = line.strip_prefix(b"committer ") {
            // "Name <mail> 1700000000 +0000"
            let close = who.iter().rposition(|&b| b == b'>').unwrap_or(0);
            time = std::str::from_utf8(&who[close + 1..])
                .ok()
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|t| t.parse().ok())
                .unwrap_or(0);
        } else if let Some(name) = line.strip_prefix(b"encoding ") {
            encoding = String::from_utf8_lossy(name).into_owned();
        }
    }
    let message = data.get(at.min(data.len())..).unwrap_or_default();
    // libgit2's summary: the first paragraph, its lines joined by spaces.
    let text = String::from_utf8_lossy(message);
    let summary = text
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Commit {
        tree: tree.ok_or_else(|| invalid("commit without a tree"))?,
        parents,
        time,
        encoding,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas_copy_and_insert() {
        let base = b"hello, world";
        // base 12, result 12: copy 7 from 0 ("hello, "), insert "there".
        let delta = [12, 12, 0x90, 7, 5, b't', b'h', b'e', b'r', b'e'];
        assert_eq!(apply_delta(base, &delta).unwrap(), b"hello, there");
        assert!(apply_delta(b"short", &delta).is_err());
    }

    #[test]
    fn a_multi_pack_index_refused_leaves_each_pack_found_through_its_own_index() {
        use crate::pack_index::tests::{midx, moved, offsets, sorted, unique, version_2};
        let objects = unique("objects");
        let pack = objects.join("pack");
        std::fs::create_dir_all(&pack).expect("the temp folder takes a folder");
        let names = sorted();
        // Its object names are said to run into its checksum.
        let whole = midx(&["pack-a.idx"], 1, None);
        let broken = moved(whole.clone(), 2, whole.len() as u64 - 21);
        for (name, bytes) in [
            ("multi-pack-index", broken),
            ("pack-a.idx", version_2(&names)),
            ("pack-a.pack", b"PACK".to_vec()),
        ] {
            std::fs::write(pack.join(name), bytes).expect("the temp folder takes a file");
        }
        let store = Store::open(&objects);
        assert!(
            store.midx.is_none(),
            "a multi-pack-index past its end was used"
        );
        assert_eq!(
            store
                .locate(&Oid(names[0]))
                .expect("the pack's own index reads"),
            Some((0, offsets(&names)[0])),
            "the pack's own index wasn't read in the refused index's place"
        );
    }

    /// A store over `midx`, its one pack's own index holding `own`.
    fn store_over(midx: Vec<u8>, own: &[[u8; 20]]) -> Store {
        use crate::pack_index::tests::{unique, version_2};
        let objects = unique("objects");
        let pack = objects.join("pack");
        std::fs::create_dir_all(&pack).expect("the temp folder takes a folder");
        for (name, bytes) in [
            ("multi-pack-index", midx),
            ("pack-a.idx", version_2(own)),
            ("pack-a.pack", b"PACK".to_vec()),
        ] {
            std::fs::write(pack.join(name), bytes).expect("the temp folder takes a file");
        }
        Store::open(&objects)
    }

    #[test]
    fn a_lookup_the_multi_pack_index_cant_answer_is_read_from_its_packs_own_indexes() {
        use crate::pack_index::tests::{midx, moved, offsets, sorted};
        let names = sorted();
        let last = names.len() - 1;
        let whole = midx(&["pack-a.idx"], 1, None);
        // Its large offsets moved to the last 8 bytes before its checksum
        // hold one, so the last name's, its second, lies past its chunk.
        let short = moved(whole.clone(), 4, whole.len() as u64 - 28);
        let store = store_over(short.clone(), &names);
        assert!(store.midx.is_some(), "a short LOFF was refused at open");
        assert_eq!(
            store
                .locate(&Oid(names[last]))
                .expect("the pack's own index reads"),
            Some((0, offsets(&names)[last])),
            "a large offset past its chunk wasn't read from the pack's own index"
        );
        // With the pack's own index not holding it either, the refusal
        // stands rather than passing for a missing object.
        let store = store_over(short, &names[..last]);
        assert_eq!(
            store
                .locate(&Oid(names[last]))
                .err()
                .map(|e| e.to_string())
                .as_deref(),
            Some("multi-pack-index large offset past its chunk"),
            "a broken entry no index answers passed for a missing object"
        );
        // Its names alternate between two packs, but it counts one, so
        // every other name is said to lie in a pack it doesn't name.
        let store = store_over(midx(&["pack-a.idx", "pack-b.idx"], 1, None), &names);
        assert_eq!(store.midx_packs, 1, "the index's count wasn't kept");
        assert_eq!(
            store
                .locate(&Oid(names[1]))
                .expect("the pack's own index reads"),
            Some((0, offsets(&names)[1])),
            "a name in a pack past the index's count wasn't read from its packs' indexes"
        );
    }

    #[test]
    fn commits_read_their_parts() {
        let tree = Oid::of("tree", b"");
        let text = format!(
            "tree {}\nparent {}\nauthor A <a@b> 1 +0000\ncommitter C <c@d> 1700000000 -0500\n\
             encoding ISO-8859-1\n\n\nSummary line\n\nBody\n",
            tree.hex(),
            tree.hex()
        );
        let commit = parse_commit(text.as_bytes()).unwrap();
        assert_eq!(commit.tree, tree);
        assert_eq!(commit.parents, vec![tree]);
        assert_eq!(commit.time, 1_700_000_000);
        assert_eq!(commit.encoding, "ISO-8859-1");
        assert_eq!(commit.summary, "Summary line");
    }
}
